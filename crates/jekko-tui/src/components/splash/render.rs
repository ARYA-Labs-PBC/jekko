use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::anim;
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

/// Animation params. Period 2s (0.5 Hz) reads as a slow breath; per-row offset
/// of 100 ms produces a downward gradient shimmer.
const PULSE_HZ: f32 = 0.5;
const ROW_PHASE_OFFSET: Duration = Duration::from_millis(100);

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
    build_lines(Duration::ZERO, ctx, false, width)
}

/// Render the splash widget into `area` for the given `elapsed` instant. Pure
/// function; same inputs produce the same output.
pub fn render_splash(
    buf: &mut Buffer,
    area: Rect,
    elapsed: Duration,
    ctx: &SplashContext,
    cfg: Option<&jekko_core::config::ui::UiConfig>,
) {
    render_splash_with_motion(buf, area, elapsed, ctx, anim::motion_enabled_with_cfg(cfg));
}

/// Internal seam used by unit tests to pin the reduced-motion branch without
/// relying on `anim::motion_enabled`'s global cache.
pub(crate) fn render_splash_with_motion(
    buf: &mut Buffer,
    area: Rect,
    elapsed: Duration,
    ctx: &SplashContext,
    motion: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let lines = build_lines(elapsed, ctx, motion, area.width);
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

fn build_lines(
    elapsed: Duration,
    ctx: &SplashContext,
    motion: bool,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> =
        Vec::with_capacity(WORDMARK.len() + SPLASH_TOP_PAD as usize + 1);

    for _ in 0..SPLASH_TOP_PAD {
        lines.push(Line::from(Span::raw("")));
    }

    for (row_idx, row) in WORDMARK.iter().enumerate() {
        let row_elapsed = elapsed + ROW_PHASE_OFFSET * row_idx as u32;
        let color = oscillate(row_elapsed, motion);
        let wordmark_style = Style::default().fg(color).add_modifier(Modifier::BOLD);
        let pad = Span::styled(SPLASH_LEFT_PAD, Style::default());
        lines.push(Line::from(vec![
            pad,
            Span::styled((*row).to_string(), wordmark_style),
        ]));
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

fn oscillate(elapsed: Duration, motion: bool) -> Color {
    if !motion {
        return codex::BLUE_PATH;
    }
    anim::oscillate_color(elapsed, PULSE_HZ, codex::BLUE_PATH, codex::CYAN_TAB)
}
