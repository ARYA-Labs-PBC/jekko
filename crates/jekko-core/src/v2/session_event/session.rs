//! Session-lifecycle events (agent/model switches, prompts, synthetic input).

use serde::{Deserialize, Serialize};

use crate::session::Prompt;
use crate::v2::model::Ref as ModelRef;
use crate::v2::session_event::BaseEvent;

/// `session.next.agent.switched`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSwitched {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Agent name.
    pub agent: String,
}

/// `session.next.model.switched`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSwitched {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Selected model.
    pub model: ModelRef,
}

/// `session.next.prompted`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prompted {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Prompt payload.
    pub prompt: Prompt,
}

/// `session.next.synthetic`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Synthetic {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Synthetic text.
    pub text: String,
}
