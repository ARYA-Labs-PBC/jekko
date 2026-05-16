//! `/api/v1/permission` — pending permission ask queue + replies.
//!
//! Ports the queue subset of
//! `packages/jekko/src/server/routes/instance/httpapi/handlers/permission.ts`.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use jekko_runtime::permission::{PermissionReply, PermissionRequest};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{ServerError, ServerResult};
use crate::state::AppState;

/// Body of `POST /api/v1/permission/:id`. The inner reply is exchanged as a
/// string (`"once"` / `"always"` / `"reject"`) so the schema does not
/// depend on `PermissionReply: ToSchema`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReplyBody {
    /// `once` / `always` / `reject`.
    pub reply: String,
}

/// Build the permission router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list))
        .route("/:id", post(reply))
}

/// `GET /api/v1/permission` — pending asks.
#[utoipa::path(
    get,
    path = "/api/v1/permission",
    responses((status = 200, description = "Pending asks"))
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
) -> ServerResult<Json<Vec<PermissionRequest>>> {
    Ok(Json(state.permissions.list_pending().await))
}

/// `POST /api/v1/permission/:id` — reply to a pending ask.
#[utoipa::path(
    post,
    path = "/api/v1/permission/{id}",
    responses((status = 200, description = "Replied"))
)]
pub async fn reply(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<ReplyBody>,
) -> ServerResult<Json<bool>> {
    let reply = match payload.reply.as_str() {
        "once" => PermissionReply::Once,
        "always" => PermissionReply::Always,
        "reject" => PermissionReply::Reject,
        other => return Err(ServerError::bad_request(format!("invalid reply: {other}"))),
    };
    state.permissions.reply(&id, reply).await?;
    Ok(Json(true))
}
