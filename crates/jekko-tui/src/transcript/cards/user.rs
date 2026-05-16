//! User-authored transcript card.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use super::theme::{COLOR_TEXT, COLOR_TEXT_MUTED, COLOR_USER};

/// User-authored card. Mirrors `UserMessage` in `session-renderers.tsx`.
#[derive(Clone, Debug)]
pub struct UserCard {
    /// Plaintext. Markdown rendering is a future enhancement on top of this
    /// card type.
    pub text: String,
    /// Optional pre-formatted timestamp label (e.g. `"12:34"`).
    pub timestamp_label: Option<String>,
}

impl UserCard {
    /// New card from a text payload.
    pub fn new(text: String) -> Self {
        Self {
            text,
            timestamp_label: None,
        }
    }
    /// Attach a timestamp label.
    pub fn with_timestamp_label(mut self, label: impl Into<String>) -> Self {
        self.timestamp_label = Some(label.into());
        self
    }
    /// Cheap row estimate. 1 chrome row (header) + content lines. No trailing
    /// chrome — vertical space is precious in the activity feed.
    pub fn estimated_rows(&self) -> u16 {
        let lines = self.text.lines().count().max(1) as u16;
        lines + 1
    }
    /// String snapshot for tests / `insta`.
    pub fn snapshot(&self) -> String {
        let ts = self.timestamp_label.as_deref().unwrap_or("--:--");
        format!("user[{ts}] {}", self.text)
    }
}

impl Widget for &UserCard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let prefix = Span::styled(
            "> ",
            Style::default().fg(COLOR_USER).add_modifier(Modifier::BOLD),
        );
        let ts = match &self.timestamp_label {
            Some(label) => {
                Span::styled(format!(" [{label}]"), Style::default().fg(COLOR_TEXT_MUTED))
            }
            None => Span::raw(""),
        };
        let header = Line::from(vec![
            prefix,
            Span::styled("you", Style::default().fg(COLOR_USER)),
            ts,
        ]);
        let mut lines: Vec<Line> = vec![header];
        for raw in self.text.lines() {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(raw.to_string(), Style::default().fg(COLOR_TEXT)),
            ]));
        }
        if self.text.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (empty)",
                Style::default().fg(COLOR_TEXT_MUTED),
            )));
        }
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}
