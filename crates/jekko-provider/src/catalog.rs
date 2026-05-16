//! Provider catalog helpers.
//!
//! Wraps `jekko_core::provider::*` with the small derived helpers from
//! `provider-schema.ts` and `provider-runtime.ts` (sort, connected provider
//! discovery, default model picking, locked-provider detection).
use std::collections::BTreeMap;

use jekko_core::provider::{
    is_locked_provider, Model, ModelStatus, ProviderInfo, ProviderListResult,
};
use serde::{Deserialize, Serialize};

/// Composite `(provider_id, model_id)` key used as a stable lookup token.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderModelKey {
    /// Provider identifier.
    pub provider_id: String,
    /// Model identifier.
    pub model_id: String,
}

impl ProviderModelKey {
    /// Format as `provider/model`.
    pub fn dotted(&self) -> String {
        format!("{}/{}", self.provider_id, self.model_id)
    }
}

/// A single catalog entry: provider metadata + its model set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderCatalogEntry {
    /// Provider metadata.
    pub info: ProviderInfo,
    /// Default model id for this provider, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_model_id: Option<String>,
}

/// In-memory catalog of providers + per-provider defaults.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProviderCatalog {
    /// All providers, keyed by `provider_id`.
    pub entries: BTreeMap<String, ProviderCatalogEntry>,
}

impl ProviderCatalog {
    /// Build a catalog from a [`ProviderListResult`] (as returned by the
    /// models.dev API or hand-built defaults).
    pub fn from_list(list: ProviderListResult) -> Self {
        let mut entries = BTreeMap::new();
        for info in list.all {
            let default = list.default.get(info.id.as_str()).cloned();
            entries.insert(
                info.id.as_str().to_string(),
                ProviderCatalogEntry {
                    info,
                    default_model_id: default,
                },
            );
        }
        Self { entries }
    }

    /// Returns the catalog entry for `provider_id`.
    pub fn get(&self, provider_id: &str) -> Option<&ProviderCatalogEntry> {
        self.entries.get(provider_id)
    }

    /// Returns the model record for `(provider_id, model_id)`, if present.
    pub fn get_model(&self, key: &ProviderModelKey) -> Option<&Model> {
        self.entries
            .get(&key.provider_id)
            .and_then(|e| e.info.models.get(&key.model_id))
    }

    /// Returns the provider ids that are *connected* (i.e. have a non-empty,
    /// non-locked model set).
    ///
    /// Mirrors `connectedProviderIDs` in `provider-schema.ts`.
    pub fn connected_provider_ids(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter_map(|(id, entry)| {
                if catalog_is_locked_provider(&entry.info) {
                    None
                } else {
                    Some(id.clone())
                }
            })
            .collect()
    }

    /// Default model per provider, computed from the first non-locked model
    /// per [`sort_models`] order.
    pub fn default_model_ids(&self) -> BTreeMap<String, String> {
        self.entries
            .iter()
            .filter_map(|(id, entry)| {
                if let Some(d) = entry.default_model_id.clone() {
                    return Some((id.clone(), d));
                }
                let mut models: Vec<&Model> = entry
                    .info
                    .models
                    .values()
                    .filter(|m| m.status != ModelStatus::Locked)
                    .collect();
                sort_models(&mut models);
                models.first().map(|m| (id.clone(), m.id.to_string()))
            })
            .collect()
    }
}

/// Whether every model in a provider is locked.
///
/// Direct re-export of `is_locked_provider` from `jekko-core` under a name
/// that matches the TS `isLockedProvider` for readability at call sites.
pub fn catalog_is_locked_provider(provider: &ProviderInfo) -> bool {
    is_locked_provider(provider)
}

/// Sorting order ported from `provider-schema.ts#sort`.
///
/// The TS implementation sorts by:
///   1. presence of any of `["gpt-5", "claude-sonnet-4", "big-pickle", "gemini-3-pro"]` as a substring (desc)
///   2. `"latest"` substring (asc — `latest` first)
///   3. id (desc)
pub fn sort_models(models: &mut [&Model]) {
    const PRIORITY_FILTERS: [&str; 4] = ["gpt-5", "claude-sonnet-4", "big-pickle", "gemini-3-pro"];
    models.sort_by(|a, b| {
        let aid = a.id.as_str();
        let bid = b.id.as_str();

        // Priority filter index: lower = better (matches earlier filter); -1 means no match.
        let a_idx = PRIORITY_FILTERS
            .iter()
            .position(|f| aid.contains(*f))
            .map(|p| p as i32)
            .unwrap_or(-1);
        let b_idx = PRIORITY_FILTERS
            .iter()
            .position(|f| bid.contains(*f))
            .map(|p| p as i32)
            .unwrap_or(-1);
        // TS sortBy uses "desc" which actually means larger value first; for a
        // -1 sentinel, no-match should come last, so we flip.
        let a_key = if a_idx < 0 { i32::MAX } else { a_idx };
        let b_key = if b_idx < 0 { i32::MAX } else { b_idx };
        match a_key.cmp(&b_key) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        // "latest" presence: `1` (latest) sorts before `0` per TS comparator.
        let a_latest = if aid.contains("latest") { 0 } else { 1 };
        let b_latest = if bid.contains("latest") { 0 } else { 1 };
        match a_latest.cmp(&b_latest) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        // id desc.
        bid.cmp(aid)
    });
}

#[cfg(test)]
mod tests {
    use jekko_core::provider::{
        InterleavedField, Model, ModelId, ModelStatus, ProviderApiInfo, ProviderCacheCost,
        ProviderCapabilities, ProviderCost, ProviderId, ProviderInfo, ProviderInterleaved,
        ProviderLimit, ProviderListResult, ProviderModalities, ProviderSource,
    };
    use std::collections::BTreeMap;

    use super::*;

    fn mock_model(id: &str, status: ModelStatus) -> Model {
        Model {
            id: ModelId::new(id),
            provider_id: ProviderId::new("anthropic"),
            api: ProviderApiInfo {
                id: id.to_string(),
                url: "https://api.anthropic.com".into(),
                npm: "@ai-sdk/anthropic".into(),
            },
            name: id.to_string(),
            family: None,
            capabilities: ProviderCapabilities {
                temperature: true,
                reasoning: false,
                attachment: true,
                toolcall: true,
                input: ProviderModalities::default(),
                output: ProviderModalities::default(),
                interleaved: ProviderInterleaved::Bool(false),
            },
            cost: ProviderCost {
                input: 1.0,
                output: 1.0,
                cache: ProviderCacheCost::default(),
                experimental_over_200k: None,
            },
            limit: ProviderLimit {
                context: 200_000.0,
                input: None,
                output: 8192.0,
            },
            status,
            options: BTreeMap::new(),
            headers: BTreeMap::new(),
            release_date: "2025-01-01".to_string(),
            variants: None,
        }
    }

    fn mock_provider(id: &str, models: Vec<Model>) -> ProviderInfo {
        let mut m = BTreeMap::new();
        for model in models {
            m.insert(model.id.to_string(), model);
        }
        ProviderInfo {
            id: ProviderId::new(id),
            name: id.to_string(),
            source: ProviderSource::Api,
            env: vec![],
            auth: None,
            options: BTreeMap::new(),
            models: m,
        }
    }

    #[test]
    fn locked_provider_detection() {
        let locked = mock_provider(
            "anthropic",
            vec![
                mock_model("a", ModelStatus::Locked),
                mock_model("b", ModelStatus::Locked),
            ],
        );
        assert!(catalog_is_locked_provider(&locked));
        let mixed = mock_provider(
            "anthropic",
            vec![
                mock_model("a", ModelStatus::Locked),
                mock_model("b", ModelStatus::Active),
            ],
        );
        assert!(!catalog_is_locked_provider(&mixed));
        let empty = mock_provider("anthropic", vec![]);
        assert!(!catalog_is_locked_provider(&empty));
    }

    #[test]
    fn sort_prefers_priority_filters_and_latest() {
        let m1 = mock_model("random-model", ModelStatus::Active);
        let m2 = mock_model("gpt-5-mini", ModelStatus::Active);
        let m3 = mock_model("claude-sonnet-4-5", ModelStatus::Active);
        let m4 = mock_model("claude-sonnet-4-latest", ModelStatus::Active);

        let mut models: Vec<&Model> = vec![&m1, &m2, &m3, &m4];
        sort_models(&mut models);
        // gpt-5 first (index 0), then claude-sonnet-4 ("latest" first), then random.
        assert_eq!(models[0].id.as_str(), "gpt-5-mini");
        assert!(models[1].id.as_str().contains("claude-sonnet-4"));
        assert_eq!(models[3].id.as_str(), "random-model");
    }

    #[test]
    fn catalog_from_list_round_trips() {
        let list = ProviderListResult {
            all: vec![mock_provider(
                "anthropic",
                vec![mock_model("claude-sonnet-4-5", ModelStatus::Active)],
            )],
            default: {
                let mut m = BTreeMap::new();
                m.insert("anthropic".to_string(), "claude-sonnet-4-5".to_string());
                m
            },
            connected: vec!["anthropic".to_string()],
        };
        let catalog = ProviderCatalog::from_list(list);
        let entry = catalog.get("anthropic").unwrap();
        assert_eq!(entry.default_model_id.as_deref(), Some("claude-sonnet-4-5"));
        assert_eq!(
            catalog
                .get_model(&ProviderModelKey {
                    provider_id: "anthropic".into(),
                    model_id: "claude-sonnet-4-5".into(),
                })
                .unwrap()
                .id
                .as_str(),
            "claude-sonnet-4-5"
        );
        assert_eq!(catalog.connected_provider_ids(), vec!["anthropic"]);
    }

    // Quiet the test-only re-export warning for InterleavedField.
    #[allow(dead_code)]
    fn _force_use(_f: InterleavedField) {}
}
