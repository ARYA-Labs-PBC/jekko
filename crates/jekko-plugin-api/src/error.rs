//! Error types for the Jekko plugin contract.
//!
//! All public APIs in this crate surface failures through [`PluginError`].
//! The error type is `non_exhaustive` so future variants can be added without
//! breaking downstream pattern matches.

use thiserror::Error;

/// Result alias used throughout the plugin contract crate.
pub type PluginResult<T> = std::result::Result<T, PluginError>;

/// Errors that can be returned by plugin contract operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PluginError {
    /// The TOML manifest text could not be parsed.
    #[error("invalid plugin manifest TOML: {0}")]
    ManifestParse(#[from] toml::de::Error),

    /// A required manifest field was missing, empty, or otherwise invalid.
    #[error("invalid plugin manifest: {0}")]
    ManifestInvalid(String),

    /// The `version` field of a manifest was not a valid semver string.
    #[error("invalid plugin manifest version `{value}`: {source}")]
    InvalidVersion {
        /// The raw string that failed semver parsing.
        value: String,
        /// The underlying semver parse error.
        #[source]
        source: semver::Error,
    },

    /// A duplicate plugin id was registered in the [`crate::PluginRegistry`].
    #[error("duplicate plugin id `{0}` registered in the plugin registry")]
    DuplicatePlugin(String),

    /// A duplicate theme name was registered in the [`crate::PluginRegistry`].
    #[error("duplicate theme `{0}` registered in the plugin registry")]
    DuplicateTheme(String),

    /// A duplicate command id was registered in the [`crate::PluginRegistry`].
    #[error("duplicate command `{0}` registered in the plugin registry")]
    DuplicateCommand(String),

    /// A duplicate model preset id was registered in the [`crate::PluginRegistry`].
    #[error("duplicate model preset `{0}` registered in the plugin registry")]
    DuplicateModelPreset(String),
}
