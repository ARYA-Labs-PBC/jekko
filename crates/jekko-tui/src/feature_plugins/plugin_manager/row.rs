//! Row types for the plugin-manager dialog.

/// Whether a plugin row is compiled in or loaded from an external manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginRowKind {
    /// Compiled-in Rust plugin via `jekko_plugin_api::JekkoPlugin`.
    Internal,
    /// External TOML manifest via `jekko_plugin_api::ExternalPluginManifest`.
    External,
}

impl PluginRowKind {
    /// Display label.
    pub fn label(self) -> &'static str {
        match self {
            PluginRowKind::Internal => "Internal",
            PluginRowKind::External => "External",
        }
    }
}

/// One plugin row. Generic over `PluginRegistry` and
/// `ExternalPluginManifest` so the panel can be tested without the wider
/// host config plumbed in.
#[derive(Clone, Debug)]
pub struct PluginRow {
    /// Stable plugin id (e.g. `internal:home-tips` or `acme.demo`).
    pub id: String,
    /// Semver version.
    pub version: String,
    /// Whether this plugin is compiled-in or external.
    pub kind: PluginRowKind,
    /// Number of theme contributions.
    pub themes: u32,
    /// Number of command contributions.
    pub commands: u32,
    /// Number of model presets.
    pub model_presets: u32,
    /// Optional short description for the side panel.
    pub description: Option<String>,
    /// Whether the plugin is currently enabled in the host registry.
    pub enabled: bool,
}

impl PluginRow {
    /// Build a row for an internal compiled plugin.
    pub fn internal(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
            kind: PluginRowKind::Internal,
            themes: 0,
            commands: 0,
            model_presets: 0,
            description: None,
            enabled: true,
        }
    }

    /// Build a row for an external manifest plugin.
    pub fn external(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
            kind: PluginRowKind::External,
            themes: 0,
            commands: 0,
            model_presets: 0,
            description: None,
            enabled: true,
        }
    }

    /// Set the theme count.
    pub fn with_themes(mut self, n: u32) -> Self {
        self.themes = n;
        self
    }

    /// Set the command count.
    pub fn with_commands(mut self, n: u32) -> Self {
        self.commands = n;
        self
    }

    /// Set the model preset count.
    pub fn with_model_presets(mut self, n: u32) -> Self {
        self.model_presets = n;
        self
    }

    /// Set the description.
    pub fn with_description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }

    /// Set the enabled flag.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}
