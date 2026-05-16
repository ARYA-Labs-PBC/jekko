//! Per-NPM-provider builders extracted from `build_model_variants`.

use jekko_core::provider::Model;
use serde_json::{json, Map, Value};

use crate::transform::shared::{OPENAI_EFFORTS, WIDELY_SUPPORTED_EFFORTS};

use super::efforts::{
    anthropic_simple_adaptive, anthropic_static_budgets, api_id_has_o_digit,
    google_thinking_config, oai_encrypted_efforts, openai_reasoning_efforts,
};

pub(super) fn openrouter(model: &Model) -> Map<String, Value> {
    if !model.id.as_str().contains("gpt")
        && !model.id.as_str().contains("gemini-3")
        && !model.id.as_str().contains("claude")
    {
        return Map::new();
    }
    OPENAI_EFFORTS
        .iter()
        .map(|e| ((*e).to_string(), json!({ "reasoning": { "effort": e } })))
        .collect()
}

pub(super) fn ai_gateway(model: &Model) -> Map<String, Value> {
    if model.api.id.starts_with("openai/") {
        let efforts = openai_reasoning_efforts(model.api.id.as_str(), model.release_date.as_str());
        if efforts.is_empty() {
            return Map::new();
        }
        return efforts
            .iter()
            .map(|e| ((*e).to_string(), json!({ "reasoningEffort": e })))
            .collect();
    }
    WIDELY_SUPPORTED_EFFORTS
        .iter()
        .map(|e| ((*e).to_string(), json!({ "reasoningEffort": e })))
        .collect()
}

pub(super) fn ai_sdk_gateway(
    model: &Model,
    id: &str,
    has_adaptive: bool,
    adaptive_efforts: &[&'static str],
) -> Map<String, Value> {
    if model.id.as_str().contains("anthropic") {
        if has_adaptive {
            return anthropic_simple_adaptive(adaptive_efforts);
        }
        return anthropic_static_budgets();
    }
    if model.id.as_str().contains("google") {
        if id.contains("2.5") {
            return google_thinking_config();
        }
        return ["low", "high"]
            .iter()
            .map(|e| {
                (
                    (*e).to_string(),
                    json!({ "includeThoughts": true, "thinkingLevel": e }),
                )
            })
            .collect();
    }
    OPENAI_EFFORTS
        .iter()
        .map(|e| ((*e).to_string(), json!({ "reasoningEffort": e })))
        .collect()
}

pub(super) fn github_copilot(model: &Model, id: &str) -> Map<String, Value> {
    if model.id.as_str().contains("gemini") {
        return Map::new();
    }
    if model.id.as_str().contains("claude") {
        return WIDELY_SUPPORTED_EFFORTS
            .iter()
            .map(|e| ((*e).to_string(), json!({ "reasoningEffort": e })))
            .collect();
    }
    let mut efforts: Vec<&'static str> =
        if id.contains("5.1-codex-max") || id.contains("5.2") || id.contains("5.3") {
            let mut v = WIDELY_SUPPORTED_EFFORTS.to_vec();
            v.push("xhigh");
            v
        } else {
            let mut v = WIDELY_SUPPORTED_EFFORTS.to_vec();
            if id.contains("gpt-5") && model.release_date.as_str() >= "2025-12-04" {
                v.push("xhigh");
            }
            v
        };
    // dedupe in case of overlap.
    efforts.sort();
    efforts.dedup();
    oai_encrypted_efforts(&efforts)
}

pub(super) fn openai_compatible_family(model: &Model) -> Map<String, Value> {
    let mut efforts: Vec<&'static str> = WIDELY_SUPPORTED_EFFORTS.to_vec();
    if model.api.id.to_lowercase().contains("deepseek-v4") {
        efforts.push("max");
    }
    efforts
        .iter()
        .map(|e| ((*e).to_string(), json!({ "reasoningEffort": e })))
        .collect()
}

pub(super) fn azure(id: &str) -> Map<String, Value> {
    if id == "o1-mini" {
        return Map::new();
    }
    let mut azure: Vec<&'static str> = vec!["low", "medium", "high"];
    if id.contains("gpt-5-") || id == "gpt-5" {
        azure.insert(0, "minimal");
    }
    oai_encrypted_efforts(&azure)
}

pub(super) fn openai_official(model: &Model) -> Map<String, Value> {
    let efforts = openai_reasoning_efforts(model.api.id.as_str(), model.release_date.as_str());
    if efforts.is_empty() {
        return Map::new();
    }
    oai_encrypted_efforts(&efforts)
}

pub(super) fn anthropic_sdk(
    model: &Model,
    has_adaptive: bool,
    adaptive_efforts: &[&'static str],
) -> Map<String, Value> {
    if has_adaptive {
        let mut efforts = adaptive_efforts.to_vec();
        if model.provider_id.as_str() == "github-copilot" {
            if model.api.id.contains("opus-4.7") {
                efforts = vec!["medium"];
            }
            efforts.retain(|v| *v != "max" && *v != "xhigh");
        }
        let display_summarized =
            model.api.id.contains("opus-4-7") || model.api.id.contains("opus-4.7");
        return efforts
            .iter()
            .map(|e| {
                let mut thinking = json!({ "type": "adaptive" });
                if display_summarized {
                    thinking["display"] = json!("summarized");
                }
                let mut obj = serde_json::Map::new();
                obj.insert("thinking".into(), thinking);
                obj.insert("effort".into(), json!(e));
                ((*e).to_string(), Value::Object(obj))
            })
            .collect();
    }
    let mut m = Map::new();
    let half = (model.limit.output as i64) / 2 - 1;
    let high_budget = std::cmp::min(16_000, half);
    let max_budget = std::cmp::min(31_999, model.limit.output as i64 - 1);
    m.insert(
        "high".into(),
        json!({ "thinking": { "type": "enabled", "budgetTokens": high_budget } }),
    );
    m.insert(
        "max".into(),
        json!({ "thinking": { "type": "enabled", "budgetTokens": max_budget } }),
    );
    m
}

pub(super) fn amazon_bedrock(
    model: &Model,
    has_adaptive: bool,
    adaptive_efforts: &[&'static str],
) -> Map<String, Value> {
    if has_adaptive {
        let display_summarized =
            model.api.id.contains("opus-4-7") || model.api.id.contains("opus-4.7");
        return adaptive_efforts
            .iter()
            .map(|e| {
                let mut cfg = json!({
                    "type": "adaptive",
                    "maxReasoningEffort": e,
                });
                if display_summarized {
                    cfg["display"] = json!("summarized");
                }
                ((*e).to_string(), json!({ "reasoningConfig": cfg }))
            })
            .collect();
    }
    if model.api.id.contains("anthropic") {
        let mut m = Map::new();
        m.insert(
            "high".into(),
            json!({ "reasoningConfig": { "type": "enabled", "budgetTokens": 16000 } }),
        );
        m.insert(
            "max".into(),
            json!({ "reasoningConfig": { "type": "enabled", "budgetTokens": 31999 } }),
        );
        return m;
    }
    WIDELY_SUPPORTED_EFFORTS
        .iter()
        .map(|e| {
            (
                (*e).to_string(),
                json!({
                    "reasoningConfig": { "type": "enabled", "maxReasoningEffort": e }
                }),
            )
        })
        .collect()
}

pub(super) fn google_sdk(id: &str) -> Map<String, Value> {
    if id.contains("2.5") {
        return google_thinking_config();
    }
    let mut levels: Vec<&'static str> = vec!["low", "high"];
    if id.contains("3.1") {
        levels = vec!["low", "medium", "high"];
    }
    levels
        .iter()
        .map(|e| {
            (
                (*e).to_string(),
                json!({
                    "thinkingConfig": { "includeThoughts": true, "thinkingLevel": e }
                }),
            )
        })
        .collect()
}

pub(super) fn mistral(model: &Model) -> Map<String, Value> {
    if !model.capabilities.reasoning {
        return Map::new();
    }
    const MISTRAL_IDS: &[&str] = &[
        "mistral-small-2603",
        "mistral-small-latest",
        "mistral-medium-3.5",
        "mistral-medium-2604",
    ];
    let id = model.api.id.to_lowercase();
    if !MISTRAL_IDS.iter().any(|m| id.contains(m)) {
        return Map::new();
    }
    let mut m = Map::new();
    m.insert("high".into(), json!({ "reasoningEffort": "high" }));
    m
}

pub(super) fn groq() -> Map<String, Value> {
    let mut efforts: Vec<&'static str> = vec!["none"];
    efforts.extend_from_slice(WIDELY_SUPPORTED_EFFORTS);
    efforts
        .iter()
        .map(|e| ((*e).to_string(), json!({ "reasoningEffort": e })))
        .collect()
}

pub(super) fn sap_ai(
    model: &Model,
    id: &str,
    has_adaptive: bool,
    adaptive_efforts: &[&'static str],
) -> Map<String, Value> {
    if model.api.id.contains("anthropic") {
        if has_adaptive {
            return anthropic_simple_adaptive(adaptive_efforts);
        }
        return anthropic_static_budgets();
    }
    if model.api.id.contains("gemini") && id.contains("2.5") {
        return google_thinking_config();
    }
    if model.api.id.contains("gpt") || api_id_has_o_digit(model.api.id.as_str()) {
        return WIDELY_SUPPORTED_EFFORTS
            .iter()
            .map(|e| ((*e).to_string(), json!({ "reasoningEffort": e })))
            .collect();
    }
    Map::new()
}
