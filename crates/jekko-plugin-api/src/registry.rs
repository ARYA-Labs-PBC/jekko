//! Internal Rust plugin contract.
//!
//! Internal plugins compile into the Jekko binary. They expose a single
//! `JekkoPlugin` impl, which writes their themes, command metadata, model
//! presets, and config defaults into a [`PluginRegistry`] at startup.
//!
//! There is no JS bridge and no dynamic loading: external plugins are
//! declarative manifests (see [`crate::manifest`]).

use std::collections::BTreeMap;

use crate::error::{PluginError, PluginResult};
use crate::manifest::{CommandEntry, ExternalPluginManifest, ModelPresetEntry, ThemeEntry};

/// The internal Rust plugin trait.
///
/// Implementors hold no runtime state of their own: `register` writes plugin
/// metadata into the supplied registry. The host iterates over a fixed list
/// of `Box<dyn JekkoPlugin>` at startup, invoking `register` on each one.
pub trait JekkoPlugin: Send + Sync {
    /// Stable plugin id (e.g. `internal:home-tips`).
    fn id(&self) -> &'static str;

    /// Populate the host registry with this plugin's contributions.
    fn register(&self, registry: &mut PluginRegistry) -> PluginResult<()>;
}

/// Aggregated registry of plugin contributions.
///
/// The registry is built once at startup and then queried by the rest of the
/// host. It tracks plugin ids to prevent duplicate registrations and exposes
/// read-only views over themes, commands, model presets, and config defaults.
#[derive(Debug, Default)]
pub struct PluginRegistry {
    plugins: Vec<String>,
    themes: BTreeMap<String, ThemeEntry>,
    commands: BTreeMap<String, CommandEntry>,
    model_presets: BTreeMap<String, ModelPresetEntry>,
    config_defaults: serde_json::Value,
}

impl PluginRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Track a plugin id. Returns `DuplicatePlugin` if already registered.
    pub fn register_plugin(&mut self, id: &str) -> PluginResult<()> {
        if self.plugins.iter().any(|existing| existing == id) {
            return Err(PluginError::DuplicatePlugin(id.to_string()));
        }
        self.plugins.push(id.to_string());
        Ok(())
    }

    /// Add a theme. Returns `DuplicateTheme` on name collision.
    pub fn register_theme(&mut self, theme: ThemeEntry) -> PluginResult<()> {
        if self.themes.contains_key(&theme.name) {
            return Err(PluginError::DuplicateTheme(theme.name));
        }
        self.themes.insert(theme.name.clone(), theme);
        Ok(())
    }

    /// Add a command. Returns `DuplicateCommand` on id collision.
    pub fn register_command(&mut self, command: CommandEntry) -> PluginResult<()> {
        if self.commands.contains_key(&command.id) {
            return Err(PluginError::DuplicateCommand(command.id));
        }
        self.commands.insert(command.id.clone(), command);
        Ok(())
    }

    /// Add a model preset. Returns `DuplicateModelPreset` on id collision.
    pub fn register_model_preset(&mut self, preset: ModelPresetEntry) -> PluginResult<()> {
        if self.model_presets.contains_key(&preset.id) {
            return Err(PluginError::DuplicateModelPreset(preset.id));
        }
        self.model_presets.insert(preset.id.clone(), preset);
        Ok(())
    }

    /// Merge a config defaults object into the existing defaults. Later
    /// merges shallow-override earlier ones at the top level. Non-object
    /// values replace the current defaults.
    pub fn merge_config_defaults(&mut self, value: serde_json::Value) {
        match (&mut self.config_defaults, value) {
            (serde_json::Value::Object(existing), serde_json::Value::Object(incoming)) => {
                for (key, val) in incoming {
                    existing.insert(key, val);
                }
            }
            (slot, incoming) => {
                *slot = incoming;
            }
        }
    }

    /// Apply an entire validated [`ExternalPluginManifest`] to the registry.
    pub fn apply_manifest(&mut self, manifest: &ExternalPluginManifest) -> PluginResult<()> {
        self.register_plugin(&manifest.id)?;
        for theme in &manifest.themes {
            self.register_theme(theme.clone())?;
        }
        for command in &manifest.commands {
            self.register_command(command.clone())?;
        }
        for preset in &manifest.model_presets {
            self.register_model_preset(preset.clone())?;
        }
        if !manifest.config_defaults.is_null() {
            self.merge_config_defaults(manifest.config_defaults.clone());
        }
        Ok(())
    }

    /// Registered plugin ids in registration order.
    pub fn plugin_ids(&self) -> &[String] {
        &self.plugins
    }

    /// Themes registered, keyed by theme name.
    pub fn themes(&self) -> &BTreeMap<String, ThemeEntry> {
        &self.themes
    }

    /// Commands registered, keyed by command id.
    pub fn commands(&self) -> &BTreeMap<String, CommandEntry> {
        &self.commands
    }

    /// Model presets registered, keyed by preset id.
    pub fn model_presets(&self) -> &BTreeMap<String, ModelPresetEntry> {
        &self.model_presets
    }

    /// Aggregated config defaults from all plugins.
    pub fn config_defaults(&self) -> &serde_json::Value {
        &self.config_defaults
    }
}
