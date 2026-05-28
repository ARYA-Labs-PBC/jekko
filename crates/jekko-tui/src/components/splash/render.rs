use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::theme::codex;

use super::SplashContext;

/// 6x38 block-glyph "JEKKO" wordmark. Each entry is one row.
const WORDMARK: &[&str] = &[
    "    ██╗███████╗██╗  ██╗██╗  ██╗ ██████╗ ",
    "    ██║██╔════╝██║ ██╔╝██║ ██╔╝██╔═══██╗",
    "    ██║█████╗  █████╔╝ █████╔╝ ██║   ██║",
    "██  ██║██╔══╝  ██╔═██╗ ██╔═██╗ ██║   ██║",
    "╚█████╔╝███████╗██║  ██╗██║  ██╗╚██████╔╝",
    " ╚════╝ ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝ ",
];

/// Static multi-stop gradient used by the wordmark. These are deliberately
/// high-chroma terminal-safe RGB values; the logo layout is owned by
/// [`WORDMARK`] and must not be adjusted here.
const LOGO_GRADIENT: &[Color] = &[
    Color::Rgb(0x00, 0xf5, 0xff),
    Color::Rgb(0x39, 0x8b, 0xff),
    Color::Rgb(0x7c, 0x4d, 0xff),
    Color::Rgb(0xff, 0x4f, 0xc8),
    Color::Rgb(0xff, 0xb8, 0x2e),
];

/// Vertical padding above the wordmark while the splash is visible. Drops
/// to zero once content starts flowing (splash is dismissed) so the
/// scrollback isn't offset.
const SPLASH_TOP_PAD: u16 = 2;

/// Left padding before each wordmark row, so the logo doesn't kiss the
/// terminal edge.
const SPLASH_LEFT_PAD: &str = "  ";

/// Number of rows the splash occupies when rendered.
pub const SPLASH_ROW_COUNT: u16 = WORDMARK.len() as u16 + SPLASH_TOP_PAD + 1;

/// Build the splash lines for emission into transcript scrollback.
pub fn snapshot_lines(ctx: &SplashContext, width: u16) -> Vec<Line<'static>> {
    build_lines(ctx, width)
}

/// Render the splash widget into `area`. The elapsed/config arguments remain
/// in the signature for call-site compatibility, but the logo itself is static:
/// no shimmer, no pulse, no redraw-only animation.
pub fn render_splash(
    buf: &mut Buffer,
    area: Rect,
    _elapsed: Duration,
    ctx: &SplashContext,
    _cfg: Option<&jekko_core::config::ui::UiConfig>,
) {
    render_splash_static_for_tests(buf, area, ctx);
}

/// Internal seam used by unit tests to inspect the static wordmark directly.
pub(crate) fn render_splash_static_for_tests(buf: &mut Buffer, area: Rect, ctx: &SplashContext) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let lines = build_lines(ctx, area.width);
    let total_h = lines.len() as u16;
    if total_h > area.height {
        return;
    }

    let render_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: total_h,
    };

    Paragraph::new(lines).render(render_area, buf);
}

fn build_lines(ctx: &SplashContext, width: u16) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> =
        Vec::with_capacity(WORDMARK.len() + SPLASH_TOP_PAD as usize + 1);

    for _ in 0..SPLASH_TOP_PAD {
        lines.push(Line::from(Span::raw("")));
    }

    for (row_idx, row) in WORDMARK.iter().enumerate() {
        let pad = Span::styled(SPLASH_LEFT_PAD, Style::default());
        let mut spans = Vec::with_capacity(row.chars().count() + 1);
        spans.push(pad);
        spans.extend(gradient_spans(row_idx, row));
        lines.push(Line::from(spans));
    }

    let subtitle = format!(
        "v{} · {} · {}",
        ctx.version,
        ctx.cwd,
        ctx.branch.as_deref().unwrap_or("(no git)")
    );
    let subtitle_width = subtitle.chars().count() as u16;
    let subtitle_left_pad = width.saturating_sub(subtitle_width) / 2;
    let subtitle_style = Style::default().fg(codex::FG_DIM);
    lines.push(Line::from(vec![
        Span::raw(" ".repeat(subtitle_left_pad as usize)),
        Span::styled(subtitle, subtitle_style),
    ]));

    lines
}

fn gradient_spans(row_idx: usize, row: &str) -> Vec<Span<'static>> {
    let row_width = row.chars().count().saturating_sub(1).max(1);
    row.chars()
        .enumerate()
        .map(|(col_idx, ch)| {
            if ch == ' ' {
                return Span::raw(ch.to_string());
            }
            let color = gradient_color(row_idx, col_idx, row_width);
            Span::styled(
                ch.to_string(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )
        })
        .collect()
}

fn gradient_color(row_idx: usize, col_idx: usize, row_width: usize) -> Color {
    let x = col_idx as f32 / row_width as f32;
    let y = row_idx as f32 / (WORDMARK.len().saturating_sub(1).max(1) as f32);
    let t = (x * 0.78 + y * 0.22).clamp(0.0, 1.0);
    let scaled = t * (LOGO_GRADIENT.len() - 1) as f32;
    let left = scaled.floor() as usize;
    let right = (left + 1).min(LOGO_GRADIENT.len() - 1);
    let local_t = scaled - left as f32;
    blend(LOGO_GRADIENT[left], LOGO_GRADIENT[right], local_t)
}

fn blend(from: Color, to: Color, t: f32) -> Color {
    match (from, to) {
        (Color::Rgb(fr, fg, fb), Color::Rgb(tr, tg, tb)) => Color::Rgb(
            blend_channel(fr, tr, t),
            blend_channel(fg, tg, t),
            blend_channel(fb, tb, t),
        ),
        _ => from,
    }
}

fn blend_channel(from: u8, to: u8, t: f32) -> u8 {
    let from = from as f32;
    let to = to as f32;
    (from + (to - from) * t).round().clamp(0.0, 255.0) as u8
}
