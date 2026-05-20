//! Minimal animated startup splash (COWBOY T1-V6, new scope).
//!
//! Pure-function renderer for the boot-time "JEKKO" wordmark + subtitle. The
//! runtime calls [`render_splash`] each frame during startup and stops calling
//! it once the user submits the first prompt. There is no internal state and no
//! self-dismiss logic — the lifecycle is the runtime's job (T1-V6b, blocked on
//! T2-P1).
//!
//! ## Layout
//!
//! Roughly ten rows tall, horizontally centered inside the supplied [`Rect`]:
//!
//! ```text
//!     ██╗███████╗██╗  ██╗██╗  ██╗ ██████╗
//!     ██║██╔════╝██║ ██╔╝██║ ██╔╝██╔═══██╗
//!     ██║█████╗  █████╔╝ █████╔╝ ██║   ██║
//! ██  ██║██╔══╝  ██╔═██╗ ██╔═██╗ ██║   ██║
//! ╚█████╔╝███████╗██║  ██╗██║  ██╗╚██████╔╝
//!  ╚════╝ ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝
//!
//!         v0.1.0 · ~/code/jekko · main
//! ```
//!
//! ## Animation
//!
//! Each wordmark row pulses with [`anim::oscillate_color`] at 0.5 Hz between
//! [`theme::codex::BLUE_PATH`] (#6eb1ff) and [`theme::codex::CYAN_TAB`]
//! (#4ed1d1) — one full cycle every two seconds. Rows are phase-staggered by
//! 100 ms per row so the gradient "shimmers" downward instead of pulsing as a
//! block.
//!
//! When reduced-motion mode is active ([`anim::motion_enabled`] returns
//! `false`), `oscillate_color` collapses to the `from` color (BLUE_PATH) and
//! the whole wordmark renders as a flat blue block.
//!
//! ## Subtitle
//!
//! A single dim-gray line below the wordmark carries `v<version> ·
//! <cwd_display> · <branch>`. Same fields as
//! [`crate::components::boot_inline::BootContext::detect`].

use std::env;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::anim;
use crate::theme::codex;

/// 6×38 block-glyph "JEKKO" wordmark. Each entry is one row.
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

/// Snapshot of the workspace data shown in the splash subtitle. Cheap to
/// build via [`SplashContext::detect`] — read once in the runtime, then pass
/// into [`render_splash`] each frame.
#[derive(Clone, Debug)]
pub struct SplashContext {
    /// Crate version (e.g. `"0.1.0"`). Defaults to `CARGO_PKG_VERSION`;
    /// `JEKKO_VERSION_OVERRIDE` wins for downstream packaging.
    pub version: String,
    /// `~`-relative cwd display (e.g. `"~/code/jekko"`). Falls back to the
    /// absolute path when `$HOME` is not a prefix of the cwd.
    pub cwd: String,
    /// Active git branch when the cwd is inside a repo and `git` is on PATH.
    pub branch: Option<String>,
}

impl SplashContext {
    /// Build context from the environment. Never returns `Err` — missing data
    /// degrades gracefully (empty branch when not in a repo, absolute cwd when
    /// `$HOME` is unset, etc.). Mirrors
    /// [`crate::components::boot_inline::BootContext::detect`].
    pub fn detect() -> Self {
        let version = env::var("JEKKO_VERSION_OVERRIDE")
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
        let cwd = current_cwd_display();
        let branch = current_git_branch();
        Self {
            version,
            cwd,
            branch,
        }
    }
}

/// Vertical padding above the wordmark while the splash is visible. Drops
/// to zero once content starts flowing (splash is dismissed) so the
/// scrollback isn't offset.
const SPLASH_TOP_PAD: u16 = 2;

/// Left padding before each wordmark row, so the logo doesn't kiss the
/// terminal edge.
const SPLASH_LEFT_PAD: &str = "  ";

/// Build the splash lines for emission into transcript scrollback. Used at
/// the dismiss moment (first prompt submit) so the wordmark scrolls UP into
/// history instead of being replaced by an empty / "welcome" block. Static
/// snapshot — reduced-motion render so the persisted history doesn't carry
/// an arbitrary-time oscillation frame.
pub fn snapshot_lines(ctx: &SplashContext, width: u16) -> Vec<Line<'static>> {
    build_lines(Duration::ZERO, ctx, false, width)
}

/// Number of rows the splash occupies when rendered. Used by the layout
/// planner to size the transcript slot in growing-bottom mode (composer
/// follows splash bottom; blank terminal rows below until real content
/// fills the screen).
pub const SPLASH_ROW_COUNT: u16 = WORDMARK.len() as u16 + SPLASH_TOP_PAD + 1;

/// Render the splash widget into `area` for the given `elapsed` instant. Pure
/// function — same inputs produce the same output.
///
/// T-GLYPH-WAVE3: `cfg` threads a resolved
/// [`jekko_core::config::ui::UiConfig`] through so the per-row oscillation
/// honors `UiConfig.accessibility.reduced_motion` from the CLI overlay rather
/// than re-reading env/file via the legacy [`anim::motion_enabled`] cache.
/// When `cfg` is `None` we fall back to the env/file path so unwired call
/// sites keep working unchanged.
pub fn render_splash(
    buf: &mut Buffer,
    area: Rect,
    elapsed: Duration,
    ctx: &SplashContext,
    cfg: Option<&jekko_core::config::ui::UiConfig>,
) {
    render_splash_with_motion(buf, area, elapsed, ctx, anim::motion_enabled_with_cfg(cfg));
}

/// Internal seam used by the unit tests so they can pin the reduced-motion
/// branch without relying on `anim::motion_enabled`'s [`std::sync::OnceLock`]
/// cache (which can't be reset between tests). Production callers go through
/// [`render_splash`].
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
        // Too short to fit the full splash. Drop trailing rows rather than
        // partial-write a clipped wordmark — keeps rendering deterministic.
        return;
    }

    // Anchor top-left of the transcript area. No vertical or horizontal
    // padding — splash sits directly above the composer chrome.
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

    // 2-row top breathing room. Drops to zero once the splash is dismissed
    // (transcript replaces the splash render entirely, so these padding rows
    // don't follow scrollback content up the screen).
    for _ in 0..SPLASH_TOP_PAD {
        lines.push(Line::from(Span::raw("")));
    }

    for (row_idx, row) in WORDMARK.iter().enumerate() {
        let row_elapsed = elapsed + ROW_PHASE_OFFSET * row_idx as u32;
        let color = oscillate(row_elapsed, motion);
        let wordmark_style = Style::default().fg(color).add_modifier(Modifier::BOLD);
        // 2-cell left padding so the logo doesn't kiss the terminal edge.
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

/// Compute the oscillating color for the current frame. Centralised so the
/// reduced-motion bypass uses one code path.
fn oscillate(elapsed: Duration, motion: bool) -> Color {
    if !motion {
        return codex::BLUE_PATH;
    }
    anim::oscillate_color(elapsed, PULSE_HZ, codex::BLUE_PATH, codex::CYAN_TAB)
}

// ── helpers (mirror `boot_inline.rs`) ────────────────────────────────────────

fn current_cwd_display() -> String {
    let cwd = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    if let Some(home) = env::var_os("HOME") {
        let home_path = Path::new(&home);
        if let Ok(rel) = cwd.strip_prefix(home_path) {
            if rel.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", rel.display());
        }
    }
    cwd.display().to_string()
}

fn current_git_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8(output.stdout).ok()?;
    let trimmed = s.trim().to_string();
    if trimmed.is_empty() || trimmed == "HEAD" {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn fixture_ctx() -> SplashContext {
        SplashContext {
            version: "1.2.3".into(),
            cwd: "~/code/jekko".into(),
            branch: Some("main".into()),
        }
    }

    fn fresh_buffer(width: u16, height: u16) -> Buffer {
        Buffer::empty(Rect::new(0, 0, width, height))
    }

    fn buffer_to_symbol_string(buf: &Buffer) -> String {
        let area = buf.area();
        let mut out = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn render_splash_fits_within_area_bounds() {
        // The renderer must never panic or write outside the supplied area at
        // common terminal sizes. Catch-unwind would mask real bugs here; rely
        // on the buffer's bounds-checked indexing instead.
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = fresh_buffer(80, 24);
        let ctx = fixture_ctx();
        render_splash(&mut buf, area, Duration::ZERO, &ctx, None);
        // Spot-check: at least one cell inside `area` was written and no
        // out-of-bounds write happened (indexing would have panicked).
        let any_glyph = (0..area.height)
            .any(|y| (0..area.width).any(|x| !buf[(x, y)].symbol().chars().all(|c| c == ' ')));
        assert!(any_glyph, "expected splash to emit at least one glyph");
    }

    #[test]
    fn render_splash_emits_wordmark() {
        // The block character `█` is the single most prevalent glyph in the
        // wordmark — its presence is a stable contract regardless of the
        // exact column layout we land on.
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = fresh_buffer(80, 24);
        let ctx = fixture_ctx();
        render_splash(&mut buf, area, Duration::ZERO, &ctx, None);
        let dump = buffer_to_symbol_string(&buf);
        assert!(
            dump.contains('█'),
            "expected wordmark block glyph, got:\n{dump}"
        );
    }

    #[test]
    fn render_splash_emits_subtitle() {
        // Subtitle carries version + cwd + branch so the splash is a useful
        // boot-time workspace signal.
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = fresh_buffer(80, 24);
        let ctx = fixture_ctx();
        render_splash(&mut buf, area, Duration::ZERO, &ctx, None);
        let dump = buffer_to_symbol_string(&buf);
        assert!(dump.contains("v1.2.3"), "expected version, got:\n{dump}");
        assert!(dump.contains("~/code/jekko"), "expected cwd, got:\n{dump}");
        assert!(dump.contains("main"), "expected branch, got:\n{dump}");
    }

    #[test]
    fn render_splash_animates_when_motion_enabled() {
        // Sample at two timestamps a quarter-period apart (250 ms within the
        // 2 s period) — the oscillator's sine wave guarantees a meaningful
        // color delta on at least one wordmark cell.
        let area = Rect::new(0, 0, 80, 24);
        let ctx = fixture_ctx();

        let mut buf_a = fresh_buffer(80, 24);
        render_splash_with_motion(&mut buf_a, area, Duration::ZERO, &ctx, true);

        let mut buf_b = fresh_buffer(80, 24);
        render_splash_with_motion(&mut buf_b, area, Duration::from_millis(500), &ctx, true);

        let mut diff_found = false;
        for y in 0..area.height {
            for x in 0..area.width {
                if buf_a[(x, y)].style().fg != buf_b[(x, y)].style().fg {
                    diff_found = true;
                    break;
                }
            }
            if diff_found {
                break;
            }
        }
        assert!(
            diff_found,
            "expected at least one cell color to change between frames"
        );
    }

    #[test]
    fn render_splash_static_when_motion_disabled() {
        // Pin the reduced-motion contract: both samples must produce
        // byte-identical color buffers when `motion = false`. We thread the
        // flag explicitly to bypass `anim::motion_enabled`'s OnceLock cache,
        // which can't be reset between tests (see module docstring).
        let area = Rect::new(0, 0, 80, 24);
        let ctx = fixture_ctx();

        let mut buf_a = fresh_buffer(80, 24);
        render_splash_with_motion(&mut buf_a, area, Duration::ZERO, &ctx, false);

        let mut buf_b = fresh_buffer(80, 24);
        render_splash_with_motion(&mut buf_b, area, Duration::from_millis(1_000), &ctx, false);

        for y in 0..area.height {
            for x in 0..area.width {
                assert_eq!(
                    buf_a[(x, y)].style().fg,
                    buf_b[(x, y)].style().fg,
                    "expected flat color when reduced-motion is active (cell {x},{y})"
                );
            }
        }
    }

    #[test]
    fn render_splash_static_uses_blue_path_color() {
        // Make the reduced-motion fill color explicit so a future refactor
        // can't silently swap to CYAN_TAB or some new accent.
        let area = Rect::new(0, 0, 80, 24);
        let ctx = fixture_ctx();
        let mut buf = fresh_buffer(80, 24);
        render_splash_with_motion(&mut buf, area, Duration::ZERO, &ctx, false);

        let mut saw_blue = false;
        for y in 0..area.height {
            for x in 0..area.width {
                if buf[(x, y)].symbol() == "█" {
                    assert_eq!(
                        buf[(x, y)].style().fg,
                        Some(codex::BLUE_PATH),
                        "wordmark cell at ({x},{y}) must be BLUE_PATH in reduced motion"
                    );
                    saw_blue = true;
                }
            }
        }
        assert!(saw_blue, "expected at least one block glyph in dump");
    }

    #[test]
    fn render_splash_no_op_when_area_too_short() {
        // Three rows isn't enough to fit the 6-row wordmark + subtitle. The
        // renderer must bail rather than partial-write a clipped graphic.
        let area = Rect::new(0, 0, 80, 3);
        let mut buf = fresh_buffer(80, 3);
        let ctx = fixture_ctx();
        render_splash(&mut buf, area, Duration::ZERO, &ctx, None);
        let dump = buffer_to_symbol_string(&buf);
        assert!(
            !dump.contains('█') && !dump.contains("v1.2.3"),
            "expected empty render when area is too short, got:\n{dump}"
        );
    }

    #[test]
    fn splash_context_detect_returns_version() {
        // Smoke test for the public detect() — never panics, version is
        // non-empty.
        let ctx = SplashContext::detect();
        assert!(!ctx.version.is_empty(), "version must be non-empty");
    }

    #[test]
    fn splash_context_omits_branch_when_unknown() {
        // Construct a context with no branch and assert the subtitle line
        // still carries the cwd while rendering the explicit fallback label.
        // Guards the public optional-branch contract.
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = fresh_buffer(80, 24);
        let ctx = SplashContext {
            version: "9.9.9".into(),
            cwd: "/tmp".into(),
            branch: None,
        };
        render_splash(&mut buf, area, Duration::ZERO, &ctx, None);
        let dump = buffer_to_symbol_string(&buf);
        assert!(dump.contains("v9.9.9"));
        assert!(
            dump.contains("/tmp"),
            "cwd should appear in splash:\n{dump}"
        );
        assert!(
            dump.contains("(no git)"),
            "branch fallback should appear when absent:\n{dump}"
        );
    }
}
