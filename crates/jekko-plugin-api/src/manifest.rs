//! Declarative external plugin manifest.
//!
//! External Jekko plugins ship a TOML manifest that the Rust host can read and
//! validate without executing any plugin code. The manifest only carries
//! metadata: themes, command metadata, model presets, and config defaults.
//! There is intentionally no executable code surface — that is what
//! distinguishes the Rust plugin contract from the v1 JS plugin system.

use std::collections::BTreeMap;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::error::{PluginError, PluginResult};

/// A complete external plugin manifest as parsed from TOML.
///
/// Construct via [`ExternalPluginManifest::from_toml_str`]. After
/// construction the manifest is validated and its `version` is guaranteed to
/// parse as semver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalPluginManifest {
    /// Stable plugin identifier (e.g. `acme.demo`).
    pub id: String,
    /// Semver version string. Parsed and validated on construction.
    pub version: String,
    /// Optional human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional author label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Optional repository URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// Themes declared by the plugin.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub themes: Vec<ThemeEntry>,
    /// Command metadata declared by the plugin. The host wires commands into
    /// the command palette; the plugin itself never ships executable code.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<CommandEntry>,
    /// Model preset entries declared by the plugin.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_presets: Vec<ModelPresetEntry>,
    /// Free-form config defaults merged into the host config on first load.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub config_defaults: serde_json::Value,
}

/// A single theme entry inside a manifest.
///
/// Color tokens are stored as ANSI strings (hex `#rrggbb`, named ANSI colors,
/// or 256-color indexes). The host is responsible for resolving and
/// validating individual tokens at install time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeEntry {
    /// Unique theme name.
    pub name: String,
    /// Optional human-readable label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Theme appearance mode, if declared (`"dark"` or `"light"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// ANSI color tokens keyed by theme slot name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub colors: BTreeMap<String, String>,
}

/// Command metadata declared by an external plugin.
///
/// This is **metadata only**. There is no `on_select` handler, no JS callback,
/// and no Rust function pointer. The host renders the entry in the command
/// palette and routes invocations through declared keybind hints or its
/// internal action dispatcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandEntry {
    /// Stable command id (e.g. `acme.demo.open`).
    pub id: String,
    /// User-facing label shown in the command palette.
    pub label: String,
    /// Optional longer description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional category for grouping in the palette.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Optional keybind hint string (e.g. `ctrl+shift+d`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keybind: Option<String>,
}

/// Model preset entry declared by an external plugin.
///
/// Model presets bundle provider + model id + tuning defaults so users can
/// pick a single named preset rather than configure each setting by hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPresetEntry {
    /// Preset id (e.g. `acme.fast`).
    pub id: String,
    /// User-facing label.
    pub label: String,
    /// Provider id this preset targets (e.g. `anthropic`, `openai`).
    pub provider: String,
    /// Provider-specific model id (e.g. `claude-opus-4-7`).
    pub model: String,
    /// Free-form tuning options that the host will merge into chat params.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub options: serde_json::Value,
}

impl ExternalPluginManifest {
    /// Parse and validate a manifest from a TOML string.
    pub fn from_toml_str(input: &str) -> PluginResult<Self> {
        let manifest: Self = toml::from_str(input)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Return the parsed semver version. Always succeeds on a validated
    /// manifest; constructors guarantee `version` parses.
    pub fn semver(&self) -> Version {
        // Safe to unwrap because `validate()` parsed the same string.
        Version::parse(&self.version).expect("validated manifest version is valid semver")
    }

    fn validate(&self) -> PluginResult<()> {
        if self.id.trim().is_empty() {
            return Err(PluginError::ManifestInvalid(
                "manifest `id` must be non-empty".into(),
            ));
        }
        if self.version.trim().is_empty() {
            return Err(PluginError::ManifestInvalid(
                "manifest `version` must be non-empty".into(),
            ));
        }
        Version::parse(&self.version).map_err(|source| PluginError::InvalidVersion {
            value: self.version.clone(),
            source,
        })?;

        for (idx, theme) in self.themes.iter().enumerate() {
            if theme.name.trim().is_empty() {
                return Err(PluginError::ManifestInvalid(format!(
                    "themes[{idx}].name must be non-empty",
                )));
            }
            if let Some(mode) = &theme.mode {
                if mode != "dark" && mode != "light" {
                    return Err(PluginError::ManifestInvalid(format!(
                        "themes[{idx}].mode must be `dark` or `light`, got `{mode}`",
                    )));
                }
            }
        }

        for (idx, cmd) in self.commands.iter().enumerate() {
            if cmd.id.trim().is_empty() {
                return Err(PluginError::ManifestInvalid(format!(
                    "commands[{idx}].id must be non-empty",
                )));
            }
            if cmd.label.trim().is_empty() {
                return Err(PluginError::ManifestInvalid(format!(
                    "commands[{idx}].label must be non-empty",
                )));
            }
        }

        for (idx, preset) in self.model_presets.iter().enumerate() {
            if preset.id.trim().is_empty() {
                return Err(PluginError::ManifestInvalid(format!(
                    "model_presets[{idx}].id must be non-empty",
                )));
            }
            if preset.provider.trim().is_empty() {
                return Err(PluginError::ManifestInvalid(format!(
                    "model_presets[{idx}].provider must be non-empty",
                )));
            }
            if preset.model.trim().is_empty() {
                return Err(PluginError::ManifestInvalid(format!(
                    "model_presets[{idx}].model must be non-empty",
                )));
            }
        }

        Ok(())
    }
}
