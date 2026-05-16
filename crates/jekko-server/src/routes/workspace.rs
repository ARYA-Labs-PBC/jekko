//! `/api/v1/workspace` — workspace listing.
//!
//! Ports `packages/jekko/src/server/routes/instance/httpapi/handlers/workspace.ts`.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ServerResult;
use crate::state::AppState;

/// Workspace descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkspaceDescriptor {
    /// Workspace id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Workspace metadata.
    #[schema(value_type = Object)]
    pub data: serde_json::Value,
}

/// Build the workspace router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/:id", get(get_workspace).delete(remove))
}

/// `GET /api/v1/workspace`.
#[utoipa::path(
    get,
    path = "/api/v1/workspace",
    responses((status = 200, description = "Workspaces"))
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
) -> ServerResult<Json<Vec<serde_json::Value>>> {
    let snap = state.workspaces.read().await;
    Ok(Json(snap.entries.values().cloned().collect()))
}

/// `POST /api/v1/workspace`.
#[utoipa::path(
    post,
    path = "/api/v1/workspace",
    request_body = WorkspaceDescriptor,
    responses((status = 200, description = "Created"))
)]
pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WorkspaceDescriptor>,
) -> ServerResult<Json<WorkspaceDescriptor>> {
    {
        let mut snap = state.workspaces.write().await;
        snap.entries.insert(
            payload.id.clone(),
            serde_json::to_value(&payload).unwrap_or(serde_json::Value::Null),
        );
    }
    let _ = state
        .bus
        .publish(
            "workspace.created",
            serde_json::to_value(&payload).unwrap_or(serde_json::Value::Null),
        )
        .await;
    Ok(Json(payload))
}

/// `GET /api/v1/workspace/:id`.
#[utoipa::path(
    get,
    path = "/api/v1/workspace/{id}",
    responses((status = 200, description = "Workspace"))
)]
pub async fn get_workspace(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<serde_json::Value>> {
    let snap = state.workspaces.read().await;
    Ok(Json(
        snap.entries
            .get(&id)
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    ))
}

/// `DELETE /api/v1/workspace/:id`.
#[utoipa::path(
    delete,
    path = "/api/v1/workspace/{id}",
    responses((status = 200, description = "Removed"))
)]
pub async fn remove(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<bool>> {
    let removed = {
        let mut snap = state.workspaces.write().await;
        snap.entries.remove(&id).is_some()
    };
    let _ = state
        .bus
        .publish(
            "workspace.removed",
            serde_json::json!({ "id": id, "ok": removed }),
        )
        .await;
    Ok(Json(removed))
}
