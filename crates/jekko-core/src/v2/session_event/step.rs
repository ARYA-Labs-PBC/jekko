//! Step lifecycle events.

use serde::{Deserialize, Serialize};

use crate::session::UnknownError;
use crate::v2::model::Ref as ModelRef;
use crate::v2::session_event::{BaseEvent, StepTokens};

/// `session.next.step.started`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepStarted {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Agent name.
    pub agent: String,
    /// Selected model.
    pub model: ModelRef,
    /// Optional snapshot id (start).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
}

/// `session.next.step.ended`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepEnded {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Finish reason.
    pub finish: String,
    /// Step cost.
    pub cost: f64,
    /// Token usage.
    pub tokens: StepTokens,
    /// Optional snapshot id (end).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
}

/// `session.next.step.failed`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepFailed {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Error payload.
    pub error: UnknownError,
}
