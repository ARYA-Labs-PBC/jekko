//! Section: Reasoning Card
//!
//! Collapsible reasoning trace card. Supports both completed and live-streaming
//! states — when `streaming = true` the title animates with a Braille spinner.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use super::theme::COLOR_TEXT_MUTED;

/// Braille spinner frames shared with the splash screen.
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Reasoning trace card. Supports collapsed, expanded, and live-streaming
/// states. When streaming, the `spinner_tick` field drives a Braille animation.
#[derive(Clone, Debug)]
pub struct ReasoningCard {
    /// Body text (accumulated from streaming deltas or set at construction).
    pub text: String,
    /// Whether the card is collapsed (header only).
    pub collapsed: bool,
    /// True while the reasoning stream is in flight.
    pub streaming: bool,
    /// Frame counter used to select the spinner glyph (incremented by the
    /// render caller via `tick()`; wraps at SPINNER_FRAMES.len()).
    pub spinner_tick: usize,
}

impl ReasoningCard {
    /// Build a completed (non-streaming) reasoning card, expanded.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            collapsed: false,
            streaming: false,
            spinner_tick: 0,
        }
    }

    /// Build a fresh streaming reasoning card with empty text.
    pub fn new_streaming() -> Self {
        Self {
            text: String::new(),
            collapsed: false,
            streaming: true,
            spinner_tick: 0,
        }
    }

    /// Append text delta from the stream.
    pub fn append(&mut self, delta: &str) {
        self.text.push_str(delta);
    }

    /// Mark the stream as complete. Spinner stops.
    pub fn mark_complete(&mut self) {
        self.streaming = false;
    }

    /// Advance the spinner one frame.
    pub fn tick(&mut self) {
        self.spinner_tick = (self.spinner_tick + 1) % SPINNER_FRAMES.len();
    }

    /// Force collapsed state at construction.
    pub fn collapsed(mut self) -> Self {
        self.collapsed = true;
        self
    }

    /// Flip the collapsed flag.
    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
    }

    /// Estimated row count (used by scroll math).
    pub fn estimated_rows(&self) -> u16 {
        if self.collapsed {
            2
        } else {
            (self.text.lines().count().max(1) as u16) + 2
        }
    }

    pub fn snapshot(&self) -> String {
        format!(
            "reasoning[collapsed={},streaming={}] {}",
            self.collapsed,
            self.streaming,
            self.text.replace('\n', "⏎")
        )
    }
}

impl Widget for &ReasoningCard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let spinner = if self.streaming {
            SPINNER_FRAMES[self.spinner_tick % SPINNER_FRAMES.len()]
        } else {
            ""
        };

        // Title line: "~ thinking  ⠋" (spinner only when streaming)
        let title_color = if self.streaming {
            Color::Rgb(0x55, 0xd6, 0xff) // INFO — cyan during stream
        } else {
            COLOR_TEXT_MUTED
        };
        let title_style = Style::default()
            .fg(title_color)
            .add_modifier(Modifier::ITALIC);

        let mut title_spans = vec![
            Span::styled("~ ", Style::default().fg(COLOR_TEXT_MUTED).add_modifier(Modifier::ITALIC)),
            Span::styled("thinking", title_style),
        ];
        if self.streaming && !spinner.is_empty() {
            title_spans.push(Span::raw("  "));
            title_spans.push(Span::styled(spinner, Style::default().fg(title_color)));
        }

        let header = Line::from(title_spans);
        let mut lines = vec![header];

        if !self.collapsed {
            if self.text.is_empty() && self.streaming {
                lines.push(Line::from(Span::styled(
                    "  …",
                    Style::default().fg(COLOR_TEXT_MUTED).add_modifier(Modifier::ITALIC),
                )));
            } else {
                for raw in self.text.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {raw}"),
                        Style::default()
                            .fg(COLOR_TEXT_MUTED)
                            .add_modifier(Modifier::ITALIC),
                    )));
                }
            }
        } else {
            let summary = if self.text.is_empty() {
                "(streaming…)".to_string()
            } else {
                format!("({} chars)", self.text.len())
            };
            lines.push(Line::from(Span::styled(
                format!("  {summary}"),
                Style::default().fg(COLOR_TEXT_MUTED),
            )));
        }

        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}
