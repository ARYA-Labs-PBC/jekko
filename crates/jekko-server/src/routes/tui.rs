//! `/api/v1/tui` — TUI control bridge.
//!
//! Ports `packages/jekko/src/server/routes/instance/httpapi/handlers/tui.ts`
//! by translating each command into a published bus event so a separate TUI
//! process can subscribe and react.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ServerResult;
use crate::state::AppState;

/// Generic command payload.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CommandBody {
    /// Command id (matches the TUI command catalog).
    pub command: String,
}

/// Prompt-append payload.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AppendPromptBody {
    /// Text to append.
    pub text: String,
    /// Whether to submit immediately after appending.
    #[serde(default)]
    pub submit: bool,
}

/// Toast notification payload.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToastBody {
    /// Toast body text.
    pub message: String,
    /// Severity: `info` / `warn` / `error`.
    #[serde(default)]
    pub level: Option<String>,
}

/// Session selection payload.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SelectSessionBody {
    /// Session id (`ses...`).
    pub session_id: String,
}

/// Build the TUI router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/append-prompt", post(append_prompt))
        .route("/open-help", post(open_help))
        .route("/open-sessions", post(open_sessions))
        .route("/open-themes", post(open_themes))
        .route("/open-models", post(open_models))
        .route("/submit-prompt", post(submit_prompt))
        .route("/clear-prompt", post(clear_prompt))
        .route("/execute", post(execute_command))
        .route("/toast", post(show_toast))
        .route("/select-session", post(select_session))
}

async fn publish_command(state: &AppState, command: &str) -> ServerResult<Json<bool>> {
    let _ = state
        .bus
        .publish("tui.command", serde_json::json!({ "command": command }))
        .await;
    Ok(Json(true))
}

/// `POST /api/v1/tui/append-prompt`.
#[utoipa::path(
    post,
    path = "/api/v1/tui/append-prompt",
    request_body = AppendPromptBody,
    responses((status = 200, description = "Appended"))
)]
pub async fn append_prompt(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AppendPromptBody>,
) -> ServerResult<Json<bool>> {
    let _ = state
        .bus
        .publish(
            "tui.prompt.append",
            serde_json::to_value(&payload).unwrap_or(serde_json::Value::Null),
        )
        .await;
    Ok(Json(true))
}

/// `POST /api/v1/tui/open-help`.
#[utoipa::path(
    post,
    path = "/api/v1/tui/open-help",
    responses((status = 200, description = "OK"))
)]
pub async fn open_help(State(state): State<Arc<AppState>>) -> ServerResult<Json<bool>> {
    publish_command(&state, "help.show").await
}

/// `POST /api/v1/tui/open-sessions`.
#[utoipa::path(
    post,
    path = "/api/v1/tui/open-sessions",
    responses((status = 200, description = "OK"))
)]
pub async fn open_sessions(State(state): State<Arc<AppState>>) -> ServerResult<Json<bool>> {
    publish_command(&state, "session.list").await
}

/// `POST /api/v1/tui/open-themes`.
#[utoipa::path(
    post,
    path = "/api/v1/tui/open-themes",
    responses((status = 200, description = "OK"))
)]
pub async fn open_themes(State(state): State<Arc<AppState>>) -> ServerResult<Json<bool>> {
    publish_command(&state, "theme.list").await
}

/// `POST /api/v1/tui/open-models`.
#[utoipa::path(
    post,
    path = "/api/v1/tui/open-models",
    responses((status = 200, description = "OK"))
)]
pub async fn open_models(State(state): State<Arc<AppState>>) -> ServerResult<Json<bool>> {
    publish_command(&state, "model.list").await
}

/// `POST /api/v1/tui/submit-prompt`.
#[utoipa::path(
    post,
    path = "/api/v1/tui/submit-prompt",
    responses((status = 200, description = "OK"))
)]
pub async fn submit_prompt(State(state): State<Arc<AppState>>) -> ServerResult<Json<bool>> {
    publish_command(&state, "prompt.submit").await
}

/// `POST /api/v1/tui/clear-prompt`.
#[utoipa::path(
    post,
    path = "/api/v1/tui/clear-prompt",
    responses((status = 200, description = "OK"))
)]
pub async fn clear_prompt(State(state): State<Arc<AppState>>) -> ServerResult<Json<bool>> {
    publish_command(&state, "prompt.clear").await
}

/// `POST /api/v1/tui/execute`.
#[utoipa::path(
    post,
    path = "/api/v1/tui/execute",
    request_body = CommandBody,
    responses((status = 200, description = "OK"))
)]
pub async fn execute_command(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CommandBody>,
) -> ServerResult<Json<bool>> {
    publish_command(&state, &payload.command).await
}

/// `POST /api/v1/tui/toast`.
#[utoipa::path(
    post,
    path = "/api/v1/tui/toast",
    request_body = ToastBody,
    responses((status = 200, description = "OK"))
)]
pub async fn show_toast(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ToastBody>,
) -> ServerResult<Json<bool>> {
    let _ = state
        .bus
        .publish(
            "tui.toast",
            serde_json::to_value(&payload).unwrap_or(serde_json::Value::Null),
        )
        .await;
    Ok(Json(true))
}

/// `POST /api/v1/tui/select-session`.
#[utoipa::path(
    post,
    path = "/api/v1/tui/select-session",
    request_body = SelectSessionBody,
    responses((status = 200, description = "OK"))
)]
pub async fn select_session(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SelectSessionBody>,
) -> ServerResult<Json<bool>> {
    let _ = state
        .bus
        .publish(
            "tui.session.select",
            serde_json::to_value(&payload).unwrap_or(serde_json::Value::Null),
        )
        .await;
    Ok(Json(true))
}
