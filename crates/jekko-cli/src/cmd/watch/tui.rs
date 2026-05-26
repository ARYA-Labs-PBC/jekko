//! Phase G2 Ratatui dashboard for `jekko watch --format tui`.
//!
//! Owns the terminal lifecycle (raw mode, alternate screen), the
//! crossterm/notify event loop, and all drawing helpers. The parent
//! `watch.rs` keeps the per-tick fold and plain/JSON emitters; this module
//! only handles the interactive surface plus its CI-friendly
//! `--tui-once-snapshot` variant.
//!
//! Quit on `q`, `Esc`, or `Ctrl-C`. Scroll the active-rules list with
//! `j`/`k`.

use std::io::{self, Stdout};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self as ct_event, Event as CtEvent, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use jankurai_runner::events::Event;
use jankurai_runner::watcher::{
    detect_and_remediate, fold_events, RemediationAction, WatcherSnapshot,
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::backend::{CrosstermBackend, TestBackend};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use super::{now_epoch_secs, read_from_offset, rule_label, TickState, WatchArgs};

/// All state the TUI needs to render a single frame.
#[derive(Clone, Debug)]
struct DashboardState {
    run_id: String,
    start_ts: Option<u64>,
    now_ts: u64,
    snap: WatcherSnapshot,
    actions: Vec<RemediationAction>,
    /// Visual offset into the active-rules list for j/k scrolling.
    rules_scroll: usize,
}

impl DashboardState {
    fn elapsed_label(&self) -> String {
        let start = self.start_ts.unwrap_or(self.now_ts);
        let secs = self.now_ts.saturating_sub(start);
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        format!("{h}:{m:02}:{s:02}")
    }
}

/// Refresh interval when no `notify` event arrives. 1s matches the spec
/// header. The crossterm poll loop wakes once per tick to give the user a
/// chance to press q/Esc/Ctrl-C.
const TUI_TICK: Duration = Duration::from_millis(1000);
/// Sub-tick used inside the keyboard-poll loop so q/Esc/Ctrl-C feel snappy
/// even when nothing else is happening.
const TUI_POLL: Duration = Duration::from_millis(100);

/// Entry point for `--format tui`. Handles the one-shot snapshot path and
/// the live event loop.
pub(super) fn run_tui(events_path: &Path, args: &WatchArgs) -> Result<()> {
    // Always drain whatever is already on disk so the very first frame is
    // representative.
    let mut offset: u64 = 0;
    let mut tick_state = TickState::default();
    let (initial_events, new_offset) = read_from_offset(events_path, offset)?;
    offset = new_offset;

    let mut dash = build_dashboard(&initial_events, &mut tick_state, args);

    if args.tui_once_snapshot {
        return render_once_snapshot(&dash);
    }

    if args.once || args.no_follow {
        // Render one frame to the real terminal, then exit. No event loop,
        // no notify watcher.
        let mut terminal = enter_terminal()?;
        let render_err = terminal
            .draw(|f| draw_dashboard(f, &dash))
            .err()
            .map(|e| anyhow::anyhow!(e));
        let _ = leave_terminal(&mut terminal);
        if let Some(err) = render_err {
            return Err(err.context("render initial tui frame"));
        }
        return Ok(());
    }

    // Live mode: set up notify on the parent dir, raw terminal, and a poll
    // loop that wakes on key events, file events, or the 1s tick.
    let watch_dir = events_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    if !watch_dir.exists() {
        std::fs::create_dir_all(&watch_dir)
            .with_context(|| format!("mkdir -p {}", watch_dir.display()))?;
    }
    let (tx, rx) = channel::<notify::Result<notify::Event>>();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })
    .context("create file watcher")?;
    watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .with_context(|| format!("watch {}", watch_dir.display()))?;

    let mut terminal = enter_terminal()?;
    let loop_result = (|| -> Result<()> {
        let mut last_draw = std::time::Instant::now();
        terminal.draw(|f| draw_dashboard(f, &dash))?;
        loop {
            // Keyboard poll — short timeout so quits feel responsive.
            if ct_event::poll(TUI_POLL).unwrap_or(false) {
                match ct_event::read() {
                    Ok(CtEvent::Key(key)) if key.kind != KeyEventKind::Release => {
                        if quit_requested(key.code, key.modifiers) {
                            break;
                        }
                        match key.code {
                            KeyCode::Char('j') => {
                                dash.rules_scroll = dash.rules_scroll.saturating_add(1);
                            }
                            KeyCode::Char('k') => {
                                dash.rules_scroll = dash.rules_scroll.saturating_sub(1);
                            }
                            _ => {}
                        }
                        terminal.draw(|f| draw_dashboard(f, &dash))?;
                    }
                    Ok(_) => {
                        terminal.draw(|f| draw_dashboard(f, &dash))?;
                    }
                    Err(_) => break,
                }
            }

            // Drain any file events without blocking. If anything fired we
            // re-read the events file and rebuild the dashboard state.
            let mut had_fs = false;
            while let Ok(_msg) = rx.try_recv() {
                had_fs = true;
            }
            if had_fs {
                let (new_events, new_offset) = read_from_offset(events_path, offset)?;
                offset = new_offset;
                dash = build_dashboard_continue(&new_events, &mut tick_state, args, &dash);
            }

            // Periodic redraw + stall-rule refresh once per tick.
            if last_draw.elapsed() >= TUI_TICK {
                dash = build_dashboard_continue(&[], &mut tick_state, args, &dash);
                terminal.draw(|f| draw_dashboard(f, &dash))?;
                last_draw = std::time::Instant::now();
            }

            if dash.snap.finished {
                // One last refresh to make sure the operator sees the final
                // numbers, then exit.
                terminal.draw(|f| draw_dashboard(f, &dash))?;
                break;
            }

            // Disconnected sender means the watcher thread died.
            if let Err(RecvTimeoutError::Disconnected) = rx.recv_timeout(Duration::from_millis(0)) {
                break;
            }
        }
        Ok(())
    })();
    leave_terminal(&mut terminal)?;
    loop_result
}

/// Build a fresh `DashboardState` from a brand-new event batch and the
/// initial (empty) `TickState`.
fn build_dashboard(
    new_events: &[Event],
    tick_state: &mut TickState,
    args: &WatchArgs,
) -> DashboardState {
    tick_state.all_events.extend(new_events.iter().cloned());
    let snap = fold_events(&tick_state.all_events);
    let now_ts = now_epoch_secs();
    let prior_gaps = if tick_state.prior_gaps_history.len() >= 3 {
        Some(tick_state.prior_gaps_history[tick_state.prior_gaps_history.len() - 3])
    } else {
        None
    };
    let actions = detect_and_remediate(
        &snap,
        now_ts,
        args.stall_threshold,
        args.error_rate_threshold,
        prior_gaps,
        tick_state.prior_hard_findings,
    );
    tick_state.prior_gaps_history.push(snap.parity_gaps_open);
    if let Some(hf) = snap.last_jankurai_hard_findings {
        tick_state.prior_hard_findings = Some(hf);
    }
    let start_ts = tick_state.all_events.first().map(|e| e.ts);
    DashboardState {
        run_id: args.run_id.clone(),
        start_ts,
        now_ts,
        snap,
        actions,
        rules_scroll: 0,
    }
}

/// Continue an existing dashboard: fold in any new events and re-run the
/// rules, preserving scroll position.
fn build_dashboard_continue(
    new_events: &[Event],
    tick_state: &mut TickState,
    args: &WatchArgs,
    prev: &DashboardState,
) -> DashboardState {
    let mut next = build_dashboard(new_events, tick_state, args);
    next.rules_scroll = prev.rules_scroll;
    next
}

/// Render one frame into a `TestBackend`, dump the rendered text to stdout,
/// and return. The output is plain (no ANSI), so tests can grep it.
fn render_once_snapshot(dash: &DashboardState) -> Result<()> {
    const W: u16 = 120;
    const H: u16 = 30;
    let backend = TestBackend::new(W, H);
    let mut terminal = Terminal::new(backend).context("init test backend")?;
    terminal
        .draw(|f| draw_dashboard(f, dash))
        .context("draw to test backend")?;
    // `Buffer::to_string` would lose line breaks; iterate row-by-row instead.
    let buf = terminal.backend().buffer().clone();
    let mut out = String::with_capacity((W as usize + 1) * H as usize);
    for y in 0..H {
        for x in 0..W {
            let cell = &buf[(x, y)];
            out.push_str(cell.symbol());
        }
        out.push('\n');
    }
    println!("{out}");
    Ok(())
}

/// Translate a keystroke into a quit decision. `q`, `Esc`, and `Ctrl-C`
/// all quit; anything else falls through.
fn quit_requested(code: KeyCode, mods: KeyModifiers) -> bool {
    matches!(code, KeyCode::Esc | KeyCode::Char('q'))
        || (code == KeyCode::Char('c') && mods.contains(KeyModifiers::CONTROL))
}

/// RAII-style terminal init: raw mode + alt screen.
fn enter_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().context("enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("enter alt screen")?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).context("init crossterm terminal")
}

/// Tear down the terminal regardless of why we're exiting.
fn leave_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
    Ok(())
}

/// Render the dashboard widgets into the frame. Pure with respect to
/// `dash` — callable from both the real terminal and `TestBackend`.
fn draw_dashboard(f: &mut Frame<'_>, dash: &DashboardState) {
    let area = f.area();
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            Span::raw(" ZYAL Watcher: "),
            Span::styled(
                dash.run_id.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("   elapsed "),
            Span::styled(
                dash.elapsed_label(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]));
    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Vertical layout: top stats row, rules list, jankurai row, hint.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // top stat panes
            Constraint::Min(3),    // active rules
            Constraint::Length(3), // jankurai row
            Constraint::Length(1), // key hint
        ])
        .split(inner);

    draw_stat_row(f, chunks[0], dash);
    draw_rules(f, chunks[1], dash);
    draw_jankurai(f, chunks[2], dash);
    draw_hint(f, chunks[3]);
}

fn draw_stat_row(f: &mut Frame<'_>, area: Rect, dash: &DashboardState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area);

    let lanes = vec![
        Line::from(format!("started:      {}", dash.snap.lanes_started)),
        Line::from(format!("finished:     {}", dash.snap.lanes_finished)),
        Line::from(format!("workers ok:   {}", dash.snap.workers_pass)),
        Line::from(format!("workers fail: {}", dash.snap.workers_fail)),
    ];
    let lanes_widget =
        Paragraph::new(lanes).block(Block::default().borders(Borders::ALL).title(" Lanes "));
    f.render_widget(lanes_widget, cols[0]);

    let parity = vec![
        Line::from(format!("open:    {}", dash.snap.parity_gaps_open)),
        Line::from(format!("closed:  {}", dash.snap.parity_gaps_closed)),
    ];
    let parity_widget =
        Paragraph::new(parity).block(Block::default().borders(Borders::ALL).title(" Parity "));
    f.render_widget(parity_widget, cols[1]);

    let err_pct = dash.snap.error_rate() * 100.0;
    let model = vec![
        Line::from(format!("attempts:    {}", dash.snap.model_attempts)),
        Line::from(format!("failures:    {}", dash.snap.model_failures)),
        Line::from(format!("error rate:  {err_pct:.1}%")),
        Line::from(format!("spend (usd): ${:.2}", dash.snap.model_spend_usd)),
    ];
    let model_widget =
        Paragraph::new(model).block(Block::default().borders(Borders::ALL).title(" Model "));
    f.render_widget(model_widget, cols[2]);
}

fn draw_rules(f: &mut Frame<'_>, area: Rect, dash: &DashboardState) {
    let items: Vec<ListItem> = if dash.actions.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "none firing",
            Style::default().add_modifier(Modifier::DIM),
        )))]
    } else {
        dash.actions
            .iter()
            .skip(dash.rules_scroll)
            .map(|a| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        rule_label(a.rule),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::raw(a.summary.clone()),
                ]))
            })
            .collect()
    };
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Active rules "),
    );
    f.render_widget(list, area);
}

fn draw_jankurai(f: &mut Frame<'_>, area: Rect, dash: &DashboardState) {
    let score = dash
        .snap
        .last_jankurai_score
        .map(|s| s.to_string())
        .unwrap_or_else(|| "-".into());
    let hard = dash
        .snap
        .last_jankurai_hard_findings
        .map(|h| h.to_string())
        .unwrap_or_else(|| "-".into());
    let line = Line::from(format!("score: {score}        hard_findings: {hard}"));
    let widget = Paragraph::new(line)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(" Jankurai "));
    f.render_widget(widget, area);
}

fn draw_hint(f: &mut Frame<'_>, area: Rect) {
    let widget = Paragraph::new(Line::from(Span::styled(
        " q quit  |  j/k scroll rules ",
        Style::default().add_modifier(Modifier::DIM),
    )));
    f.render_widget(widget, area);
}
