//! Tool-call lifecycle events (input streaming, call, progress, success, failure).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::session::UnknownError;
use crate::v2::session_event::{BaseEvent, ProviderMeta};
use crate::v2::tool_output::{Content as ToolOutputContent, Structured as ToolOutputStructured};

/// `session.next.tool.input.started`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInputStarted {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Tool call id.
    #[serde(rename = "callID")]
    pub call_id: String,
    /// Tool name.
    pub name: String,
}

/// `session.next.tool.input.delta`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInputDelta {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Tool call id.
    #[serde(rename = "callID")]
    pub call_id: String,
    /// Streamed delta.
    pub delta: String,
}

/// `session.next.tool.input.ended`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInputEnded {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Tool call id.
    #[serde(rename = "callID")]
    pub call_id: String,
    /// Final assembled text.
    pub text: String,
}

/// `session.next.tool.called`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCalled {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Tool call id.
    #[serde(rename = "callID")]
    pub call_id: String,
    /// Tool name.
    pub tool: String,
    /// Tool input payload.
    pub input: BTreeMap<String, serde_json::Value>,
    /// Provider metadata.
    pub provider: ProviderMeta,
}

/// `session.next.tool.progress`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolProgress {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Tool call id.
    #[serde(rename = "callID")]
    pub call_id: String,
    /// Structured output snapshot.
    pub structured: ToolOutputStructured,
    /// Content blocks accumulated so far.
    pub content: Vec<ToolOutputContent>,
}

/// `session.next.tool.success`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSuccess {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Tool call id.
    #[serde(rename = "callID")]
    pub call_id: String,
    /// Final structured output.
    pub structured: ToolOutputStructured,
    /// Final content blocks.
    pub content: Vec<ToolOutputContent>,
    /// Provider metadata.
    pub provider: ProviderMeta,
}

/// `session.next.tool.failed`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFailed {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Tool call id.
    #[serde(rename = "callID")]
    pub call_id: String,
    /// Error payload.
    pub error: UnknownError,
    /// Provider metadata.
    pub provider: ProviderMeta,
}
