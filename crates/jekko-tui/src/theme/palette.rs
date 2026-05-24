use jekko_core::theme::ThemeMode;
use ratatui::style::Color;

use super::{color_mode, ColorMode};

// Kept because non-codex callers still reach for them. Prefer `codex::*` for
// new widget work.
pub const TEXT: Color = Color::Rgb(0xd7, 0xde, 0xe8);
pub const TEXT_MUTED: Color = Color::Rgb(0x7a, 0x85, 0x94);
pub const ACCENT: Color = Color::Rgb(0xf4, 0xc5, 0x42);
pub const INFO: Color = Color::Rgb(0x55, 0xd6, 0xff);
pub const WARNING: Color = Color::Rgb(0xf5, 0xa5, 0x24);
pub const BORDER: Color = Color::Rgb(0x26, 0x31, 0x3d);

/// Small palette consumed by markdown syntax and other theme-mode-aware callers.
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
pub fn monochrome_palette() -> ThemePalette {
    ThemePalette {
        text: Color::Reset,
        text_muted: Color::Reset,
        accent: Color::Reset,
        border: Color::Reset,
    }
}
