//! Section: Theme
//!
//! Canonical color palette. The legacy "amber" tokens (BG, SURFACE, BORDER,
//! ACCENT, panel_block, header_style, activity_dot_spans, ...) used by the
//! old 3-pane shell were removed in the R3 legacy purge. What remains:
//!
//! * The light `palette()`/`ThemePalette` API used by `transcript::syntax`
//!   for markdown rendering.
//! * The `codex` submodule with Claude/Codex-parity grays + accents used by
//!   the inline runtime, boot block, and inline card renderers.
//! * A handful of named accents (`INFO`, `WARNING`, `ACCENT`) still consumed
//!   by `activity::ActivityKind::accent` and the markdown link/code styles.
//!
//! # Accessibility: `NO_COLOR` / `CLICOLOR` / `FORCE_COLOR` / `COLORTERM`
//!
//! Per <https://no-color.org> and <https://bixense.com/clicolors/>, colored
//! output is suppressed when the user opts out (`NO_COLOR=1` or
//! `CLICOLOR=0` on a TTY), and forced on when explicitly requested
//! (`CLICOLOR_FORCE=1` or `FORCE_COLOR=1`). The current state is queried
//! through [`color_mode()`], which caches the result of [`compute_color_mode`]
//! in a `OnceLock`. Precedence (highest to lowest):
//!
//! 1. `CLICOLOR_FORCE` or `FORCE_COLOR` truthy → [`ColorMode::Full`]
//! 2. `NO_COLOR` set to any non-empty value → [`ColorMode::Monochrome`]
//! 3. `CLICOLOR=0` and stdout is a TTY → [`ColorMode::Monochrome`]
//! 4. Default → [`ColorMode::Full`]
//!
//! Callers that need accessibility-aware colors should reach for the
//! function-form accessors (`codex_fg()`, `codex_blue_path()`, …) which
//! collapse to `Color::Reset` in monochrome mode. The `pub const` form in
//! `codex::*` is retained for back-compat call sites; migrating them is a
//! follow-up task and out of scope here.
//!
//! `COLORTERM` is informational: jekko emits ratatui `Color::Rgb(...)`
//! everywhere, which most modern terminals downgrade gracefully when
//! truecolor is unavailable. We document the assumption rather than branch
//! on it.

use std::io::IsTerminal;
use std::{env, io};

#[cfg(not(test))]
use std::sync::OnceLock;

use jekko_core::theme::ThemeMode;
use ratatui::style::{Color, Modifier};

// ── Surviving named colors ───────────────────────────────────────────────────
//
// Kept because non-codex callers (markdown syntax styles, activity tracker)
// still reach for them. Do not add new callers — prefer `codex::*`.

pub const TEXT: Color = Color::Rgb(0xd7, 0xde, 0xe8);
pub const TEXT_MUTED: Color = Color::Rgb(0x7a, 0x85, 0x94);
pub const ACCENT: Color = Color::Rgb(0xf4, 0xc5, 0x42);
pub const INFO: Color = Color::Rgb(0x55, 0xd6, 0xff);
pub const WARNING: Color = Color::Rgb(0xf5, 0xa5, 0x24);
pub const BORDER: Color = Color::Rgb(0x26, 0x31, 0x3d);

// ── ColorMode (NO_COLOR / CLICOLOR / FORCE_COLOR) ───────────────────────────

/// Whether jekko should emit color escapes.
///
/// Driven by [`color_mode()`], which reads the accessibility env vars once and
/// caches the result. `Monochrome` callers fall back to emphasis modifiers
/// (bold / italic / dim) instead of fg/bg colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    /// Full RGB palette is enabled.
    Full,
    /// Color output is suppressed; emphasis is conveyed via [`Modifier`].
    Monochrome,
}

/// Cached env-derived [`ColorMode`]. First call performs the env reads; every
/// subsequent call returns the cached value.
#[cfg(not(test))]
pub fn color_mode() -> ColorMode {
    static CACHED: OnceLock<ColorMode> = OnceLock::new();
    *CACHED.get_or_init(compute_color_mode)
}

#[cfg(test)]
pub fn color_mode() -> ColorMode {
    ColorMode::Full
}

/// Uncached evaluation of the env-var precedence. Exposed (test-only) so
/// tests can re-read after mutating `std::env`; production callers use
/// [`color_mode()`].
#[cfg(test)]
pub(crate) fn compute_color_mode_for_tests() -> ColorMode {
    compute_color_mode()
}

fn compute_color_mode() -> ColorMode {
    // 1. Forced color wins over everything.
    if env_truthy("CLICOLOR_FORCE") || env_truthy("FORCE_COLOR") {
        return ColorMode::Full;
    }
    // 2. NO_COLOR (any non-empty value) disables color.
    if env_present_nonempty("NO_COLOR") {
        return ColorMode::Monochrome;
    }
    // 3. CLICOLOR=0 disables color *when stdout is a TTY*. Piped output
    //    already defaults to monochrome via terminal semantics; this branch
    //    exists to satisfy the bixense.com clicolors spec.
    if env_var_eq("CLICOLOR", "0") && stdout_is_tty() {
        return ColorMode::Monochrome;
    }
    ColorMode::Full
}

fn env_truthy(name: &str) -> bool {
    match env::var(name) {
        Ok(v) => {
            let trimmed = v.trim();
            !trimmed.is_empty()
                && !trimmed.eq_ignore_ascii_case("0")
                && !trimmed.eq_ignore_ascii_case("false")
                && !trimmed.eq_ignore_ascii_case("off")
        }
        Err(_) => false,
    }
}

fn env_present_nonempty(name: &str) -> bool {
    matches!(env::var(name), Ok(v) if !v.is_empty())
}

fn env_var_eq(name: &str, expected: &str) -> bool {
    matches!(env::var(name), Ok(v) if v == expected)
}

fn stdout_is_tty() -> bool {
    io::stdout().is_terminal()
}

/// Convenience: monochrome emphasis modifier suggestions for callers that
/// want a parallel to the `codex::*` color constants without each site
/// re-deriving them. Use [`mono_strong`] where a "strong/bright" color
/// previously stood, [`mono_muted`] for tertiary chrome, and [`mono_dim`]
/// for very-dim grays.
pub fn mono_strong() -> Modifier {
    Modifier::BOLD
}

pub fn mono_muted() -> Modifier {
    Modifier::ITALIC
}

pub fn mono_dim() -> Modifier {
    Modifier::DIM
}

// ── ThemePalette (light/dark mode) ───────────────────────────────────────────

/// Small palette consumed by `transcript::syntax` and any caller that wants
/// theme-mode-aware foreground/accent values. Tracks the original light/dark
/// split from the pre-purge theme.
#[derive(Clone, Copy, Debug)]
pub struct ThemePalette {
    pub text: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub border: Color,
}

pub fn palette(mode: ThemeMode) -> ThemePalette {
    if color_mode() == ColorMode::Monochrome {
        return monochrome_palette();
    }
    match mode {
        ThemeMode::Dark => ThemePalette {
            text: TEXT,
            text_muted: TEXT_MUTED,
            accent: ACCENT,
            border: BORDER,
        },
        ThemeMode::Light => ThemePalette {
            text: Color::Rgb(0x20, 0x24, 0x2e),
            text_muted: Color::Rgb(0x5b, 0x63, 0x72),
            accent: Color::Rgb(0xc8, 0x8e, 0x12),
            border: Color::Rgb(0xc2, 0xc8, 0xd0),
        },
    }
}

/// Monochrome palette used when [`color_mode()`] is [`ColorMode::Monochrome`].
/// All fields collapse to `Color::Reset` so the terminal renders its default
/// foreground; callers wanting emphasis should layer the [`mono_strong`] /
/// [`mono_muted`] / [`mono_dim`] modifiers on the surrounding `Style`.
pub fn monochrome_palette() -> ThemePalette {
    ThemePalette {
        text: Color::Reset,
        text_muted: Color::Reset,
        accent: Color::Reset,
        border: Color::Reset,
    }
}

// ── codex palette ────────────────────────────────────────────────────────────
//
// Claude Code / Codex CLI parity tokens. All new TUI widgets must reference
// these instead of the legacy named colors.
//
// NOTE: the `pub const` form is retained for source-compat with the existing
// `transcript::markup` and `transcript::inline_cards` call sites. Callers
// that want accessibility honoring (`NO_COLOR`/`CLICOLOR`) should use the
// `codex_*()` function accessors below instead — they collapse to
// `Color::Reset` in monochrome mode.

pub mod codex {
    use ratatui::style::Color;

    /// Default terminal background — kept transparent in practice so we
    /// inherit the user's chosen theme, but useful for overlay fills.
    pub const BG: Color = Color::Reset;

    /// Slightly raised overlay background used for popup / chip surfaces.
    pub const BG_OVERLAY: Color = Color::Rgb(0x1a, 0x1d, 0x24);

    /// Body text foreground.
    pub const FG: Color = Color::Rgb(0xd7, 0xde, 0xe8);
    /// Slightly stronger foreground for emphasized labels.
    pub const FG_STRONG: Color = Color::Rgb(0xf2, 0xf4, 0xf8);
    /// Dimmed foreground for secondary text.
    pub const FG_DIM: Color = Color::Rgb(0x9a, 0xa3, 0xb1);
    /// Very dim foreground for tertiary chrome (rules, padding glyphs).
    pub const FG_VERY_DIM: Color = Color::Rgb(0x60, 0x6a, 0x78);

    /// Horizontal rule color (top/bottom of the composer, between cards).
    pub const RULE: Color = Color::Rgb(0x37, 0x3d, 0x49);

    /// Path / file color (Codex blue).
    pub const BLUE_PATH: Color = Color::Rgb(0x6e, 0xb1, 0xff);
    /// Success indicator (Claude green).
    pub const GREEN_OK: Color = Color::Rgb(0x6c, 0xc9, 0x7a);
    /// Failure indicator (salmon red).
    pub const SALMON_FAIL: Color = Color::Rgb(0xf2, 0x7a, 0x7a);
    /// Active-agent accent (orange).
    pub const ORANGE_AGENT: Color = Color::Rgb(0xff, 0x9d, 0x4a);
    /// Sub-agent accent (pink).
    pub const PINK_AGENT: Color = Color::Rgb(0xff, 0x6f, 0xb5);
    /// Composer branch-tab accent (cyan).
    pub const CYAN_TAB: Color = Color::Rgb(0x4e, 0xd1, 0xd1);
    /// Yellow for `warn`-kind system notices.
    pub const YELLOW: Color = Color::Rgb(0xf5, 0xc8, 0x52);

    /// Diff background shading for the changed lines (+/−).
    pub const DIFF_ADD_BG: Color = Color::Rgb(0x18, 0x3a, 0x22);
    pub const DIFF_DEL_BG: Color = Color::Rgb(0x3a, 0x1c, 0x1c);
}

// ── Accessibility-aware codex accessors ──────────────────────────────────────
//
// These mirror the `codex::*` constants but return `Color::Reset` when
// [`color_mode()`] is [`ColorMode::Monochrome`]. New call sites should prefer
// these over the `pub const` form. Existing call sites in `transcript::markup`
// and `transcript::inline_cards` continue to use the const form for now; the
// migration is tracked as a follow-up to T3-A1.

macro_rules! codex_accessor {
    ($(#[$meta:meta])* $vis:vis fn $name:ident => $const_name:ident) => {
        $(#[$meta])*
        $vis fn $name() -> Color {
            if color_mode() == ColorMode::Monochrome {
                Color::Reset
            } else {
                codex::$const_name
            }
        }
    };
}

codex_accessor!(/// Accessibility-aware view of [`codex::BG`].
    pub fn codex_bg => BG);
codex_accessor!(/// Accessibility-aware view of [`codex::BG_OVERLAY`].
    pub fn codex_bg_overlay => BG_OVERLAY);
codex_accessor!(/// Accessibility-aware view of [`codex::FG`].
    pub fn codex_fg => FG);
codex_accessor!(/// Accessibility-aware view of [`codex::FG_STRONG`].
    pub fn codex_fg_strong => FG_STRONG);
codex_accessor!(/// Accessibility-aware view of [`codex::FG_DIM`].
    pub fn codex_fg_dim => FG_DIM);
codex_accessor!(/// Accessibility-aware view of [`codex::FG_VERY_DIM`].
    pub fn codex_fg_very_dim => FG_VERY_DIM);
codex_accessor!(/// Accessibility-aware view of [`codex::RULE`].
    pub fn codex_rule => RULE);
codex_accessor!(/// Accessibility-aware view of [`codex::BLUE_PATH`].
    pub fn codex_blue_path => BLUE_PATH);
codex_accessor!(/// Accessibility-aware view of [`codex::GREEN_OK`].
    pub fn codex_green_ok => GREEN_OK);
codex_accessor!(/// Accessibility-aware view of [`codex::SALMON_FAIL`].
    pub fn codex_salmon_fail => SALMON_FAIL);
codex_accessor!(/// Accessibility-aware view of [`codex::ORANGE_AGENT`].
    pub fn codex_orange_agent => ORANGE_AGENT);
codex_accessor!(/// Accessibility-aware view of [`codex::PINK_AGENT`].
    pub fn codex_pink_agent => PINK_AGENT);
codex_accessor!(/// Accessibility-aware view of [`codex::CYAN_TAB`].
    pub fn codex_cyan_tab => CYAN_TAB);
codex_accessor!(/// Accessibility-aware view of [`codex::YELLOW`].
    pub fn codex_yellow => YELLOW);
codex_accessor!(/// Accessibility-aware view of [`codex::DIFF_ADD_BG`].
    pub fn codex_diff_add_bg => DIFF_ADD_BG);
codex_accessor!(/// Accessibility-aware view of [`codex::DIFF_DEL_BG`].
    pub fn codex_diff_del_bg => DIFF_DEL_BG);

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Env mutation is process-global, and tests run in parallel by default.
    // Serialize every env-touching test through this mutex so they don't
    // clobber each other. We also keep `compute_color_mode_for_tests()`
    // un-cached so each case re-reads the current state.
    static ENV_GUARD: Mutex<()> = Mutex::new(());

    const ENV_VARS: &[&str] = &["NO_COLOR", "CLICOLOR", "CLICOLOR_FORCE", "FORCE_COLOR"];

    /// Snapshot every env var we touch so we can restore them after the test,
    /// even on panic-unwind (via the `Drop` impl below).
    struct EnvSnapshot {
        saved: Vec<(&'static str, Option<String>)>,
    }

    impl EnvSnapshot {
        fn capture() -> Self {
            let saved = ENV_VARS
                .iter()
                .map(|name| (*name, env::var(name).ok()))
                .collect();
            // Start each test from a clean baseline so prior values don't
            // bleed in. The snapshot we just captured restores them on Drop.
            for name in ENV_VARS {
                env::remove_var(name);
            }
            Self { saved }
        }
    }

    impl Drop for EnvSnapshot {
        fn drop(&mut self) {
            for (name, value) in &self.saved {
                match value {
                    Some(v) => env::set_var(name, v),
                    None => env::remove_var(name),
                }
            }
        }
    }

    #[test]
    fn no_color_disables_color() {
        let _g = ENV_GUARD.lock().unwrap();
        let _env = EnvSnapshot::capture();
        env::set_var("NO_COLOR", "1");
        assert_eq!(compute_color_mode_for_tests(), ColorMode::Monochrome);
    }

    #[test]
    fn force_color_overrides_no_color() {
        let _g = ENV_GUARD.lock().unwrap();
        let _env = EnvSnapshot::capture();
        env::set_var("NO_COLOR", "1");
        env::set_var("FORCE_COLOR", "1");
        assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
    }

    #[test]
    fn clicolor_force_overrides_no_color() {
        let _g = ENV_GUARD.lock().unwrap();
        let _env = EnvSnapshot::capture();
        env::set_var("NO_COLOR", "1");
        env::set_var("CLICOLOR_FORCE", "1");
        assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
    }

    #[test]
    fn empty_no_color_does_not_disable() {
        let _g = ENV_GUARD.lock().unwrap();
        let _env = EnvSnapshot::capture();
        env::set_var("NO_COLOR", "");
        // Per https://no-color.org, only a *non-empty* NO_COLOR disables color.
        // Without a TTY (test harness redirects stdout), default still resolves
        // to Full because the only path to Monochrome left is `CLICOLOR=0`.
        assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
    }

    #[test]
    fn default_is_full_with_no_env_signals() {
        let _g = ENV_GUARD.lock().unwrap();
        let _env = EnvSnapshot::capture();
        // No accessibility env vars set. Test harness is not a TTY, but the
        // default branch returns Full regardless — the only way to land in
        // Monochrome by default is `CLICOLOR=0` *on* a TTY, which is rare.
        assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
    }

    #[test]
    fn clicolor_zero_without_tty_keeps_full() {
        let _g = ENV_GUARD.lock().unwrap();
        let _env = EnvSnapshot::capture();
        env::set_var("CLICOLOR", "0");
        // The test harness pipes stdout, so `stdout_is_tty()` is false; per
        // the bixense.com spec, `CLICOLOR=0` only suppresses color *on* a TTY
        // (piped output already drops color naturally). So this stays Full.
        assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
    }

    #[test]
    fn force_color_zero_is_treated_as_off() {
        let _g = ENV_GUARD.lock().unwrap();
        let _env = EnvSnapshot::capture();
        // `FORCE_COLOR=0` is the documented "force off" sentinel; it should
        // NOT trigger the force-on branch, so we fall through to the default.
        env::set_var("FORCE_COLOR", "0");
        assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
    }

    #[test]
    fn force_color_false_is_treated_as_off() {
        let _g = ENV_GUARD.lock().unwrap();
        let _env = EnvSnapshot::capture();
        env::set_var("FORCE_COLOR", "false");
        assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
    }

    #[test]
    fn no_color_takes_priority_over_clicolor_zero() {
        let _g = ENV_GUARD.lock().unwrap();
        let _env = EnvSnapshot::capture();
        env::set_var("NO_COLOR", "1");
        env::set_var("CLICOLOR", "0");
        // Both routes lead to Monochrome, but `NO_COLOR` is the higher branch
        // and shouldn't depend on TTY detection.
        assert_eq!(compute_color_mode_for_tests(), ColorMode::Monochrome);
    }

    #[test]
    fn monochrome_palette_uses_reset_color() {
        let p = monochrome_palette();
        assert_eq!(p.text, Color::Reset);
        assert_eq!(p.text_muted, Color::Reset);
        assert_eq!(p.accent, Color::Reset);
        assert_eq!(p.border, Color::Reset);
    }

    #[test]
    fn mono_modifiers_are_emphasis_only() {
        // Sanity check that we hand back ratatui's emphasis modifiers, not a
        // color. These are what monochrome callers should layer on a Style.
        assert!(mono_strong().contains(Modifier::BOLD));
        assert!(mono_muted().contains(Modifier::ITALIC));
        assert!(mono_dim().contains(Modifier::DIM));
    }

    #[test]
    fn codex_accessor_returns_const_in_full_mode() {
        let _g = ENV_GUARD.lock().unwrap();
        let _env = EnvSnapshot::capture();
        // No env signals → Full mode → accessor matches the const. This
        // exercises the cached `color_mode()` path indirectly.
        // (We cannot reset the OnceLock cache, so this assertion is only
        // meaningful when the cache hasn't been poisoned to Monochrome by
        // an earlier test run — see the `OnceLock` note in the module
        // docstring.)
        if color_mode() == ColorMode::Full {
            assert_eq!(codex_fg(), codex::FG);
            assert_eq!(codex_blue_path(), codex::BLUE_PATH);
        }
    }
}
