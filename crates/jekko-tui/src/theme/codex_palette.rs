use ratatui::style::Color;

use super::{color_mode, ColorMode};

/// Claude Code / Codex CLI parity tokens. New TUI widgets should reference
/// these instead of legacy named colors.
pub mod codex {
    use ratatui::style::Color;

    /// Default terminal background.
    pub const BG: Color = Color::Reset;
    /// Slightly raised overlay background used for popup / chip surfaces.
    pub const BG_OVERLAY: Color = Color::Rgb(0x1a, 0x1d, 0x24);

    /// Body text foreground.
    pub const FG: Color = Color::Rgb(0xd7, 0xde, 0xe8);
    /// Slightly stronger foreground for emphasized labels.
    pub const FG_STRONG: Color = Color::Rgb(0xf2, 0xf4, 0xf8);
    /// Dimmed foreground for secondary text.
    pub const FG_DIM: Color = Color::Rgb(0x9a, 0xa3, 0xb1);
    /// Very dim foreground for tertiary chrome.
    pub const FG_VERY_DIM: Color = Color::Rgb(0x60, 0x6a, 0x78);

    /// Horizontal rule color.
    pub const RULE: Color = Color::Rgb(0x37, 0x3d, 0x49);

    /// Path / file color.
    pub const BLUE_PATH: Color = Color::Rgb(0x6e, 0xb1, 0xff);
    /// Success indicator.
    pub const GREEN_OK: Color = Color::Rgb(0x6c, 0xc9, 0x7a);
    /// Failure indicator.
    pub const SALMON_FAIL: Color = Color::Rgb(0xf2, 0x7a, 0x7a);
    /// Active-agent accent.
    pub const ORANGE_AGENT: Color = Color::Rgb(0xff, 0x9d, 0x4a);
    /// Sub-agent accent.
    pub const PINK_AGENT: Color = Color::Rgb(0xff, 0x6f, 0xb5);
    /// Composer branch-tab accent.
    pub const CYAN_TAB: Color = Color::Rgb(0x4e, 0xd1, 0xd1);
    /// Warning accent.
    pub const YELLOW: Color = Color::Rgb(0xf5, 0xc8, 0x52);

    /// Diff background shading for changed lines.
    pub const DIFF_ADD_BG: Color = Color::Rgb(0x18, 0x3a, 0x22);
    pub const DIFF_DEL_BG: Color = Color::Rgb(0x3a, 0x1c, 0x1c);
}

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
