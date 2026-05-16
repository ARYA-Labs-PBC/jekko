//! Migration helpers for the v1 (JS) plugin system.
//!
//! The Rust host can still load configs that reference v1 plugins.
//! Those plugins cannot execute under the Rust runtime, but we want users to
//! see a clear warning rather than a silent skip. This module recognizes the
//! shape of v1 entries (npm specs referencing `@jekko-ai/plugin`, file paths
//! ending in `.ts/.tsx/.js/.mjs`) without executing anything.

use serde_json::Value;

/// Reasons a config entry was flagged as a v1 plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationReason {
    /// Entry imports or references the v1 `@jekko-ai/plugin` package.
    LegacyPluginPackage,
    /// Entry points at a JS or TypeScript source file.
    JsPluginFile,
    /// Entry is an npm spec referencing the prior `jekko-ai/plugin` namespace.
    LegacyNpmSpec,
}

/// Warning surfaced when a v1 plugin reference is detected in config.
///
/// The host should log this warning, skip the plugin, and continue startup.
/// Nothing in this module attempts to execute the referenced module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationWarning {
    /// The plugin spec (file path or npm spec) that triggered the warning.
    pub spec: String,
    /// Why this spec was identified as v1.
    pub reason: MigrationReason,
    /// Human-readable message suitable for logging or UI display.
    pub message: String,
}

/// Inspect a TUI config value and return a warning if it references v1 plugins.
///
/// Recognizes both the canonical `plugin: [...]` array and tuple-form entries
/// `[spec, options]`. Returns the first detected v1 entry, if any.
pub fn detect_legacy_plugin(config: &Value) -> Option<MigrationWarning> {
    detect_legacy_plugins(config).into_iter().next()
}

/// Like [`detect_legacy_plugin`] but returns every detected v1 entry.
pub fn detect_legacy_plugins(config: &Value) -> Vec<MigrationWarning> {
    let mut out = Vec::new();
    collect_from_config(config, &mut out);
    out
}

fn collect_from_config(config: &Value, out: &mut Vec<MigrationWarning>) {
    let Some(obj) = config.as_object() else {
        return;
    };

    if let Some(list) = obj.get("plugin").and_then(Value::as_array) {
        for entry in list {
            if let Some(spec) = extract_spec(entry) {
                if let Some(warning) = classify(&spec) {
                    out.push(warning);
                }
            }
        }
    }

    if let Some(list) = obj.get("plugins").and_then(Value::as_array) {
        for entry in list {
            if let Some(spec) = extract_spec(entry) {
                if let Some(warning) = classify(&spec) {
                    out.push(warning);
                }
            }
        }
    }
}

fn extract_spec(entry: &Value) -> Option<String> {
    match entry {
        Value::String(s) => Some(s.clone()),
        Value::Array(items) => items.first().and_then(Value::as_str).map(str::to_string),
        Value::Object(map) => map.get("spec").and_then(Value::as_str).map(str::to_string),
        _ => None,
    }
}

fn classify(spec: &str) -> Option<MigrationWarning> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();

    if lower.contains("@jekko-ai/plugin") || lower.contains("jekko-ai/plugin") {
        return Some(MigrationWarning {
            spec: trimmed.to_string(),
            reason: MigrationReason::LegacyPluginPackage,
            message: format!(
                "plugin `{trimmed}` targets the v1 `@jekko-ai/plugin` package, which is not supported by the Rust runtime; remove it or migrate to the declarative TOML manifest",
            ),
        });
    }

    if is_js_path(&lower) {
        return Some(MigrationWarning {
            spec: trimmed.to_string(),
            reason: MigrationReason::JsPluginFile,
            message: format!(
                "plugin `{trimmed}` looks like a v1 JS/TS plugin file; the Rust runtime does not execute JS plugins. Convert it to a declarative TOML manifest.",
            ),
        });
    }

    if lower.starts_with("@jekko-ai/") || lower.starts_with("jekko-ai/") {
        return Some(MigrationWarning {
            spec: trimmed.to_string(),
            reason: MigrationReason::LegacyNpmSpec,
            message: format!(
                "plugin `{trimmed}` references the prior `jekko-ai` npm namespace and will not load under the Rust runtime",
            ),
        });
    }

    None
}

fn is_js_path(spec_lower: &str) -> bool {
    const EXTENSIONS: &[&str] = &[".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"];
    if let Some(no_query) = spec_lower.split('?').next() {
        return EXTENSIONS.iter().any(|ext| no_query.ends_with(ext));
    }
    false
}
