//! `jekko.json` / `.jekko/tui.json` configuration shapes.
//!
//! Ported from `packages/jekko/src/config/config-schema.ts`. This module
//! defines only the *types* — it does not read from disk and does not
//! understand JSONC comments. Use [`serde_json`] (or a JSONC pre-processor)
//! to turn raw bytes into [`Config`] at the call-site.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::permission::PermissionInput;

/// UI-specific TOML configuration schema (pure overlay types, no I/O).
///
/// The filesystem + environment loader for this schema lives in `jekko-cli`
/// (`jekko_cli::config_loader::UiConfigLoader`) so this crate stays free of
/// filesystem, environment, network, and clock access per the crate-root
/// invariant.
pub mod ui;

/// Log verbosity, mirroring the `LogLevel` literal union.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogLevel {
    /// Verbose debug logging.
    #[serde(rename = "DEBUG")]
    Debug,
    /// Informational logging (default).
    #[serde(rename = "INFO")]
    Info,
    /// Warnings only.
    #[serde(rename = "WARN")]
    Warn,
    /// Errors only.
    #[serde(rename = "ERROR")]
    Error,
}

/// Sharing policy literal (`manual` / `auto` / `disabled`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SharePolicy {
    /// Manual sharing only.
    Manual,
    /// Automatic sharing.
    Auto,
    /// Sharing disabled.
    Disabled,
}

/// Either `true`/`false` or the literal `"notify"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AutoUpdate {
    /// Boolean form.
    Bool(bool),
    /// Notify-only form.
    Notify(NotifyLiteral),
}

/// Helper enum so serde can prefer the boolean variant in [`AutoUpdate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotifyLiteral {
    /// `"notify"` literal.
    #[serde(rename = "notify")]
    Notify,
}

/// Layout literal (`stretch` is the only valid value today; the field is
/// retained for compatibility).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    /// Stretch layout.
    Stretch,
}

/// Tool-output truncation thresholds.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ToolOutputConfig {
    /// Maximum lines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_lines: Option<u32>,
    /// Maximum bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<u32>,
}

/// Compaction tuning knobs.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// Whether to compact automatically when context is full.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto: Option<bool>,
    /// Whether to prune prior tool outputs during compaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prune: Option<bool>,
    /// Number of recent user turns to retain verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail_turns: Option<u32>,
    /// Maximum tokens kept verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preserve_recent_tokens: Option<u32>,
    /// Token buffer reserved during compaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserved: Option<u32>,
}

/// Watcher (file-watcher) tuning.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WatcherConfig {
    /// Patterns to ignore.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
}

/// Enterprise-server configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EnterpriseConfig {
    /// Enterprise URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Experimental feature flags.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExperimentalConfig {
    /// Disable paste-summary expansion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_paste_summary: Option<bool>,
    /// Enable the batch tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_tool: Option<bool>,
    /// Enable OpenTelemetry traces.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "openTelemetry"
    )]
    pub open_telemetry: Option<bool>,
    /// Tools restricted to primary agents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_tools: Option<Vec<String>>,
    /// Whether to continue the loop when a tool call is denied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continue_loop_on_deny: Option<bool>,
    /// MCP request timeout in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_timeout: Option<u32>,
}

/// Top-level configuration shape (`jekko.json`).
///
/// Most fields are intentionally typed loosely (`serde_json::Value`) to
/// faithfully round-trip user configurations while leaving structured
/// validation to higher layers. The strongly-typed fields are those that
/// are central to the core domain (permissions, default models, log
/// levels, share policy, etc.).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// JSON schema URL.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// Default shell binary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    /// Log verbosity.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "logLevel")]
    pub log_level: Option<LogLevel>,
    /// Server config (free-form for now).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<serde_json::Value>,
    /// Command map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<BTreeMap<String, serde_json::Value>>,
    /// Skills directories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<serde_json::Value>,
    /// Filesystem watcher tuning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watcher: Option<WatcherConfig>,
    /// Whether snapshots are recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<bool>,
    /// Plugin specifications.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<Vec<serde_json::Value>>,
    /// Sharing policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share: Option<SharePolicy>,
    /// @discouraged use `share` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoshare: Option<bool>,
    /// Auto-update behaviour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoupdate: Option<AutoUpdate>,
    /// Disabled providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_providers: Option<Vec<String>>,
    /// Allow-list of providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled_providers: Option<Vec<String>>,
    /// Default model, as a `provider/model` string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Small-task model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub small_model: Option<String>,
    /// Default agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_agent: Option<String>,
    /// Displayed username override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// @discouraged Use `agent` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<BTreeMap<String, serde_json::Value>>,
    /// Agent definitions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<BTreeMap<String, serde_json::Value>>,
    /// Provider configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<BTreeMap<String, serde_json::Value>>,
    /// MCP server configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp: Option<BTreeMap<String, serde_json::Value>>,
    /// Formatter configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatter: Option<serde_json::Value>,
    /// LSP configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lsp: Option<serde_json::Value>,
    /// Extra instruction files/patterns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Vec<String>>,
    /// @discouraged Always stretch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<Layout>,
    /// Permission rules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionInput>,
    /// Tool allow/deny map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<BTreeMap<String, bool>>,
    /// Enterprise configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enterprise: Option<EnterpriseConfig>,
    /// Tool output truncation tuning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<ToolOutputConfig>,
    /// Compaction tuning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compaction: Option<CompactionConfig>,
    /// Experimental flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<ExperimentalConfig>,
}

impl Config {
    /// Return a fresh default config (all fields `None`).
    pub fn defaults() -> Self {
        Self::default()
    }

    /// Merge `other` into `self`, preferring `other`'s `Some(value)` fields
    /// over `self`'s. Map-valued fields (e.g. `provider`, `agent`) are merged
    /// by key — entries from `other` override the same key in `self`. This
    /// mirrors how `jekko.json` overrides are applied in TypeScript.
    pub fn merge(mut self, other: Self) -> Self {
        macro_rules! pick {
            ($($field:ident),* $(,)?) => {
                $(
                    if other.$field.is_some() {
                        self.$field = other.$field;
                    }
                )*
            };
        }

        macro_rules! merge_map {
            ($field:ident) => {
                match (self.$field.take(), other.$field) {
                    (Some(mut existing), Some(incoming)) => {
                        for (k, v) in incoming {
                            existing.insert(k, v);
                        }
                        self.$field = Some(existing);
                    }
                    (None, Some(incoming)) => self.$field = Some(incoming),
                    (existing, None) => self.$field = existing,
                }
            };
        }

        pick!(
            schema,
            shell,
            log_level,
            server,
            skills,
            watcher,
            snapshot,
            plugin,
            share,
            autoshare,
            autoupdate,
            disabled_providers,
            enabled_providers,
            model,
            small_model,
            default_agent,
            username,
            formatter,
            lsp,
            instructions,
            layout,
            permission,
            tools,
            enterprise,
            tool_output,
            compaction,
            experimental
        );

        merge_map!(command);
        merge_map!(mode);
        merge_map!(agent);
        merge_map!(provider);
        merge_map!(mcp);

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permission::{PermissionAction, PermissionInput};

    #[test]
    fn defaults_are_empty() {
        let cfg = Config::defaults();
        assert!(cfg.model.is_none());
        assert!(cfg.permission.is_none());
    }

    #[test]
    fn merge_overrides_scalars() {
        let base = Config {
            model: Some("anthropic/claude-sonnet-4".to_string()),
            ..Config::default()
        };
        let overlay = Config {
            model: Some("openai/gpt-5".to_string()),
            ..Config::default()
        };
        let merged = base.merge(overlay);
        assert_eq!(merged.model.as_deref(), Some("openai/gpt-5"));
    }

    #[test]
    fn merge_preserves_base_when_overlay_missing() {
        let base = Config {
            shell: Some("zsh".to_string()),
            ..Config::default()
        };
        let merged = base.clone().merge(Config::default());
        assert_eq!(merged.shell, base.shell);
    }

    #[test]
    fn merge_unions_maps() {
        let mut base_map: BTreeMap<String, serde_json::Value> = BTreeMap::new();
        base_map.insert("anthropic".to_string(), serde_json::json!({"a": 1}));
        let mut overlay_map: BTreeMap<String, serde_json::Value> = BTreeMap::new();
        overlay_map.insert("openai".to_string(), serde_json::json!({"b": 2}));

        let base = Config {
            provider: Some(base_map),
            ..Config::default()
        };
        let overlay = Config {
            provider: Some(overlay_map),
            ..Config::default()
        };
        let merged = base.merge(overlay);
        let providers = merged.provider.unwrap();
        assert!(providers.contains_key("anthropic"));
        assert!(providers.contains_key("openai"));
    }

    #[test]
    fn permission_shorthand_round_trips() {
        let json = serde_json::json!({"permission": "allow"});
        let cfg: Config = serde_json::from_value(json).unwrap();
        assert_eq!(
            cfg.permission,
            Some(PermissionInput::Action(PermissionAction::Allow))
        );
    }
}
