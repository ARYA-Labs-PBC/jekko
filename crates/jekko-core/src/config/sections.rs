use serde::{Deserialize, Serialize};

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
