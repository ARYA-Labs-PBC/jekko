//! Theme JSON shape: [`ColorRef`], [`ThemeJson`], reference resolution.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::color::Color;
use super::resolved::ResolvedTheme;
use super::{ThemeError, ThemeMode, DEFAULT_THINKING_OPACITY};

/// Either a hex color (`"#fbf7ee"`) or a reference to another `defs`/theme key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColorRef {
    /// Adaptive entry — chooses based on dark/light mode.
    Variant {
        /// Dark-mode value.
        dark: Box<ColorRef>,
        /// Light-mode value.
        light: Box<ColorRef>,
    },
    /// Plain string (hex literal or named reference).
    String(String),
    /// Pre-resolved color (uncommon in source JSON; supported for round-trip).
    Color(Color),
}

/// Top-level theme JSON shape, mirroring `ThemeJson` in `theme-core.ts`.
///
/// The theme map is split between [`Self::theme`] (color tokens — every value
/// resolves to a [`Color`]) and [`Self::thinking_opacity`] (a free-form numeric
/// extracted out of the same JSON map so it doesn't break color deserialization).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(from = "RawThemeJson", into = "RawThemeJson")]
pub struct ThemeJson {
    /// JSON schema reference.
    pub schema: Option<String>,
    /// Named color definitions (`{ "bone1": "#fbf7ee", ... }`).
    pub defs: BTreeMap<String, ColorRef>,
    /// Token map (all values are color references).
    pub theme: BTreeMap<String, ColorRef>,
    /// Opacity (0.0-1.0) used to render "thinking" blocks. Defaults to 0.6.
    pub thinking_opacity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RawThemeJson {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    schema: Option<String>,
    #[serde(default)]
    defs: BTreeMap<String, ColorRef>,
    #[serde(default)]
    theme: BTreeMap<String, serde_json::Value>,
}

impl From<RawThemeJson> for ThemeJson {
    fn from(raw: RawThemeJson) -> Self {
        let mut thinking_opacity = DEFAULT_THINKING_OPACITY;
        let mut theme = BTreeMap::new();
        for (key, value) in raw.theme {
            if key == "thinkingOpacity" {
                if let Some(opacity) = value.as_f64() {
                    thinking_opacity = opacity;
                }
                continue;
            }
            // Skip silently if a non-color, non-thinkingOpacity value somehow appears.
            if let Ok(c) = serde_json::from_value::<ColorRef>(value) {
                theme.insert(key, c);
            }
        }
        Self {
            schema: raw.schema,
            defs: raw.defs,
            theme,
            thinking_opacity,
        }
    }
}

impl From<ThemeJson> for RawThemeJson {
    fn from(value: ThemeJson) -> Self {
        let mut theme: BTreeMap<String, serde_json::Value> = value
            .theme
            .into_iter()
            .map(|(k, v)| (k, serde_json::to_value(v).expect("ColorRef serialises")))
            .collect();
        theme.insert(
            "thinkingOpacity".to_string(),
            serde_json::Value::from(value.thinking_opacity),
        );
        Self {
            schema: value.schema,
            defs: value.defs,
            theme,
        }
    }
}

impl ThemeJson {
    /// Resolve every token in the theme into a concrete [`Color`]. References
    /// are recursively followed against `defs` and `theme`; cycles are
    /// detected and reported.
    pub fn resolve(&self, mode: ThemeMode) -> Result<ResolvedTheme, ThemeError> {
        let mut tokens = BTreeMap::new();
        for (name, value) in &self.theme {
            let color = resolve_color(value, &self.defs, &self.theme, mode, &mut Vec::new())?;
            tokens.insert(name.clone(), color);
        }
        Ok(ResolvedTheme { tokens })
    }
}

fn resolve_color(
    value: &ColorRef,
    defs: &BTreeMap<String, ColorRef>,
    theme: &BTreeMap<String, ColorRef>,
    mode: ThemeMode,
    chain: &mut Vec<String>,
) -> Result<Color, ThemeError> {
    match value {
        ColorRef::Color(c) => Ok(*c),
        ColorRef::Variant { dark, light } => {
            let pick = match mode {
                ThemeMode::Dark => dark,
                ThemeMode::Light => light,
            };
            resolve_color(pick, defs, theme, mode, chain)
        }
        ColorRef::String(s) => {
            if s == "transparent" || s == "none" {
                return Ok(Color::transparent());
            }
            if s.starts_with('#') {
                return Color::parse_hex(s);
            }
            if chain.iter().any(|x| x == s) {
                chain.push(s.clone());
                return Err(ThemeError::CircularReference(chain.join(" -> ")));
            }
            let next = match defs.get(s) {
                Some(next) => next,
                None => match theme.get(s) {
                    Some(next) => next,
                    None => return Err(ThemeError::UnknownReference(s.clone())),
                },
            };
            chain.push(s.clone());
            let resolved = resolve_color(next, defs, theme, mode, chain);
            chain.pop();
            resolved
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_simple_theme() {
        let mut defs = BTreeMap::new();
        defs.insert("a".to_string(), ColorRef::String("#ff0000".to_string()));
        let mut theme = BTreeMap::new();
        theme.insert("primary".to_string(), ColorRef::String("a".to_string()));
        let json = ThemeJson {
            schema: None,
            defs,
            theme,
            thinking_opacity: DEFAULT_THINKING_OPACITY,
        };
        let resolved = json.resolve(ThemeMode::Dark).unwrap();
        assert_eq!(resolved.get("primary"), Some(Color::rgb(255, 0, 0)));
    }

    #[test]
    fn detect_cycle() {
        let mut defs = BTreeMap::new();
        defs.insert("a".to_string(), ColorRef::String("b".to_string()));
        defs.insert("b".to_string(), ColorRef::String("a".to_string()));
        let mut theme = BTreeMap::new();
        theme.insert("primary".to_string(), ColorRef::String("a".to_string()));
        let json = ThemeJson {
            schema: None,
            defs,
            theme,
            thinking_opacity: DEFAULT_THINKING_OPACITY,
        };
        let err = json.resolve(ThemeMode::Dark).unwrap_err();
        assert!(matches!(err, ThemeError::CircularReference(_)));
    }
}
