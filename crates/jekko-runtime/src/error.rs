//! Crate-wide error type used by every runtime service.

use std::io;

use thiserror::Error;

/// Result alias used across the runtime crate.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Unified error returned by runtime services.
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// Underlying I/O failure.
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialisation/deserialisation failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Lookup failed for a known kind of identifier.
    #[error("{kind} not found: {id}")]
    NotFound {
        /// What kind of entity was missing (`session`, `permission`, …).
        kind: &'static str,
        /// Stringified identifier that failed to resolve.
        id: String,
    },

    /// A permission ask was rejected by the user.
    #[error("permission rejected: {0}")]
    PermissionRejected(String),

    /// A permission ask was denied by ruleset.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Underlying storage error (rusqlite, drizzle migrations, …).
    #[error("store error: {0}")]
    Store(String),

    /// An external command failed.
    #[error("command failed: {0}")]
    Command(String),

    /// Invalid arguments supplied to a runtime call.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Catch-all for unforeseen errors. Prefer adding a dedicated variant
    /// when a new failure mode becomes common.
    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for RuntimeError {
    fn from(value: anyhow::Error) -> Self {
        RuntimeError::Other(value.to_string())
    }
}

impl From<jekko_store::error::StoreError> for RuntimeError {
    fn from(value: jekko_store::error::StoreError) -> Self {
        RuntimeError::Store(value.to_string())
    }
}

impl From<jekko_provider::error::ProviderError> for RuntimeError {
    fn from(value: jekko_provider::error::ProviderError) -> Self {
        RuntimeError::Other(value.to_string())
    }
}

impl RuntimeError {
    /// Convenience constructor for [`RuntimeError::NotFound`].
    pub fn not_found(kind: &'static str, id: impl Into<String>) -> Self {
        RuntimeError::NotFound {
            kind,
            id: id.into(),
        }
    }

    /// Convenience constructor for [`RuntimeError::InvalidInput`].
    pub fn invalid(msg: impl Into<String>) -> Self {
        RuntimeError::InvalidInput(msg.into())
    }

    /// Convenience constructor for [`RuntimeError::Other`].
    pub fn other(msg: impl Into<String>) -> Self {
        RuntimeError::Other(msg.into())
    }
}
