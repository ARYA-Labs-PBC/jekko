//! JEKKO logo. Renders one of two faces:
//!
//! * `Logo` (default unit struct) — picks the right face for the surrounding
//!   area at render time: a 5×7 pixel-font wordmark for ≥ 60×8, otherwise a
//!   compact ASCII alternative.
//! * `LogoVariant::Pixel` — a 5×7 pixel-font wordmark drawn with Unicode
//!   half-blocks (`▀`/`▄`/`█`), giving square-ish glyphs and a crisp arcade
//!   silhouette (`scale_x = 2`, `gap = 2`).
//! * `LogoVariant::Ascii` — the compact textual alternative, kept for narrow
//!   terminals and tests that need a legible width guarantee.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

const ASCII_LOGO: &str = r"
   __  ___ _   _  _   __   __
   \ \/ // | / \| | / / / /
   _\ // /| |/ /\ |/ / / /
   /_//_/ |_|_/  |_/  /_/
";

// --- 5x7 pixel font (J E K K O, plus blank) -----------------------------
// Each row is a column-pattern string of `1`/`0`; rows are top→bottom.
// Mirrors `PIXEL_FONT_5X7` from `logo.tsx`.

type Glyph = [&'static str; 7];

const GLYPH_J: Glyph = [
    "11111", "00010", "00010", "00010", "00010", "10010", "01100",
];
const GLYPH_E: Glyph = [
    "11111", "10000", "10000", "11110", "10000", "10000", "11111",
];
const GLYPH_K: Glyph = [
    "10001", "10010", "10100", "11000", "10100", "10010", "10001",
];
const GLYPH_O: Glyph = [
    "01110", "10001", "10001", "10001", "10001", "10001", "01110",
];
const GLYPH_SPACE: Glyph = [
    "00000", "00000", "00000", "00000", "00000", "00000", "00000",
];

fn glyph_for(ch: char) -> &'static Glyph {
    match ch {
        'J' | 'j' => &GLYPH_J,
        'E' | 'e' => &GLYPH_E,
        'K' | 'k' => &GLYPH_K,
        'O' | 'o' => &GLYPH_O,
        _ => &GLYPH_SPACE,
    }
}

/// Render the supplied word as a vector of terminal lines built from
/// `▀`/`▄`/`█` half-block characters. `scale_x` widens each pixel column
/// horizontally to compensate for terminal cell aspect ratio; `gap` is the
/// number of pixel columns between letters.
fn render_pixel_word(text: &str, scale_x: usize, gap: usize) -> Vec<String> {
    let glyphs: Vec<&'static Glyph> = text.chars().map(glyph_for).collect();
    if glyphs.is_empty() {
        return Vec::new();
    }
    let pixel_height = 7usize;
    let terminal_rows = pixel_height.div_ceil(2);
    let mut rows: Vec<String> = Vec::with_capacity(terminal_rows);
    for tr in 0..terminal_rows {
        let top_row = 2 * tr;
        let bot_row = 2 * tr + 1;
        let mut pieces: Vec<String> = Vec::with_capacity(glyphs.len());
        for glyph in &glyphs {
            let top = glyph.get(top_row).copied().unwrap_or("");
            let bot = glyph.get(bot_row).copied().unwrap_or("");
            let cols = top.len().max(bot.len()).max(5);
            let mut piece = String::with_capacity(cols * scale_x * 3);
            for c in 0..cols {
                let t = top.as_bytes().get(c).copied() == Some(b'1');
                let b = bot.as_bytes().get(c).copied() == Some(b'1');
                let ch = if t && b {
                    '\u{2588}' // █
                } else if t {
                    '\u{2580}' // ▀
                } else if b {
                    '\u{2584}' // ▄
                } else {
                    ' '
                };
                for _ in 0..scale_x {
                    piece.push(ch);
                }
            }
            pieces.push(piece);
        }
        rows.push(pieces.join(&" ".repeat(gap)));
    }
    rows
}

/// Which face to draw.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LogoVariant {
    /// Compact 4-line ASCII alternative.
    Ascii,
    /// 5×7 pixel font drawn with half-block characters.
    #[default]
    Pixel,
}

/// JEKKO logo widget. Use the unit-struct form `Logo` for auto-sized
/// rendering, or construct a `LogoBuilder` via [`Logo::pixel`] /
/// [`Logo::ascii`] for explicit control.
///
/// The default face is a single-tone gold pixel font that lines up with the
/// rest of the Ratatui chrome.
#[derive(Clone, Copy, Debug, Default)]
pub struct Logo;

/// Configurable logo. Use this when you want a specific face or want to add
/// support / status subtitle lines underneath the wordmark.
#[derive(Clone, Debug)]
pub struct LogoBuilder {
    variant: Option<LogoVariant>,
    word: &'static str,
    support: Option<&'static str>,
    status: Option<&'static str>,
    alignment: ratatui::layout::Alignment,
}

impl Logo {
    /// Pixel-font face. Use this when the surrounding area is at least
    /// 60 columns wide and 8 rows tall.
    pub fn pixel() -> LogoBuilder {
        LogoBuilder::new(Some(LogoVariant::Pixel))
    }

    /// Compact ASCII alternative.
    pub fn ascii() -> LogoBuilder {
        LogoBuilder::new(Some(LogoVariant::Ascii))
    }

    /// Width of the rendered pixel word in columns. Useful when callers want
    /// to size a wrapping frame around the logo without rendering twice.
    pub fn pixel_width(word: &str) -> usize {
        let rows = render_pixel_word(word, 2, 2);
        rows.iter().map(|r| r.chars().count()).max().unwrap_or(0)
    }

    /// Pick the right face for the supplied area. ≥ 60×5 picks the pixel
    /// face; smaller terminals get the ASCII face. (The pixel face needs
    /// 1 row for the divider plus 4 rows for the wordmark.)
    pub fn pick(area: Rect) -> LogoVariant {
        if area.width >= 60 && area.height >= 5 {
            LogoVariant::Pixel
        } else {
            LogoVariant::Ascii
        }
    }
}

impl LogoBuilder {
    pub fn default_face() -> Self {
        Self::new(None)
    }

    fn new(variant: Option<LogoVariant>) -> Self {
        Self {
            variant,
            word: "JEKKO",
            support: None,
            status: None,
            alignment: ratatui::layout::Alignment::Center,
        }
    }

    pub fn with_alignment(mut self, alignment: ratatui::layout::Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn variant(&self) -> Option<LogoVariant> {
        self.variant
    }

    pub fn with_support(mut self, support: &'static str) -> Self {
        self.support = Some(support);
        self
    }

    pub fn with_status(mut self, status: &'static str) -> Self {
        self.status = Some(status);
        self
    }

    pub fn word(&self) -> &'static str {
        self.word
    }
}

const GOLD: Color = Color::Rgb(0xd4, 0xa8, 0x43);
const GOLD_DIM: Color = Color::Rgb(0x6a, 0x54, 0x21);
const TEXT_MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);
const ACCENT_DIVIDER: Color = Color::Rgb(0x3a, 0x40, 0x4a);

fn pixel_lines(builder: &LogoBuilder) -> Vec<Line<'static>> {
    let rows = render_pixel_word(builder.word, 2, 2);
    let mut lines: Vec<Line<'static>> = Vec::with_capacity(rows.len() + 3);
    // Top trim row — a faint horizontal rule above the wordmark.
    lines.push(Line::from(Span::styled(
        "\u{2500}".repeat(Logo::pixel_width(builder.word)),
        Style::default().fg(ACCENT_DIVIDER),
    )));
    for row in rows {
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
        )));
    }
    // Subtitle row(s) — mirrors the support/status props on `LogoProps`.
    if let (Some(support), Some(status)) = (builder.support, builder.status) {
        lines.push(Line::from(vec![
            Span::styled(
                support.to_string(),
                Style::default().fg(GOLD_DIM).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(status.to_string(), Style::default().fg(TEXT_MUTED)),
        ]));
    } else if let Some(support) = builder.support {
        lines.push(Line::from(Span::styled(
            support.to_string(),
            Style::default().fg(GOLD_DIM).add_modifier(Modifier::BOLD),
        )));
    } else if let Some(status) = builder.status {
        lines.push(Line::from(Span::styled(
            status.to_string(),
            Style::default().fg(TEXT_MUTED),
        )));
    }
    lines
}

fn ascii_lines() -> Vec<Line<'static>> {
    ASCII_LOGO
        .lines()
        .map(|line| {
            Line::from(line.to_string())
                .style(Style::default().fg(GOLD).add_modifier(Modifier::BOLD))
        })
        .collect()
}

fn render_variant(variant: LogoVariant, builder: &LogoBuilder, area: Rect, buf: &mut Buffer) {
    let lines = match variant {
        LogoVariant::Pixel => pixel_lines(builder),
        LogoVariant::Ascii => ascii_lines(),
    };
    Paragraph::new(lines)
        .alignment(builder.alignment)
        .render(area, buf);
}

impl Widget for &Logo {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let variant = Logo::pick(area);
        let builder = LogoBuilder::new(None);
        render_variant(variant, &builder, area, buf);
    }
}

impl Widget for &LogoBuilder {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let variant = match self.variant {
            Some(v) => v,
            None => Logo::pick(area),
        };
        render_variant(variant, self, area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_word_renders_four_rows() {
        let rows = render_pixel_word("JEKKO", 2, 2);
        // 7 pixel rows → 4 terminal rows once divided by 2.
        assert_eq!(rows.len(), 4);
    }

    #[test]
    fn pixel_word_uses_half_blocks() {
        let rows = render_pixel_word("J", 1, 0);
        let joined: String = rows.join("\n");
        assert!(
            joined.contains('\u{2588}')
                || joined.contains('\u{2580}')
                || joined.contains('\u{2584}')
        );
    }

    #[test]
    fn auto_picks_pixel_for_wide_area() {
        let area = Rect::new(0, 0, 80, 10);
        assert_eq!(Logo::pick(area), LogoVariant::Pixel);
    }

    #[test]
    fn auto_picks_ascii_for_small_area() {
        let area = Rect::new(0, 0, 40, 4);
        assert_eq!(Logo::pick(area), LogoVariant::Ascii);
    }

    #[test]
    fn auto_picks_ascii_for_narrow_area() {
        let area = Rect::new(0, 0, 30, 10);
        assert_eq!(Logo::pick(area), LogoVariant::Ascii);
    }

    #[test]
    fn unknown_chars_render_as_blanks() {
        // Asterisk is not in the JEKKO subset; should not panic and produce
        // blank pixels.
        let rows = render_pixel_word("*", 1, 0);
        assert_eq!(rows.len(), 4);
    }
}
