use jekko_plugin_api::{detect_legacy_plugin, detect_legacy_plugins, MigrationReason};
use serde_json::json;

#[test]
fn detects_legacy_npm_plugin_package_string() {
    let config = json!({
        "plugin": ["@jekko-ai/plugin"],
    });
    let warning = detect_legacy_plugin(&config).expect("legacy plugin detected");
    assert_eq!(warning.spec, "@jekko-ai/plugin");
    assert_eq!(warning.reason, MigrationReason::LegacyPluginPackage);
    assert!(warning.message.contains("Rust runtime"));
}

#[test]
fn detects_legacy_npm_plugin_tuple_entry() {
    let config = json!({
        "plugin": [["@jekko-ai/plugin@1.2.3", { "label": "demo" }]],
    });
    let warning = detect_legacy_plugin(&config).expect("legacy plugin detected");
    assert_eq!(warning.spec, "@jekko-ai/plugin@1.2.3");
    assert_eq!(warning.reason, MigrationReason::LegacyPluginPackage);
}

#[test]
fn detects_js_file_plugin() {
    let config = json!({
        "plugin": ["./plugins/demo.tsx"],
    });
    let warning = detect_legacy_plugin(&config).expect("legacy plugin detected");
    assert_eq!(warning.spec, "./plugins/demo.tsx");
    assert_eq!(warning.reason, MigrationReason::JsPluginFile);
}

#[test]
fn detects_legacy_jekko_ai_namespace_spec() {
    let config = json!({
        "plugin": ["@jekko-ai/sample"],
    });
    let warning = detect_legacy_plugin(&config).expect("legacy plugin detected");
    assert_eq!(warning.spec, "@jekko-ai/sample");
    assert_eq!(warning.reason, MigrationReason::LegacyNpmSpec);
}

#[test]
fn detect_returns_none_for_clean_config() {
    let config = json!({
        "plugin": ["@acme/jekko-plugin"],
    });
    assert!(detect_legacy_plugin(&config).is_none());
}

#[test]
fn detect_returns_none_for_missing_plugin_key() {
    let config = json!({ "theme": "smoke" });
    assert!(detect_legacy_plugin(&config).is_none());
}

#[test]
fn detects_multiple_legacy_entries() {
    let config = json!({
        "plugin": [
            "@jekko-ai/plugin",
            "./plugins/demo.tsx",
            "@acme/jekko-plugin",
            ["@jekko-ai/sample", { "label": "x" }],
        ],
    });
    let warnings = detect_legacy_plugins(&config);
    assert_eq!(warnings.len(), 3);
    assert_eq!(warnings[0].reason, MigrationReason::LegacyPluginPackage);
    assert_eq!(warnings[1].reason, MigrationReason::JsPluginFile);
    assert_eq!(warnings[2].reason, MigrationReason::LegacyNpmSpec);
}

#[test]
fn detects_entries_under_plural_plugins_key() {
    let config = json!({
        "plugins": [{ "spec": "@jekko-ai/plugin" }],
    });
    let warning = detect_legacy_plugin(&config).expect("plural plugins detected");
    assert_eq!(warning.reason, MigrationReason::LegacyPluginPackage);
}

#[test]
fn does_not_execute_anything() {
    // Functional check: detect_legacy_plugin must be a pure inspection.
    // Construct a value that would crash if any code tried to resolve it.
    let config = json!({
        "plugin": [
            { "spec": "@jekko-ai/plugin", "exec": "rm -rf /" },
            "./plugins/nonexistent.js",
        ],
    });
    let warnings = detect_legacy_plugins(&config);
    assert!(!warnings.is_empty());
    // No side effects: a second call returns the same result.
    let again = detect_legacy_plugins(&config);
    assert_eq!(warnings, again);
}
