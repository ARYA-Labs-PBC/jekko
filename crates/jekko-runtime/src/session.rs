//! Session lifecycle and in-memory store.
//!
//! Ported from `packages/jekko/src/session/session.ts`. The TS module is
//! large; here we cover the **observable** surface area that other runtime
//! services need:
//!
//! - Session creation (with id + slug + title defaults).
//! - Message append / list ordered by `(time_created, id)`.
//! - Persistence via a [`SessionStore`] trait so the same service works
//!   against `jekko-store` SQLite and an in-memory test double.
//!
//! Heavier session logic (revert pointers, share URL, summary aggregation,
//! agent + model selection) is intentionally deferred — those concerns
//! still live in TS and will be ported on demand.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use jekko_core::session::{MessageId, SessionId};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::bus::Bus;
use crate::error::{RuntimeError, RuntimeResult};

/// Input passed to [`SessionService::create`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSessionInput {
    /// FK to a project id.
    pub project_id: String,
    /// Workspace id, if any.
    #[serde(default)]
    pub workspace_id: Option<String>,
    /// Parent session id, if forking.
    #[serde(default)]
    pub parent_id: Option<String>,
    /// Working directory at create-time.
    pub directory: String,
    /// Custom title; auto-generated if absent.
    #[serde(default)]
    pub title: Option<String>,
}

/// Materialised session row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session id.
    pub id: SessionId,
    /// Project id.
    pub project_id: String,
    /// Workspace id.
    pub workspace_id: Option<String>,
    /// Parent session id, if any.
    pub parent_id: Option<String>,
    /// Working directory.
    pub directory: String,
    /// URL-safe slug.
    pub slug: String,
    /// Title.
    pub title: String,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
}

/// One message belonging to a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageInfo {
    /// Message id.
    pub id: MessageId,
    /// FK to [`SessionInfo::id`].
    pub session_id: SessionId,
    /// Role (e.g. `"user"`, `"assistant"`, `"tool"`).
    pub role: String,
    /// Free-form content (JSON or plain text).
    pub data: serde_json::Value,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
}

/// Append input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppendMessageInput {
    /// Session id.
    pub session_id: SessionId,
    /// Role tag.
    pub role: String,
    /// Free-form content payload.
    pub data: serde_json::Value,
}

/// Pluggable backing store. Implementations live in this crate (in-memory)
/// and downstream (`jekko-store` SQLite).
#[async_trait]
pub trait SessionStore: Send + Sync + std::fmt::Debug {
    /// Persist a session row.
    async fn put_session(&self, info: &SessionInfo) -> RuntimeResult<()>;
    /// Fetch a session row.
    async fn get_session(&self, id: &SessionId) -> RuntimeResult<Option<SessionInfo>>;
    /// List sessions for a project.
    async fn list_sessions(&self, project_id: &str) -> RuntimeResult<Vec<SessionInfo>>;
    /// Append a message.
    async fn put_message(&self, message: &MessageInfo) -> RuntimeResult<()>;
    /// List messages for a session ordered by `(time_created, id)`.
    async fn list_messages(&self, session_id: &SessionId) -> RuntimeResult<Vec<MessageInfo>>;
}

/// In-memory implementation of [`SessionStore`] for tests.
#[derive(Debug, Default)]
pub struct InMemorySessionStore {
    inner: Mutex<InMemoryState>,
}

#[derive(Debug, Default)]
struct InMemoryState {
    sessions: Vec<SessionInfo>,
    messages: Vec<MessageInfo>,
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn put_session(&self, info: &SessionInfo) -> RuntimeResult<()> {
        let mut s = self.inner.lock().await;
        if let Some(slot) = s.sessions.iter_mut().find(|r| r.id == info.id) {
            *slot = info.clone();
        } else {
            s.sessions.push(info.clone());
        }
        Ok(())
    }

    async fn get_session(&self, id: &SessionId) -> RuntimeResult<Option<SessionInfo>> {
        let s = self.inner.lock().await;
        Ok(s.sessions.iter().find(|r| r.id == *id).cloned())
    }

    async fn list_sessions(&self, project_id: &str) -> RuntimeResult<Vec<SessionInfo>> {
        let s = self.inner.lock().await;
        Ok(s.sessions
            .iter()
            .filter(|r| r.project_id == project_id)
            .cloned()
            .collect())
    }

    async fn put_message(&self, message: &MessageInfo) -> RuntimeResult<()> {
        let mut s = self.inner.lock().await;
        if let Some(slot) = s.messages.iter_mut().find(|r| r.id == message.id) {
            *slot = message.clone();
        } else {
            s.messages.push(message.clone());
        }
        Ok(())
    }

    async fn list_messages(&self, session_id: &SessionId) -> RuntimeResult<Vec<MessageInfo>> {
        let s = self.inner.lock().await;
        let mut out: Vec<_> = s
            .messages
            .iter()
            .filter(|m| m.session_id == *session_id)
            .cloned()
            .collect();
        out.sort_by_key(|m| (m.time_created, m.id.as_str().to_string()));
        Ok(out)
    }
}

/// Session service.
#[derive(Debug)]
pub struct SessionService {
    store: Arc<dyn SessionStore>,
    bus: Option<Arc<Bus>>,
}

impl Default for SessionService {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionService {
    /// Construct with the default in-memory backing store.
    pub fn new() -> Self {
        Self {
            store: Arc::new(InMemorySessionStore::default()),
            bus: None,
        }
    }

    /// Construct with a caller-supplied store.
    pub fn with_store(store: Arc<dyn SessionStore>) -> Self {
        Self { store, bus: None }
    }

    /// Construct with the default in-memory backing store and a bus.
    pub fn with_bus(bus: Arc<Bus>) -> Self {
        Self {
            store: Arc::new(InMemorySessionStore::default()),
            bus: Some(bus),
        }
    }

    /// Construct with a caller-supplied store and bus.
    pub fn with_store_and_bus(store: Arc<dyn SessionStore>, bus: Arc<Bus>) -> Self {
        Self {
            store,
            bus: Some(bus),
        }
    }

    /// Create a session and persist it via the backing store.
    pub async fn create(&self, input: CreateSessionInput) -> RuntimeResult<SessionInfo> {
        if input.project_id.is_empty() {
            return Err(RuntimeError::invalid("project_id is required"));
        }
        if input.directory.is_empty() {
            return Err(RuntimeError::invalid("directory is required"));
        }
        let id = SessionId::new(format!("session_{}", Uuid::new_v4().simple()));
        let now = Utc::now().timestamp_millis();
        let title = match input.title {
            Some(t) => t,
            None => format!(
                "New session - {}",
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            ),
        };
        let slug = slug_from_title(&title);
        let info = SessionInfo {
            id: id.clone(),
            project_id: input.project_id,
            workspace_id: input.workspace_id,
            parent_id: input.parent_id,
            directory: input.directory,
            slug,
            title,
            time_created: now,
            time_updated: now,
        };
        self.store.put_session(&info).await?;
        if let Some(bus) = &self.bus {
            let _ = bus
                .publish(
                    "session.started",
                    serde_json::json!({
                        "sessionID": info.id.as_str(),
                        "session_id": info.id.as_str(),
                        "session": info.clone(),
                    }),
                )
                .await;
        }
        Ok(info)
    }

    /// Append a message.
    pub async fn append(&self, input: AppendMessageInput) -> RuntimeResult<MessageInfo> {
        if self.store.get_session(&input.session_id).await?.is_none() {
            return Err(RuntimeError::not_found(
                "session",
                input.session_id.as_str(),
            ));
        }
        let id = MessageId::new(format!("msg_{}", Uuid::new_v4().simple()));
        let now = Utc::now().timestamp_millis();
        let message = MessageInfo {
            id,
            session_id: input.session_id,
            role: input.role,
            data: input.data,
            time_created: now,
            time_updated: now,
        };
        self.store.put_message(&message).await?;
        if let Some(bus) = &self.bus {
            let _ = bus
                .publish(
                    "session.message.appended",
                    serde_json::to_value(&message).map_err(RuntimeError::Json)?,
                )
                .await;
        }
        Ok(message)
    }

    /// Mark a session as ended. This is the explicit lifecycle hook used by
    /// attach/daemon code when a session stream closes cleanly.
    pub async fn end(&self, session_id: &SessionId) -> RuntimeResult<()> {
        let info = match self.store.get_session(session_id).await? {
            Some(info) => info,
            None => return Err(RuntimeError::not_found("session", session_id.as_str())),
        };
        if let Some(bus) = &self.bus {
            let _ = bus
                .publish(
                    "session.ended",
                    serde_json::json!({
                        "sessionID": info.id.as_str(),
                        "session_id": info.id.as_str(),
                        "session": info,
                    }),
                )
                .await;
        }
        Ok(())
    }

    /// List messages for `session_id` ordered by `(time_created, id)`.
    pub async fn messages(&self, session_id: &SessionId) -> RuntimeResult<Vec<MessageInfo>> {
        self.store.list_messages(session_id).await
    }

    /// Fetch a session.
    pub async fn get(&self, session_id: &SessionId) -> RuntimeResult<Option<SessionInfo>> {
        self.store.get_session(session_id).await
    }

    /// List sessions for a project.
    pub async fn list(&self, project_id: &str) -> RuntimeResult<Vec<SessionInfo>> {
        self.store.list_sessions(project_id).await
    }
}

fn slug_from_title(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut prev_dash = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "session".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_then_append_then_list() {
        let svc = SessionService::new();
        let info = svc
            .create(CreateSessionInput {
                project_id: "proj_1".into(),
                workspace_id: None,
                parent_id: None,
                directory: "/tmp".into(),
                title: Some("hello".into()),
            })
            .await
            .unwrap();
        assert_eq!(info.title, "hello");
        let msg = svc
            .append(AppendMessageInput {
                session_id: info.id.clone(),
                role: "user".into(),
                data: serde_json::json!({ "text": "hi" }),
            })
            .await
            .unwrap();
        assert_eq!(msg.role, "user");
        let listed = svc.messages(&info.id).await.unwrap();
        assert_eq!(listed.len(), 1);
    }

    #[tokio::test]
    async fn lifecycle_events_publish_when_bus_is_present() {
        let bus = Arc::new(Bus::new());
        let svc = SessionService::with_bus(bus.clone());
        let mut started = bus.subscribe("session.started").await;
        let mut appended = bus.subscribe("session.message.appended").await;
        let mut ended = bus.subscribe("session.ended").await;

        let info = svc
            .create(CreateSessionInput {
                project_id: "proj_1".into(),
                workspace_id: None,
                parent_id: None,
                directory: "/tmp".into(),
                title: Some("hello".into()),
            })
            .await
            .unwrap();
        assert_eq!(started.recv().await.unwrap().kind, "session.started");

        let _ = svc
            .append(AppendMessageInput {
                session_id: info.id.clone(),
                role: "user".into(),
                data: serde_json::json!({ "text": "hi" }),
            })
            .await
            .unwrap();
        assert_eq!(
            appended.recv().await.unwrap().kind,
            "session.message.appended"
        );

        svc.end(&info.id).await.unwrap();
        assert_eq!(ended.recv().await.unwrap().kind, "session.ended");
    }

    #[tokio::test]
    async fn append_requires_session() {
        let svc = SessionService::new();
        let err = svc
            .append(AppendMessageInput {
                session_id: SessionId::new("missing"),
                role: "user".into(),
                data: serde_json::json!({}),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, RuntimeError::NotFound { .. }));
    }

    #[test]
    fn slug_strips_punct() {
        assert_eq!(slug_from_title("Hello, World!"), "hello-world");
        assert_eq!(slug_from_title(""), "session");
    }
}
