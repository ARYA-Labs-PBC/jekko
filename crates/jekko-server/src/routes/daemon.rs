//! `/api/v1/daemon` — daemon control + status.
//!
//! Ports the subset of `packages/jekko/src/server/routes/instance/httpapi/handlers/daemon.ts`
//! that does not yet require the runtime daemon trait. CRUD operates against
//! [`crate::state::DaemonRegistry`] which the runtime will eventually feed.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{ServerError, ServerResult};
use crate::state::AppState;

/// Request body for `POST /api/v1/daemon/preview`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PreviewBody {
    /// Free-form text to preview.
    pub text: String,
}

/// Build the daemon router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list))
        .route("/preview", post(preview))
        .route("/:run_id", get(get_run))
        .route("/:run_id/pause", post(pause))
        .route("/:run_id/resume", post(resume))
        .route("/:run_id/abort", post(abort))
}

/// `GET /api/v1/daemon` — list runs.
#[utoipa::path(
    get,
    path = "/api/v1/daemon",
    responses((status = 200, description = "Daemon runs"))
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
) -> ServerResult<Json<Vec<serde_json::Value>>> {
    let snap = state.daemons.read().await;
    Ok(Json(snap.runs.values().cloned().collect()))
}

/// `POST /api/v1/daemon/preview` — synthesise a preview without executing.
#[utoipa::path(
    post,
    path = "/api/v1/daemon/preview",
    request_body = PreviewBody,
    responses((status = 200, description = "Preview"))
)]
pub async fn preview(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<PreviewBody>,
) -> ServerResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "text": payload.text,
        "preview": {
            "summary": "preview deferred (daemon facade pending)",
        },
    })))
}

/// `GET /api/v1/daemon/:run_id`.
#[utoipa::path(
    get,
    path = "/api/v1/daemon/{run_id}",
    responses(
        (status = 200, description = "Daemon run"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn get_run(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> ServerResult<Json<serde_json::Value>> {
    let snap = state.daemons.read().await;
    match snap.runs.get(&run_id).cloned() {
        Some(v) => Ok(Json(v)),
        None => Err(ServerError::not_found(format!("daemon: {run_id}"))),
    }
}

async fn publish_action(state: &AppState, run_id: &str, action: &str) -> ServerResult<Json<bool>> {
    let _ = state
        .bus
        .publish(
            "daemon.action",
            serde_json::json!({ "runID": run_id, "action": action }),
        )
        .await;
    Ok(Json(true))
}

/// `POST /api/v1/daemon/:run_id/pause`.
#[utoipa::path(
    post,
    path = "/api/v1/daemon/{run_id}/pause",
    responses((status = 200, description = "Paused"))
)]
pub async fn pause(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> ServerResult<Json<bool>> {
    publish_action(&state, &run_id, "pause").await
}

/// `POST /api/v1/daemon/:run_id/resume`.
#[utoipa::path(
    post,
    path = "/api/v1/daemon/{run_id}/resume",
    responses((status = 200, description = "Resumed"))
)]
pub async fn resume(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> ServerResult<Json<bool>> {
    publish_action(&state, &run_id, "resume").await
}

/// `POST /api/v1/daemon/:run_id/abort`.
#[utoipa::path(
    post,
    path = "/api/v1/daemon/{run_id}/abort",
    responses((status = 200, description = "Aborted"))
)]
pub async fn abort(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> ServerResult<Json<bool>> {
    publish_action(&state, &run_id, "abort").await
}
