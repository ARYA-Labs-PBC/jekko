//! V2 session info schema.
//!
//! Ported from `packages/jekko/src/v2/session.ts`. The Rust port retains
//! only the structural shape (`Info`, `Delivery`); service implementations
//! belong in higher crates.
use serde::{Deserialize, Serialize};

use crate::project::ProjectId;
use crate::session::{Delivery, SessionId, WorkspaceId};
use crate::v2::model::Ref as ModelRef;
use crate::v2::schema::UtcMillis;

pub use crate::session::Delivery as SessionDelivery;

/// Default delivery mode (matches `DefaultDelivery` in `session.ts`).
pub fn default_delivery() -> Delivery {
    Delivery::Immediate
}

/// Session metadata record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Info {
    /// Session id.
    pub id: SessionId,
    /// Optional parent session id.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "parentID")]
    pub parent_id: Option<SessionId>,
    /// Project id.
    #[serde(rename = "projectID")]
    pub project_id: ProjectId,
    /// Optional workspace id.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "workspaceID"
    )]
    pub workspace_id: Option<WorkspaceId>,
    /// Working directory (filesystem path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Selected agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Selected model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelRef>,
    /// Time metadata.
    pub time: Time,
    /// Display title.
    pub title: String,
}

/// Time metadata for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Time {
    /// Creation time (UTC millis).
    pub created: UtcMillis,
    /// Last update time (UTC millis).
    pub updated: UtcMillis,
    /// Optional archival time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<UtcMillis>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_delivery_is_immediate() {
        assert_eq!(default_delivery(), Delivery::Immediate);
    }
}
