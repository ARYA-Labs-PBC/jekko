//! Session status tracker.
//!
//! Ported from `packages/jekko/src/session/status.ts`. Status events are
//! published as `session.status` on the bus, with `session.idle` emitted
//! when a session transitions back to idle.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::bus::Bus;

/// Session status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Status {
    /// Idle (no active work).
    Idle,
    /// Currently busy.
    Busy,
    /// Retrying after a transient failure.
    Retry {
        /// Attempt number (1-indexed).
        attempt: u32,
        /// Human-readable retry reason.
        message: String,
        /// Next retry in ms.
        next: u64,
    },
}

/// Per-session status tracker.
#[derive(Debug)]
pub struct StatusService {
    bus: Arc<Bus>,
    inner: RwLock<HashMap<String, Status>>,
}

impl StatusService {
    /// Construct.
    pub fn new(bus: Arc<Bus>) -> Self {
        Self {
            bus,
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// Current status for `session_id` (defaults to [`Status::Idle`]).
    pub async fn get(&self, session_id: &str) -> Status {
        self.inner
            .read()
            .await
            .get(session_id)
            .cloned()
            .unwrap_or(Status::Idle)
    }

    /// Snapshot of all tracked sessions and their status.
    pub async fn list(&self) -> HashMap<String, Status> {
        self.inner.read().await.clone()
    }

    /// Set the status for `session_id`. Publishes `session.status` (always)
    /// and `session.idle` (when transitioning to idle).
    pub async fn set(&self, session_id: &str, status: Status) {
        let payload = serde_json::json!({
            "sessionID": session_id,
            "status": status,
        });
        let _ = self.bus.publish("session.status", payload).await;
        if matches!(status, Status::Idle) {
            let _ = self
                .bus
                .publish(
                    "session.idle",
                    serde_json::json!({ "sessionID": session_id }),
                )
                .await;
            self.inner.write().await.remove(session_id);
        } else {
            self.inner
                .write()
                .await
                .insert(session_id.to_string(), status);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn busy_then_idle() {
        let bus = Arc::new(Bus::new());
        let svc = StatusService::new(bus.clone());
        let mut sub = bus.subscribe("session.status").await;
        let mut idle_sub = bus.subscribe("session.idle").await;

        svc.set("s1", Status::Busy).await;
        assert_eq!(svc.get("s1").await, Status::Busy);
        let _ = sub.recv().await.unwrap();

        svc.set("s1", Status::Idle).await;
        assert_eq!(svc.get("s1").await, Status::Idle);
        assert_eq!(idle_sub.recv().await.unwrap().kind, "session.idle");
    }
}
