//! `/api/v1/config` — read + write the in-memory configuration.
//!
//! Ports `packages/jekko/src/server/routes/instance/httpapi/handlers/config.ts`.
//! Persistence to disk is deferred — the in-memory shape under
//! [`crate::state::AppState::config`] is the source of truth for the running
//! process.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use jekko_core::config::Config;

use crate::error::ServerResult;
use crate::state::AppState;

/// Build the config router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_config).put(update_config))
        .route("/providers", get(providers))
}

/// `GET /api/v1/config` — current config snapshot.
#[utoipa::path(
    get,
    path = "/api/v1/config",
    responses((status = 200, description = "Current config"))
)]
pub async fn get_config(
    State(state): State<Arc<AppState>>,
) -> ServerResult<Json<serde_json::Value>> {
    let cfg = state.config.read().await.clone();
    Ok(Json(serde_json::to_value(&cfg)?))
}

/// `PUT /api/v1/config` — replace the in-memory config.
#[utoipa::path(
    put,
    path = "/api/v1/config",
    responses((status = 200, description = "Updated config"))
)]
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> ServerResult<Json<serde_json::Value>> {
    let parsed: Config = serde_json::from_value(payload.clone())?;
    let mut guard = state.config.write().await;
    *guard = parsed;
    Ok(Json(payload))
}

/// `GET /api/v1/config/providers` — provider catalog snapshot.
#[utoipa::path(
    get,
    path = "/api/v1/config/providers",
    responses((status = 200, description = "Provider catalog"))
)]
pub async fn providers(
    State(_state): State<Arc<AppState>>,
) -> ServerResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "providers": [],
        "default": serde_json::Value::Null,
    })))
}
