//! Reasoning-chain streaming events.

use serde::{Deserialize, Serialize};

use crate::v2::session_event::BaseEvent;

/// `session.next.reasoning.started`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReasoningStarted {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Reasoning chain id.
    #[serde(rename = "reasoningID")]
    pub reasoning_id: String,
}

/// `session.next.reasoning.delta`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReasoningDelta {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Reasoning chain id.
    #[serde(rename = "reasoningID")]
    pub reasoning_id: String,
    /// Streamed delta.
    pub delta: String,
}

/// `session.next.reasoning.ended`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReasoningEnded {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Reasoning chain id.
    #[serde(rename = "reasoningID")]
    pub reasoning_id: String,
    /// Final assembled text.
    pub text: String,
}
