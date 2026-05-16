//! Section: Footer
//!
//! Context-aware bottom hint bar. Renders `[key] label` badges from a
//! `KeyHint` slice, dropping lowest-priority hints when space is tight.
//! Never truncates a hint mid-word.
//!
//! ```text
//! [Tab] Pane   [/] Commands   [Enter] Send   [?] Help             line 1 · col 0
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::keybind::{hints_for, FocusTarget, KeyHint};
use crate::theme;

// ── FooterBand ───────────────────────────────────────────────────────────────

/// Context-aware footer hint bar.
#[derive(Clone, Debug)]
pub struct FooterBand<'a> {
    pub focus: FocusTarget,
    pub right_label: Option<&'a str>,
    pub background: Color,
    pub border: Color,
}

impl<'a> FooterBand<'a> {
    pub fn new(focus: FocusTarget) -> Self {
        Self {
            focus,
            right_label: None,
            background: theme::BG,
            border: theme::BORDER,
        }
    }

    pub fn with_right(mut self, label: &'a str) -> Self {
        self.right_label = Some(label);
        self
    }
}

impl<'a> Widget for &FooterBand<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Top border
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(self.border))
            .style(Style::default().bg(self.background));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width == 0 {
            return;
        }

        // Hints sorted by priority (lowest number = highest priority).
        let all_hints = hints_for(self.focus);
        let right_text = self.right_label.unwrap_or("");
        let right_width = if right_text.is_empty() {
            0
        } else {
            right_text.len() + 2
        };

        // Build the hints string, dropping lowest-priority hints until it fits.
        let available = inner.width.saturating_sub(right_width as u16) as usize;
        let spans = build_hint_spans(all_hints, available);

        Paragraph::new(Line::from(spans))
            .style(Style::default().bg(self.background))
            .render(inner, buf);

        if !right_text.is_empty() {
            use ratatui::layout::Alignment;
            Paragraph::new(Line::from(Span::styled(
                right_text,
                Style::default().fg(theme::TEXT_MUTED).bg(self.background),
            )))
            .alignment(Alignment::Right)
            .render(inner, buf);
        }
    }
}

/// Build `[key] label` badge spans that fit within `available_cols`.
/// Drops lowest-priority (highest number) hints until everything fits.
fn build_hint_spans(hints: &[KeyHint], available_cols: usize) -> Vec<Span<'static>> {
    // Sort a copy by priority then drop from the end until we fit.
    let mut sorted: Vec<&KeyHint> = hints.iter().collect();
    sorted.sort_by_key(|h| h.priority);

    // Try progressively fewer hints until total width fits or only 1 remains.
    'outer: loop {
        if sorted.is_empty() {
            break;
        }
        let total: usize = sorted
            .iter()
            .enumerate()
            .map(|(i, h)| {
                h.render_width() + if i > 0 { 3 } else { 0 } // 3-space gap between hints
            })
            .sum();
        if total <= available_cols || sorted.len() == 1 {
            break;
        }
        // Drop the hint with the highest priority number (lowest importance).
        // If tie, drop the last one in the sorted list.
        if let Some(pos) = sorted
            .iter()
            .rposition(|h| h.priority == sorted.last().map(|l| l.priority).unwrap_or(0))
        {
            sorted.remove(pos);
        } else {
            sorted.pop();
            break 'outer;
        }
    }

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, hint) in sorted.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(
            format!("[{}]", hint.key),
            Style::default().fg(Color::Rgb(0xf4, 0xc5, 0x42)), // ACCENT
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            hint.label,
            Style::default().fg(Color::Rgb(0xd7, 0xde, 0xe8)), // TEXT
        ));
    }
    spans
}

// ── Legacy shim ─────────────────────────────────────────────────────────────
//
// `FooterBandLegacy` accepts the old `Vec<(&str, &str)>` hint API so callers
// that haven't been migrated yet continue to compile.

/// Legacy footer band that accepts raw `(key, label)` pairs.
pub struct FooterBandLegacy<'a> {
    pub background: Color,
    pub border: Color,
    pub hints: Vec<(&'a str, &'a str)>,
}

impl<'a> FooterBandLegacy<'a> {
    pub fn new(hints: Vec<(&'a str, &'a str)>) -> Self {
        Self {
            background: theme::BG,
            border: theme::BORDER,
            hints,
        }
    }
}

impl<'a> Widget for &FooterBandLegacy<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(self.border))
            .style(Style::default().bg(self.background));
        let inner = block.inner(area);
        block.render(area, buf);

        let mut spans: Vec<Span> = Vec::new();
        let mut first = true;
        for (key, label) in &self.hints {
            if !first {
                spans.push(Span::raw("  "));
            }
            first = false;
            spans.push(Span::styled(
                format!("[{key}]"),
                Style::default().fg(Color::Rgb(0xf4, 0xc5, 0x42)),
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                (*label).to_string(),
                Style::default().fg(Color::Rgb(0xd7, 0xde, 0xe8)),
            ));
        }
        Paragraph::new(Line::from(spans)).render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_hint_spans_drops_low_priority_when_tight() {
        use crate::keybind::HINTS_COMPOSER;
        // Very narrow — should only keep the highest-priority hints.
        let spans = build_hint_spans(HINTS_COMPOSER, 20);
        // Should not be empty.
        assert!(!spans.is_empty());
    }

    #[test]
    fn build_hint_spans_fits_wide_area() {
        use crate::keybind::HINTS_COMPOSER;
        let spans = build_hint_spans(HINTS_COMPOSER, 200);
        // All hints should be present in some form.
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("Tab"));
        assert!(text.contains("Send"));
    }
}
