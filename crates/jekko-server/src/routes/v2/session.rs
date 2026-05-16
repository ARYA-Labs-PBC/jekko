//! `/api/v2/session` — paginated session list + minimal prompt control.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use jekko_runtime::session::SessionInfo;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ServerResult;
use crate::state::AppState;

/// Default delivery mode for `POST /api/v2/session/:id/prompt` when omitted.
const DEFAULT_DELIVERY_MODE: &str = "immediate";
/// Default page size for `GET /api/v2/session` when no `limit` query is given.
const DEFAULT_SESSION_PAGE_LIMIT: usize = 50;

/// Query parameters for `GET /api/v2/session`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListQuery {
    /// Filter by directory.
    #[serde(default)]
    pub directory: Option<String>,
    /// Filter by project id.
    #[serde(default)]
    pub project_id: Option<String>,
    /// Result cap.
    #[serde(default)]
    pub limit: Option<usize>,
    /// Ordering (`asc` / `desc`).
    #[serde(default)]
    pub order: Option<String>,
}

/// Paginated response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPage {
    /// Items.
    pub items: Vec<SessionInfo>,
    /// Total in this page.
    pub count: usize,
}

/// Body of `POST /api/v2/session/:id/prompt`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PromptBody {
    /// User prompt (jekko-core `Prompt` shape, serialised as JSON).
    #[schema(value_type = Object)]
    pub prompt: serde_json::Value,
    /// Delivery mode (`immediate` / `deferred`).
    #[serde(default)]
    pub delivery: Option<String>,
}

/// Build the v2 session router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list))
        .route("/:id/prompt", post(prompt))
        .route("/:id/compact", post(compact))
        .route("/:id/wait", post(wait))
}

/// `GET /api/v2/session`.
#[utoipa::path(
    get,
    path = "/api/v2/session",
    responses((status = 200, description = "Session page"))
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> ServerResult<Json<SessionPage>> {
    // Explicit typed branching: an empty `project_id` query selects the
    // global session list, which is a typed state the service expects.
    #[allow(clippy::manual_unwrap_or_default)]
    let project: String = match q.project_id.clone() {
        Some(id) => id,
        None => String::new(),
    };
    let mut items = state.sessions.list(&project).await?;
    let asc = q.order.as_deref() == Some("asc");
    if asc {
        items.sort_by_key(|s| s.time_created);
    } else {
        items.sort_by_key(|s| std::cmp::Reverse(s.time_created));
    }
    if let Some(dir) = q.directory.as_deref() {
        items.retain(|s| s.directory == dir);
    }
    let limit = q.limit.unwrap_or(DEFAULT_SESSION_PAGE_LIMIT);
    items.truncate(limit);
    let count = items.len();
    Ok(Json(SessionPage { items, count }))
}

/// `POST /api/v2/session/:id/prompt`.
#[utoipa::path(
    post,
    path = "/api/v2/session/{id}/prompt",
    responses((status = 200, description = "Accepted"))
)]
pub async fn prompt(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<PromptBody>,
) -> ServerResult<Json<serde_json::Value>> {
    let delivery = match payload.delivery {
        Some(d) => d,
        None => DEFAULT_DELIVERY_MODE.to_string(),
    };
    let _ = state
        .bus
        .publish(
            "session.prompt.requested",
            serde_json::json!({
                "sessionID": id,
                "prompt": payload.prompt,
                "delivery": delivery,
            }),
        )
        .await;
    Ok(Json(
        serde_json::json!({ "sessionID": id, "accepted": true }),
    ))
}

/// `POST /api/v2/session/:id/compact`.
#[utoipa::path(
    post,
    path = "/api/v2/session/{id}/compact",
    responses((status = 200, description = "Compacted"))
)]
pub async fn compact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<bool>> {
    let _ = state
        .bus
        .publish(
            "session.compact.requested",
            serde_json::json!({ "sessionID": id }),
        )
        .await;
    Ok(Json(true))
}

/// `POST /api/v2/session/:id/wait`.
#[utoipa::path(
    post,
    path = "/api/v2/session/{id}/wait",
    responses((status = 200, description = "Waited"))
)]
pub async fn wait(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<bool>> {
    let _ = state
        .bus
        .publish(
            "session.wait.requested",
            serde_json::json!({ "sessionID": id }),
        )
        .await;
    Ok(Json(true))
}
