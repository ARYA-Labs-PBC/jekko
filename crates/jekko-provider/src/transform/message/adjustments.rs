//! Provider-specific message-level adjustments.
//!
//! Splits the bulky `normalize_messages` chain into one helper per provider
//! quirk: empty-content filtering, claude/mistral id scrubbing, anthropic
//! tool-call splitting, deepseek reasoning suffix injection, and
//! OpenAI-compatible interleaved reasoning relocation.

use jekko_core::provider::Model;
use serde_json::{json, Value};

use super::sanitize::scrub_tool_call_ids;
use super::ModelMessage;

pub(super) fn filter_empty_content_if_needed(
    msgs: Vec<ModelMessage>,
    model: &Model,
) -> Vec<ModelMessage> {
    let npm = model.api.npm.as_str();
    if npm != "@ai-sdk/anthropic" && npm != "@ai-sdk/amazon-bedrock" {
        return msgs;
    }
    msgs.into_iter()
        .filter_map(|mut msg| {
            match msg.get("content") {
                Some(Value::String(s)) if s.is_empty() => return None,
                Some(Value::Array(_)) => {
                    if let Some(arr) = msg.get_mut("content").and_then(Value::as_array_mut) {
                        arr.retain(|p| {
                            let ty = p.get("type").and_then(Value::as_str).unwrap_or("");
                            if ty == "text" || ty == "reasoning" {
                                p.get("text").and_then(Value::as_str) != Some("")
                            } else {
                                true
                            }
                        });
                        if arr.is_empty() {
                            return None;
                        }
                    }
                }
                _ => {}
            }
            Some(msg)
        })
        .collect()
}

pub(super) fn scrub_claude_tool_call_ids_if_needed(
    msgs: Vec<ModelMessage>,
    model: &Model,
) -> Vec<ModelMessage> {
    if !model.id.as_str().contains("claude") {
        return msgs;
    }
    msgs.into_iter()
        .map(|mut msg| {
            scrub_tool_call_ids(&mut msg, |id| {
                id.chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect()
            });
            msg
        })
        .collect()
}

pub(super) fn split_anthropic_tool_call_messages_if_needed(
    msgs: Vec<ModelMessage>,
    model: &Model,
) -> Vec<ModelMessage> {
    let npm = model.api.npm.as_str();
    if npm != "@ai-sdk/anthropic" && npm != "@ai-sdk/google-vertex/anthropic" {
        return msgs;
    }
    let mut result = Vec::with_capacity(msgs.len());
    for msg in msgs {
        let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
        if role != "assistant" {
            result.push(msg);
            continue;
        }
        let Some(parts) = msg.get("content").and_then(Value::as_array) else {
            result.push(msg);
            continue;
        };
        let parts: Vec<Value> = parts.clone();
        let first = parts
            .iter()
            .position(|p| p.get("type").and_then(Value::as_str) == Some("tool-call"));
        let Some(first) = first else {
            result.push(msg);
            continue;
        };
        let needs_split = parts[first..]
            .iter()
            .any(|p| p.get("type").and_then(Value::as_str) != Some("tool-call"));
        if !needs_split {
            result.push(msg);
            continue;
        }
        let non_tool: Vec<Value> = parts
            .iter()
            .filter(|p| p.get("type").and_then(Value::as_str) != Some("tool-call"))
            .cloned()
            .collect();
        let tool: Vec<Value> = parts
            .iter()
            .filter(|p| p.get("type").and_then(Value::as_str) == Some("tool-call"))
            .cloned()
            .collect();
        let mut a = msg.clone();
        a["content"] = Value::Array(non_tool);
        let mut b = msg;
        b["content"] = Value::Array(tool);
        result.push(a);
        result.push(b);
    }
    result
}

pub(super) fn apply_mistral_adjustments_if_needed(
    msgs: Vec<ModelMessage>,
    model: &Model,
) -> Vec<ModelMessage> {
    let provider_id = model.provider_id.as_str();
    let api_id_lower = model.api.id.to_lowercase();
    if provider_id != "mistral"
        && !api_id_lower.contains("mistral")
        && !api_id_lower.contains("devstral")
    {
        return msgs;
    }
    let scrub = |id: &str| -> String {
        let mut filtered: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        filtered.truncate(9);
        while filtered.len() < 9 {
            filtered.push('0');
        }
        filtered
    };

    let mut result: Vec<ModelMessage> = Vec::new();
    for mut msg in msgs.into_iter() {
        scrub_tool_call_ids(&mut msg, scrub);
        result.push(msg);
    }
    // Second pass: inject synthetic assistant after any tool followed by user.
    let mut second = Vec::with_capacity(result.len());
    for (i, msg) in result.iter().enumerate() {
        second.push(msg.clone());
        let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
        if role == "tool" {
            if let Some(next) = result.get(i + 1) {
                if next.get("role").and_then(Value::as_str) == Some("user") {
                    second.push(json!({
                        "role": "assistant",
                        "content": [{ "type": "text", "text": "Done." }],
                    }));
                }
            }
        }
    }
    second
}

pub(super) fn apply_deepseek_adjustments_if_needed(
    msgs: Vec<ModelMessage>,
    model: &Model,
) -> Vec<ModelMessage> {
    if !model.api.id.to_lowercase().contains("deepseek") {
        return msgs;
    }
    msgs.into_iter()
        .map(|mut msg| {
            let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
            if role != "assistant" {
                return msg;
            }
            match msg.get("content").cloned() {
                Some(Value::Array(mut arr)) => {
                    if arr
                        .iter()
                        .any(|p| p.get("type").and_then(Value::as_str) == Some("reasoning"))
                    {
                        return msg;
                    }
                    arr.push(json!({ "type": "reasoning", "text": "" }));
                    msg["content"] = Value::Array(arr);
                    msg
                }
                Some(Value::String(s)) => {
                    let mut arr = Vec::new();
                    if !s.is_empty() {
                        arr.push(json!({ "type": "text", "text": s }));
                    }
                    arr.push(json!({ "type": "reasoning", "text": "" }));
                    msg["content"] = Value::Array(arr);
                    msg
                }
                _ => {
                    msg["content"] = Value::Array(vec![json!({ "type": "reasoning", "text": "" })]);
                    msg
                }
            }
        })
        .collect()
}

pub(super) fn apply_interleaved_reasoning_if_needed(
    msgs: Vec<ModelMessage>,
    model: &Model,
) -> Vec<ModelMessage> {
    use jekko_core::provider::{InterleavedField, ProviderInterleaved};
    let interleaved = &model.capabilities.interleaved;
    let field = match interleaved {
        ProviderInterleaved::Field { field } => *field,
        _ => return msgs,
    };
    if model.api.npm == "@openrouter/ai-sdk-provider" {
        return msgs;
    }
    let field_name = match field {
        InterleavedField::ReasoningContent => "reasoning_content",
        InterleavedField::ReasoningDetails => "reasoning_details",
    };

    msgs.into_iter()
        .map(|mut msg| {
            let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
            if role != "assistant" {
                return msg;
            }
            let Some(arr) = msg.get("content").and_then(Value::as_array).cloned() else {
                return msg;
            };
            let mut reasoning_text = String::new();
            let mut filtered = Vec::new();
            for part in arr {
                if part.get("type").and_then(Value::as_str) == Some("reasoning") {
                    if let Some(t) = part.get("text").and_then(Value::as_str) {
                        reasoning_text.push_str(t);
                    }
                } else {
                    filtered.push(part);
                }
            }
            msg["content"] = Value::Array(filtered);

            // Merge provider options.
            let opts_entry = msg
                .as_object_mut()
                .map(|m| m.entry("providerOptions").or_insert(json!({})));
            if let Some(opts) = opts_entry {
                let oc = opts
                    .as_object_mut()
                    .map(|m| m.entry("openaiCompatible").or_insert(json!({})));
                if let Some(oc) = oc {
                    if let Some(o) = oc.as_object_mut() {
                        o.insert(field_name.to_string(), Value::String(reasoning_text));
                    }
                }
            }
            msg
        })
        .collect()
}
