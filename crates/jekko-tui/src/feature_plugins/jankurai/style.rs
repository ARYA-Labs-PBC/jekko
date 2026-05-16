//! Style constants and missing-value glyphs for the Jankurai panel.

use ratatui::style::Color;

/// Glyph rendered for absent optional string fields (age, decision, level).
pub(super) const EM_DASH_GLYPH: &str = "—";
/// Glyph rendered for an unknown numeric score.
pub(super) const HYPHEN_GLYPH: &str = "-";
/// Glyph rendered for an unknown auditor version string.
pub(super) const QUESTION_GLYPH: &str = "?";

pub(super) const GOLD: Color = Color::Rgb(0xf5, 0xa6, 0x23);
pub(super) const GREEN: Color = Color::Rgb(0x22, 0xc5, 0x5e);
pub(super) const RED: Color = Color::Rgb(0xff, 0x47, 0x57);
pub(super) const BLUE: Color = Color::Rgb(0x3b, 0x82, 0xf6);
pub(super) const MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);
pub(super) const TEXT: Color = Color::Rgb(0xd8, 0xde, 0xe9);
pub(super) const CYAN: Color = Color::Rgb(0x00, 0xe5, 0xff);

pub(super) const SPARK_WIDTH: usize = 24;
