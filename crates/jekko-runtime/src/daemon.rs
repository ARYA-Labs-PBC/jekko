//! Daemon registry.
//!
//! Ported in skeleton form from the very large
//! `packages/jekko/src/session/daemon*.ts` family. The TS daemon system
//! tracks "runs", "iterations", "tasks", "workers", "artifacts", and a
//! handful of orthogonal state machines per run. Here we ship the data
//! shape and an in-memory registry; the full state machine still lives
//! in TS pending a focused port pass.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{RuntimeError, RuntimeResult};

/// Lifecycle status of a daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DaemonStatus {
    /// Newly registered, not yet running.
    Pending,
    /// Active run in flight.
    Running,
    /// Run paused awaiting input.
    Paused,
    /// Run terminated successfully.
    Done,
    /// Run terminated with an error.
    Failed,
}

/// Materialised daemon record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonRecord {
    /// Daemon id.
    pub id: String,
    /// Owning session id.
    pub session_id: String,
    /// Human-readable label.
    pub name: String,
    /// Current status.
    pub status: DaemonStatus,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
    /// Free-form metadata.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// In-memory daemon registry.
#[derive(Debug, Default)]
pub struct DaemonRegistry {
    inner: RwLock<HashMap<String, DaemonRecord>>,
}

impl DaemonRegistry {
    /// Construct an empty registry.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Register a new daemon.
    pub async fn register(
        &self,
        session_id: impl Into<String>,
        name: impl Into<String>,
    ) -> DaemonRecord {
        let now = Utc::now().timestamp_millis();
        let record = DaemonRecord {
            id: format!("daemon_{}", Uuid::new_v4().simple()),
            session_id: session_id.into(),
            name: name.into(),
            status: DaemonStatus::Pending,
            time_created: now,
            time_updated: now,
            metadata: serde_json::json!({}),
        };
        self.inner
            .write()
            .await
            .insert(record.id.clone(), record.clone());
        record
    }

    /// Transition a daemon's status.
    pub async fn set_status(&self, id: &str, status: DaemonStatus) -> RuntimeResult<()> {
        let mut inner = self.inner.write().await;
        let rec = match inner.get_mut(id) {
            Some(r) => r,
            None => return Err(RuntimeError::not_found("daemon", id)),
        };
        rec.status = status;
        rec.time_updated = Utc::now().timestamp_millis();
        Ok(())
    }

    /// Fetch a daemon record.
    pub async fn get(&self, id: &str) -> Option<DaemonRecord> {
        self.inner.read().await.get(id).cloned()
    }

    /// List daemons for a given session.
    pub async fn list_for_session(&self, session_id: &str) -> Vec<DaemonRecord> {
        self.inner
            .read()
            .await
            .values()
            .filter(|d| d.session_id == session_id)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lifecycle() {
        let reg = DaemonRegistry::new();
        let rec = reg.register("session_1", "checker").await;
        assert_eq!(rec.status, DaemonStatus::Pending);
        reg.set_status(&rec.id, DaemonStatus::Running)
            .await
            .unwrap();
        assert_eq!(
            reg.get(&rec.id).await.unwrap().status,
            DaemonStatus::Running
        );
    }

    #[tokio::test]
    async fn list_for_session_filters() {
        let reg = DaemonRegistry::new();
        let _ = reg.register("session_1", "a").await;
        let _ = reg.register("session_1", "b").await;
        let _ = reg.register("session_2", "c").await;
        assert_eq!(reg.list_for_session("session_1").await.len(), 2);
        assert_eq!(reg.list_for_session("session_2").await.len(), 1);
    }
}
