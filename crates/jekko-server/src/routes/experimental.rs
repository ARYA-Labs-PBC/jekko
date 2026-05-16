//! `/api/v1/experimental` — feature flag toggles.
//!
//! Ports `packages/jekko/src/server/routes/instance/httpapi/handlers/experimental.ts`.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ServerResult;
use crate::state::AppState;

/// Body of `PUT /api/v1/experimental/:flag`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlagBody {
    /// New value.
    #[schema(value_type = Object)]
    pub value: serde_json::Value,
}

/// Build the experimental router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list))
        .route("/:flag", get(get_flag).put(set_flag))
}

/// `GET /api/v1/experimental`.
#[utoipa::path(
    get,
    path = "/api/v1/experimental",
    responses((status = 200, description = "All experimental flags"))
)]
pub async fn list(State(state): State<Arc<AppState>>) -> ServerResult<Json<serde_json::Value>> {
    let snap = state.experimental.read().await;
    let value = snap
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<serde_json::Map<_, _>>();
    Ok(Json(serde_json::Value::Object(value)))
}

/// `GET /api/v1/experimental/:flag`.
#[utoipa::path(
    get,
    path = "/api/v1/experimental/{flag}",
    responses((status = 200, description = "Flag"))
)]
pub async fn get_flag(
    State(state): State<Arc<AppState>>,
    Path(flag): Path<String>,
) -> ServerResult<Json<serde_json::Value>> {
    let snap = state.experimental.read().await;
    Ok(Json(
        snap.get(&flag).cloned().unwrap_or(serde_json::Value::Null),
    ))
}

/// `PUT /api/v1/experimental/:flag`.
#[utoipa::path(
    put,
    path = "/api/v1/experimental/{flag}",
    request_body = FlagBody,
    responses((status = 200, description = "Updated"))
)]
pub async fn set_flag(
    State(state): State<Arc<AppState>>,
    Path(flag): Path<String>,
    Json(payload): Json<FlagBody>,
) -> ServerResult<Json<serde_json::Value>> {
    {
        let mut snap = state.experimental.write().await;
        snap.insert(flag.clone(), payload.value.clone());
    }
    let _ = state
        .bus
        .publish(
            "experimental.flag.changed",
            serde_json::json!({ "flag": flag, "value": payload.value }),
        )
        .await;
    Ok(Json(payload.value))
}
