//! Assistant turn card and its part variants.

use std::time::Instant;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use super::theme::{COLOR_ACCENT, COLOR_TEXT, COLOR_TEXT_MUTED};

/// 10-frame Braille spinner glyphs used by the pending-state animation.
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_FRAME_MS: u128 = 80;
const PENDING_LABEL: &str = "thinking…";

/// What kind of assistant content a part holds.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AssistantPartKind {
    /// Plain reply text.
    Text,
    /// Reasoning trace (rendered dim italic).
    Reasoning,
    /// A pointer to a tool call (rendered as a chip; the body is in a
    /// sibling `ToolCard`).
    ToolCall,
}

/// One slice of an assistant turn.
#[derive(Clone, Debug)]
pub struct AssistantPart {
    /// Kind of content.
    pub kind: AssistantPartKind,
    /// Raw text body.
    pub text: String,
}

impl AssistantPart {
    /// Build a new part.
    pub fn new(kind: AssistantPartKind, text: String) -> Self {
        Self { kind, text }
    }
}

/// Assistant turn card. Mirrors `AssistantMessage` in `session-renderers.tsx`.
#[derive(Clone, Debug)]
pub struct AssistantCard {
    /// Ordered list of parts.
    pub parts: Vec<AssistantPart>,
    /// Optional model name (e.g. `"claude-opus-4-7"`).
    pub model: Option<String>,
    /// Optional duration in seconds.
    pub duration_secs: Option<f32>,
    /// When `Some`, the card is waiting on a streamed response and should
    /// render an animated spinner instead of `(empty)`. Cleared by the
    /// transcript on the first text delta.
    pub pending_since: Option<Instant>,
}

impl AssistantCard {
    /// Build from a parts list.
    pub fn new(parts: Vec<AssistantPart>) -> Self {
        Self {
            parts,
            model: None,
            duration_secs: None,
            pending_since: None,
        }
    }
    /// Attach a model identifier.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
    /// Attach a duration in seconds.
    pub fn with_duration_secs(mut self, secs: f32) -> Self {
        self.duration_secs = Some(secs);
        self
    }
    /// Mark the card as awaiting a streamed response. Render shows an
    /// animated Braille spinner + label until the first delta clears it via
    /// [`AssistantCard::mark_streaming`].
    pub fn with_pending_now(mut self) -> Self {
        self.pending_since = Some(Instant::now());
        self
    }
    /// Clear the pending state. Called by the transcript on the first text
    /// delta so the spinner is replaced by streamed content.
    pub fn mark_streaming(&mut self) {
        self.pending_since = None;
    }
    /// `true` while the card is still awaiting a response.
    pub fn is_pending(&self) -> bool {
        self.pending_since.is_some()
    }
    /// Current spinner glyph based on elapsed time since the pending state
    /// was set. Public for snapshot tests.
    pub fn pending_glyph(&self) -> Option<&'static str> {
        let since = self.pending_since?;
        let idx = (since.elapsed().as_millis() / SPINNER_FRAME_MS) as usize % SPINNER_FRAMES.len();
        Some(SPINNER_FRAMES[idx])
    }
    /// Cheap row estimate. 1 chrome row (header) + content lines per part. No
    /// trailing chrome — vertical space is precious in the activity feed.
    pub fn estimated_rows(&self) -> u16 {
        let mut rows: u16 = 1;
        for part in &self.parts {
            rows = rows.saturating_add(part.text.lines().count().max(1) as u16);
        }
        rows
    }
    /// Snapshot.
    pub fn snapshot(&self) -> String {
        let model = self.model.as_deref().unwrap_or("model:--");
        let duration = match self.duration_secs {
            Some(d) => format!(" {d:.1}s"),
            None => String::new(),
        };
        let mut out = format!("assistant[{model}{duration}]\n");
        for part in &self.parts {
            out.push_str(match part.kind {
                AssistantPartKind::Text => "  text: ",
                AssistantPartKind::Reasoning => "  reason: ",
                AssistantPartKind::ToolCall => "  tool: ",
            });
            out.push_str(&part.text);
            out.push('\n');
        }
        out
    }
}

impl Widget for &AssistantCard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut header_spans = vec![
            Span::styled(
                "◆ ",
                Style::default()
                    .fg(COLOR_ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Jekko", Style::default().fg(COLOR_ACCENT)),
        ];
        if let Some(model) = &self.model {
            header_spans.push(Span::raw(" "));
            header_spans.push(Span::styled(
                format!("· {model}"),
                Style::default().fg(COLOR_TEXT_MUTED),
            ));
        }
        if let Some(d) = self.duration_secs {
            header_spans.push(Span::raw(" "));
            header_spans.push(Span::styled(
                format!("· {d:.1}s"),
                Style::default().fg(COLOR_TEXT_MUTED),
            ));
        }
        let mut lines = vec![Line::from(header_spans)];
        for part in &self.parts {
            let style = match part.kind {
                AssistantPartKind::Text => Style::default().fg(COLOR_TEXT),
                AssistantPartKind::Reasoning => Style::default()
                    .fg(COLOR_TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
                AssistantPartKind::ToolCall => Style::default().fg(COLOR_ACCENT),
            };
            let prefix = match part.kind {
                AssistantPartKind::Text => "  ",
                AssistantPartKind::Reasoning => "  ~ ",
                AssistantPartKind::ToolCall => "  ⚙ ",
            };
            for raw in part.text.lines() {
                lines.push(Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(raw.to_string(), style),
                ]));
            }
            if part.text.is_empty() {
                if let Some(glyph) = self.pending_glyph() {
                    let secs = self
                        .pending_since
                        .map(|since| since.elapsed().as_secs())
                        .unwrap_or(0);
                    lines.push(Line::from(vec![
                        Span::raw(prefix),
                        Span::styled(
                            glyph.to_string(),
                            Style::default()
                                .fg(COLOR_ACCENT)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(PENDING_LABEL, Style::default().fg(COLOR_TEXT_MUTED)),
                        Span::styled(format!("  {secs}s"), Style::default().fg(COLOR_TEXT_MUTED)),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("{prefix}(empty)"),
                        Style::default().fg(COLOR_TEXT_MUTED),
                    )));
                }
            }
        }
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_card_is_not_pending() {
        let card = AssistantCard::new(vec![AssistantPart::new(
            AssistantPartKind::Text,
            String::new(),
        )]);
        assert!(!card.is_pending());
        assert!(card.pending_glyph().is_none());
    }

    #[test]
    fn with_pending_now_emits_braille_glyph() {
        let card = AssistantCard::new(vec![AssistantPart::new(
            AssistantPartKind::Text,
            String::new(),
        )])
        .with_pending_now();
        let glyph = card.pending_glyph().expect("glyph");
        assert!(SPINNER_FRAMES.contains(&glyph));
    }

    #[test]
    fn mark_streaming_clears_pending() {
        let mut card = AssistantCard::new(vec![AssistantPart::new(
            AssistantPartKind::Text,
            String::new(),
        )])
        .with_pending_now();
        assert!(card.is_pending());
        card.mark_streaming();
        assert!(!card.is_pending());
        assert!(card.pending_glyph().is_none());
    }
}
