//! Provider adapter trait and shared request/response types.
//!
//! Each provider adapter wraps a thin HTTP client and the canonical transform
//! layer to convert a [`ProviderRequest`] into a provider-specific request.
use std::collections::BTreeMap;
use std::pin::Pin;

use async_trait::async_trait;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio_util::sync::CancellationToken;

use crate::error::{ProviderError, ProviderResult};
use crate::stream::{ProviderCapabilities, ProviderEvent};

/// Tool definition passed in by the runtime layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderTool {
    /// Tool name (e.g. `"Read"`).
    pub name: String,
    /// Human-readable tool description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON schema for input validation.
    pub input_schema: Value,
}

/// Canonical request shape passed to every adapter.
///
/// This mirrors the call-site contract of `streamText({...})` in
/// `session/llm.ts`: pre-system + messages + tools + provider options + caps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderRequest {
    /// Target model id, namespaced by provider (e.g. `"anthropic/claude-sonnet-4-5"`).
    pub model: String,
    /// Concrete API model id sent in the request body (e.g. `claude-sonnet-4-5-20250901`).
    pub api_model_id: String,
    /// Session id (used as cache-key seed).
    pub session_id: String,
    /// System prompt segments (joined / inlined by the adapter).
    pub system: Vec<String>,
    /// Canonical messages (already passed through [`crate::transform::message::message`]).
    pub messages: Vec<Value>,
    /// Tool definitions.
    #[serde(default)]
    pub tools: Vec<ProviderTool>,
    /// Tool selection mode (`auto` / `required` / `none`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    /// Free-form provider-specific options as produced by
    /// [`crate::transform::options::options`].
    #[serde(default)]
    pub options: Map<String, Value>,
    /// Free-form HTTP headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Max output tokens.
    pub max_output_tokens: u32,
    /// Sampling temperature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Top-P.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Top-K.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Credential used to authenticate (provider-specific format).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential: Option<ProviderCredential>,
    /// Override base URL (used by gateways / proxies).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

/// Credential payload attached to a request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProviderCredential {
    /// Bearer token (sent as `Authorization: Bearer ...`).
    Bearer {
        /// Raw token value.
        token: String,
    },
    /// API key (sent in a provider-specific header, e.g. `x-api-key`).
    ApiKey {
        /// Raw key value.
        key: String,
    },
    /// OAuth credential (treated as Bearer by default).
    OAuth {
        /// Raw access token.
        access_token: String,
    },
}

/// Boxed stream of provider events.
pub type ProviderStream =
    Pin<Box<dyn Stream<Item = ProviderResult<ProviderEvent>> + Send + 'static>>;

/// Adapter trait implemented per provider.
#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    /// Stream a provider response, yielding canonical [`ProviderEvent`]s.
    async fn stream(
        &self,
        req: ProviderRequest,
        abort: CancellationToken,
    ) -> ProviderResult<ProviderStream>;

    /// Adapter capabilities.
    fn capabilities(&self) -> ProviderCapabilities;
}

/// Helper: returns an error if the credential is missing.
pub fn require_credential<'a>(
    req: &'a ProviderRequest,
    provider_id: &str,
) -> ProviderResult<&'a ProviderCredential> {
    match req.credential.as_ref() {
        Some(credential) => Ok(credential),
        None => Err(ProviderError::MissingCredential(provider_id.to_string())),
    }
}
