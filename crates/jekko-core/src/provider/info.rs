//! Provider / model side-info types: API metadata, modalities, capabilities,
//! pricing, limits, and auth/source descriptors.

use serde::{Deserialize, Serialize};

/// API endpoint metadata (`{ id, url, npm }`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderApiInfo {
    /// API id.
    pub id: String,
    /// API base URL.
    pub url: String,
    /// NPM package backing this provider's runtime.
    pub npm: String,
}

/// Modality matrix.
///
/// Ported from `ProviderModalities` in `provider-schema.ts`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderModalities {
    /// Supports text.
    pub text: bool,
    /// Supports audio.
    pub audio: bool,
    /// Supports image.
    pub image: bool,
    /// Supports video.
    pub video: bool,
    /// Supports PDF.
    pub pdf: bool,
}

/// Interleaved-mode descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProviderInterleaved {
    /// Boolean form (`true`/`false`).
    Bool(bool),
    /// Object form, naming the field on the response that carries interleaved data.
    Field {
        /// Which field carries the interleaved content.
        field: InterleavedField,
    },
}

/// Allowed values for `ProviderInterleaved::Field`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterleavedField {
    /// `reasoning_content` field.
    ReasoningContent,
    /// `reasoning_details` field.
    ReasoningDetails,
}

/// Provider/model capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Supports a temperature parameter.
    pub temperature: bool,
    /// Supports reasoning blocks.
    pub reasoning: bool,
    /// Supports attachments.
    pub attachment: bool,
    /// Supports tool-calling.
    pub toolcall: bool,
    /// Input modality matrix.
    pub input: ProviderModalities,
    /// Output modality matrix.
    pub output: ProviderModalities,
    /// Interleaved mode descriptor.
    pub interleaved: ProviderInterleaved,
}

/// Cache pricing breakdown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ProviderCacheCost {
    /// Cost per cache read unit.
    pub read: f64,
    /// Cost per cache write unit.
    pub write: f64,
}

/// Pricing structure for a single tier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderCostTier {
    /// Cost per input unit.
    pub input: f64,
    /// Cost per output unit.
    pub output: f64,
    /// Cache cost breakdown.
    pub cache: ProviderCacheCost,
}

/// Pricing structure (optionally including an experimental >200K tier).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderCost {
    /// Cost per input unit.
    pub input: f64,
    /// Cost per output unit.
    pub output: f64,
    /// Cache cost breakdown.
    pub cache: ProviderCacheCost,
    /// Optional experimental >200K-context tier pricing.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "experimentalOver200K"
    )]
    pub experimental_over_200k: Option<ProviderCostTier>,
}

/// Context/output window limits for a model.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProviderLimit {
    /// Total context window in tokens.
    pub context: f64,
    /// Maximum input length (optional; often equals `context`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,
    /// Maximum output length in tokens.
    pub output: f64,
}

/// Per-model authentication metadata (Info.auth).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderAuthInfo {
    /// Whether auth is configured for this provider.
    pub configured: bool,
    /// Whether the configured auth is currently active.
    pub active: bool,
    /// Source of the credential.
    pub source: ProviderAuthSource,
    /// Env var name, when sourced from env.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "envName")]
    pub env_name: Option<String>,
    /// User dir id that produced the value when `source == UserLlmEnv`.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "userID")]
    pub user_id: Option<String>,
    /// Reason the credential is inactive, if applicable.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "inactiveReason"
    )]
    pub inactive_reason: Option<String>,
}

/// Where a provider credential came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderAuthSource {
    /// Legacy `~/.jekko/jekko.env` file.
    #[serde(rename = "jekko.env")]
    JekkoEnv,
    /// `~/.jekko/users/<user_id>/llm.env` file.
    /// The owning [`ProviderAuthInfo`] carries the `user_id`.
    #[serde(rename = "users-llm.env")]
    UserLlmEnv,
    /// Process env (`process.env.X`).
    #[serde(rename = "process-env")]
    ProcessEnv,
    /// OAuth flow.
    #[serde(rename = "oauth")]
    OAuth,
    /// Config file.
    #[serde(rename = "config-file")]
    ConfigFile,
    /// No auth (public provider).
    #[serde(rename = "public")]
    Public,
}

/// Where the provider definition came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderSource {
    /// Provider defined via environment variables.
    Env,
    /// Provider defined via a key file.
    #[serde(rename = "keyfile")]
    KeyFile,
    /// Provider defined in `jekko.json`.
    Config,
    /// Provider defined as a custom entry.
    Custom,
    /// Provider sourced from the models.dev API.
    Api,
}
