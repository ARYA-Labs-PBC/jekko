//! JSON schema transform for tool-input schemas.
//!
//! Ported from `packages/jekko/src/provider/transform-schema.ts`.
use jekko_core::provider::Model;
use serde_json::Value;

/// Apply per-provider sanitisation to a JSON schema before it is sent to the
/// provider as part of a tool definition.
///
/// Mirrors `schema(...)` in `transform-schema.ts`.
pub fn schema(model: &Model, mut schema: Value) -> Value {
    let provider_id = model.provider_id.as_str();
    let api_id_lower = model.api.id.to_lowercase();
    let api_id = model.api.id.as_str();

    if provider_id == "moonshotai" || api_id_lower.contains("kimi") {
        schema = sanitize_moonshot(schema);
    }
    if provider_id == "google" || api_id.contains("gemini") {
        schema = sanitize_gemini(schema);
    }
    schema
}

fn sanitize_moonshot(value: Value) -> Value {
    match value {
        Value::Array(arr) => Value::Array(arr.into_iter().map(sanitize_moonshot).collect()),
        Value::Object(mut obj) => {
            if let Some(r) = obj.get("$ref").cloned() {
                if r.is_string() {
                    let mut out = serde_json::Map::new();
                    out.insert("$ref".to_string(), r);
                    return Value::Object(out);
                }
            }
            let mut out = serde_json::Map::with_capacity(obj.len());
            for (k, v) in obj.iter() {
                out.insert(k.clone(), sanitize_moonshot(v.clone()));
            }
            // items: array -> first element only.
            if let Some(Value::Array(items)) = out.get_mut("items").map(|v| v.clone()) {
                let first = items
                    .into_iter()
                    .next()
                    .unwrap_or(Value::Object(serde_json::Map::new()));
                out.insert("items".into(), first);
            }
            // Use the original obj if we made no changes (preserves order in some edge cases).
            let _ = &mut obj;
            Value::Object(out)
        }
        v => v,
    }
}

fn is_plain_object(v: &Value) -> bool {
    v.is_object()
}

fn has_combiner(v: &Value) -> bool {
    if let Some(obj) = v.as_object() {
        obj.get("anyOf").is_some_and(Value::is_array)
            || obj.get("oneOf").is_some_and(Value::is_array)
            || obj.get("allOf").is_some_and(Value::is_array)
    } else {
        false
    }
}

fn has_schema_intent(v: &Value) -> bool {
    if !is_plain_object(v) {
        return false;
    }
    if has_combiner(v) {
        return true;
    }
    let keys = [
        "type",
        "properties",
        "items",
        "prefixItems",
        "enum",
        "const",
        "$ref",
        "additionalProperties",
        "patternProperties",
        "required",
        "not",
        "if",
        "then",
        "else",
    ];
    keys.iter().any(|k| v.get(*k).is_some())
}

fn sanitize_gemini(value: Value) -> Value {
    match value {
        Value::Array(arr) => Value::Array(arr.into_iter().map(sanitize_gemini).collect()),
        Value::Object(obj) => {
            let mut out = serde_json::Map::with_capacity(obj.len());
            for (k, v) in obj.into_iter() {
                if k == "enum" {
                    if let Value::Array(arr) = v {
                        let mapped: Vec<Value> = arr
                            .into_iter()
                            .map(|e| match e {
                                Value::String(s) => Value::String(s),
                                _ => Value::String(e.to_string().trim_matches('"').to_string()),
                            })
                            .collect();
                        out.insert("enum".to_string(), Value::Array(mapped));
                        if let Some(cur_type) = out.get("type").and_then(Value::as_str) {
                            if cur_type == "integer" || cur_type == "number" {
                                out.insert("type".to_string(), Value::String("string".into()));
                            }
                        }
                    } else {
                        out.insert(k, v);
                    }
                } else if v.is_object() || v.is_array() {
                    out.insert(k, sanitize_gemini(v));
                } else {
                    out.insert(k, v);
                }
            }

            // result.type === "object" && result.properties && Array.isArray(result.required)
            let typ: String = match out.get("type").and_then(Value::as_str) {
                Some(s) => s.to_string(),
                None => String::new(),
            };
            if typ == "object" {
                if let Some(Value::Object(props)) = out.get("properties").cloned() {
                    if let Some(Value::Array(req)) = out.get("required").cloned() {
                        let prop_keys: std::collections::BTreeSet<String> =
                            props.keys().cloned().collect();
                        let filtered: Vec<Value> = req
                            .into_iter()
                            .filter(|f| f.as_str().map(|s| prop_keys.contains(s)).unwrap_or(false))
                            .collect();
                        out.insert("required".to_string(), Value::Array(filtered));
                    }
                }
            }

            if typ == "array" && !has_combiner(&Value::Object(out.clone())) {
                if out.get("items").map(Value::is_null).unwrap_or(true) {
                    out.insert("items".to_string(), Value::Object(serde_json::Map::new()));
                }
                let items_clone = out.get("items").cloned();
                if let Some(Value::Object(items)) = items_clone {
                    if !has_schema_intent(&Value::Object(items.clone())) {
                        let mut updated = items;
                        updated.insert("type".to_string(), Value::String("string".into()));
                        out.insert("items".to_string(), Value::Object(updated));
                    }
                }
            }

            if !typ.is_empty() && typ != "object" && !has_combiner(&Value::Object(out.clone())) {
                out.remove("properties");
                out.remove("required");
            }

            Value::Object(out)
        }
        v => v,
    }
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

    fn gemini_model() -> Model {
        Model {
            id: ModelId::new("google/gemini-3-pro"),
            provider_id: ProviderId::new("google"),
            api: ProviderApiInfo {
                id: "gemini-3-pro".into(),
                url: "https://api.test.com".into(),
                npm: "@ai-sdk/google".into(),
            },
            name: "Gemini 3".into(),
            family: None,
            capabilities: ProviderCapabilities {
                temperature: true,
                reasoning: true,
                attachment: true,
                toolcall: true,
                input: ProviderModalities::default(),
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
            release_date: "2025-01-01".into(),
            variants: None,
        }
    }

    #[test]
    fn gemini_adds_missing_array_items() {
        let m = gemini_model();
        let s = json!({
            "type": "object",
            "properties": {
                "nodes": { "type": "array" },
                "edges": { "type": "array", "items": { "type": "string" } }
            }
        });
        let out = schema(&m, s);
        let nodes_items = out["properties"]["nodes"]["items"].clone();
        assert!(nodes_items.is_object());
        let edges_items_type = out["properties"]["edges"]["items"]["type"]
            .as_str()
            .unwrap();
        assert_eq!(edges_items_type, "string");
    }

    #[test]
    fn moonshot_collapses_array_items() {
        let mut m = gemini_model();
        m.provider_id = ProviderId::new("moonshotai");
        m.api.id = "kimi-something".into();
        let s = json!({
            "type": "object",
            "properties": {
                "x": { "type": "array", "items": [ { "type": "string" }, { "type": "number" } ] }
            }
        });
        let out = schema(&m, s);
        let items = &out["properties"]["x"]["items"];
        assert!(items.is_object());
        assert_eq!(items["type"], "string");
    }
}
