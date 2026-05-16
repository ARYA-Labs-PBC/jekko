//! Theme tokens, color parsing, and built-in presets.
//!
//! Ported from `packages/jekko/src/cli/cmd/tui/context/theme-core.ts` and
//! `theme-presets.ts`. The Rust port stores resolved colors as a normalised
//! 8-bit-per-channel struct (see [`Color`]) — the equivalent of the
//! TypeScript runtime's hex-resolved color.

use thiserror::Error;

mod color;
mod json;
mod resolved;

pub use color::Color;
pub use json::{ColorRef, ThemeJson};
pub use resolved::ResolvedTheme;

/// Errors returned while parsing or resolving a theme.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ThemeError {
    /// A hex color (`#rrggbb` / `#rrggbbaa`) was malformed.
    #[error("invalid hex color '{0}'")]
    InvalidHex(String),
    /// A color reference (`"primary"`, `"bone3"`) was not present in the
    /// theme's `defs` or `theme` maps.
    #[error("unknown color reference '{0}'")]
    UnknownReference(String),
    /// A color reference resolved into a cycle.
    #[error("circular color reference: {0}")]
    CircularReference(String),
}

/// Default thinking-opacity used when a theme omits the field.
pub const DEFAULT_THINKING_OPACITY: f64 = 0.6;

/// Mode used while resolving an [`ColorRef::Variant`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    /// Dark mode.
    Dark,
    /// Light mode.
    Light,
}

const JEKKO_DARK_JSON: &str = include_str!("../theme_assets/jekko.json");
const JEKKO_LIGHT_JSON: &str = include_str!("../theme_assets/jekko-light.json");

/// Default dark theme (ported from `cli/cmd/tui/context/theme/jekko.json`).
pub fn default_dark() -> ThemeJson {
    serde_json::from_str(JEKKO_DARK_JSON).expect("embedded jekko dark theme JSON")
}

/// Default light theme (ported from `cli/cmd/tui/context/theme/jekko-light.json`).
pub fn default_light() -> ThemeJson {
    serde_json::from_str(JEKKO_LIGHT_JSON).expect("embedded jekko light theme JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dark_resolves() {
        let theme = default_dark();
        let resolved = theme.resolve(ThemeMode::Dark).unwrap();
        assert!(resolved.get("primary").is_some());
        assert!(resolved.get("background").is_some());
    }

    #[test]
    fn default_light_resolves() {
        let theme = default_light();
        let resolved = theme.resolve(ThemeMode::Light).unwrap();
        assert!(resolved.get("primary").is_some());
    }
}
