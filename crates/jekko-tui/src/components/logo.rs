//! JEKKO logo cell renderer.
//!
//! This ports the retired OpenTUI logo algorithm into native Ratatui: fixed
//! 80-column frame, crisp 5x7 half-block wordmark, and two amber shadow
//! extrusion layers. The public `Logo` / `LogoBuilder` API is preserved for
//! existing callers.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

pub const INNER_WIDTH: usize = 78;
pub const OUTER_WIDTH: usize = INNER_WIDTH + 2;

const ASCII_LOGO: &str = r"
   __  ___ _   _  _   __   __
   \ \/ // | / \| | / / / /
   _\ // /| |/ /\ |/ / / /
   /_//_/ |_|_/  |_/  /_/
";

const AMBER_STOPS: [Color; 8] = [
    Color::Rgb(0x3d, 0x26, 0x06),
    Color::Rgb(0x5c, 0x3a, 0x0e),
    Color::Rgb(0x8b, 0x5f, 0x1a),
    Color::Rgb(0xb5, 0x7f, 0x28),
    Color::Rgb(0xd4, 0xa8, 0x43),
    Color::Rgb(0xe8, 0xc0, 0x55),
    Color::Rgb(0xef, 0xd1, 0x7a),
    Color::Rgb(0xf8, 0xe3, 0xb3),
];

const AMBER_LIGHT_STOPS: [Color; 8] = [
    Color::Rgb(0x2d, 0x1a, 0x04),
    Color::Rgb(0x4f, 0x34, 0x08),
    Color::Rgb(0x55, 0x37, 0x07),
    Color::Rgb(0x6e, 0x4a, 0x0a),
    Color::Rgb(0x7c, 0x5a, 0x11),
    Color::Rgb(0x8c, 0x5f, 0x0d),
    Color::Rgb(0xa6, 0x78, 0x17),
    Color::Rgb(0xb9, 0x88, 0x28),
];

const TEXT_MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogoLayer {
    Global,
    Wordmark,
    WordmarkShadowNear,
    WordmarkShadowMid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogoCell {
    pub ch: char,
    pub fg: Color,
    pub bg: Option<Color>,
    pub bold: bool,
    pub layer: LogoLayer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogoRow {
    pub cells: Vec<LogoCell>,
}

impl LogoRow {
    pub fn text(&self) -> String {
        self.cells.iter().map(|cell| cell.ch).collect()
    }
}

fn render_pixel_word(text: &str, scale_x: usize, gap: usize) -> Vec<String> {
    let glyphs: Vec<&'static Glyph> = text.chars().map(glyph_for).collect();
    if glyphs.is_empty() {
        return Vec::new();
    }

    let terminal_rows = 7usize.div_ceil(2);
    let mut rows = Vec::with_capacity(terminal_rows);
    for tr in 0..terminal_rows {
        let top_row = 2 * tr;
        let bot_row = 2 * tr + 1;
        let mut pieces = Vec::with_capacity(glyphs.len());
        for glyph in &glyphs {
            let top = glyph.get(top_row).copied().unwrap_or("");
            let bot = glyph.get(bot_row).copied().unwrap_or("");
            let cols = top.len().max(bot.len()).max(5);
            let mut piece = String::with_capacity(cols * scale_x);
            for c in 0..cols {
                let t = top.as_bytes().get(c).copied() == Some(b'1');
                let b = bot.as_bytes().get(c).copied() == Some(b'1');
                let ch = if t && b {
                    '█'
                } else if t {
                    '▀'
                } else if b {
                    '▄'
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LogoVariant {
    Ascii,
    #[default]
    Pixel,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Logo;

#[derive(Clone, Debug)]
pub struct LogoBuilder {
    variant: Option<LogoVariant>,
    word: &'static str,
    support: Option<&'static str>,
    status: Option<&'static str>,
    alignment: Alignment,
    idle: bool,
}

impl Logo {
    pub fn pixel() -> LogoBuilder {
        LogoBuilder::new(Some(LogoVariant::Pixel))
    }

    pub fn ascii() -> LogoBuilder {
        LogoBuilder::new(Some(LogoVariant::Ascii))
    }

    pub fn pixel_width(word: &str) -> usize {
        render_pixel_word(word, 2, 2)
            .iter()
            .map(|r| r.chars().count())
            .max()
            .unwrap_or(0)
    }

    pub fn pick(area: Rect) -> LogoVariant {
        if area.width >= OUTER_WIDTH as u16 && area.height >= 11 {
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
            alignment: Alignment::Center,
            idle: false,
        }
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn with_support(mut self, support: &'static str) -> Self {
        self.support = Some(support);
        self
    }

    pub fn with_status(mut self, status: &'static str) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_idle(mut self, idle: bool) -> Self {
        self.idle = idle;
        self
    }

    pub fn variant(&self) -> Option<LogoVariant> {
        self.variant
    }

    pub fn word(&self) -> &'static str {
        self.word
    }

    pub fn rows(&self) -> Vec<LogoRow> {
        build_logo_rows(self)
    }
}

fn fit(text: &str, width: usize, alignment: Alignment) -> String {
    let chars: Vec<char> = text.chars().collect();
    let clipped: String = chars.iter().take(width).collect();
    let len = clipped.chars().count();
    let remaining = width.saturating_sub(len);
    match alignment {
        Alignment::Left => format!("{clipped}{}", " ".repeat(remaining)),
        Alignment::Right => format!("{}{}", " ".repeat(remaining), clipped),
        _ => {
            let left = remaining / 2;
            let right = remaining - left;
            format!("{}{}{}", " ".repeat(left), clipped, " ".repeat(right))
        }
    }
}

fn pair(left: &str, right: &str, width: usize) -> String {
    let left: String = left.chars().take(width).collect();
    let right: String = right.chars().take(width).collect();
    let left_len = left.chars().count();
    let right_len = right.chars().count();
    let gap = width.saturating_sub(left_len + right_len);
    if gap < 1 {
        fit(&format!("{left} {right}"), width, Alignment::Left)
    } else {
        format!("{left}{}{right}", " ".repeat(gap))
    }
}

fn framed(content: &str, alignment: Alignment) -> String {
    format!("│{}│", fit(content, INNER_WIDTH, alignment))
}

fn framed_pair(left: &str, right: &str) -> String {
    format!("│{}│", pair(left, right, INNER_WIDTH))
}

fn top_border() -> String {
    format!("╭{}╮", "─".repeat(INNER_WIDTH))
}

fn divider() -> String {
    format!("├{}┤", "─".repeat(INNER_WIDTH))
}

fn bottom_border() -> String {
    format!("╰{}╯", "─".repeat(INNER_WIDTH))
}

fn row_from_text(text: String, bold: bool, dim: bool) -> LogoRow {
    let len = text.chars().count().max(1);
    let cells = text
        .chars()
        .enumerate()
        .map(|(x, ch)| LogoCell {
            ch,
            fg: if dim {
                TEXT_MUTED
            } else {
                amber_at(x, 0, len, 1, false)
            },
            bg: None,
            bold,
            layer: LogoLayer::Global,
        })
        .collect();
    LogoRow { cells }
}

fn empty_cell() -> LogoCell {
    LogoCell {
        ch: ' ',
        fg: TEXT_MUTED,
        bg: None,
        bold: false,
        layer: LogoLayer::Global,
    }
}

fn frame_cells(cells: Vec<LogoCell>) -> LogoRow {
    let mut inner = cells.into_iter().take(INNER_WIDTH).collect::<Vec<_>>();
    while inner.len() < INNER_WIDTH {
        inner.push(empty_cell());
    }
    let mut out = Vec::with_capacity(OUTER_WIDTH);
    out.push(LogoCell {
        ch: '│',
        fg: AMBER_STOPS[4],
        bg: None,
        bold: false,
        layer: LogoLayer::Global,
    });
    out.extend(inner);
    out.push(LogoCell {
        ch: '│',
        fg: AMBER_STOPS[4],
        bg: None,
        bold: false,
        layer: LogoLayer::Global,
    });
    LogoRow { cells: out }
}

fn shadowed_wordmark_rows(word: &str) -> Vec<LogoRow> {
    let art = render_pixel_word(word, 2, 2);
    let art_width = art.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    let visual_width = art_width + 2;
    let visual_height = art.len() + 1;
    let left = INNER_WIDTH.saturating_sub(visual_width) / 2;
    let mut canvas = vec![vec![empty_cell(); INNER_WIDTH]; visual_height];

    for (dx, dy, layer) in [
        (2usize, 1usize, LogoLayer::WordmarkShadowMid),
        (1usize, 1usize, LogoLayer::WordmarkShadowNear),
    ] {
        for (y, line) in art.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }
                let px = left + x + dx;
                let py = y + dy;
                if py < canvas.len() && px < INNER_WIDTH {
                    canvas[py][px] = LogoCell {
                        ch: '█',
                        fg: shadow_color(x, y, art_width, art.len(), layer),
                        bg: None,
                        bold: false,
                        layer,
                    };
                }
            }
        }
    }

    for (y, line) in art.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let px = left + x;
            if px < INNER_WIDTH {
                canvas[y][px] = LogoCell {
                    ch,
                    fg: amber_at(x, y, art_width, art.len(), false),
                    bg: None,
                    bold: true,
                    layer: LogoLayer::Wordmark,
                };
            }
        }
    }

    canvas.into_iter().map(frame_cells).collect()
}

fn build_logo_rows(builder: &LogoBuilder) -> Vec<LogoRow> {
    let support = builder.support.unwrap_or("ZYAL");
    let status = builder.status.unwrap_or(if builder.idle {
        "camouflage idle • watching the wall"
    } else {
        "safe autonomous coding ready"
    });
    let header_right = if builder.idle {
        "gecko mode idle   ● ● ●"
    } else {
        "gecko mode active ● ● ●"
    };

    let mut rows = vec![
        row_from_text(top_border(), false, false),
        row_from_text(framed_pair(" ›_ JEKKO", header_right), true, false),
        row_from_text(divider(), false, false),
        row_from_text(framed("", Alignment::Center), false, false),
    ];
    rows.extend(shadowed_wordmark_rows(builder.word));
    rows.push(row_from_text(
        framed(
            &format!("AI coding gecko • {support} support • climbs hard problems"),
            Alignment::Center,
        ),
        false,
        false,
    ));
    rows.push(row_from_text(
        framed(&format!("gecko:// {status}"), Alignment::Center),
        false,
        builder.idle,
    ));
    rows.push(row_from_text(bottom_border(), false, false));
    rows
}

fn amber_at(x: usize, y: usize, width: usize, height: usize, light: bool) -> Color {
    let tx = if width <= 1 {
        0.0
    } else {
        x as f32 / (width - 1) as f32
    };
    let ty = if height <= 1 {
        0.0
    } else {
        y as f32 / (height - 1) as f32
    };
    let t = (tx * 0.74 + ty * 0.26).clamp(0.0, 1.0);
    color_stop(
        if light {
            &AMBER_LIGHT_STOPS
        } else {
            &AMBER_STOPS
        },
        t,
    )
}

fn shadow_color(x: usize, y: usize, width: usize, height: usize, layer: LogoLayer) -> Color {
    let base = amber_at(x, y, width, height, false);
    let amount = match layer {
        LogoLayer::WordmarkShadowMid => 0.80,
        LogoLayer::WordmarkShadowNear => 0.70,
        _ => 0.0,
    };
    dim_color(base, amount)
}

fn color_stop(stops: &[Color], t: f32) -> Color {
    let idx = ((stops.len().saturating_sub(1)) as f32 * t).round() as usize;
    stops[idx.min(stops.len().saturating_sub(1))]
}

fn dim_color(color: Color, amount: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => {
            let k = (1.0 - amount).clamp(0.0, 1.0);
            Color::Rgb(
                (r as f32 * k) as u8,
                (g as f32 * k) as u8,
                (b as f32 * k) as u8,
            )
        }
        other => other,
    }
}

fn pixel_lines(builder: &LogoBuilder) -> Vec<Line<'static>> {
    build_logo_rows(builder)
        .into_iter()
        .map(|row| {
            Line::from(
                row.cells
                    .into_iter()
                    .map(|cell| {
                        let mut style = Style::default().fg(cell.fg);
                        if let Some(bg) = cell.bg {
                            style = style.bg(bg);
                        }
                        if cell.bold {
                            style = style.add_modifier(Modifier::BOLD);
                        }
                        Span::styled(cell.ch.to_string(), style)
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn ascii_lines() -> Vec<Line<'static>> {
    ASCII_LOGO
        .lines()
        .map(|line| {
            Line::from(line.to_string()).style(
                Style::default()
                    .fg(AMBER_STOPS[4])
                    .add_modifier(Modifier::BOLD),
            )
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
        let variant = self.variant.unwrap_or_else(|| Logo::pick(area));
        render_variant(variant, self, area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_word_renders_four_rows() {
        assert_eq!(render_pixel_word("JEKKO", 2, 2).len(), 4);
    }

    #[test]
    fn logo_rows_match_old_frame_text() {
        let rows = Logo::pixel().rows();
        assert_eq!(rows.len(), 11);
        assert_eq!(rows[0].text(), format!("╭{}╮", "─".repeat(78)));
        assert!(rows[1].text().starts_with("│ ›_ JEKKO"));
        assert_eq!(rows[2].text(), format!("├{}┤", "─".repeat(78)));
        assert!(rows[9]
            .text()
            .contains("gecko:// safe autonomous coding ready"));
        assert_eq!(rows[10].text(), format!("╰{}╯", "─".repeat(78)));
    }

    #[test]
    fn wordmark_contains_shadow_layers() {
        let rows = Logo::pixel().rows();
        assert!(rows
            .iter()
            .flat_map(|row| row.cells.iter())
            .any(|cell| cell.layer == LogoLayer::WordmarkShadowNear));
        assert!(rows
            .iter()
            .flat_map(|row| row.cells.iter())
            .any(|cell| cell.layer == LogoLayer::WordmarkShadowMid));
    }

    #[test]
    fn auto_picks_pixel_for_full_logo_area() {
        assert_eq!(Logo::pick(Rect::new(0, 0, 80, 11)), LogoVariant::Pixel);
    }

    #[test]
    fn auto_picks_ascii_for_small_area() {
        assert_eq!(Logo::pick(Rect::new(0, 0, 40, 4)), LogoVariant::Ascii);
    }
}
