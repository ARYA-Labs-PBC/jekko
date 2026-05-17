//! Compact live activity row used for long-running process output.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::activity::ActivityKind;
use crate::transcript::terminal_tokenize::{strip_ansi, tokenize_terminal, TerminalScope};

use super::theme::{COLOR_ACCENT, COLOR_ERROR, COLOR_SUCCESS, COLOR_TEXT, COLOR_TEXT_MUTED, COLOR_WARN};

/// One-row compact telemetry entry.
#[derive(Clone, Debug)]
pub struct ActivityCard {
    /// Activity kind.
    pub kind: ActivityKind,
    /// Label shown in the left rail.
    pub label: String,
    /// Raw message body.
    pub text: String,
    /// Optional status line.
    pub status: Option<String>,
    /// Optional progress badge.
    pub progress: Option<(u64, u64)>,
}

impl ActivityCard {
    /// Build a new activity row.
    pub fn new(kind: ActivityKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            label: kind.label().to_string(),
            text: text.into(),
            status: None,
            progress: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    pub fn with_progress(mut self, current: u64, total: u64) -> Self {
        self.progress = Some((current, total));
        self
    }

    pub fn estimated_rows(&self) -> u16 {
        1
    }

    pub fn snapshot(&self) -> String {
        let mut out = format!("activity[{:?}] {}", self.kind, self.label);
        if let Some(status) = &self.status {
            out.push_str(&format!(" | {status}"));
        }
        if let Some((current, total)) = self.progress {
            out.push_str(&format!(" | {current}/{total}"));
        }
        if !self.text.is_empty() {
            out.push_str(&format!(" :: {}", self.text.replace('\n', " ")));
        }
        out
    }
}

impl Widget for &ActivityCard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let accent = self.kind.accent();
        let mut spans = vec![
            Span::styled(
                format!("▌{} ", self.label),
                Style::default()
                    .fg(accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        if let Some(status) = &self.status {
            spans.push(Span::styled(
                format!("{status} "),
                Style::default().fg(COLOR_WARN),
            ));
        }
        if let Some((current, total)) = self.progress {
            spans.push(Span::styled(
                format!("{current}/{total} "),
                Style::default().fg(COLOR_ACCENT),
            ));
        }

        let body = strip_ansi(&self.text);
        if body.is_empty() {
            spans.push(Span::styled("(idle)", Style::default().fg(COLOR_TEXT_MUTED)));
            Paragraph::new(Line::from(spans)).render(area, buf);
            return;
        }

        let mut token_spans = Vec::new();
        let tokens = tokenize_terminal(&body);
        let mut cursor = 0;
        for token in tokens {
            if token.start > cursor {
                token_spans.push(Span::styled(
                    body[cursor..token.start].to_string(),
                    Style::default().fg(COLOR_TEXT),
                ));
            }
            let text = body[token.start..token.end].to_string();
            token_spans.push(Span::styled(text, scope_style(token.scope)));
            cursor = token.end;
        }
        if cursor < body.len() {
            token_spans.push(Span::styled(
                body[cursor..].to_string(),
                Style::default().fg(COLOR_TEXT),
            ));
        }
        spans.extend(token_spans);
        Paragraph::new(Line::from(spans)).render(area, buf);
    }
}

fn scope_style(scope: TerminalScope) -> Style {
    match scope {
        TerminalScope::Success => Style::default().fg(COLOR_SUCCESS).add_modifier(Modifier::BOLD),
        TerminalScope::Error => Style::default().fg(COLOR_ERROR).add_modifier(Modifier::BOLD),
        TerminalScope::Warning => Style::default().fg(COLOR_WARN).add_modifier(Modifier::BOLD),
        TerminalScope::Time => Style::default().fg(COLOR_TEXT_MUTED),
        TerminalScope::Command => Style::default().fg(COLOR_SUCCESS).add_modifier(Modifier::BOLD),
        TerminalScope::StringLit => Style::default().fg(COLOR_ACCENT),
        TerminalScope::Punctuation => Style::default().fg(COLOR_TEXT_MUTED),
        TerminalScope::Number => Style::default().fg(COLOR_ACCENT),
        TerminalScope::Keyword => Style::default().fg(COLOR_WARN),
        TerminalScope::Prompt => Style::default().fg(COLOR_ACCENT),
    }
}
