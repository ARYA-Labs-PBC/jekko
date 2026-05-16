//! `/api/v1/sync` — event sync helpers.
//!
//! Ports the read-only subset of
//! `packages/jekko/src/server/routes/instance/httpapi/handlers/sync.ts`.
//! The write side (replay, steal) is kept separate from the read-only path
//! until the corresponding service surface lands.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ServerResult;
use crate::state::AppState;

/// Empty payload for `POST /api/v1/sync/start`.
#[derive(Debug, Default, Clone, Serialize, Deserialize, ToSchema)]
pub struct StartBody {}

/// Build the sync router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(history))
        .route("/start", post(start))
        .route("/replay", post(replay))
}

/// `GET /api/v1/sync` — return an empty event history snapshot.
#[utoipa::path(
    get,
    path = "/api/v1/sync",
    responses((status = 200, description = "Event history"))
)]
pub async fn history(
    State(_state): State<Arc<AppState>>,
) -> ServerResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(Vec::new()))
}

/// `POST /api/v1/sync/start` — request workspace sync.
#[utoipa::path(
    post,
    path = "/api/v1/sync/start",
    responses((status = 200, description = "Started"))
)]
pub async fn start(State(state): State<Arc<AppState>>) -> ServerResult<Json<bool>> {
    let _ = state
        .bus
        .publish("sync.started", serde_json::json!({}))
        .await;
    Ok(Json(true))
}

/// `POST /api/v1/sync/replay` — accept a batch of events to replay.
#[utoipa::path(
    post,
    path = "/api/v1/sync/replay",
    responses((status = 200, description = "Replayed"))
)]
pub async fn replay(
    State(state): State<Arc<AppState>>,
    Json(events): Json<Vec<serde_json::Value>>,
) -> ServerResult<Json<usize>> {
    let count = events.len();
    for event in events {
        let _ = state.bus.publish("sync.replay", event).await;
    }
    Ok(Json(count))
}
