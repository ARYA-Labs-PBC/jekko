//! Shell command events.

use serde::{Deserialize, Serialize};

use crate::v2::session_event::BaseEvent;

/// `session.next.shell.started`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellStarted {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Call id (provider-assigned).
    #[serde(rename = "callID")]
    pub call_id: String,
    /// Command being executed.
    pub command: String,
}

/// `session.next.shell.ended`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellEnded {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Call id.
    #[serde(rename = "callID")]
    pub call_id: String,
    /// Captured output.
    pub output: String,
}
