//! Error types for the storage layer.

use thiserror::Error;

/// Errors emitted by the `jekko-store` crate.
#[derive(Debug, Error)]
pub enum StoreError {
    /// A row was expected but not found.
    #[error("row not found: {0}")]
    NotFound(String),

    /// Wrapper for any underlying [`rusqlite`] failure.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// JSON column failed to (de)serialize.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Migration journal hit an unrepairable state.
    #[error("migration error: {0}")]
    Migration(String),

    /// Catch-all for misc. validation failures.
    #[error("invalid value: {0}")]
    Invalid(String),
}

/// Convenience alias used by every public function.
pub type StoreResult<T> = Result<T, StoreError>;
