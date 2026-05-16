//! Retry events.

use serde::{Deserialize, Serialize};

use crate::session::RetryError;
use crate::v2::session_event::BaseEvent;

/// `session.next.retried`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retried {
    /// Common base fields.
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Retry attempt counter.
    pub attempt: u32,
    /// Retry-error payload.
    pub error: RetryError,
}
