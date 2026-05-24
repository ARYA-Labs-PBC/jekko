use ratatui::style::Style;

use crate::glyph_set;

/// A single status-row segment with drop priority.
#[derive(Clone, Debug)]
pub struct Segment {
    /// Visible text for the segment. Owned so the result `Vec<Span<'static>>`
    /// can outlive the caller's borrow.
    pub text: String,
    /// Style applied to the segment span.
    pub style: Style,
    /// Drop priority: `0` survives forever, `255` drops first.
    pub priority: u8,
}

impl Segment {
    pub fn new(text: impl Into<String>, style: Style, priority: u8) -> Self {
        Self {
            text: text.into(),
            style,
            priority,
        }
    }
}

/// Tunables for the packer.
#[derive(Clone, Copy, Debug)]
pub struct PackOptions {
    /// Separator string inserted between kept segments.
    pub separator: &'static str,
    /// Style applied to separator spans.
    pub separator_style: Style,
    /// Ellipsis appended when a sole survivor is truncated.
    pub ellipsis: &'static str,
}

impl Default for PackOptions {
    fn default() -> Self {
        Self {
            separator: " · ",
            separator_style: Style::default(),
            ellipsis: glyph_set::current().ellipsis,
        }
    }
}
