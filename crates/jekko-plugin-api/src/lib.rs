//! Jekko Rust plugin contract.
//!
//! This crate is the type surface every Jekko Rust plugin compiles against.
//! It intentionally avoids any reference to the previous v1 JS plugin system
//! (the prior JavaScript runtime types and renderer types) because those types
//! belong to the runtime that is being replaced.
//!
//! There are two plugin shapes here:
//!
//! * **Internal Rust plugins** implement [`JekkoPlugin`] and contribute themes,
//!   command metadata, model presets, and config defaults via a
//!   [`PluginRegistry`] at startup. They compile into the Rust binary.
//! * **External declarative plugins** ship as a TOML
//!   [`ExternalPluginManifest`] that the host parses, validates, and applies
//!   to a [`PluginRegistry`]. There is no executable code path for external
//!   plugins; everything is metadata.
//!
//! The crate also exposes [`detect_legacy_plugin`] for surfacing migration
//! warnings when a host config still references v1 JS plugins.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod error;
pub mod manifest;
pub mod migration;
pub mod registry;

pub use error::{PluginError, PluginResult};
pub use manifest::{CommandEntry, ExternalPluginManifest, ModelPresetEntry, ThemeEntry};
pub use migration::{
    detect_legacy_plugin, detect_legacy_plugins, MigrationReason, MigrationWarning,
};
pub use registry::{JekkoPlugin, PluginRegistry};
