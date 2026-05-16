//! System status row card.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use super::theme::{COLOR_ERROR, COLOR_SUCCESS, COLOR_TEXT, COLOR_TEXT_MUTED, COLOR_WARN};

/// Kind of system row.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SystemKind {
    /// Neutral informational note.
    Info,
    /// Success notice.
    Success,
    /// Warning.
    Warning,
    /// Error.
    Error,
}

/// System status row. Used for revert markers, daemon transitions, etc.
#[derive(Clone, Debug)]
pub struct SystemCard {
    /// Body text.
    pub text: String,
    /// Severity.
    pub kind: SystemKind,
}

impl SystemCard {
    /// Build a new system card.
    pub fn new(text: impl Into<String>, kind: SystemKind) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }
    /// Snapshot.
    pub fn snapshot(&self) -> String {
        format!("system[{:?}] {}", self.kind, self.text)
    }
}

impl Widget for &SystemCard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (icon, color) = match self.kind {
            SystemKind::Info => ("•", COLOR_TEXT_MUTED),
            SystemKind::Success => ("✓", COLOR_SUCCESS),
            SystemKind::Warning => ("△", COLOR_WARN),
            SystemKind::Error => ("✗", COLOR_ERROR),
        };
        let line = Line::from(vec![
            Span::styled(format!(" {icon} "), Style::default().fg(color)),
            Span::styled(self.text.clone(), Style::default().fg(COLOR_TEXT)),
        ]);
        Paragraph::new(line).render(area, buf);
    }
}
