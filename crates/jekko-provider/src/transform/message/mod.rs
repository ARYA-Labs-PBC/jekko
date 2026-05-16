//! Message transform: canonical `ModelMessage[]` -> provider-shaped messages.
//!
//! Ported from:
//! - `packages/jekko/src/provider/transform-message.ts`
//! - `packages/jekko/src/provider/transform-message-cache.ts`
//! - `packages/jekko/src/provider/transform-message-utils.ts`
//!
//! Strategy: we keep messages as untyped `serde_json::Value` so we don't have
//! to commit to a closed Rust enum for every provider-specific content part.
//! This mirrors the TS `ModelMessage` type from the `ai` SDK which is itself
//! a loose union.

use jekko_core::provider::Model;
use serde_json::{Map, Value};

use super::shared::sdk_key;

mod adjustments;
mod cache;
mod parts;
mod sanitize;

/// Type alias for canonical message documents (loose JSON, like the TS `ModelMessage`).
pub type ModelMessage = Value;

/// Type alias for a content part within a message.
pub type Part = Value;

/// Top-level entry point: apply all message-level transforms.
///
/// Mirrors `message(...)` in `transform-message.ts`.
pub fn message(
    msgs: Vec<ModelMessage>,
    model: &Model,
    options: &Map<String, Value>,
) -> Vec<ModelMessage> {
    let mut msgs = parts::unsupported_parts(msgs, model);
    msgs = normalize_messages(msgs, model, options);

    let api_id = model.api.id.as_str();
    let api_npm = model.api.npm.as_str();
    let provider_id = model.provider_id.as_str();
    let model_id = model.id.as_str();
    let should_cache = (provider_id == "anthropic"
        || provider_id == "google-vertex-anthropic"
        || api_id.contains("anthropic")
        || api_id.contains("claude")
        || model_id.contains("anthropic")
        || model_id.contains("claude")
        || api_npm == "@ai-sdk/anthropic"
        || api_npm == "@ai-sdk/alibaba")
        && api_npm != "@ai-sdk/gateway";
    if should_cache {
        msgs = cache::apply_caching(msgs, model);
    }

    if let Some(key) = sdk_key(api_npm) {
        if key != provider_id {
            msgs = cache::remap_provider_options(msgs, provider_id, key);
        }
    }

    msgs
}

/// Normalise messages: sanitize surrogates, filter empty content, scrub
/// claude tool-call ids, split anthropic tool-call messages, apply mistral
/// and deepseek model adjustments, interleaved reasoning fields, and so on.
fn normalize_messages(
    msgs: Vec<ModelMessage>,
    model: &Model,
    _options: &Map<String, Value>,
) -> Vec<ModelMessage> {
    let mut msgs = msgs;
    for msg in msgs.iter_mut() {
        sanitize::sanitize_surrogates_msg(msg);
    }
    msgs = adjustments::filter_empty_content_if_needed(msgs, model);
    msgs = adjustments::scrub_claude_tool_call_ids_if_needed(msgs, model);
    msgs = adjustments::split_anthropic_tool_call_messages_if_needed(msgs, model);
    msgs = adjustments::apply_mistral_adjustments_if_needed(msgs, model);
    msgs = adjustments::apply_deepseek_adjustments_if_needed(msgs, model);
    msgs = adjustments::apply_interleaved_reasoning_if_needed(msgs, model);
    msgs
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use jekko_core::provider::{
        ModelId, ModelStatus, ProviderApiInfo, ProviderCacheCost, ProviderCapabilities,
        ProviderCost, ProviderId, ProviderInterleaved, ProviderLimit, ProviderModalities,
    };
    use std::collections::BTreeMap;

    fn anthropic_model() -> Model {
        Model {
            id: ModelId::new("anthropic/claude-3-5-sonnet"),
            provider_id: ProviderId::new("anthropic"),
            api: ProviderApiInfo {
                id: "claude-3-5-sonnet-20241022".into(),
                url: "https://api.anthropic.com".into(),
                npm: "@ai-sdk/anthropic".into(),
            },
            name: "Claude".into(),
            family: None,
            capabilities: ProviderCapabilities {
                temperature: true,
                reasoning: false,
                attachment: true,
                toolcall: true,
                input: ProviderModalities {
                    text: true,
                    audio: false,
                    image: true,
                    video: false,
                    pdf: true,
                },
                output: ProviderModalities::default(),
                interleaved: ProviderInterleaved::Bool(false),
            },
            cost: ProviderCost {
                input: 0.0,
                output: 0.0,
                cache: ProviderCacheCost::default(),
                experimental_over_200k: None,
            },
            limit: ProviderLimit {
                context: 200_000.0,
                input: None,
                output: 8192.0,
            },
            status: ModelStatus::Active,
            options: BTreeMap::new(),
            headers: BTreeMap::new(),
            release_date: "2024-01-01".into(),
            variants: None,
        }
    }

    #[test]
    fn applies_cache_to_system_and_final_for_anthropic() {
        let model = anthropic_model();
        let msgs = vec![
            json!({"role": "system", "content": "you are helpful"}),
            json!({"role": "user", "content": "hi"}),
        ];
        let opts = Map::new();
        let out = message(msgs, &model, &opts);
        let po = out[0].get("providerOptions").unwrap();
        assert!(po.get("anthropic").unwrap().get("cacheControl").is_some());
        assert_eq!(
            po["anthropic"]["cacheControl"]["type"].as_str().unwrap(),
            "ephemeral"
        );
    }
}
