//! Ratatui rendering for the Jnoccio inspector panel.
//!
//! Layout (inside panel_block border):
//!
//! ```text
//! ╭─ Fusion ─────────────────── Live ─╮
//! │ Models     78 / 79                 │
//! │ Agents      0 /  0                 │
//! │ Calls             0                │
//! │                                    │
//! │ [1] Board  [2] Speed  [3] Vault    │
//! │ [4] Limits [5] Feed   [6] Agents   │
//! │────────────────────────────────────│
//! │ (body content / empty state)       │
//! │                                    │
//! │ 1-6 tabs  j/k nav  Enter detail    │
//! ╰────────────────────────────────────╯
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use super::model::{JnoccioConnection, JnoccioTab, BG, GOLD, MUTED, RED, TEXT};
use super::panel::JnoccioPanel;
use crate::theme;

impl Widget for &JnoccioPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let conn_label = if self.connection == JnoccioConnection::Error {
            "disconnected"
        } else {
            self.connection.label()
        };
        let block = theme::panel_block("Fusion", Some(conn_label), false);
        let inner = block.inner(area);
        block.render(area, buf);

        // Background fill
        let bg_block = ratatui::widgets::Block::default()
            .style(Style::default().bg(BG));
        bg_block.render(inner, buf);

        if inner.height == 0 {
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // stats rows
                Constraint::Length(1), // blank separator
                Constraint::Length(2), // tab bar (2 rows × 3 tabs)
                Constraint::Min(0),    // body
                Constraint::Length(1), // footer hints
            ])
            .split(inner);

        render_stats(self, chunks[0], buf);
        // blank row: chunks[1] left empty
        render_tabs(self, chunks[2], buf);
        if self.help_open {
            render_help(chunks[3], buf);
        } else {
            render_body(self, chunks[3], buf);
        }
        render_footer(chunks[4], buf);
    }
}

fn render_stats(panel: &JnoccioPanel, area: Rect, buf: &mut Buffer) {
    let s = &panel.snapshot;
    let is_live = matches!(
        panel.connection,
        JnoccioConnection::Live | JnoccioConnection::Connecting
    );
    let val_color = if is_live { TEXT } else { RED };

    // Right-align the numbers within a fixed field. Use a key-label + right-padded value layout.
    let lines = vec![
        stat_line("Models", s.enabled_models, s.total_models, val_color),
        stat_line("Agents", s.agents, s.max_agents, val_color),
        count_line("Calls", s.calls, val_color),
    ];
    Paragraph::new(lines).render(area, buf);
}

fn stat_line(label: &str, a: u32, b: u32, val_color: ratatui::style::Color) -> Line<'static> {
    let key = format!("{label:<8}");
    let val = format!("{a:>3} / {b:<3}");
    Line::from(vec![
        Span::styled(key, Style::default().fg(MUTED)),
        Span::styled(val, Style::default().fg(val_color)),
    ])
}

fn count_line(label: &str, n: u64, val_color: ratatui::style::Color) -> Line<'static> {
    let key = format!("{label:<8}");
    let val = format!("{:>9}", fmt_n(n));
    Line::from(vec![
        Span::styled(key, Style::default().fg(MUTED)),
        Span::styled(val, Style::default().fg(val_color)),
    ])
}

fn render_tabs(panel: &JnoccioPanel, area: Rect, buf: &mut Buffer) {
    // 6 tabs split across 2 rows: [1][2][3] on row 1, [4][5][6] on row 2.
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    for (row_idx, tab_slice) in [&JnoccioTab::ALL[..3], &JnoccioTab::ALL[3..]].iter().enumerate() {
        let mut spans: Vec<Span> = Vec::new();
        for (i, tab) in tab_slice.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            let active = *tab == panel.tab;
            let key_style = if active {
                Style::default().fg(GOLD).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MUTED)
            };
            let label_style = if active {
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MUTED)
            };
            spans.push(Span::styled(format!("[{}]", tab.shortcut()), key_style));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(tab.label().to_string(), label_style));
        }
        if row_idx < rows.len() {
            Paragraph::new(Line::from(spans)).render(rows[row_idx], buf);
        }
    }
}

fn render_body(panel: &JnoccioPanel, area: Rect, buf: &mut Buffer) {
    // Disconnected: show warning above content.
    if panel.connection == JnoccioConnection::Error {
        let warn = Line::from(Span::styled(
            "⚠ No agent connected",
            Style::default().fg(RED).add_modifier(Modifier::BOLD),
        ));
        Paragraph::new(warn).render(area, buf);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // Sort mode for Board tab.
    if panel.tab == JnoccioTab::Board {
        lines.push(Line::from(vec![
            Span::styled("sort: ", Style::default().fg(MUTED)),
            Span::styled(panel.sort_label().to_string(), Style::default().fg(TEXT)),
        ]));
    }

    // Active search filter.
    if panel.search_active || !panel.search_query.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("filter: ", Style::default().fg(MUTED)),
            Span::styled(panel.search_query.clone(), Style::default().fg(GOLD)),
            Span::styled(
                if panel.search_active { "_" } else { "" },
                Style::default().fg(GOLD).add_modifier(Modifier::SLOW_BLINK),
            ),
        ]));
    }

    // Empty state when no data.
    if panel.snapshot.calls == 0 {
        lines.push(Line::from(Span::styled(
            panel.tab.empty_state_label().to_string(),
            Style::default().fg(MUTED),
        )));
    }

    Paragraph::new(lines).render(area, buf);
}

fn render_help(area: Rect, buf: &mut Buffer) {
    let lines = vec![
        Line::from(vec![Span::styled(
            "Keyboard Shortcuts",
            Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(Span::styled("1–6   switch tab", Style::default().fg(MUTED))),
        Line::from(Span::styled("j/k   move cursor", Style::default().fg(MUTED))),
        Line::from(Span::styled("g/G   top / bottom", Style::default().fg(MUTED))),
        Line::from(Span::styled("Enter open detail", Style::default().fg(MUTED))),
        Line::from(Span::styled("/     search", Style::default().fg(MUTED))),
        Line::from(Span::styled("s     cycle sort", Style::default().fg(MUTED))),
        Line::from(Span::styled("p     pause / resume", Style::default().fg(MUTED))),
        Line::from(Span::styled("?     close help", Style::default().fg(MUTED))),
        Line::from(Span::styled("Esc   back", Style::default().fg(MUTED))),
    ];
    Paragraph::new(lines).render(area, buf);
}

fn render_footer(area: Rect, buf: &mut Buffer) {
    let hints = vec![
        Span::styled("1-6", Style::default().fg(MUTED).add_modifier(Modifier::BOLD)),
        Span::styled(" tabs  ", Style::default().fg(MUTED)),
        Span::styled("j/k", Style::default().fg(MUTED).add_modifier(Modifier::BOLD)),
        Span::styled(" nav  ", Style::default().fg(MUTED)),
        Span::styled("Enter", Style::default().fg(MUTED).add_modifier(Modifier::BOLD)),
        Span::styled(" detail  ", Style::default().fg(MUTED)),
        Span::styled("?", Style::default().fg(MUTED).add_modifier(Modifier::BOLD)),
        Span::styled(" help", Style::default().fg(MUTED)),
    ];
    Paragraph::new(Line::from(hints)).render(area, buf);
}

pub(super) fn fmt_n(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[allow(dead_code)]
pub(super) fn fmt_ms(ms: f64) -> String {
    if !ms.is_finite() || ms <= 0.0 {
        return "-".to_string();
    }
    if ms < 1000.0 {
        format!("{}ms", ms.round() as i64)
    } else {
        format!("{:.1}s", ms / 1000.0)
    }
}

#[allow(dead_code)]
pub(super) fn fmt_pct(ratio: f64) -> String {
    if !ratio.is_finite() {
        return "-".to_string();
    }
    let pct = ratio * 100.0;
    if pct >= 10.0 {
        format!("{}%", pct.round() as i64)
    } else {
        format!("{:.1}%", pct)
    }
}
