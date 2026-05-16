//! Error and result types for the provider crate.
use thiserror::Error;

/// Specialized result alias for [`ProviderError`].
pub type ProviderResult<T> = std::result::Result<T, ProviderError>;

/// Provider crate error type covering HTTP, SSE decode, transform and auth
/// failure modes.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// Network transport failure (DNS, TLS, connection reset, …).
    #[error("transport error: {0}")]
    Transport(String),

    /// HTTP request returned a non-2xx status.
    #[error("http {status}: {body}")]
    Http {
        /// HTTP status code returned by the server.
        status: u16,
        /// Raw response body (truncated to a sane length upstream).
        body: String,
    },

    /// SSE parser hit an unexpected frame layout.
    #[error("sse decode error: {0}")]
    SseDecode(String),

    /// JSON serialisation/deserialisation failure.
    #[error("json error: {0}")]
    Json(String),

    /// The request was aborted via the cancellation token.
    #[error("request aborted")]
    Aborted,

    /// Authentication is required but no credential is available.
    #[error("missing credential for provider `{0}`")]
    MissingCredential(String),

    /// Provider returned an explicit error event.
    #[error("provider error: {0}")]
    ProviderEvent(String),

    /// Wraps a generic anyhow error from a downstream adapter.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<reqwest::Error> for ProviderError {
    fn from(err: reqwest::Error) -> Self {
        Self::Transport(err.to_string())
    }
}

impl From<serde_json::Error> for ProviderError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err.to_string())
    }
}
