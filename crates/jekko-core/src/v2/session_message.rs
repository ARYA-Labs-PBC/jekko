//! Persisted session-message types.
//!
//! Ported from `packages/jekko/src/v2/session-message.ts`. The TS code builds
//! the union as `Schema.Union(...).pipe(Schema.toTaggedUnion("type"))`; the
//! Rust port mirrors that with `#[serde(tag = "type")]`.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::session::{
    AgentAttachment as PromptAgentAttachment, EventId, FileAttachment as PromptFileAttachment,
    UnknownError,
};
use crate::v2::model::Ref as ModelRef;
use crate::v2::schema::UtcMillis;
use crate::v2::session_event::CompactionReason;
use crate::v2::tool_output::{Content as ToolOutputContent, Structured as ToolOutputStructured};

/// Fields shared by every persisted message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageBase {
    /// Event id.
    pub id: EventId,
    /// Optional metadata bag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, serde_json::Value>>,
    /// Time bookkeeping.
    pub time: BaseTime,
}

/// Creation time shared by every persisted message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseTime {
    /// Creation time (UTC millis).
    pub created: UtcMillis,
}

/// Tool-state union.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ToolState {
    /// Input is still being streamed (`status: "pending"`).
    Pending {
        /// Raw input streamed so far (string).
        input: String,
    },
    /// Tool is executing (`status: "running"`).
    Running {
        /// Decoded input map.
        input: BTreeMap<String, serde_json::Value>,
        /// Structured output snapshot.
        structured: ToolOutputStructured,
        /// Content blocks accumulated so far.
        content: Vec<ToolOutputContent>,
    },
    /// Tool completed successfully (`status: "completed"`).
    Completed {
        /// Decoded input map.
        input: BTreeMap<String, serde_json::Value>,
        /// Final content blocks.
        content: Vec<ToolOutputContent>,
        /// Final structured payload.
        structured: ToolOutputStructured,
        /// Optional attachments produced by the tool.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attachments: Option<Vec<PromptFileAttachment>>,
    },
    /// Tool errored (`status: "error"`).
    Error {
        /// Decoded input map.
        input: BTreeMap<String, serde_json::Value>,
        /// Content blocks accumulated before the failure.
        content: Vec<ToolOutputContent>,
        /// Structured payload accumulated before the failure.
        structured: ToolOutputStructured,
        /// Error payload.
        error: UnknownError,
    },
}

/// Provider-execution metadata attached to a tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolProvider {
    /// Whether the provider was actually executed.
    pub executed: bool,
    /// Optional provider metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, serde_json::Value>>,
}

/// Tool-call entry inside an assistant message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantTool {
    /// Tool call id.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Optional provider metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<ToolProvider>,
    /// Tool state.
    pub state: ToolState,
    /// Time bookkeeping.
    pub time: ToolTime,
}

/// Time bookkeeping for a tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ToolTime {
    /// Created.
    pub created: UtcMillis,
    /// Started running.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ran: Option<UtcMillis>,
    /// Completed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<UtcMillis>,
    /// Pruned (no longer counted in the assistant message).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pruned: Option<UtcMillis>,
}

/// One assistant message content block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[allow(clippy::large_enum_variant)]
pub enum AssistantContent {
    /// Plain text.
    Text {
        /// Body text.
        text: String,
    },
    /// Reasoning chain.
    Reasoning {
        /// Reasoning id.
        id: String,
        /// Body text.
        text: String,
    },
    /// Tool call.
    Tool(AssistantTool),
}

/// Time bookkeeping for an assistant message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssistantTime {
    /// Created.
    pub created: UtcMillis,
    /// Completed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<UtcMillis>,
}

/// Token usage payload.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct AssistantTokens {
    /// Input tokens.
    pub input: f64,
    /// Output tokens.
    pub output: f64,
    /// Reasoning tokens.
    pub reasoning: f64,
    /// Cache breakdown.
    pub cache: AssistantCacheTokens,
}

/// Cache portion of [`AssistantTokens`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct AssistantCacheTokens {
    /// Cache hits.
    pub read: f64,
    /// Cache writes.
    pub write: f64,
}

/// Snapshot pointer pair attached to assistant messages.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssistantSnapshot {
    /// Snapshot id at start of step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// Snapshot id at end of step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

/// Time bookkeeping for a shell message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellTime {
    /// Created.
    pub created: UtcMillis,
    /// Completed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<UtcMillis>,
}

/// One persisted session-message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(clippy::large_enum_variant)]
pub enum Message {
    /// Agent switched.
    #[serde(rename = "agent-switched")]
    AgentSwitched {
        /// Common base.
        #[serde(flatten)]
        base: MessageBase,
        /// Agent name.
        agent: String,
    },
    /// Model switched.
    #[serde(rename = "model-switched")]
    ModelSwitched {
        /// Common base.
        #[serde(flatten)]
        base: MessageBase,
        /// New model.
        model: ModelRef,
    },
    /// User message.
    #[serde(rename = "user")]
    User {
        /// Common base.
        #[serde(flatten)]
        base: MessageBase,
        /// User text.
        text: String,
        /// Optional inline files.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        files: Option<Vec<PromptFileAttachment>>,
        /// Optional inline agent mentions.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agents: Option<Vec<PromptAgentAttachment>>,
    },
    /// Synthetic (system-injected) message.
    #[serde(rename = "synthetic")]
    Synthetic {
        /// Common base.
        #[serde(flatten)]
        base: MessageBase,
        /// Session id this synthetic message belongs to.
        #[serde(rename = "sessionID")]
        session_id: crate::session::SessionId,
        /// Body text.
        text: String,
    },
    /// Shell command output.
    #[serde(rename = "shell")]
    Shell {
        /// Common base.
        #[serde(flatten)]
        base: MessageBase,
        /// Tool call id.
        #[serde(rename = "callID")]
        call_id: String,
        /// Command being executed.
        command: String,
        /// Captured output.
        output: String,
        /// Shell-specific time bookkeeping (overrides `MessageBase::time`).
        time: ShellTime,
    },
    /// Assistant message.
    #[serde(rename = "assistant")]
    Assistant {
        /// Common base.
        #[serde(flatten)]
        base: MessageBase,
        /// Agent name.
        agent: String,
        /// Selected model.
        model: ModelRef,
        /// Content blocks.
        content: Vec<AssistantContent>,
        /// Snapshot pair.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        snapshot: Option<AssistantSnapshot>,
        /// Finish reason.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        finish: Option<String>,
        /// Step cost.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cost: Option<f64>,
        /// Token usage.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tokens: Option<AssistantTokens>,
        /// Error payload, if the step failed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<UnknownError>,
        /// Assistant-specific time bookkeeping (overrides `MessageBase::time`).
        time: AssistantTime,
    },
    /// Compaction marker.
    #[serde(rename = "compaction")]
    Compaction {
        /// Common base.
        #[serde(flatten)]
        base: MessageBase,
        /// Compaction reason.
        reason: CompactionReason,
        /// Compaction summary.
        summary: String,
        /// Optional included context note.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        include: Option<String>,
    },
}
