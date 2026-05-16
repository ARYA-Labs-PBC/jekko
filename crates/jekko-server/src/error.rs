//! Crate-wide error type with a Hono-equivalent `IntoResponse` impl.
//!
//! Ported behaviourally from `packages/jekko/src/server/error.ts`. Every
//! handler returns [`ServerResult`]; conversion into an HTTP response is
//! handled here so the call sites stay tidy.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::json;
use thiserror::Error;

use jekko_runtime::error::RuntimeError;

/// Crate-wide result alias.
pub type ServerResult<T> = Result<T, ServerError>;

/// Errors returned by route handlers.
#[derive(Debug, Error)]
pub enum ServerError {
    /// Resource not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Bad request (validation / decoding).
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Unauthorized request (missing / wrong credentials).
    #[error("unauthorized")]
    Unauthorized,

    /// Forbidden request.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// Generic upstream failure from `jekko-runtime`.
    #[error("runtime error: {0}")]
    Runtime(String),

    /// Generic internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl ServerError {
    /// Convenience constructor for [`ServerError::NotFound`].
    pub fn not_found(msg: impl Into<String>) -> Self {
        ServerError::NotFound(msg.into())
    }

    /// Convenience constructor for [`ServerError::BadRequest`].
    pub fn bad_request(msg: impl Into<String>) -> Self {
        ServerError::BadRequest(msg.into())
    }

    /// Convenience constructor for [`ServerError::Internal`].
    pub fn internal(msg: impl Into<String>) -> Self {
        ServerError::Internal(msg.into())
    }
}

impl From<RuntimeError> for ServerError {
    fn from(err: RuntimeError) -> Self {
        match err {
            RuntimeError::NotFound { kind, id } => ServerError::NotFound(format!("{kind}: {id}")),
            RuntimeError::PermissionDenied(msg) => ServerError::Forbidden(msg),
            RuntimeError::PermissionRejected(msg) => ServerError::Forbidden(msg),
            RuntimeError::InvalidInput(msg) => ServerError::BadRequest(msg),
            other => ServerError::Runtime(other.to_string()),
        }
    }
}

impl From<serde_json::Error> for ServerError {
    fn from(value: serde_json::Error) -> Self {
        ServerError::BadRequest(value.to_string())
    }
}

impl From<anyhow::Error> for ServerError {
    fn from(value: anyhow::Error) -> Self {
        ServerError::Internal(value.to_string())
    }
}

impl From<std::io::Error> for ServerError {
    fn from(value: std::io::Error) -> Self {
        ServerError::Internal(value.to_string())
    }
}

/// JSON body returned for error responses.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    /// Tag identifying the error category.
    pub error: String,
    /// Human-readable message.
    pub message: String,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, tag, message) = match &self {
            ServerError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg.clone()),
            ServerError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg.clone()),
            ServerError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "missing or invalid credentials".to_string(),
            ),
            ServerError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg.clone()),
            ServerError::Runtime(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "runtime_error",
                msg.clone(),
            ),
            ServerError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                msg.clone(),
            ),
        };
        let body = Json(json!({
            "error": tag,
            "message": message,
        }));
        (status, body).into_response()
    }
}
