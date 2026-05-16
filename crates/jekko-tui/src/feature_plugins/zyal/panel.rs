//! `ZyalPanel` Ratatui widget + per-row rendering.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use super::palette::{
    fmt_n, DEFAULT_STATUS_LABEL, ERROR, GOLD, MUTED, NEON_CACHE, NEON_COST, NEON_FAILS,
    NEON_LATENCY, NEON_LOOPS, NEON_SEPARATOR, NEON_TOKENS_IN, NEON_TOKENS_OUT, NEON_TOKENS_TOTAL,
    NEON_UPTIME, NEON_WORKERS_ACTIVE, NEON_WORKERS_MAX, SUCCESS, TEXT, WARNING,
};
use super::snapshot::ZyalSnapshot;

/// ZYAL panel widget.
#[derive(Clone, Debug)]
pub struct ZyalPanel {
    snapshot: ZyalSnapshot,
    exit_requested: bool,
}

impl ZyalPanel {
    /// Construct a panel from a snapshot.
    pub fn new(snapshot: ZyalSnapshot) -> Self {
        Self {
            snapshot,
            exit_requested: false,
        }
    }

    /// Replace the snapshot.
    pub fn set_snapshot(&mut self, snapshot: ZyalSnapshot) {
        self.snapshot = snapshot;
    }

    /// Read-only snapshot access.
    pub fn snapshot(&self) -> &ZyalSnapshot {
        &self.snapshot
    }

    /// True after the user pressed `Esc` or `q`.
    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    /// Reset the exit flag.
    pub fn clear_exit(&mut self) {
        self.exit_requested = false;
    }

    /// Dispatch a key. Returns `true` when consumed.
    pub fn dispatch_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.exit_requested = true;
                true
            }
            _ => false,
        }
    }
}

impl Widget for &ZyalPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(NEON_SEPARATOR))
            .title(Span::styled(
                " ∞ ZYAL MODE ",
                Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        block.render(area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // exit / sigil row
                Constraint::Length(1), // status row
                Constraint::Length(6), // counters block
                Constraint::Length(4), // paste detector
                Constraint::Min(0),    // runbook preview
            ])
            .split(inner);

        render_exit(self, chunks[0], buf);
        render_status(self, chunks[1], buf);
        render_counters(self, chunks[2], buf);
        render_paste(self, chunks[3], buf);
        render_runbook(self, chunks[4], buf);
    }
}

fn render_exit(panel: &ZyalPanel, area: Rect, buf: &mut Buffer) {
    let lines: Vec<Line> = match &panel.snapshot.exit {
        Some(rec) => vec![
            Line::from(Span::styled(
                rec.tone.label(),
                Style::default()
                    .fg(rec.tone.color())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("● ", Style::default().fg(rec.tone.color())),
                Span::styled(rec.status.clone(), Style::default().fg(rec.tone.color())),
                Span::raw("  "),
                Span::styled(rec.reason.clone(), Style::default().fg(TEXT)),
            ]),
        ],
        None => vec![
            Line::from(vec![Span::styled(
                "✓ ZYAL",
                Style::default()
                    .fg(NEON_TOKENS_OUT)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(Span::styled(
                "research-loop executor",
                Style::default().fg(MUTED),
            )),
        ],
    };
    Paragraph::new(lines).render(area, buf);
}

fn render_status(panel: &ZyalPanel, area: Rect, buf: &mut Buffer) {
    let status = match panel.snapshot.status.clone() {
        Some(s) => s,
        None => DEFAULT_STATUS_LABEL.to_string(),
    };
    let color = match status.as_str() {
        "active" => SUCCESS,
        "paused" => WARNING,
        "error" | "failed" => ERROR,
        _ => MUTED,
    };
    let mut spans = vec![Span::styled("● ", Style::default().fg(color))];
    if let Some(id) = &panel.snapshot.run_id {
        spans.push(Span::styled("run ", Style::default().fg(MUTED)));
        spans.push(Span::styled(id.clone(), Style::default().fg(TEXT)));
        spans.push(Span::raw("  "));
    }
    spans.push(Span::styled(status, Style::default().fg(color)));
    Paragraph::new(Line::from(spans)).render(area, buf);
}

fn render_counters(panel: &ZyalPanel, area: Rect, buf: &mut Buffer) {
    let s = &panel.snapshot;
    let mut lines = Vec::new();

    let loops_text = if s.tasks_completed > 0 || s.tasks_incubated > 0 {
        format!(
            "{} ({}✓ {}🜨)",
            s.loops_completed, s.tasks_completed, s.tasks_incubated
        )
    } else {
        s.loops_completed.to_string()
    };
    lines.push(Line::from(vec![
        Span::styled("Loops   ", Style::default().fg(MUTED)),
        Span::styled(
            loops_text,
            Style::default().fg(NEON_LOOPS).add_modifier(Modifier::BOLD),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Tokens  ", Style::default().fg(MUTED)),
        Span::styled(
            fmt_n(s.total_tokens),
            Style::default()
                .fg(NEON_TOKENS_TOTAL)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("in ", Style::default().fg(MUTED)),
        Span::styled(fmt_n(s.input_tokens), Style::default().fg(NEON_TOKENS_IN)),
        Span::raw("  "),
        Span::styled("out ", Style::default().fg(MUTED)),
        Span::styled(fmt_n(s.output_tokens), Style::default().fg(NEON_TOKENS_OUT)),
        Span::raw("  "),
        Span::styled("cache ", Style::default().fg(MUTED)),
        Span::styled(fmt_n(s.cache_tokens), Style::default().fg(NEON_CACHE)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Workers ", Style::default().fg(MUTED)),
        Span::styled(
            format!("{}", s.workers_active),
            Style::default()
                .fg(NEON_WORKERS_ACTIVE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" / ", Style::default().fg(MUTED)),
        Span::styled(
            format!("{}", s.workers_max),
            Style::default().fg(NEON_WORKERS_MAX),
        ),
    ]));

    if let Some(up) = &s.uptime {
        lines.push(Line::from(vec![
            Span::styled("Uptime  ", Style::default().fg(MUTED)),
            Span::styled(
                up.clone(),
                Style::default()
                    .fg(NEON_UPTIME)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    if s.cost_usd > 0.0 {
        lines.push(Line::from(vec![
            Span::styled("Cost    ", Style::default().fg(MUTED)),
            Span::styled(
                format!("${:.2}", s.cost_usd),
                Style::default().fg(NEON_COST).add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    if let Some(j) = s.jankurai_findings {
        lines.push(Line::from(vec![
            Span::styled("Jankurai ", Style::default().fg(MUTED)),
            Span::styled(
                format!("{}", j),
                Style::default().fg(NEON_FAILS).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" open", Style::default().fg(MUTED)),
        ]));
    }

    Paragraph::new(lines).render(area, buf);
}

fn render_paste(panel: &ZyalPanel, area: Rect, buf: &mut Buffer) {
    let s = &panel.snapshot;
    let mut lines = vec![Line::from(Span::styled(
        "─ paste detector ─",
        Style::default().fg(NEON_SEPARATOR),
    ))];
    if let Some(sig) = &s.paste_signature {
        lines.push(Line::from(vec![
            Span::styled("sig ", Style::default().fg(MUTED)),
            Span::styled(sig.clone(), Style::default().fg(NEON_TOKENS_TOTAL)),
            Span::raw("  "),
            Span::styled("bytes ", Style::default().fg(MUTED)),
            Span::styled(fmt_n(s.paste_bytes), Style::default().fg(NEON_LATENCY)),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            "no paste recorded",
            Style::default().fg(MUTED),
        )));
    }
    Paragraph::new(lines).render(area, buf);
}

fn render_runbook(panel: &ZyalPanel, area: Rect, buf: &mut Buffer) {
    let mut lines = vec![Line::from(Span::styled(
        "Runbook preview",
        Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
    ))];
    if panel.snapshot.runbook_preview.is_empty() {
        lines.push(Line::from(Span::styled(
            "(no runbook loaded)",
            Style::default().fg(MUTED),
        )));
    } else {
        for line in &panel.snapshot.runbook_preview {
            lines.push(Line::from(vec![
                Span::styled(format!("{:>3}. ", line.step), Style::default().fg(MUTED)),
                Span::styled(line.text.clone(), Style::default().fg(TEXT)),
            ]));
        }
    }
    Paragraph::new(lines).render(area, buf);
}
