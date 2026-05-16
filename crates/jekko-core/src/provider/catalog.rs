//! Provider catalog records: [`Model`], [`ProviderInfo`], and the
//! `Provider.list()` / `Provider.configProviders()` result shapes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::ids::ProviderId;
use super::info::{
    ProviderApiInfo, ProviderAuthInfo, ProviderCapabilities, ProviderCost, ProviderLimit,
    ProviderSource,
};
use super::status::ModelStatus;
use crate::provider::ModelId;

/// A single model record inside a provider catalog entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// Model identifier.
    pub id: ModelId,
    /// Provider identifier.
    #[serde(rename = "providerID")]
    pub provider_id: ProviderId,
    /// API endpoint metadata.
    pub api: ProviderApiInfo,
    /// Human-friendly model name.
    pub name: String,
    /// Optional family grouping (e.g. `"claude-sonnet"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// Capability matrix.
    pub capabilities: ProviderCapabilities,
    /// Pricing.
    pub cost: ProviderCost,
    /// Token limits.
    pub limit: ProviderLimit,
    /// Lifecycle status.
    pub status: ModelStatus,
    /// Free-form options passed to the runtime SDK.
    #[serde(default)]
    pub options: BTreeMap<String, serde_json::Value>,
    /// Free-form HTTP headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Release date (ISO-8601 string).
    pub release_date: String,
    /// Optional variant overrides (per-variant option maps).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variants: Option<BTreeMap<String, BTreeMap<String, serde_json::Value>>>,
}

/// A provider catalog entry containing one or more models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider identifier.
    pub id: ProviderId,
    /// Human-friendly provider name.
    pub name: String,
    /// Where this provider definition was sourced from.
    pub source: ProviderSource,
    /// Env var names this provider looks at.
    pub env: Vec<String>,
    /// Auth metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<ProviderAuthInfo>,
    /// Default options.
    #[serde(default)]
    pub options: BTreeMap<String, serde_json::Value>,
    /// Models keyed by model id.
    #[serde(default)]
    pub models: BTreeMap<String, Model>,
}

/// Map of `provider_id -> default_model_id`.
pub type DefaultModelIds = BTreeMap<String, String>;

/// Result of `Provider.list()`.
///
/// Ported from `ListResult` in `provider-schema.ts`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProviderListResult {
    /// All known providers.
    pub all: Vec<ProviderInfo>,
    /// Default model per provider.
    #[serde(default)]
    pub default: DefaultModelIds,
    /// Connected (auth-configured) provider ids.
    #[serde(default)]
    pub connected: Vec<String>,
}

/// Result of `Provider.configProviders()`.
///
/// Ported from `ConfigProvidersResult` in `provider-schema.ts`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ConfigProvidersResult {
    /// Providers loaded from user config.
    pub providers: Vec<ProviderInfo>,
    /// Default model per provider.
    #[serde(default)]
    pub default: DefaultModelIds,
}

/// True iff every model in `provider.models` reports [`ModelStatus::Locked`].
pub fn is_locked_provider(provider: &ProviderInfo) -> bool {
    !provider.models.is_empty()
        && provider
            .models
            .values()
            .all(|m| m.status == ModelStatus::Locked)
}
