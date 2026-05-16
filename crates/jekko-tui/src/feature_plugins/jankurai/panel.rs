//! `JankuraiPanel` Ratatui widget + per-row rendering.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use super::delta::{compute_delta, format_delta, DeltaMetric};
use super::snapshot::JankuraiSnapshot;
use super::sparkline::sparkline;
use super::style::{
    CYAN, EM_DASH_GLYPH, GOLD, HYPHEN_GLYPH, MUTED, QUESTION_GLYPH, SPARK_WIDTH, TEXT,
};

/// Jankurai audit-live dashboard panel.
#[derive(Clone, Debug)]
pub struct JankuraiPanel {
    snapshot: JankuraiSnapshot,
    exit_requested: bool,
}

impl JankuraiPanel {
    /// Build a panel with a given snapshot.
    pub fn new(snapshot: JankuraiSnapshot) -> Self {
        Self {
            snapshot,
            exit_requested: false,
        }
    }

    /// Replace the snapshot.
    pub fn set_snapshot(&mut self, snapshot: JankuraiSnapshot) {
        self.snapshot = snapshot;
    }

    /// Access the snapshot.
    pub fn snapshot(&self) -> &JankuraiSnapshot {
        &self.snapshot
    }

    /// True when the user pressed `Esc` or `q`.
    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    /// Clear the exit flag.
    pub fn clear_exit(&mut self) {
        self.exit_requested = false;
    }

    /// Dispatch a key event. Returns `true` when consumed.
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

impl Widget for &JankuraiPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(0x21, 0x26, 0x30)))
            .title(Span::styled(
                " Jankurai — Audit Live ",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        block.render(area, buf);

        if !self.snapshot.jankurai_installed {
            let lines = vec![
                Line::from(Span::styled(
                    "Jankurai not installed.",
                    Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::raw("")),
                Line::from(Span::styled("Get it:", Style::default().fg(MUTED))),
                Line::from(Span::styled(
                    super::detect::JANKURAI_INSTALL_URL,
                    Style::default().fg(CYAN),
                )),
                Line::from(Span::raw("")),
                Line::from(Span::styled(
                    "Then run /audit to start.",
                    Style::default().fg(MUTED),
                )),
            ];
            Paragraph::new(lines).render(inner, buf);
            return;
        }

        if self.snapshot.score.is_none() {
            Paragraph::new(Line::from(Span::styled(
                "No audit yet. Run /audit to start.",
                Style::default().fg(MUTED),
            )))
            .render(inner, buf);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // score row
                Constraint::Length(1), // decision / age
                Constraint::Length(6), // body (counts + deltas)
                Constraint::Min(2),    // workers
            ])
            .split(inner);
        render_score_row(self, chunks[0], buf);
        render_meta_row(self, chunks[1], buf);
        render_body(self, chunks[2], buf);
        render_workers(self, chunks[3], buf);
    }
}

fn render_score_row(panel: &JankuraiPanel, area: Rect, buf: &mut Buffer) {
    let snap = &panel.snapshot;
    let score_delta = compute_delta(snap.score, snap.baseline_score, DeltaMetric::Score);
    let spark = sparkline(&snap.history, SPARK_WIDTH);
    let line = Line::from(vec![
        Span::styled(
            format!(
                "Score {}",
                match snap.score {
                    Some(s) => format!("{:.1}", s),
                    None => HYPHEN_GLYPH.to_string(),
                }
            ),
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(spark, Style::default().fg(GOLD)),
        Span::raw("  "),
        Span::styled("vs main: ", Style::default().fg(MUTED)),
        Span::styled(
            format_delta(&score_delta),
            Style::default().fg(score_delta.direction.color()),
        ),
    ]);
    Paragraph::new(line).render(area, buf);
}

fn render_meta_row(panel: &JankuraiPanel, area: Rect, buf: &mut Buffer) {
    let snap = &panel.snapshot;
    let age = match snap.last_run_age.clone() {
        Some(s) => s,
        None => EM_DASH_GLYPH.to_string(),
    };
    let decision = match snap.decision.clone() {
        Some(s) => s,
        None => EM_DASH_GLYPH.to_string(),
    };
    let auditor_version = match snap.auditor_version.clone() {
        Some(s) => s,
        None => QUESTION_GLYPH.to_string(),
    };
    let line = Line::from(vec![
        Span::styled("Audit · ", Style::default().fg(MUTED)),
        Span::styled(age, Style::default().fg(TEXT)),
        Span::styled(" · ", Style::default().fg(MUTED)),
        Span::styled(decision, Style::default().fg(TEXT)),
        Span::styled(" · v", Style::default().fg(MUTED)),
        Span::styled(auditor_version, Style::default().fg(TEXT)),
    ]);
    Paragraph::new(line).render(area, buf);
}

fn render_body(panel: &JankuraiPanel, area: Rect, buf: &mut Buffer) {
    let snap = &panel.snapshot;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(0)])
        .split(area);

    let lvl = match snap.conformance_level.clone() {
        Some(s) => s,
        None => EM_DASH_GLYPH.to_string(),
    };
    let counts = vec![
        Line::from(Span::styled(
            format!("Caps  {}", fmt_maybe(snap.caps_applied)),
            Style::default().fg(TEXT),
        )),
        Line::from(Span::styled(
            format!("Hard  {}", fmt_maybe(snap.hard_findings)),
            Style::default().fg(TEXT),
        )),
        Line::from(Span::styled(
            format!("Soft  {}", fmt_maybe(snap.soft_findings)),
            Style::default().fg(TEXT),
        )),
        Line::from(Span::styled(
            format!("Level {}", lvl),
            Style::default().fg(TEXT),
        )),
    ];
    Paragraph::new(counts).render(cols[0], buf);

    let caps_d = compute_delta(snap.caps_applied, snap.baseline_caps, DeltaMetric::Caps);
    let hard_d = compute_delta(snap.hard_findings, snap.baseline_hard, DeltaMetric::Hard);
    let soft_d = compute_delta(snap.soft_findings, snap.baseline_soft, DeltaMetric::Soft);
    let score_d = compute_delta(snap.score, snap.baseline_score, DeltaMetric::Score);

    let deltas = vec![
        Line::from(Span::styled(
            format!("Δ caps  {}", format_delta(&caps_d)),
            Style::default().fg(caps_d.direction.color()),
        )),
        Line::from(Span::styled(
            format!("Δ hard  {}", format_delta(&hard_d)),
            Style::default().fg(hard_d.direction.color()),
        )),
        Line::from(Span::styled(
            format!("Δ soft  {}", format_delta(&soft_d)),
            Style::default().fg(soft_d.direction.color()),
        )),
        Line::from(Span::styled(
            format!("Δ score {}", format_delta(&score_d)),
            Style::default().fg(score_d.direction.color()),
        )),
    ];
    Paragraph::new(deltas).render(cols[1], buf);
}

fn render_workers(panel: &JankuraiPanel, area: Rect, buf: &mut Buffer) {
    let mut lines = vec![Line::from(Span::styled(
        "Workers",
        Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
    ))];
    if panel.snapshot.workers.is_empty() {
        lines.push(Line::from(Span::styled(
            "(no live workers)",
            Style::default().fg(MUTED),
        )));
    } else {
        for w in &panel.snapshot.workers {
            lines.push(Line::from(vec![
                Span::styled("▣ ", Style::default().fg(MUTED)),
                Span::styled(w.id.clone(), Style::default().fg(TEXT)),
                Span::styled(" · ", Style::default().fg(MUTED)),
                Span::styled(w.kind.clone(), Style::default().fg(MUTED)),
            ]));
        }
    }
    Paragraph::new(lines).render(area, buf);
}

fn fmt_maybe(v: Option<f64>) -> String {
    match v {
        Some(x) => {
            if (x - x.round()).abs() < 1e-9 {
                format!("{}", x.round() as i64)
            } else {
                format!("{:.1}", x)
            }
        }
        None => EM_DASH_GLYPH.to_string(),
    }
}
