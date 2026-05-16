//! `/api/v1/question` — interactive question queue.
//!
//! Mirrors the shape of `packages/jekko/src/server/routes/instance/httpapi/handlers/question.ts`.
//! Backing store is [`crate::state::QuestionRegistry`] until a runtime
//! service is exposed.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{ServerError, ServerResult};
use crate::state::AppState;

/// Body for `POST /api/v1/question/:id`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnswerBody {
    /// Free-form answer payload.
    #[schema(value_type = Object)]
    pub answer: serde_json::Value,
}

/// Build the question router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list))
        .route("/:id", get(get_question).post(answer))
}

/// `GET /api/v1/question`.
#[utoipa::path(
    get,
    path = "/api/v1/question",
    responses((status = 200, description = "Pending questions"))
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
) -> ServerResult<Json<Vec<serde_json::Value>>> {
    let snap = state.questions.read().await;
    Ok(Json(snap.pending.values().cloned().collect()))
}

/// `GET /api/v1/question/:id`.
#[utoipa::path(
    get,
    path = "/api/v1/question/{id}",
    responses(
        (status = 200, description = "Question"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn get_question(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<serde_json::Value>> {
    let snap = state.questions.read().await;
    match snap.pending.get(&id).cloned() {
        Some(v) => Ok(Json(v)),
        None => Err(ServerError::not_found(format!("question: {id}"))),
    }
}

/// `POST /api/v1/question/:id`.
#[utoipa::path(
    post,
    path = "/api/v1/question/{id}",
    request_body = AnswerBody,
    responses((status = 200, description = "Answered"))
)]
pub async fn answer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<AnswerBody>,
) -> ServerResult<Json<bool>> {
    {
        let mut snap = state.questions.write().await;
        snap.pending.remove(&id);
    }
    let _ = state
        .bus
        .publish(
            "question.answered",
            serde_json::json!({ "id": id, "answer": payload.answer }),
        )
        .await;
    Ok(Json(true))
}
