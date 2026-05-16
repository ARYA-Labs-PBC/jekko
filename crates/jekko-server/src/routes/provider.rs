//! `/api/v1/provider` — provider catalog + auth status.
//!
//! Ports `packages/jekko/src/server/routes/instance/httpapi/handlers/provider.ts`.
//! The handler is wired to a static catalog snapshot until the provider
//! service is plugged into [`crate::state::AppState`].

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ServerResult;
use crate::state::AppState;

/// Generic provider descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProviderListItem {
    /// Provider id (e.g. `anthropic`).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Whether the provider currently has credentials configured.
    pub connected: bool,
}

/// Response body for `GET /api/v1/provider`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProviderListResponse {
    /// All known providers.
    pub all: Vec<ProviderListItem>,
    /// Connected provider ids.
    pub connected: Vec<String>,
    /// Default model id per provider, if any.
    pub default: serde_json::Value,
}

/// Build the provider router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list))
        .route("/auth", get(auth_methods))
        .route("/:id/authorize", get(authorize_get))
}

/// `GET /api/v1/provider` — provider catalog snapshot.
#[utoipa::path(
    get,
    path = "/api/v1/provider",
    responses((status = 200, description = "Providers", body = ProviderListResponse))
)]
pub async fn list(State(_state): State<Arc<AppState>>) -> ServerResult<Json<ProviderListResponse>> {
    Ok(Json(ProviderListResponse {
        all: Vec::new(),
        connected: Vec::new(),
        default: serde_json::Value::Null,
    }))
}

/// `GET /api/v1/provider/auth`.
#[utoipa::path(
    get,
    path = "/api/v1/provider/auth",
    responses((status = 200, description = "Auth methods"))
)]
pub async fn auth_methods(
    State(_state): State<Arc<AppState>>,
) -> ServerResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "methods": [] })))
}

/// `GET /api/v1/provider/:id/authorize`.
#[utoipa::path(
    get,
    path = "/api/v1/provider/{id}/authorize",
    responses((status = 200, description = "Authorize response"))
)]
pub async fn authorize_get(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "providerID": id,
        "status": "not_implemented",
    })))
}
