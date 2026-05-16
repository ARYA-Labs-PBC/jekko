//! `/api/v1/session` — session CRUD + messages.
//!
//! Ports `packages/jekko/src/server/routes/instance/httpapi/handlers/session.ts`
//! against the in-memory [`jekko_runtime::session::SessionService`].

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use jekko_core::session::SessionId;
use jekko_runtime::session::{AppendMessageInput, CreateSessionInput, MessageInfo, SessionInfo};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{ServerError, ServerResult};
use crate::state::AppState;

/// Request body for `POST /api/v1/session`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSessionBody {
    /// Project id.
    pub project_id: String,
    /// Workspace id.
    #[serde(default)]
    pub workspace_id: Option<String>,
    /// Parent session id.
    #[serde(default)]
    pub parent_id: Option<String>,
    /// Working directory.
    pub directory: String,
    /// Title override.
    #[serde(default)]
    pub title: Option<String>,
}

/// Request body for `POST /api/v1/session/:id/message`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AppendMessageBody {
    /// Role.
    pub role: String,
    /// Free-form payload (JSON or string).
    #[schema(value_type = Object)]
    pub data: serde_json::Value,
}

/// Query params for `GET /api/v1/session?project_id=…`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListQuery {
    /// Project id.
    pub project_id: Option<String>,
}

/// Build the session router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/:id", get(get_session).delete(delete_session))
        .route("/:id/message", get(messages).post(append_message))
        .route("/:id/abort", post(abort))
}

/// `GET /api/v1/session?project_id=…` — list sessions for a project.
#[utoipa::path(
    get,
    path = "/api/v1/session",
    responses((status = 200, description = "Sessions"))
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> ServerResult<Json<Vec<SessionInfo>>> {
    // Explicit typed branching: an empty `project_id` query selects the
    // global session list, which is a typed state the service expects.
    #[allow(clippy::manual_unwrap_or_default)]
    let project_id: String = match q.project_id {
        Some(id) => id,
        None => String::new(),
    };
    let rows = state.sessions.list(&project_id).await?;
    Ok(Json(rows))
}

/// `POST /api/v1/session` — create a session.
#[utoipa::path(
    post,
    path = "/api/v1/session",
    request_body = CreateSessionBody,
    responses((status = 200, description = "Created session"))
)]
pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateSessionBody>,
) -> ServerResult<Json<SessionInfo>> {
    let info = state
        .sessions
        .create(CreateSessionInput {
            project_id: payload.project_id,
            workspace_id: payload.workspace_id,
            parent_id: payload.parent_id,
            directory: payload.directory,
            title: payload.title,
        })
        .await?;
    let _ = state
        .bus
        .publish(
            "session.created",
            serde_json::to_value(&info).unwrap_or(serde_json::Value::Null),
        )
        .await;
    Ok(Json(info))
}

/// `GET /api/v1/session/:id`.
#[utoipa::path(
    get,
    path = "/api/v1/session/{id}",
    responses((status = 200, description = "Session"), (status = 404, description = "Not found"))
)]
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<SessionInfo>> {
    let sid = SessionId::new(id.clone());
    match state.sessions.get(&sid).await? {
        Some(info) => Ok(Json(info)),
        None => Err(ServerError::not_found(format!("session: {id}"))),
    }
}

/// `DELETE /api/v1/session/:id` — best-effort delete (no-op in-memory).
#[utoipa::path(
    delete,
    path = "/api/v1/session/{id}",
    responses((status = 200, description = "Deleted"))
)]
pub async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<bool>> {
    let _ = state
        .bus
        .publish("session.deleted", serde_json::json!({ "sessionID": id }))
        .await;
    Ok(Json(true))
}

/// `POST /api/v1/session/:id/abort` — request that a running session abort.
#[utoipa::path(
    post,
    path = "/api/v1/session/{id}/abort",
    responses((status = 200, description = "Acknowledged"))
)]
pub async fn abort(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<bool>> {
    let _ = state
        .bus
        .publish("session.aborted", serde_json::json!({ "sessionID": id }))
        .await;
    Ok(Json(true))
}

/// `GET /api/v1/session/:id/message`.
#[utoipa::path(
    get,
    path = "/api/v1/session/{id}/message",
    responses((status = 200, description = "Messages"))
)]
pub async fn messages(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<Vec<MessageInfo>>> {
    let sid = SessionId::new(id);
    let rows = state.sessions.messages(&sid).await?;
    Ok(Json(rows))
}

/// `POST /api/v1/session/:id/message`.
#[utoipa::path(
    post,
    path = "/api/v1/session/{id}/message",
    request_body = AppendMessageBody,
    responses((status = 200, description = "Appended"))
)]
pub async fn append_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<AppendMessageBody>,
) -> ServerResult<Json<MessageInfo>> {
    let sid = SessionId::new(id);
    let msg = state
        .sessions
        .append(AppendMessageInput {
            session_id: sid,
            role: payload.role,
            data: payload.data,
        })
        .await?;
    let _ = state
        .bus
        .publish(
            "session.message",
            serde_json::to_value(&msg).unwrap_or(serde_json::Value::Null),
        )
        .await;
    Ok(Json(msg))
}
