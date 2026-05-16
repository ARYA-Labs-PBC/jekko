//! Per-NPM-provider dispatch for `variants(model)`.

use jekko_core::provider::Model;
use serde_json::{json, Map, Value};

use super::efforts::anthropic_adaptive_efforts;
use super::providers;

pub(super) fn build_model_variants(model: &Model) -> Map<String, Value> {
    if !model.capabilities.reasoning {
        return Map::new();
    }
    let id = model.id.to_string().to_lowercase();
    let adaptive_efforts = anthropic_adaptive_efforts(model.api.id.as_str());
    let has_adaptive = !adaptive_efforts.is_empty();

    const SKIP_FAMILIES: &[&str] = &[
        "deepseek-chat",
        "deepseek-reasoner",
        "deepseek-r1",
        "deepseek-v3",
        "minimax",
        "glm",
        "kimi",
        "k2p",
        "qwen",
        "big-pickle",
    ];
    if SKIP_FAMILIES.iter().any(|f| id.contains(f)) {
        return Map::new();
    }

    if let Some(out) = handle_grok(&id, model) {
        return out;
    }

    match model.api.npm.as_str() {
        "@openrouter/ai-sdk-provider" => providers::openrouter(model),
        "ai-gateway-provider" => providers::ai_gateway(model),
        "@ai-sdk/gateway" => providers::ai_sdk_gateway(model, &id, has_adaptive, &adaptive_efforts),
        "@ai-sdk/github-copilot" => providers::github_copilot(model, &id),
        "@ai-sdk/cerebras"
        | "@ai-sdk/togetherai"
        | "@ai-sdk/xai"
        | "@ai-sdk/deepinfra"
        | "venice-ai-sdk-provider"
        | "@ai-sdk/openai-compatible" => providers::openai_compatible_family(model),
        "@ai-sdk/azure" => providers::azure(&id),
        "@ai-sdk/openai" => providers::openai_official(model),
        "@ai-sdk/anthropic" | "@ai-sdk/google-vertex/anthropic" => {
            providers::anthropic_sdk(model, has_adaptive, &adaptive_efforts)
        }
        "@ai-sdk/amazon-bedrock" => {
            providers::amazon_bedrock(model, has_adaptive, &adaptive_efforts)
        }
        "@ai-sdk/google-vertex" | "@ai-sdk/google" => providers::google_sdk(&id),
        "@ai-sdk/mistral" => providers::mistral(model),
        "@ai-sdk/cohere" => Map::new(),
        "@ai-sdk/groq" => providers::groq(),
        "@ai-sdk/perplexity" => Map::new(),
        "@jerome-benoit/sap-ai-provider-v2" => {
            providers::sap_ai(model, &id, has_adaptive, &adaptive_efforts)
        }
        _ => Map::new(),
    }
}

fn handle_grok(id: &str, model: &Model) -> Option<Map<String, Value>> {
    if id.contains("grok") && id.contains("grok-3-mini") {
        if model.api.npm == "@openrouter/ai-sdk-provider" {
            let mut m = Map::new();
            m.insert("low".into(), json!({ "reasoning": { "effort": "low" } }));
            m.insert("high".into(), json!({ "reasoning": { "effort": "high" } }));
            return Some(m);
        }
        let mut m = Map::new();
        m.insert("low".into(), json!({ "reasoningEffort": "low" }));
        m.insert("high".into(), json!({ "reasoningEffort": "high" }));
        return Some(m);
    }
    if id.contains("grok") {
        return Some(Map::new());
    }
    None
}
