//! `/api/v1/mcp` — MCP server proxy handlers.
//!
//! Ports the GET surface of
//! `packages/jekko/src/server/routes/instance/httpapi/handlers/mcp.ts`. The
//! actual proxy/spawn logic stays in `jekko-runtime::mcp` once the trait
//! stabilises.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::error::ServerResult;
use crate::state::AppState;

/// Build the MCP router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list))
        .route("/:name", get(get_server))
}

/// `GET /api/v1/mcp` — list configured MCP servers.
#[utoipa::path(
    get,
    path = "/api/v1/mcp",
    responses((status = 200, description = "MCP servers"))
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
) -> ServerResult<Json<Vec<serde_json::Value>>> {
    let cfg = state.config.read().await;
    let servers: Vec<serde_json::Value> = match cfg.mcp.as_ref() {
        Some(m) => m
            .iter()
            .map(|(name, value)| serde_json::json!({ "name": name, "config": value }))
            .collect(),
        None => Vec::new(),
    };
    Ok(Json(servers))
}

/// `GET /api/v1/mcp/:name` — fetch one server.
#[utoipa::path(
    get,
    path = "/api/v1/mcp/{name}",
    responses((status = 200, description = "MCP server"))
)]
pub async fn get_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> ServerResult<Json<serde_json::Value>> {
    let cfg = state.config.read().await;
    Ok(Json(
        cfg.mcp
            .as_ref()
            .and_then(|m| m.get(&name))
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    ))
}
