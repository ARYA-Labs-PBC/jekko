use jekko_plugin_api::{ExternalPluginManifest, PluginError};

const SAMPLE: &str = r##"
id = "acme.demo"
version = "1.2.3"
description = "Demo plugin"
author = "Acme"
repository = "https://example.com/acme/demo"

config_defaults = { feature_x = true, ttl = 30 }

[[themes]]
name = "acme-dark"
label = "Acme Dark"
mode = "dark"

[themes.colors]
primary = "#ff00aa"
background = "#000000"

[[commands]]
id = "acme.demo.open"
label = "Open Acme Demo"
description = "Open the demo screen"
category = "acme"
keybind = "ctrl+shift+a"

[[model_presets]]
id = "acme.fast"
label = "Acme Fast"
provider = "anthropic"
model = "claude-opus-4-7"
options = { temperature = 0.4 }
"##;

#[test]
fn parses_sample_manifest_round_trip() {
    let manifest = ExternalPluginManifest::from_toml_str(SAMPLE).expect("parse sample manifest");

    assert_eq!(manifest.id, "acme.demo");
    assert_eq!(manifest.version, "1.2.3");
    assert_eq!(manifest.semver().major, 1);
    assert_eq!(manifest.semver().minor, 2);
    assert_eq!(manifest.semver().patch, 3);
    assert_eq!(manifest.description.as_deref(), Some("Demo plugin"));
    assert_eq!(manifest.author.as_deref(), Some("Acme"));
    assert_eq!(
        manifest.repository.as_deref(),
        Some("https://example.com/acme/demo"),
    );

    assert_eq!(manifest.themes.len(), 1);
    let theme = &manifest.themes[0];
    assert_eq!(theme.name, "acme-dark");
    assert_eq!(theme.label.as_deref(), Some("Acme Dark"));
    assert_eq!(theme.mode.as_deref(), Some("dark"));
    assert_eq!(
        theme.colors.get("primary").map(String::as_str),
        Some("#ff00aa")
    );
    assert_eq!(
        theme.colors.get("background").map(String::as_str),
        Some("#000000")
    );

    assert_eq!(manifest.commands.len(), 1);
    let cmd = &manifest.commands[0];
    assert_eq!(cmd.id, "acme.demo.open");
    assert_eq!(cmd.label, "Open Acme Demo");
    assert_eq!(cmd.description.as_deref(), Some("Open the demo screen"));
    assert_eq!(cmd.category.as_deref(), Some("acme"));
    assert_eq!(cmd.keybind.as_deref(), Some("ctrl+shift+a"));

    assert_eq!(manifest.model_presets.len(), 1);
    let preset = &manifest.model_presets[0];
    assert_eq!(preset.id, "acme.fast");
    assert_eq!(preset.label, "Acme Fast");
    assert_eq!(preset.provider, "anthropic");
    assert_eq!(preset.model, "claude-opus-4-7");
    assert_eq!(
        preset.options.get("temperature").and_then(|v| v.as_f64()),
        Some(0.4),
    );

    assert_eq!(
        manifest
            .config_defaults
            .get("feature_x")
            .and_then(|v| v.as_bool()),
        Some(true),
    );
    assert_eq!(
        manifest.config_defaults.get("ttl").and_then(|v| v.as_i64()),
        Some(30),
    );
}

#[test]
fn rejects_empty_id() {
    let toml = r#"
id = ""
version = "1.0.0"
"#;
    let err = ExternalPluginManifest::from_toml_str(toml).expect_err("empty id must fail");
    assert!(matches!(err, PluginError::ManifestInvalid(_)));
}

#[test]
fn rejects_invalid_semver() {
    let toml = r#"
id = "ok"
version = "not-a-version"
"#;
    let err = ExternalPluginManifest::from_toml_str(toml).expect_err("bad semver must fail");
    match err {
        PluginError::InvalidVersion { value, .. } => assert_eq!(value, "not-a-version"),
        other => panic!("expected InvalidVersion, got {other:?}"),
    }
}

#[test]
fn rejects_unknown_field() {
    let toml = r#"
id = "ok"
version = "1.0.0"
unexpected = "boom"
"#;
    let err = ExternalPluginManifest::from_toml_str(toml).expect_err("unknown fields rejected");
    assert!(matches!(err, PluginError::ManifestParse(_)));
}

#[test]
fn rejects_invalid_theme_mode() {
    let toml = r#"
id = "ok"
version = "1.0.0"

[[themes]]
name = "weird"
mode = "neon"
"#;
    let err = ExternalPluginManifest::from_toml_str(toml).expect_err("bad mode must fail");
    assert!(matches!(err, PluginError::ManifestInvalid(_)));
}

#[test]
fn minimal_manifest_parses() {
    let toml = r#"
id = "min"
version = "0.1.0"
"#;
    let manifest = ExternalPluginManifest::from_toml_str(toml).expect("minimal manifest parses");
    assert!(manifest.themes.is_empty());
    assert!(manifest.commands.is_empty());
    assert!(manifest.model_presets.is_empty());
    assert!(manifest.config_defaults.is_null());
}
