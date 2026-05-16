//! `/api/v1/instance` — instance metadata + simple control.
//!
//! Ports `packages/jekko/src/server/routes/instance/httpapi/handlers/instance.ts`.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ServerResult;
use crate::state::AppState;

/// Path info exposed by `GET /api/v1/instance/path`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PathInfo {
    /// `$HOME`.
    pub home: String,
    /// Runtime state directory.
    pub state: String,
    /// Config directory.
    pub config: String,
    /// Worktree path (Git, if any).
    pub worktree: Option<String>,
    /// Working directory.
    pub directory: String,
}

/// Top-level info exposed by `GET /api/v1/instance`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstanceInfo {
    /// Instance id.
    pub id: String,
    /// Path info.
    pub path: PathInfo,
}

/// VCS info exposed by `GET /api/v1/instance/vcs`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VcsInfo {
    /// Active branch (if any).
    pub branch: Option<String>,
    /// Default branch (if known).
    pub default_branch: Option<String>,
}

/// Build the instance router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_instance))
        .route("/path", get(get_path))
        .route("/dispose", post(dispose))
        .route("/vcs", get(get_vcs))
}

/// `GET /api/v1/instance` — full instance descriptor.
#[utoipa::path(
    get,
    path = "/api/v1/instance",
    responses((status = 200, description = "Instance descriptor", body = InstanceInfo))
)]
pub async fn get_instance(State(state): State<Arc<AppState>>) -> ServerResult<Json<InstanceInfo>> {
    let meta = state.instance.clone();
    Ok(Json(InstanceInfo {
        id: meta.instance_id.clone(),
        path: PathInfo {
            home: meta.home.clone(),
            state: meta.state_dir.clone(),
            config: meta.config_dir.clone(),
            worktree: meta.worktree.clone(),
            directory: meta.directory.clone(),
        },
    }))
}

/// `GET /api/v1/instance/path` — instance paths.
#[utoipa::path(
    get,
    path = "/api/v1/instance/path",
    responses((status = 200, description = "Filesystem paths", body = PathInfo))
)]
pub async fn get_path(State(state): State<Arc<AppState>>) -> ServerResult<Json<PathInfo>> {
    let meta = state.instance.clone();
    Ok(Json(PathInfo {
        home: meta.home.clone(),
        state: meta.state_dir.clone(),
        config: meta.config_dir.clone(),
        worktree: meta.worktree.clone(),
        directory: meta.directory.clone(),
    }))
}

/// `POST /api/v1/instance/dispose` — mark this instance for disposal.
#[utoipa::path(
    post,
    path = "/api/v1/instance/dispose",
    responses((status = 200, description = "Disposed"))
)]
pub async fn dispose(State(state): State<Arc<AppState>>) -> ServerResult<Json<bool>> {
    let _ = state
        .bus
        .publish(
            "instance.disposed",
            serde_json::json!({ "id": state.instance.instance_id }),
        )
        .await;
    Ok(Json(true))
}

/// `GET /api/v1/instance/vcs` — branch metadata.
#[utoipa::path(
    get,
    path = "/api/v1/instance/vcs",
    responses((status = 200, description = "VCS info", body = VcsInfo))
)]
pub async fn get_vcs(State(_state): State<Arc<AppState>>) -> ServerResult<Json<VcsInfo>> {
    Ok(Json(VcsInfo {
        branch: None,
        default_branch: None,
    }))
}
