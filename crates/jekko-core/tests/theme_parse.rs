//! Theme parsing tests: hex, ANSI, default presets, and reference resolution.
use jekko_core::theme::{default_dark, default_light, Color, ThemeJson, ThemeMode};

#[test]
fn parse_six_digit_hex() {
    let c: Color = "#fbf7ee".parse().unwrap();
    assert_eq!(c.r, 0xfb);
    assert_eq!(c.g, 0xf7);
    assert_eq!(c.b, 0xee);
    assert_eq!(c.a, 0xff);
}

#[test]
fn parse_three_digit_hex_expands() {
    let c: Color = "#abc".parse().unwrap();
    assert_eq!(c, Color::rgb(0xaa, 0xbb, 0xcc));
}

#[test]
fn ansi256_palette_covers_grayscale() {
    let c = Color::from_ansi256(255);
    assert_eq!(c.r, c.g);
    assert_eq!(c.g, c.b);
}

#[test]
fn default_dark_has_expected_tokens() {
    let theme = default_dark();
    let resolved = theme.resolve(ThemeMode::Dark).unwrap();
    for token in [
        "primary",
        "secondary",
        "background",
        "backgroundPanel",
        "text",
        "syntaxKeyword",
    ] {
        assert!(resolved.get(token).is_some(), "missing token {token}");
    }
}

#[test]
fn default_light_resolves_in_light_mode() {
    let theme = default_light();
    let resolved = theme.resolve(ThemeMode::Light).unwrap();
    assert!(resolved.get("primary").is_some());
    assert!(resolved.get("background").is_some());
}

#[test]
fn parses_theme_json_with_named_refs() {
    let raw = r##"{
        "$schema": "https://jekko.ai/theme.json",
        "defs": { "a": "#ff0000", "b": "a" },
        "theme": {
            "primary": "b",
            "thinkingOpacity": 0.42
        }
    }"##;
    let theme: ThemeJson = serde_json::from_str(raw).unwrap();
    assert!((theme.thinking_opacity - 0.42).abs() < 1e-9);
    let resolved = theme.resolve(ThemeMode::Dark).unwrap();
    assert_eq!(resolved.get("primary"), Some(Color::rgb(0xff, 0, 0)));
}
