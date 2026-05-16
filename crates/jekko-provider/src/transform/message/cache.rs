//! Cache-control markers and per-provider option remapping.
//!
//! Ported from `transform-message-cache.ts` and the cache branch of
//! `transform-message.ts`.

use jekko_core::provider::Model;
use serde_json::{json, Value};

use super::ModelMessage;

/// Apply per-provider cache-control markers to system+final messages.
///
/// Mirrors `applyCaching` in `transform-message-cache.ts`.
pub(super) fn apply_caching(msgs: Vec<ModelMessage>, model: &Model) -> Vec<ModelMessage> {
    let mut msgs = msgs;
    // System messages: first 2.
    let mut system_idx: Vec<usize> = msgs
        .iter()
        .enumerate()
        .filter_map(|(i, m)| (m.get("role").and_then(Value::as_str) == Some("system")).then_some(i))
        .take(2)
        .collect();
    // Final non-system: last 2.
    let mut final_idx: Vec<usize> = msgs
        .iter()
        .enumerate()
        .rev()
        .filter_map(|(i, m)| (m.get("role").and_then(Value::as_str) != Some("system")).then_some(i))
        .take(2)
        .collect();
    final_idx.reverse();

    // Deduplicate while preserving order (system first, then final).
    system_idx.append(&mut final_idx);
    let mut targets = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for i in system_idx {
        if seen.insert(i) {
            targets.push(i);
        }
    }

    let use_message_level_options = model.provider_id.as_str() == "anthropic"
        || model.provider_id.as_str().contains("bedrock")
        || model.api.npm == "@ai-sdk/amazon-bedrock";

    for idx in targets {
        let Some(msg) = msgs.get_mut(idx) else {
            continue;
        };
        let should_use_content =
            !use_message_level_options && msg.get("content").is_some_and(|c| c.is_array());

        if should_use_content {
            let content_arr = msg.get_mut("content").and_then(Value::as_array_mut);
            if let Some(arr) = content_arr {
                if let Some(last) = arr.last_mut() {
                    let part_type = last.get("type").and_then(Value::as_str).unwrap_or("");
                    if part_type != "tool-approval-request" && part_type != "tool-approval-response"
                    {
                        merge_provider_cache_options(last);
                        continue;
                    }
                }
            }
        }
        merge_provider_cache_options(msg);
    }

    msgs
}

fn provider_cache_options_template() -> Value {
    json!({
      "anthropic": { "cacheControl": { "type": "ephemeral" } },
      "openrouter": { "cacheControl": { "type": "ephemeral" } },
      "bedrock": { "cachePoint": { "type": "default" } },
      "openaiCompatible": { "cache_control": { "type": "ephemeral" } },
      "copilot": { "copilot_cache_control": { "type": "ephemeral" } },
      "alibaba": { "cacheControl": { "type": "ephemeral" } }
    })
}

fn merge_provider_cache_options(node: &mut Value) {
    let template = provider_cache_options_template();
    let opts = node
        .as_object_mut()
        .map(|m| m.entry("providerOptions").or_insert(json!({})));
    if let Some(opts) = opts {
        merge_deep(opts, &template);
    }
}

/// Recursive deep merge mirroring `remeda.mergeDeep`.
fn merge_deep(target: &mut Value, source: &Value) {
    match (target, source) {
        (Value::Object(t), Value::Object(s)) => {
            for (k, v) in s {
                let entry = t.entry(k.clone()).or_insert(Value::Null);
                merge_deep(entry, v);
            }
        }
        (t, s) => {
            *t = s.clone();
        }
    }
}

pub(super) fn remap_provider_options(
    msgs: Vec<ModelMessage>,
    from: &str,
    to: &str,
) -> Vec<ModelMessage> {
    msgs.into_iter()
        .map(|mut msg| {
            // Top-level message providerOptions.
            if let Some(opts) = msg
                .get_mut("providerOptions")
                .and_then(Value::as_object_mut)
            {
                if let Some(taken) = opts.remove(from) {
                    opts.insert(to.to_string(), taken);
                }
            }
            // Per-content-part providerOptions when content is an array.
            if let Some(arr) = msg.get_mut("content").and_then(Value::as_array_mut) {
                for part in arr.iter_mut() {
                    if let Some(part_type) = part.get("type").and_then(Value::as_str) {
                        if part_type == "tool-approval-request"
                            || part_type == "tool-approval-response"
                        {
                            continue;
                        }
                    }
                    if let Some(opts) = part
                        .get_mut("providerOptions")
                        .and_then(Value::as_object_mut)
                    {
                        if let Some(taken) = opts.remove(from) {
                            opts.insert(to.to_string(), taken);
                        }
                    }
                }
            }
            msg
        })
        .collect()
}
