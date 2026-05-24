//! Section: Theme
//!
//! Canonical color palette. Public names are re-exported here so existing
//! callers can continue to use `crate::theme::*` and `crate::theme::codex::*`.

mod codex_palette;
mod color_mode;
mod palette;

#[cfg(test)]
mod tests;

pub use codex_palette::{
    codex, codex_bg, codex_bg_overlay, codex_blue_path, codex_cyan_tab, codex_diff_add_bg,
    codex_diff_del_bg, codex_fg, codex_fg_dim, codex_fg_strong, codex_fg_very_dim, codex_green_ok,
    codex_orange_agent, codex_pink_agent, codex_rule, codex_salmon_fail, codex_yellow,
};
pub use color_mode::{color_mode, mono_dim, mono_muted, mono_strong, ColorMode};
pub use palette::{
    monochrome_palette, palette, ThemePalette, ACCENT, BORDER, INFO, TEXT, TEXT_MUTED, WARNING,
};

#[cfg(test)]
pub(crate) use color_mode::compute_color_mode_for_tests;
