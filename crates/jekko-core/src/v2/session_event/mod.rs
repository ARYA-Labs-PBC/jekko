//! Session-event union (`Schema.Union(...).pipe(Schema.toTaggedUnion("type"))`).
//!
//! Ported from `packages/jekko/src/v2/session-event.ts`. The TS code groups
//! events under namespaces (`Shell.Started`, `Step.Ended`, etc.); the Rust
//! port flattens these into a single tagged enum with the same string tags
//! so the JSON wire format round-trips byte-for-byte.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::session::SessionId;
use crate::v2::schema::UtcMillis;

mod compaction;
mod reasoning;
mod retry;
mod session;
mod shell;
mod step;
mod text;
mod tool;

pub use compaction::*;
pub use reasoning::*;
pub use retry::*;
pub use session::*;
pub use shell::*;
pub use step::*;
pub use text::*;
pub use tool::*;

/// Inline source range, mirroring `Source` in `session-event.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// Inclusive start offset.
    pub start: u32,
    /// Exclusive end offset.
    pub end: u32,
    /// Covered text.
    pub text: String,
}

/// Token usage shape carried by step-ended events.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct StepTokens {
    /// Input tokens.
    pub input: u64,
    /// Output tokens.
    pub output: u64,
    /// Reasoning tokens.
    pub reasoning: u64,
    /// Cache hits/misses.
    pub cache: StepCacheTokens,
}

/// Cache portion of [`StepTokens`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct StepCacheTokens {
    /// Cache hits.
    pub read: u64,
    /// Cache writes.
    pub write: u64,
}

/// Provider invocation metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderMeta {
    /// Whether the provider was actually executed.
    pub executed: bool,
    /// Optional provider-specific metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, serde_json::Value>>,
}

/// Compaction reason (`auto` / `manual`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactionReason {
    /// Triggered automatically when context filled up.
    Auto,
    /// Triggered explicitly via the `/compact` command.
    Manual,
}

/// Fields shared by every event payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseEvent {
    /// Timestamp.
    pub timestamp: UtcMillis,
    /// Session id.
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
}

/// Payload data for [`SessionEvent`] variants.
///
/// Each variant corresponds 1:1 with a TypeScript `EventV2.define(...)` call
/// in `session-event.ts`. The discriminator field is `type`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionEvent {
    /// `session.next.agent.switched`
    #[serde(rename = "session.next.agent.switched")]
    AgentSwitched(AgentSwitched),
    /// `session.next.model.switched`
    #[serde(rename = "session.next.model.switched")]
    ModelSwitched(ModelSwitched),
    /// `session.next.prompted`
    #[serde(rename = "session.next.prompted")]
    Prompted(Prompted),
    /// `session.next.synthetic`
    #[serde(rename = "session.next.synthetic")]
    Synthetic(Synthetic),
    /// `session.next.shell.started`
    #[serde(rename = "session.next.shell.started")]
    ShellStarted(ShellStarted),
    /// `session.next.shell.ended`
    #[serde(rename = "session.next.shell.ended")]
    ShellEnded(ShellEnded),
    /// `session.next.step.started`
    #[serde(rename = "session.next.step.started")]
    StepStarted(StepStarted),
    /// `session.next.step.ended`
    #[serde(rename = "session.next.step.ended")]
    StepEnded(StepEnded),
    /// `session.next.step.failed`
    #[serde(rename = "session.next.step.failed")]
    StepFailed(StepFailed),
    /// `session.next.text.started`
    #[serde(rename = "session.next.text.started")]
    TextStarted(TextStarted),
    /// `session.next.text.delta`
    #[serde(rename = "session.next.text.delta")]
    TextDelta(TextDelta),
    /// `session.next.text.ended`
    #[serde(rename = "session.next.text.ended")]
    TextEnded(TextEnded),
    /// `session.next.tool.input.started`
    #[serde(rename = "session.next.tool.input.started")]
    ToolInputStarted(ToolInputStarted),
    /// `session.next.tool.input.delta`
    #[serde(rename = "session.next.tool.input.delta")]
    ToolInputDelta(ToolInputDelta),
    /// `session.next.tool.input.ended`
    #[serde(rename = "session.next.tool.input.ended")]
    ToolInputEnded(ToolInputEnded),
    /// `session.next.tool.called`
    #[serde(rename = "session.next.tool.called")]
    ToolCalled(ToolCalled),
    /// `session.next.tool.progress`
    #[serde(rename = "session.next.tool.progress")]
    ToolProgress(ToolProgress),
    /// `session.next.tool.success`
    #[serde(rename = "session.next.tool.success")]
    ToolSuccess(ToolSuccess),
    /// `session.next.tool.failed`
    #[serde(rename = "session.next.tool.failed")]
    ToolFailed(ToolFailed),
    /// `session.next.reasoning.started`
    #[serde(rename = "session.next.reasoning.started")]
    ReasoningStarted(ReasoningStarted),
    /// `session.next.reasoning.delta`
    #[serde(rename = "session.next.reasoning.delta")]
    ReasoningDelta(ReasoningDelta),
    /// `session.next.reasoning.ended`
    #[serde(rename = "session.next.reasoning.ended")]
    ReasoningEnded(ReasoningEnded),
    /// `session.next.retried`
    #[serde(rename = "session.next.retried")]
    Retried(Retried),
    /// `session.next.compaction.started`
    #[serde(rename = "session.next.compaction.started")]
    CompactionStarted(CompactionStarted),
    /// `session.next.compaction.delta`
    #[serde(rename = "session.next.compaction.delta")]
    CompactionDelta(CompactionDelta),
    /// `session.next.compaction.ended`
    #[serde(rename = "session.next.compaction.ended")]
    CompactionEnded(CompactionEnded),
}
