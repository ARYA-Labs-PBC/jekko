//! JEKKO logo cell renderer.
//!
//! This ports the historical ANSI-art logo into native Ratatui. The logo data
//! is kept as the original bracket-style ANSI payload, parsed once into styled
//! cells, then fit to the target rectangle at render time.

use std::sync::OnceLock;

#[path = "logo_ansi_art.rs"]
mod logo_ansi_art;

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::theme;
use jekko_core::theme::ThemeMode;

use logo_ansi_art::JEKKO_ANSI_ART;

pub const INNER_WIDTH: usize = 78;
pub const OUTER_WIDTH: usize = INNER_WIDTH + 2;

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
    mode: Option<ThemeMode>,
    animation_tick: Option<u64>,
}

impl Logo {
    pub fn pixel() -> LogoBuilder {
        LogoBuilder::new(Some(LogoVariant::Pixel))
    }

    pub fn ascii() -> LogoBuilder {
        LogoBuilder::new(Some(LogoVariant::Ascii))
    }

    pub fn pixel_width(word: &str) -> usize {
        word.chars().count()
    }

    pub fn pick(area: Rect) -> LogoVariant {
        if area.width >= 48 && area.height >= 5 {
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
            mode: None,
            animation_tick: None,
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

    pub fn with_mode(mut self, mode: ThemeMode) -> Self {
        self.mode = Some(mode);
        self
    }

    pub fn with_animation_tick(mut self, tick: u64) -> Self {
        self.animation_tick = Some(tick);
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

fn default_fg(mode: ThemeMode) -> Color {
    if mode == ThemeMode::Light {
        theme::palette(mode).text_muted
    } else {
        TEXT_MUTED
    }
}

fn fit(text: &str, width: usize, alignment: Alignment) -> String {
    let clipped: String = text.chars().take(width).collect();
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

fn row_from_text(text: String, bold: bool, dim: bool, mode: ThemeMode) -> LogoRow {
    let len = text.chars().count().max(1);
    let cells = text
        .chars()
        .enumerate()
        .map(|(x, ch)| LogoCell {
            ch,
            fg: if dim {
                default_fg(mode)
            } else {
                amber_at(x, 0, len, 1, false, mode)
            },
            bg: None,
            bold,
            layer: LogoLayer::Global,
        })
        .collect();
    LogoRow { cells }
}

fn apply_sgr(token: &str, state: &mut SgrState) {
    let mut values = token.split(';').peekable();
    while let Some(value) = values.next() {
        match value.parse::<u16>() {
            Ok(0) => *state = SgrState::default(),
            Ok(1) => state.bold = true,
            Ok(22) => state.bold = false,
            Ok(38) => {
                if values.next() != Some("2") {
                    continue;
                }
                let Some(r) = values.next().and_then(parse_u8) else { continue };
                let Some(g) = values.next().and_then(parse_u8) else { continue };
                let Some(b) = values.next().and_then(parse_u8) else { continue };
                state.fg = Some(Color::Rgb(r, g, b));
            }
            Ok(48) => {
                if values.next() != Some("2") {
                    continue;
                }
                let Some(r) = values.next().and_then(parse_u8) else { continue };
                let Some(g) = values.next().and_then(parse_u8) else { continue };
                let Some(b) = values.next().and_then(parse_u8) else { continue };
                state.bg = Some(Color::Rgb(r, g, b));
            }
            Ok(39) => state.fg = None,
            Ok(49) => state.bg = None,
            _ => {}
        }
    }
}

fn parse_u8(value: &str) -> Option<u8> {
    value.parse::<u8>().ok()
}

#[derive(Clone, Copy, Debug, Default)]
struct SgrState {
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
}

fn next_char(source: &str, index: usize) -> Option<(char, usize)> {
    let ch = source[index..].chars().next()?;
    Some((ch, ch.len_utf8()))
}

fn parse_bracket_ansi_art(source: &str) -> Vec<LogoRow> {
    let mut rows = Vec::new();
    let mut current = LogoRow { cells: Vec::new() };
    let mut state = SgrState::default();
    let mut index = 0;

    while index < source.len() {
        let Some((ch, ch_len)) = next_char(source, index) else {
            break;
        };

        match ch {
            '\r' => {
                index += ch_len;
            }
            '\n' => {
                rows.push(current);
                current = LogoRow { cells: Vec::new() };
                index += ch_len;
            }
            '[' => {
                let mut token_end = index + ch_len;
                while token_end < source.len() {
                    let Some((next, next_len)) = next_char(source, token_end) else {
                        break;
                    };
                    if next.is_ascii_digit() || next == ';' {
                        token_end += next_len;
                    } else {
                        break;
                    }
                }
                let term = source[token_end..].chars().next();
                match term {
                    Some('m') => {
                        let token = &source[index + ch_len..token_end];
                        apply_sgr(token, &mut state);
                        index = token_end + 1;
                    }
                    Some('J') | Some('H') => {
                        index = token_end + 1;
                    }
                    _ => {
                        current.cells.push(LogoCell {
                            ch,
                            fg: state.fg.unwrap_or(TEXT_MUTED),
                            bg: state.bg,
                            bold: state.bold,
                            layer: LogoLayer::Global,
                        });
                        index += ch_len;
                    }
                }
            }
            _ => {
                current.cells.push(LogoCell {
                    ch,
                    fg: state.fg.unwrap_or(TEXT_MUTED),
                    bg: state.bg,
                    bold: state.bold,
                    layer: LogoLayer::Global,
                });
                index += ch_len;
            }
        }
    }

    if !current.cells.is_empty() || rows.is_empty() {
        rows.push(current);
    }

    rows
}

fn ansi_rows() -> &'static [LogoRow] {
    static ROWS: OnceLock<Vec<LogoRow>> = OnceLock::new();
    ROWS.get_or_init(|| parse_bracket_ansi_art(JEKKO_ANSI_ART))
}

fn row_is_blank(row: &LogoRow) -> bool {
    row.cells.iter().all(|cell| cell.ch == ' ')
}

fn trim_rows(rows: &[LogoRow]) -> &[LogoRow] {
    let Some(first) = rows.iter().position(|row| !row_is_blank(row)) else {
        return rows;
    };
    let last = rows
        .iter()
        .rposition(|row| !row_is_blank(row))
        .unwrap_or(first);
    &rows[first..=last]
}

fn mix_u8(left: u8, right: u8, t: f32) -> u8 {
    let t = t.clamp(0.0, 1.0);
    (left as f32 + (right as f32 - left as f32) * t).round() as u8
}

fn blend_colors(left: Color, right: Color, t: f32) -> Color {
    match (left, right) {
        (Color::Rgb(lr, lg, lb), Color::Rgb(rr, rg, rb)) => {
            Color::Rgb(mix_u8(lr, rr, t), mix_u8(lg, rg, t), mix_u8(lb, rb, t))
        }
        _ => right,
    }
}

fn color_luma(color: Color) -> f32 {
    match color {
        Color::Rgb(r, g, b) => (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0,
        _ => 0.5,
    }
}

fn theme_logo_color(color: Color, mode: ThemeMode, background: bool) -> Color {
    if mode == ThemeMode::Dark {
        return color;
    }

    let pal = theme::palette(mode);
    let target = if background { pal.surface } else { pal.text };
    let luma = color_luma(color);
    let blend = if background {
        if luma < 0.25 {
            0.78
        } else if luma < 0.5 {
            0.42
        } else {
            0.15
        }
    } else if luma < 0.25 {
        0.52
    } else if luma < 0.5 {
        0.24
    } else {
        0.08
    };
    blend_colors(color, target, blend)
}

fn theme_row(row: LogoRow, mode: ThemeMode) -> LogoRow {
    let cells = row
        .cells
        .into_iter()
        .map(|cell| LogoCell {
            fg: theme_logo_color(cell.fg, mode, false),
            bg: cell.bg.map(|bg| theme_logo_color(bg, mode, true)),
            ..cell
        })
        .collect();
    LogoRow { cells }
}

fn animate_row(row: LogoRow, mode: ThemeMode, tick: u64) -> LogoRow {
    let pal = theme::palette(mode);
    let cells = row
        .cells
        .into_iter()
        .enumerate()
        .map(|(x, cell)| {
            if cell.ch == ' ' {
                return cell;
            }

            let phase = ((tick as usize).wrapping_add(x)) % 12;
            let pulse = match phase {
                0 => 0.22,
                1 | 2 => 0.16,
                3 | 4 => 0.10,
                5..=7 => 0.05,
                _ => 0.0,
            };

            let mut fg = cell.fg;
            if pulse > 0.0 {
                fg = blend_colors(fg, pal.accent, pulse as f32);
            }

            LogoCell { fg, ..cell }
        })
        .collect();
    LogoRow { cells }
}

fn footer_rows(builder: &LogoBuilder, mode: ThemeMode) -> Vec<LogoRow> {
    let support = builder.support.unwrap_or("ZYAL");
    let status = builder.status.unwrap_or(if builder.idle {
        "camouflage idle • watching the wall"
    } else {
        "safe autonomous coding ready"
    });

    let mut rows = Vec::new();
    rows.push(row_from_text(
        fit(
            &format!("AI coding gecko • {support} support • climbs hard problems"),
            INNER_WIDTH,
            Alignment::Center,
        ),
        false,
        false,
        mode,
    ));
    rows.push(row_from_text(
        fit(&format!("gecko:// {status}"), INNER_WIDTH, Alignment::Center),
        false,
        builder.idle,
        mode,
    ));
    rows
}

fn build_logo_rows(builder: &LogoBuilder) -> Vec<LogoRow> {
    let mode = builder.mode.unwrap_or(ThemeMode::Dark);
    match builder.variant.unwrap_or(Logo::pick(Rect::new(0, 0, 80, 10))) {
        LogoVariant::Ascii => ascii_lines(mode)
            .into_iter()
            .map(line_to_row)
            .collect(),
        LogoVariant::Pixel => {
            let mut rows = trim_rows(ansi_rows())
                .iter()
                .cloned()
                .map(|row| theme_row(row, mode))
                .collect::<Vec<_>>();
            rows.extend(footer_rows(builder, mode));
            if let Some(tick) = builder.animation_tick {
                rows = rows
                    .into_iter()
                    .map(|row| animate_row(row, mode, tick))
                    .collect();
            }
            rows
        }
    }
}

fn line_to_row(line: Line<'static>) -> LogoRow {
    let mut cells = Vec::new();
    for span in line.spans {
        let fg = span.style.fg.unwrap_or(TEXT_MUTED);
        let bg = span.style.bg;
        let bold = span.style.add_modifier.contains(Modifier::BOLD);
        cells.extend(span.content.chars().map(|ch| LogoCell {
            ch,
            fg,
            bg,
            bold,
            layer: LogoLayer::Global,
        }));
    }
    LogoRow { cells }
}

fn max_width(rows: &[LogoRow]) -> usize {
    rows.iter().map(|row| row.cells.len()).max().unwrap_or(0)
}

fn render_rows(rows: &[LogoRow], area: Rect, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 || rows.is_empty() {
        return;
    }

    let source_width = max_width(rows).max(1);
    let source_height = rows.len().max(1);

    let width_scale = area.width as f32 / source_width as f32;
    let height_scale = area.height as f32 / source_height as f32;
    let scale = width_scale.min(height_scale).min(1.0);

    let target_width = ((source_width as f32 * scale).floor() as usize).max(1);
    let target_height = ((source_height as f32 * scale).floor() as usize).max(1);

    let x = area.x + (area.width.saturating_sub(target_width as u16)) / 2;
    let y = area.y + (area.height.saturating_sub(target_height as u16)) / 2;
    let render_area = Rect::new(x, y, target_width as u16, target_height as u16);

    let mut lines = Vec::with_capacity(target_height);
    for ty in 0..target_height {
        let sy = ty * source_height / target_height;
        let source_row = &rows[sy.min(rows.len().saturating_sub(1))];
        let mut spans = Vec::with_capacity(target_width);
        for tx in 0..target_width {
            let sx = tx * source_width / target_width;
            let cell = source_row
                .cells
                .get(sx.min(source_row.cells.len().saturating_sub(1)))
                .cloned()
                .unwrap_or(LogoCell {
                    ch: ' ',
                    fg: TEXT_MUTED,
                    bg: None,
                    bold: false,
                    layer: LogoLayer::Global,
                });
            let mut style = Style::default().fg(cell.fg);
            if let Some(bg) = cell.bg {
                style = style.bg(bg);
            }
            if cell.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            spans.push(Span::styled(cell.ch.to_string(), style));
        }
        lines.push(Line::from(spans));
    }

    Paragraph::new(lines).render(render_area, buf);
}

fn ascii_lines(mode: ThemeMode) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "╭──────────────────────────────────────────────────────────────────────────╮",
            Style::default().fg(if mode == ThemeMode::Light {
                AMBER_LIGHT_STOPS[1]
            } else {
                AMBER_STOPS[1]
            }),
        )),
        Line::from(Span::styled(
            "│  █▀▀█ █▀▀▀ █ █▀    █ █▀ █▀▀█                                              │",
            Style::default()
                .fg(if mode == ThemeMode::Light {
                    AMBER_LIGHT_STOPS[4]
                } else {
                    AMBER_STOPS[4]
                })
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "│  __█ █^^^ ██▀     ██▀  █__█                                              │",
            Style::default()
                .fg(if mode == ThemeMode::Light {
                    AMBER_LIGHT_STOPS[4]
                } else {
                    AMBER_STOPS[4]
                })
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "│  ▀▀▀  ▀▀▀▀ ▀ ▀▀    ▀ ▀▀ ▀▀▀▀                                             │",
            Style::default().fg(if mode == ThemeMode::Light {
                AMBER_LIGHT_STOPS[4]
            } else {
                AMBER_STOPS[4]
            }),
        )),
        Line::from(Span::styled(
            "╰──────────────────────────────────────────────────────────────────────────╯",
            Style::default().fg(if mode == ThemeMode::Light {
                AMBER_LIGHT_STOPS[1]
            } else {
                AMBER_STOPS[1]
            }),
        )),
    ]
}

fn amber_at(
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    light: bool,
    mode: ThemeMode,
) -> Color {
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
        if light || mode == ThemeMode::Light {
            &AMBER_LIGHT_STOPS
        } else {
            &AMBER_STOPS
        },
        t,
    )
}

fn color_stop(stops: &[Color], t: f32) -> Color {
    let idx = ((stops.len().saturating_sub(1)) as f32 * t).round() as usize;
    stops[idx.min(stops.len().saturating_sub(1))]
}

fn render_variant(variant: LogoVariant, builder: &LogoBuilder, area: Rect, buf: &mut Buffer) {
    let mode = builder.mode.unwrap_or(ThemeMode::Dark);
    match variant {
        LogoVariant::Pixel => render_rows(&build_logo_rows(builder), area, buf),
        LogoVariant::Ascii => Paragraph::new(ascii_lines(mode))
            .alignment(builder.alignment)
            .render(area, buf),
    }
}

impl Widget for &Logo {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let builder = LogoBuilder::new(None);
        let variant = Logo::pick(area);
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
    fn parses_historical_ansi_logo() {
        let rows = ansi_rows();
        assert!(rows.len() >= 10);
        let rendered = rows.iter().map(LogoRow::text).collect::<String>();
        assert!(rendered.contains('█') || rendered.contains('░') || rendered.contains('◆'));
    }

    #[test]
    fn logo_rows_include_footer_when_requested() {
        let rows = Logo::pixel()
            .with_support("ZYAL")
            .with_status("safe autonomous coding ready")
            .rows();
        assert!(rows.len() >= 12);
        let rendered = rows.iter().map(|row| row.text()).collect::<String>();
        assert!(rendered.contains("ZYAL"));
        assert!(rendered.contains("gecko:// safe autonomous coding ready"));
    }

    #[test]
    fn light_mode_rethemes_the_historical_art() {
        let dark = Logo::pixel().with_mode(ThemeMode::Dark).rows();
        let light = Logo::pixel().with_mode(ThemeMode::Light).rows();

        assert_ne!(dark[0].cells[0].fg, light[0].cells[0].fg);
        assert_ne!(dark[0].cells[0].bg, light[0].cells[0].bg);
    }

    #[test]
    fn animation_tick_changes_the_logo_palette() {
        let still = Logo::pixel().rows();
        let shimmer = Logo::pixel().with_animation_tick(1).rows();

        assert!(still.iter().zip(shimmer.iter()).any(|(a, b)| {
            a.cells
                .iter()
                .zip(&b.cells)
                .any(|(left, right)| left.fg != right.fg || left.bg != right.bg)
        }));
    }

    #[test]
    fn auto_picks_pixel_for_full_logo_area() {
        assert_eq!(Logo::pick(Rect::new(0, 0, 80, 10)), LogoVariant::Pixel);
    }

    #[test]
    fn auto_picks_ascii_for_small_area() {
        assert_eq!(Logo::pick(Rect::new(0, 0, 40, 4)), LogoVariant::Ascii);
    }
}
