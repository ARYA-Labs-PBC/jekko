use std::collections::HashMap;
use std::path::Path;

use jekko_core::config::ui::UiConfig;

use super::env;
use super::error::LoadError;
use super::provenance::{apply_env_with_provenance, record_toml_overlay};
use super::{ProvenanceMap, ResolvedUiConfig, Source, TRACKED_FIELDS};

/// TOML + env loader for the extended UI configuration.
///
/// Stateless namespace -- every entry point is a free function on the loader
/// rather than on [`UiConfig`] itself, so the pure-schema crate stays free of
/// filesystem and environment access.
#[derive(Debug)]
pub struct UiConfigLoader;

impl UiConfigLoader {
    /// Load the config from the default XDG path, falling back to defaults if
    /// no file exists. Returns `Err` if a file exists but cannot be read or
    /// parsed.
    pub fn load_default_path() -> Result<UiConfig, LoadError> {
        match Self::resolved_path() {
            Some(path) if path.exists() => Self::load_from_path(&path),
            _ => Ok(UiConfig::defaults()),
        }
    }

    /// Load the config from an explicit path. Reads the file then parses TOML.
    ///
    /// The returned value is the on-disk overlay merged on top of the runtime
    /// defaults, so callers can read fields directly without an extra `or`
    /// fallback layer.
    pub fn load_from_path(path: &Path) -> Result<UiConfig, LoadError> {
        let text = std::fs::read_to_string(path).map_err(LoadError::Io)?;
        Self::parse_str(&text)
    }

    /// Parse a TOML string into a [`UiConfig`], merged onto runtime defaults.
    ///
    /// Missing fields inherit defaults via the overlay merge; unknown keys are
    /// tolerated (the overlay shape uses serde defaults rather than
    /// `deny_unknown_fields` so future schema growth doesn't break old configs).
    pub fn parse_str(text: &str) -> Result<UiConfig, LoadError> {
        let overlay: UiConfig = toml::from_str(text).map_err(LoadError::Parse)?;
        Ok(UiConfig::defaults().merge(overlay))
    }

    /// Apply environment-variable overrides on top of an already-loaded config.
    /// See the module-level docs for the supported variables.
    pub fn merge_env(cfg: UiConfig) -> UiConfig {
        env::merge_env(cfg)
    }

    /// Resolve the UI config with per-field provenance -- the canonical
    /// loader entry point for T-CLI-OVERRIDES-PROVENANCE.
    ///
    /// Precedence (highest wins, applied in order):
    /// 1. Compiled-in defaults (every tracked field starts as
    ///    [`Source::Default`]).
    /// 2. The TOML overlay at `path` (or the default XDG path when `None`).
    ///    Fields explicitly set by the file flip to
    ///    [`Source::TomlPath`].
    /// 3. Process environment ([`UiConfigLoader::merge_env`]) -- every env
    ///    variable that fires marks its target field as [`Source::Env`].
    ///
    /// CLI flag overrides are applied AFTER this returns by the caller
    /// (`main.rs`), which is responsible for calling
    /// [`ResolvedUiConfig::record_cli_override`] for every flag it consumes.
    /// Doing it that way keeps the loader free of CLI-shape knowledge.
    pub fn resolve_with_provenance(path: Option<&Path>) -> Result<ResolvedUiConfig, LoadError> {
        let cfg = UiConfig::defaults();
        let mut provenance: ProvenanceMap = HashMap::new();
        for field in TRACKED_FIELDS {
            provenance.insert(*field, Source::Default);
        }

        let (cfg, toml_path_used) = match path {
            Some(p) => {
                let text = std::fs::read_to_string(p).map_err(LoadError::Io)?;
                let overlay: UiConfig = toml::from_str(&text).map_err(LoadError::Parse)?;
                let merged = cfg.merge(overlay.clone());
                record_toml_overlay(&overlay, p, &mut provenance);
                (merged, Some(p.to_path_buf()))
            }
            None => match Self::resolved_path() {
                Some(default_path) if default_path.exists() => {
                    let text = std::fs::read_to_string(&default_path).map_err(LoadError::Io)?;
                    let overlay: UiConfig = toml::from_str(&text).map_err(LoadError::Parse)?;
                    let merged = cfg.merge(overlay.clone());
                    record_toml_overlay(&overlay, &default_path, &mut provenance);
                    (merged, Some(default_path))
                }
                _ => (cfg, None),
            },
        };

        let cfg = apply_env_with_provenance(cfg, &mut provenance);

        Ok(ResolvedUiConfig {
            config: cfg,
            provenance,
            toml_path: toml_path_used,
        })
    }
}
