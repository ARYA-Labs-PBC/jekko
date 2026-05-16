//! Generic event-envelope schema.
//!
//! Ported from `packages/jekko/src/v2/event.ts`.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::session::EventId;

/// A generic event envelope with a typed `data` payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event<T> {
    /// Event identifier.
    pub id: EventId,
    /// Optional metadata bag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, serde_json::Value>>,
    /// Discriminant string (matches the TS `Schema.Literal(type)`).
    #[serde(rename = "type")]
    pub kind: String,
    /// Payload.
    pub data: T,
}
