//! Section: Theme
//!
//! Canonical color palette and shared panel-block factory.
//!
//! All pane renders (Reasoning, Inspector/Fusion, Composer) must use
//! `panel_block()` so focus state and border style are consistent.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Padding};

// ── Palette ─────────────────────────────────────────────────────────────────

pub const BG: Color = Color::Rgb(0x0b, 0x0f, 0x14);
pub const SURFACE: Color = Color::Rgb(0x11, 0x18, 0x20);
pub const SURFACE_ALT: Color = Color::Rgb(0x15, 0x1e, 0x28);
pub const BORDER: Color = Color::Rgb(0x26, 0x31, 0x3d);
pub const BORDER_FOCUSED: Color = Color::Rgb(0xf4, 0xc5, 0x42);
pub const TEXT: Color = Color::Rgb(0xd7, 0xde, 0xe8);
pub const TEXT_MUTED: Color = Color::Rgb(0x7a, 0x85, 0x94);
pub const ACCENT: Color = Color::Rgb(0xf4, 0xc5, 0x42);
pub const SUCCESS: Color = Color::Rgb(0x42, 0xd4, 0x7d);
pub const WARNING: Color = Color::Rgb(0xf5, 0xa5, 0x24);
pub const DANGER: Color = Color::Rgb(0xff, 0x5c, 0x5c);
pub const INFO: Color = Color::Rgb(0x55, 0xd6, 0xff);

// ── panel_block ──────────────────────────────────────────────────────────────

/// Build a rounded-border block with a title and optional right-side status
/// text. Focused panes use `BORDER_FOCUSED` (amber); unfocused use `BORDER`.
///
/// ```text
/// ╭─ Title ──────────────────────── status ─╮
/// │ content                                 │
/// ╰─────────────────────────────────────────╯
/// ```
pub fn panel_block<'a>(title: &'a str, status: Option<&'a str>, focused: bool) -> Block<'a> {
    let border_color = if focused { BORDER_FOCUSED } else { BORDER };
    let title_color = if focused { ACCENT } else { TEXT };
    let title_style = Style::default()
        .fg(title_color)
        .add_modifier(Modifier::BOLD);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::horizontal(1));

    match status {
        Some(s) => block
            .title(Line::from(Span::styled(format!(" {title} "), title_style)))
            .title_bottom(Line::from(Span::styled(
                format!(" {s} "),
                Style::default().fg(TEXT_MUTED),
            ))),
        None => block.title(Line::from(Span::styled(format!(" {title} "), title_style))),
    }
}

/// Borderless header-row style for status/nav lines that use BG as their
/// background (no border, no padding — callers use the full row height).
pub fn header_style() -> Style {
    Style::default().fg(TEXT).bg(BG)
}

/// Generate a 3-dot animated activity indicator.
///
/// Each dot cycles through a gradient from dark amber → bright amber → white
/// and back, with a phase offset of ~120° per dot so they shimmer left-to-right.
/// `tick` is the monotonic frame counter from the app loop (~60 fps).
///
/// Returns a formatted string with ANSI-free text — the caller wraps it in
/// styled `Span`s. For simplicity in panel_block's title API (which takes
/// `&str`), we return dot chars and let the caller do per-dot coloring via
/// the `activity_dot_spans` helper.
pub fn activity_dot_spans(tick: u64) -> Line<'static> {
    // Cycle period in ticks (~60fps → ~1.5s full cycle = 90 ticks).
    let period: f64 = 90.0;
    let phase_offset: f64 = period / 3.0; // 120° between dots

    let dots: Vec<Span<'static>> = (0..3)
        .map(|i| {
            let phase = ((tick as f64 + i as f64 * phase_offset) % period) / period;
            // Triangle wave: 0→1→0 over one period.
            let brightness = 1.0 - (2.0 * phase - 1.0).abs();
            let color = interpolate_amber(brightness);
            Span::styled("●", Style::default().fg(color))
        })
        .collect();

    let mut spans = vec![Span::raw(" ")];
    for (i, dot) in dots.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(dot);
    }
    spans.push(Span::raw(" "));
    Line::from(spans)
}

/// Interpolate between dark amber and bright amber/white.
/// `t` in 0.0..1.0 where 0 = dark, 1 = bright.
fn interpolate_amber(t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    // Dark amber: (0x3a, 0x2a, 0x0a)
    // ACCENT amber: (0xf4, 0xc5, 0x42)
    // Bright: (0xff, 0xef, 0xc0)
    let (r, g, b) = if t < 0.5 {
        // Dark → amber (first half)
        let s = t * 2.0;
        (
            lerp_u8(0x3a, 0xf4, s),
            lerp_u8(0x2a, 0xc5, s),
            lerp_u8(0x0a, 0x42, s),
        )
    } else {
        // Amber → bright (second half)
        let s = (t - 0.5) * 2.0;
        (
            lerp_u8(0xf4, 0xff, s),
            lerp_u8(0xc5, 0xef, s),
            lerp_u8(0x42, 0xc0, s),
        )
    };
    Color::Rgb(r, g, b)
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    let result = a as f64 + (b as f64 - a as f64) * t;
    result.round().clamp(0.0, 255.0) as u8
}
