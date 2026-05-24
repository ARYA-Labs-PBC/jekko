use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::permission::PermissionInput;

use super::{
    AutoUpdate, CompactionConfig, EnterpriseConfig, ExperimentalConfig, Layout, LogLevel,
    SharePolicy, ToolOutputConfig, WatcherConfig,
};

/// Top-level configuration shape (`jekko.json`).
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

    /// Merge `other` into `self`, preferring `other`'s `Some(value)` fields.
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

        merge_map(&mut self.command, other.command);
        merge_map(&mut self.mode, other.mode);
        merge_map(&mut self.agent, other.agent);
        merge_map(&mut self.provider, other.provider);
        merge_map(&mut self.mcp, other.mcp);

        self
    }
}

fn merge_map<T>(target: &mut Option<BTreeMap<String, T>>, incoming: Option<BTreeMap<String, T>>) {
    match (target.take(), incoming) {
        (Some(mut existing), Some(incoming)) => {
            for (k, v) in incoming {
                existing.insert(k, v);
            }
            *target = Some(existing);
        }
        (None, Some(incoming)) => *target = Some(incoming),
        (existing, None) => *target = existing,
    }
}
