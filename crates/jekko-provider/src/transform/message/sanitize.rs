//! Surrogate scrubbing and tool-call id sanitisation.
//!
//! Mirrors helpers in `transform-message-utils.ts` that walk message content
//! to clean up text bodies and tool-call identifiers.

use serde_json::Value;

use crate::transform::shared::sanitize_surrogates;

pub(super) fn sanitize_surrogates_msg(msg: &mut Value) {
    let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
    match role {
        "tool" => {
            if let Some(arr) = msg.get_mut("content").and_then(Value::as_array_mut) {
                for part in arr.iter_mut() {
                    sanitize_tool_result(part);
                }
            }
        }
        "system" => {
            if let Some(s) = msg.get("content").and_then(Value::as_str).map(String::from) {
                msg["content"] = Value::String(sanitize_surrogates(&s));
            }
        }
        "user" => match msg.get_mut("content") {
            Some(Value::String(s)) => *s = sanitize_surrogates(s),
            Some(Value::Array(arr)) => {
                for part in arr.iter_mut() {
                    if part.get("type").and_then(Value::as_str) == Some("text") {
                        if let Some(t) = part.get("text").and_then(Value::as_str).map(String::from)
                        {
                            part["text"] = Value::String(sanitize_surrogates(&t));
                        }
                    }
                }
            }
            _ => {}
        },
        "assistant" => match msg.get_mut("content") {
            Some(Value::String(s)) => *s = sanitize_surrogates(s),
            Some(Value::Array(arr)) => {
                for part in arr.iter_mut() {
                    let ty = part
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    if ty == "text" || ty == "reasoning" {
                        if let Some(t) = part.get("text").and_then(Value::as_str).map(String::from)
                        {
                            part["text"] = Value::String(sanitize_surrogates(&t));
                        }
                    }
                    if ty == "tool-result" {
                        sanitize_tool_result(part);
                    }
                }
            }
            _ => {}
        },
        _ => {}
    }
}

fn sanitize_tool_result(part: &mut Value) {
    let Some(out) = part.get_mut("output") else {
        return;
    };
    let ty = out.get("type").and_then(Value::as_str).unwrap_or("");
    match ty {
        "text" | "error-text" => {
            if let Some(s) = out.get("value").and_then(Value::as_str).map(String::from) {
                out["value"] = Value::String(sanitize_surrogates(&s));
            }
        }
        "content" => {
            if let Some(arr) = out.get_mut("value").and_then(Value::as_array_mut) {
                for item in arr.iter_mut() {
                    if item.get("type").and_then(Value::as_str) == Some("text") {
                        if let Some(t) = item.get("text").and_then(Value::as_str).map(String::from)
                        {
                            item["text"] = Value::String(sanitize_surrogates(&t));
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

pub(super) fn scrub_tool_call_ids<F: Fn(&str) -> String>(msg: &mut Value, scrub: F) {
    let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
    let include_tool_call = role == "assistant";
    let allow = role == "assistant" || role == "tool";
    if !allow {
        return;
    }
    if let Some(arr) = msg.get_mut("content").and_then(Value::as_array_mut) {
        for part in arr.iter_mut() {
            let ty = part
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let is_relevant = ty == "tool-result" || (include_tool_call && ty == "tool-call");
            if !is_relevant {
                continue;
            }
            if let Some(id) = part
                .get("toolCallId")
                .and_then(Value::as_str)
                .map(String::from)
            {
                part["toolCallId"] = Value::String(scrub(&id));
            }
        }
    }
}
