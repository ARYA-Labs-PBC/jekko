//! Request / result payloads for the agent runtime boundary.
//!
//! Split out of [`crate::agent`] so the public payload definitions live in
//! one small file separate from the executor and provider plumbing.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::prompt;
use crate::session::{MessageInfo, SessionInfo};

/// Request to run a one-shot prompt through the agent runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunRequest {
    /// Raw prompt text.
    pub prompt: String,
    /// Working directory for the run.
    pub cwd: PathBuf,
    /// Optional agent name.
    #[serde(default)]
    pub agent: Option<String>,
    /// Optional provider identifier.
    #[serde(default)]
    pub provider: Option<String>,
    /// Optional model identifier.
    #[serde(default)]
    pub model: Option<String>,
    /// Whether this run should persist a session row.
    pub ephemeral: bool,
}

/// Request passed to the agent executor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTurnRequest {
    /// Raw prompt text.
    pub prompt: String,
    /// Parsed prompt payload.
    pub parsed_prompt: prompt::ParsedPrompt,
    /// Working directory.
    pub cwd: PathBuf,
    /// Session id used for the turn.
    pub session_id: String,
    /// Optional agent name.
    #[serde(default)]
    pub agent: Option<String>,
    /// Optional provider identifier.
    #[serde(default)]
    pub provider: Option<String>,
    /// Optional model identifier.
    #[serde(default)]
    pub model: Option<String>,
    /// Whether the parent run is ephemeral.
    pub ephemeral: bool,
}

/// Result returned by the agent executor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTurnResult {
    /// Provider used for the turn.
    pub provider_id: String,
    /// Model used for the turn.
    pub model_id: String,
    /// Assistant text accumulated from the provider stream.
    pub assistant_text: String,
    /// Reasoning text accumulated from the provider stream.
    #[serde(default)]
    pub reasoning_text: String,
    /// Completed tool calls observed in the stream.
    #[serde(default)]
    pub tool_calls: Vec<Value>,
}

/// Result of a one-shot run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunResult {
    /// Parsed prompt payload.
    pub parsed_prompt: prompt::ParsedPrompt,
    /// Session created for the run, if persisted.
    #[serde(default)]
    pub session: Option<SessionInfo>,
    /// Message appended for the user's prompt, if persisted.
    #[serde(default)]
    pub message: Option<MessageInfo>,
    /// Assistant message appended for the model response, if persisted.
    #[serde(default)]
    pub assistant_message: Option<MessageInfo>,
    /// Assistant text returned by the provider turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assistant_text: Option<String>,
    /// Reasoning text returned by the provider turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_text: Option<String>,
    /// Provider used for the turn, if one was selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    /// Model used for the turn, if one was selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    /// Tool calls observed during the turn.
    #[serde(default)]
    pub tool_calls: Vec<Value>,
    /// Whether the runtime accepted the request.
    pub accepted: bool,
}
