//! Helpers that build per-effort variant entries (OpenAI, Anthropic, Google).

use serde_json::{json, Map, Value};

use crate::transform::shared::{
    is_gpt5_family, OPENAI_NONE_EFFORT_RELEASE_DATE, OPENAI_XHIGH_EFFORT_RELEASE_DATE,
    WIDELY_SUPPORTED_EFFORTS,
};

pub(super) fn openai_reasoning_efforts(api_id: &str, release_date: &str) -> Vec<&'static str> {
    let id = api_id.to_lowercase();
    if id == "gpt-5-pro" || id == "openai/gpt-5-pro" {
        return Vec::new();
    }
    if id.contains("codex") {
        if id.contains("5.2") || id.contains("5.3") {
            let mut v: Vec<&'static str> = WIDELY_SUPPORTED_EFFORTS.to_vec();
            v.push("xhigh");
            return v;
        }
        return WIDELY_SUPPORTED_EFFORTS.to_vec();
    }
    let mut efforts: Vec<&'static str> = WIDELY_SUPPORTED_EFFORTS.to_vec();
    if is_gpt5_family(&id) {
        efforts.insert(0, "minimal");
    }
    if release_date >= OPENAI_NONE_EFFORT_RELEASE_DATE {
        efforts.insert(0, "none");
    }
    if release_date >= OPENAI_XHIGH_EFFORT_RELEASE_DATE {
        efforts.push("xhigh");
    }
    efforts
}

pub(super) fn anthropic_adaptive_efforts(api_id: &str) -> Vec<&'static str> {
    if ["opus-4-7", "opus-4.7"].iter().any(|v| api_id.contains(v)) {
        return vec!["low", "medium", "high", "xhigh", "max"];
    }
    if ["opus-4-6", "opus-4.6", "sonnet-4-6", "sonnet-4.6"]
        .iter()
        .any(|v| api_id.contains(v))
    {
        return vec!["low", "medium", "high", "max"];
    }
    Vec::new()
}

pub(super) fn oai_encrypted_efforts(efforts: &[&str]) -> Map<String, Value> {
    efforts
        .iter()
        .map(|effort| {
            (
                (*effort).to_string(),
                json!({
                    "reasoningEffort": effort,
                    "reasoningSummary": "auto",
                    "include": ["reasoning.encrypted_content"],
                }),
            )
        })
        .collect()
}

pub(super) fn google_thinking_config() -> Map<String, Value> {
    let mut m = Map::new();
    m.insert(
        "high".into(),
        json!({ "thinkingConfig": { "includeThoughts": true, "thinkingBudget": 16000 } }),
    );
    m.insert(
        "max".into(),
        json!({ "thinkingConfig": { "includeThoughts": true, "thinkingBudget": 24576 } }),
    );
    m
}

pub(super) fn anthropic_simple_adaptive(efforts: &[&str]) -> Map<String, Value> {
    efforts
        .iter()
        .map(|e| {
            (
                (*e).to_string(),
                json!({ "thinking": { "type": "adaptive" }, "effort": e }),
            )
        })
        .collect()
}

pub(super) fn anthropic_static_budgets() -> Map<String, Value> {
    let mut m = Map::new();
    m.insert(
        "high".into(),
        json!({ "thinking": { "type": "enabled", "budgetTokens": 16000 } }),
    );
    m.insert(
        "max".into(),
        json!({ "thinking": { "type": "enabled", "budgetTokens": 31999 } }),
    );
    m
}

pub(super) fn api_id_has_o_digit(api_id: &str) -> bool {
    // matches `/\bo[1-9]/` in TS, i.e. an `o` preceded by word boundary
    // (start, non-word) followed by digits 1-9.
    let chars: Vec<char> = api_id.chars().collect();
    for i in 0..chars.len() {
        if chars[i] == 'o' || chars[i] == 'O' {
            let before_ok =
                i == 0 || !(chars[i - 1].is_ascii_alphanumeric() || chars[i - 1] == '_');
            if before_ok {
                if let Some(&n) = chars.get(i + 1) {
                    if n.is_ascii_digit() && n != '0' {
                        return true;
                    }
                }
            }
        }
    }
    false
}
