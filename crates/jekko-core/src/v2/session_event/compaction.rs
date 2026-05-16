//! Compaction lifecycle events.

use serde::{Deserialize, Serialize};

use crate::v2::session_event::{BaseEvent, CompactionReason};

/// `session.next.compaction.started`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionStarted {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Reason.
    pub reason: CompactionReason,
}

/// `session.next.compaction.delta`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionDelta {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Streamed delta.
    pub text: String,
}

/// `session.next.compaction.ended`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionEnded {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Final compaction summary.
    pub text: String,
    /// Optional retained include note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include: Option<String>,
}
