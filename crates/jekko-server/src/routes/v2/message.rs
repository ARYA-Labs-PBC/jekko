//! `/api/v2/session/:id/message` — paginated message listing.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use jekko_core::session::SessionId;
use jekko_runtime::session::MessageInfo;
use serde::{Deserialize, Serialize};

use crate::error::ServerResult;
use crate::state::AppState;

/// Query for `GET /api/v2/session/:id/message`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListQuery {
    /// Cap on returned messages.
    #[serde(default)]
    pub limit: Option<usize>,
    /// Order (`asc` / `desc`).
    #[serde(default)]
    pub order: Option<String>,
}

/// Paginated response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePage {
    /// Items.
    pub items: Vec<MessageInfo>,
    /// Count in this page.
    pub count: usize,
}

/// Build the v2 message router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/:id/message", get(list))
}

/// `GET /api/v2/session/:id/message`.
#[utoipa::path(
    get,
    path = "/api/v2/session/{id}/message",
    responses((status = 200, description = "Message page"))
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(q): Query<ListQuery>,
) -> ServerResult<Json<MessagePage>> {
    let sid = SessionId::new(id);
    let mut items = state.sessions.messages(&sid).await?;
    let asc = q.order.as_deref() == Some("asc");
    if asc {
        items.sort_by_key(|m| m.time_created);
    } else {
        items.sort_by_key(|m| std::cmp::Reverse(m.time_created));
    }
    let limit = q.limit.unwrap_or(50);
    items.truncate(limit);
    let count = items.len();
    Ok(Json(MessagePage { items, count }))
}
