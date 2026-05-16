//! `/api/v1/file` — read / list / glob filesystem ops.
//!
//! Ports `packages/jekko/src/server/routes/instance/httpapi/handlers/file.ts`
//! against the helpers in [`jekko_runtime::file`].

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use jekko_runtime::file as fsx;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ServerResult;
use crate::state::AppState;

/// Query for `GET /api/v1/file/content?path=…`.
#[derive(Debug, Deserialize)]
pub struct ContentQuery {
    /// Absolute path to read.
    pub path: String,
}

/// Query for `GET /api/v1/file/list?path=…`.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Absolute path to list.
    pub path: String,
}

/// Query for `GET /api/v1/file/find?query=…`.
#[derive(Debug, Deserialize)]
pub struct FindQuery {
    /// Pattern (e.g. `**/*.rs`).
    pub query: String,
    /// Optional base directory; defaults to the instance directory.
    #[serde(default)]
    pub base: Option<String>,
    /// Cap on returned entries.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Query for `GET /api/v1/file/find-text?pattern=…`.
#[derive(Debug, Deserialize)]
pub struct FindTextQuery {
    /// Plain-text pattern.
    pub pattern: String,
    /// Cap on returned hits.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Response body for `GET /api/v1/file/list`.
#[derive(Debug, Serialize, ToSchema)]
pub struct DirEntryDto {
    /// Path.
    pub path: String,
    /// Kind (`file` / `directory` / `symlink`).
    pub kind: String,
    /// Size in bytes.
    pub size: u64,
}

/// Build the file router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/content", get(content))
        .route("/list", get(list))
        .route("/find", get(find))
        .route("/find-text", get(find_text))
}

/// `GET /api/v1/file/content?path=…`.
#[utoipa::path(
    get,
    path = "/api/v1/file/content",
    responses((status = 200, description = "File contents"))
)]
pub async fn content(
    State(_state): State<Arc<AppState>>,
    Query(q): Query<ContentQuery>,
) -> ServerResult<Json<String>> {
    let text = fsx::read_file(&q.path, fsx::DEFAULT_MAX_BYTES * 32).await?;
    Ok(Json(text))
}

/// `GET /api/v1/file/list?path=…`.
#[utoipa::path(
    get,
    path = "/api/v1/file/list",
    responses((status = 200, description = "Directory entries"))
)]
pub async fn list(
    State(_state): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> ServerResult<Json<Vec<DirEntryDto>>> {
    let entries = fsx::list_dir(&q.path).await?;
    let out: Vec<DirEntryDto> = entries
        .into_iter()
        .map(|e| DirEntryDto {
            path: e.path.display().to_string(),
            kind: match e.kind {
                fsx::EntryKind::File => "file".to_string(),
                fsx::EntryKind::Directory => "directory".to_string(),
                fsx::EntryKind::Symlink => "symlink".to_string(),
            },
            size: e.size,
        })
        .collect();
    Ok(Json(out))
}

/// `GET /api/v1/file/find?query=&base=&limit=`.
#[utoipa::path(
    get,
    path = "/api/v1/file/find",
    responses((status = 200, description = "Glob hits"))
)]
pub async fn find(
    State(state): State<Arc<AppState>>,
    Query(q): Query<FindQuery>,
) -> ServerResult<Json<Vec<String>>> {
    let base = match q.base.clone() {
        Some(b) => b,
        None => state.instance.directory.clone(),
    };
    let mut hits = fsx::glob(&base, &q.query)?;
    if let Some(limit) = q.limit {
        hits.truncate(limit);
    }
    Ok(Json(
        hits.into_iter().map(|p| p.display().to_string()).collect(),
    ))
}

/// `GET /api/v1/file/find-text?pattern=&limit=`.
#[utoipa::path(
    get,
    path = "/api/v1/file/find-text",
    responses((status = 200, description = "Ripgrep hits"))
)]
pub async fn find_text(
    State(_state): State<Arc<AppState>>,
    Query(_q): Query<FindTextQuery>,
) -> ServerResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(Vec::new()))
}
