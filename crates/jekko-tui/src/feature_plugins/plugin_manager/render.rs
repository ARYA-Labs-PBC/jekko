//! Ratatui rendering for the plugin-manager dialog.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::dialog::frame::{render_frame, DialogFrame};

use super::manager::PluginManager;
use super::row::PluginRowKind;

const GOLD: Color = Color::Rgb(0xf5, 0xa6, 0x23);
const MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);
const SUCCESS: Color = Color::Rgb(0x22, 0xc5, 0x5e);
const ERROR: Color = Color::Rgb(0xff, 0x47, 0x57);
const TEXT: Color = Color::Rgb(0xd8, 0xde, 0xe9);
const TEAL: Color = Color::Rgb(0x36, 0xd7, 0xb7);

impl Widget for &PluginManager {
    fn render(self, outer: Rect, buf: &mut Buffer) {
        let frame = DialogFrame::new(self.width, self.height).with_title("Plugins");
        let inner = render_frame(&frame, outer, buf);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner);

        let mut lines = Vec::with_capacity(self.rows.len().max(1));
        if self.rows.is_empty() {
            lines.push(Line::from(Span::styled(
                "no plugins registered",
                Style::default().fg(MUTED),
            )));
        }
        for (idx, row) in self.rows.iter().enumerate() {
            let active = idx == self.cursor;
            let id_style = if !row.enabled {
                Style::default().fg(MUTED)
            } else if active {
                Style::default().fg(GOLD).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };
            let kind_color = match row.kind {
                PluginRowKind::Internal => TEAL,
                PluginRowKind::External => GOLD,
            };
            let state_color = if row.enabled { SUCCESS } else { ERROR };
            let state_label = if row.enabled { "active" } else { "disabled" };
            let counts = format!(
                "th={} cmd={} mp={}",
                row.themes, row.commands, row.model_presets
            );
            lines.push(Line::from(vec![
                Span::raw(if active { "> " } else { "  " }),
                Span::styled(row.id.clone(), id_style),
                Span::raw("  "),
                Span::styled(format!("v{}", row.version), Style::default().fg(MUTED)),
                Span::raw("  "),
                Span::styled(row.kind.label(), Style::default().fg(kind_color)),
                Span::raw("  "),
                Span::styled(counts, Style::default().fg(MUTED)),
                Span::raw("  "),
                Span::styled(state_label, Style::default().fg(state_color)),
            ]));
        }
        Paragraph::new(lines).render(chunks[0], buf);

        let hints = Line::from(vec![
            Span::styled(
                "j/k",
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" move  ", Style::default().fg(MUTED)),
            Span::styled(
                "Space",
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" toggle  ", Style::default().fg(MUTED)),
            Span::styled(
                "Shift+I",
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" install  ", Style::default().fg(MUTED)),
            Span::styled("q", Style::default().fg(MUTED).add_modifier(Modifier::BOLD)),
            Span::styled(" exit", Style::default().fg(MUTED)),
        ]);
        Paragraph::new(hints).render(chunks[1], buf);
    }
}
