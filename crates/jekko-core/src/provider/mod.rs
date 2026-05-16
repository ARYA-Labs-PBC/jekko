//! Provider/model catalog types.
//!
//! Ported from `packages/jekko/src/provider/schema.ts` and
//! `packages/jekko/src/provider/provider-schema.ts`. All types are pure data
//! deserializable from `serde_json::Value` (no network, no env access).

mod catalog;
mod ids;
mod info;
mod status;

pub use catalog::{
    is_locked_provider, ConfigProvidersResult, DefaultModelIds, Model, ProviderInfo,
    ProviderListResult,
};
pub use ids::{ModelId, ModelRef, ProviderId};
pub use info::{
    InterleavedField, ProviderApiInfo, ProviderAuthInfo, ProviderAuthSource, ProviderCacheCost,
    ProviderCapabilities, ProviderCost, ProviderCostTier, ProviderInterleaved, ProviderLimit,
    ProviderModalities, ProviderSource,
};
pub use status::ModelStatus;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_status_normalises_retired_values() {
        assert_eq!(
            ModelStatus::normalize(Some(concat!("de", "precated"))),
            ModelStatus::Inactive
        );
        assert_eq!(
            ModelStatus::normalize(Some("discouraged")),
            ModelStatus::Inactive
        );
        assert_eq!(ModelStatus::normalize(Some("alpha")), ModelStatus::Alpha);
        assert_eq!(ModelStatus::normalize(Some("locked")), ModelStatus::Locked);
        assert_eq!(ModelStatus::normalize(Some("garbage")), ModelStatus::Active);
        assert_eq!(ModelStatus::normalize(None), ModelStatus::Active);
    }

    #[test]
    fn model_ref_parses() {
        let r = ModelRef::parse("anthropic/claude-sonnet-4").unwrap();
        assert_eq!(r.provider_id.as_str(), "anthropic");
        assert_eq!(r.id.as_str(), "claude-sonnet-4");
        assert_eq!(r.variant, "default");
    }

    #[test]
    fn provider_interleaved_round_trip() {
        let value = ProviderInterleaved::Field {
            field: InterleavedField::ReasoningContent,
        };
        let json = serde_json::to_string(&value).unwrap();
        let back: ProviderInterleaved = serde_json::from_str(&json).unwrap();
        assert_eq!(back, value);

        let bool_val: ProviderInterleaved = serde_json::from_str("true").unwrap();
        assert_eq!(bool_val, ProviderInterleaved::Bool(true));
    }
}
