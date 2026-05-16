//! Streaming text events.

use serde::{Deserialize, Serialize};

use crate::v2::session_event::BaseEvent;

/// `session.next.text.started`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextStarted {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
}

/// `session.next.text.delta`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextDelta {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Streamed delta.
    pub delta: String,
}

/// `session.next.text.ended`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEnded {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Final assembled text.
    pub text: String,
}
