//! Transform parity tests against the TypeScript fixtures.
//!
//! Each test mirrors a `packages/jekko/test/provider/transform.NN.*.test.ts`
//! file and asserts that the Rust transform layer produces the same shape as
//! the TypeScript reference for the same input.

use std::collections::BTreeMap;

use jekko_core::provider::{
    InterleavedField, Model, ModelId, ModelStatus, ProviderApiInfo, ProviderCacheCost,
    ProviderCapabilities, ProviderCost, ProviderId, ProviderInterleaved, ProviderLimit,
    ProviderModalities,
};
use jekko_provider::transform::{
    max_output_tokens, message, options, provider_options, sampling_params, schema, small_options,
    variants, OptionsInput,
};
use serde_json::{json, Map, Value};

fn base_model() -> Model {
    Model {
        id: ModelId::new("anthropic/claude-3-5-sonnet"),
        provider_id: ProviderId::new("anthropic"),
        api: ProviderApiInfo {
            id: "claude-3-5-sonnet-20241022".into(),
            url: "https://api.anthropic.com".into(),
            npm: "@ai-sdk/anthropic".into(),
        },
        name: "Claude 3.5 Sonnet".into(),
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
            input: 0.003,
            output: 0.015,
            cache: ProviderCacheCost {
                read: 0.0003,
                write: 0.00375,
            },
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
        release_date: "2024-10-22".into(),
        variants: None,
    }
}

fn model_with(provider_id: &str, npm: &str, api_id: &str) -> Model {
    let mut m = base_model();
    m.provider_id = ProviderId::new(provider_id);
    m.api = ProviderApiInfo {
        id: api_id.into(),
        url: "https://api.example.com".into(),
        npm: npm.into(),
    };
    m
}

#[test]
fn t01_options_set_cache_key_true_sets_prompt_cache() {
    // transform.01 — setCacheKey true.
    let model = base_model();
    let mut prov_opts = Map::new();
    prov_opts.insert("setCacheKey".into(), Value::Bool(true));
    let out = options(OptionsInput {
        model: &model,
        session_id: "test-session-123",
        provider_options: Some(&prov_opts),
    });
    assert_eq!(out["promptCacheKey"], "test-session-123");
}

#[test]
fn t01b_options_set_cache_key_false_omits_prompt_cache_for_non_openai() {
    let model = base_model();
    let mut prov_opts = Map::new();
    prov_opts.insert("setCacheKey".into(), Value::Bool(false));
    let out = options(OptionsInput {
        model: &model,
        session_id: "test-session-123",
        provider_options: Some(&prov_opts),
    });
    assert!(out.get("promptCacheKey").is_none());
}

#[test]
fn t01c_options_openai_always_sets_prompt_cache() {
    let m = model_with("openai", "@ai-sdk/openai", "gpt-4");
    let out = options(OptionsInput {
        model: &m,
        session_id: "test-session-123",
        provider_options: None,
    });
    assert_eq!(out["promptCacheKey"], "test-session-123");
    assert_eq!(out["store"], false);
}

#[test]
fn t01d_options_azure_sets_store_and_prompt_cache() {
    let m = model_with("azure", "@ai-sdk/azure", "gpt-4");
    let out = options(OptionsInput {
        model: &m,
        session_id: "sess",
        provider_options: None,
    });
    assert_eq!(out["store"], false);
    assert_eq!(out["promptCacheKey"], "sess");
}

#[test]
fn t04_gpt5_sets_text_verbosity_low() {
    // transform.04 — gpt-5.* should get textVerbosity = low.
    let m = model_with("openai", "@ai-sdk/openai", "gpt-5.4");
    let out = options(OptionsInput {
        model: &m,
        session_id: "sess",
        provider_options: None,
    });
    assert_eq!(out["textVerbosity"], "low");
    assert_eq!(out["reasoningEffort"], "medium");
}

#[test]
fn t05_gateway_sets_caching_auto() {
    let m = model_with("vercel", "@ai-sdk/gateway", "anthropic/claude-sonnet-4");
    let out = options(OptionsInput {
        model: &m,
        session_id: "sess",
        provider_options: None,
    });
    let gw = out.get("gateway").and_then(Value::as_object).unwrap();
    assert_eq!(gw["caching"], "auto");
}

#[test]
fn t06_provider_options_wraps_under_key() {
    // OpenAI: result is { openai: {...} }.
    let m = model_with("openai", "@ai-sdk/openai", "gpt-4");
    let mut inner = Map::new();
    inner.insert("store".into(), Value::Bool(false));
    let out = provider_options(&m, inner);
    assert!(out.get("openai").is_some());
}

#[test]
fn t06b_azure_wraps_under_both_keys() {
    let m = model_with("azure", "@ai-sdk/azure", "gpt-4");
    let mut inner = Map::new();
    inner.insert("store".into(), Value::Bool(false));
    let out = provider_options(&m, inner);
    assert!(out.get("openai").is_some());
    assert!(out.get("azure").is_some());
}

#[test]
fn t07_gemini_adds_missing_array_items() {
    let mut m = base_model();
    m.provider_id = ProviderId::new("google");
    m.api.id = "gemini-3-pro".into();
    m.api.npm = "@ai-sdk/google".into();
    let s = json!({
        "type": "object",
        "properties": {
            "nodes": { "type": "array" },
            "edges": { "type": "array", "items": { "type": "string" } }
        }
    });
    let out = schema(&m, s);
    assert!(out["properties"]["nodes"]["items"].is_object());
    assert_eq!(out["properties"]["edges"]["items"]["type"], "string");
}

#[test]
fn t11_moonshot_collapses_array_items() {
    let mut m = base_model();
    m.provider_id = ProviderId::new("moonshotai");
    m.api.id = "kimi-something".into();
    let s = json!({
        "type": "object",
        "properties": {
            "x": { "type": "array", "items": [ { "type": "string" }, { "type": "number" } ] }
        }
    });
    let out = schema(&m, s);
    assert_eq!(out["properties"]["x"]["items"]["type"], "string");
}

#[test]
fn t13_surrogate_sanitization_passes_through() {
    // Rust strings are valid UTF-8 — lone surrogates can't exist at this layer.
    // We assert that valid characters (including emoji + replacement char) pass
    // through unchanged.
    let m = base_model();
    let msgs = vec![
        json!({"role": "system", "content": "system text \u{FFFD} \u{1F680}"}),
        json!({"role": "user", "content": "hi"}),
    ];
    let opts = Map::new();
    let out = message(msgs, &m, &opts);
    assert!(out[0]["content"].as_str().unwrap().contains("\u{1F680}"));
}

#[test]
fn t15_anthropic_filters_empty_content() {
    let m = base_model(); // @ai-sdk/anthropic
    let msgs = vec![
        json!({"role": "user", "content": ""}),
        json!({"role": "user", "content": "real"}),
    ];
    let opts = Map::new();
    let out = message(msgs, &m, &opts);
    // The empty user message should be removed.
    assert_eq!(out.len(), 1);
    assert_eq!(out[0]["content"], "real");
}

#[test]
fn t17_provider_options_key_remap_for_copilot() {
    let m = model_with(
        "github-copilot",
        "@ai-sdk/github-copilot",
        "claude-sonnet-4",
    );
    let msgs = vec![json!({
        "role": "user",
        "content": "hi",
        "providerOptions": { "github-copilot": { "x": true } }
    })];
    let opts = Map::new();
    let out = message(msgs, &m, &opts);
    let po = out[0]["providerOptions"].as_object().unwrap();
    assert!(po.contains_key("copilot"));
    assert!(!po.contains_key("github-copilot"));
}

#[test]
fn t20_cache_control_on_anthropic_sets_provider_options() {
    let m = base_model();
    let msgs = vec![
        json!({"role": "system", "content": "you are helpful"}),
        json!({"role": "user", "content": "hi"}),
    ];
    let opts = Map::new();
    let out = message(msgs, &m, &opts);
    assert_eq!(
        out[0]["providerOptions"]["anthropic"]["cacheControl"]["type"],
        "ephemeral"
    );
    assert_eq!(
        out[0]["providerOptions"]["bedrock"]["cachePoint"]["type"],
        "default"
    );
    assert_eq!(
        out[0]["providerOptions"]["openaiCompatible"]["cache_control"]["type"],
        "ephemeral"
    );
}

#[test]
fn t20b_cache_control_skipped_on_gateway() {
    let m = model_with("vercel", "@ai-sdk/gateway", "anthropic/claude-sonnet-4");
    let msgs = vec![
        json!({"role": "system", "content": "you are helpful"}),
        json!({"role": "user", "content": "hi"}),
    ];
    let opts = Map::new();
    let out = message(msgs, &m, &opts);
    assert!(out[0]
        .get("providerOptions")
        .map(|v| v.as_object().is_none_or(|m| m.is_empty()))
        .unwrap_or(true));
}

#[test]
fn t21_mistral_tool_call_id_scrubbing() {
    let m = model_with("mistral", "@ai-sdk/openai", "mistral");
    let msgs = vec![
        json!({
            "role": "assistant",
            "content": [
                { "type": "tool-call", "toolCallId": "abc-123", "toolName": "lookup", "input": { "q": "one" } },
                { "type": "tool-result", "toolCallId": "abc-123", "output": { "type": "text", "value": "done" } }
            ]
        }),
        json!({
            "role": "tool",
            "content": [{ "type": "tool-result", "toolCallId": "tool.id", "output": { "type": "text", "value": "done" } }]
        }),
    ];
    let opts = Map::new();
    let out = message(msgs, &m, &opts);
    assert_eq!(out[0]["content"][0]["toolCallId"], "abc123000");
    assert_eq!(out[0]["content"][1]["toolCallId"], "abc123000");
    assert_eq!(out[1]["content"][0]["toolCallId"], "toolid000");
}

#[test]
fn variants_kimi_returns_empty() {
    let m = model_with("kimi", "@ai-sdk/openai-compatible", "kimi-anything");
    let out = variants(&m);
    assert!(out.is_empty());
}

#[test]
fn variants_anthropic_static_budgets() {
    let mut m = base_model();
    m.capabilities.reasoning = true;
    let out = variants(&m);
    assert!(out.contains_key("high"));
    assert!(out.contains_key("max"));
}

#[test]
fn small_options_openai_gpt5_uses_low_effort() {
    let m = model_with("openai", "@ai-sdk/openai", "gpt-5.4");
    let out = small_options(&m);
    assert_eq!(out["store"], false);
    assert_eq!(out["reasoningEffort"], "low");
}

#[test]
fn small_options_google_gemini_3_uses_minimal() {
    let m = model_with("google", "@ai-sdk/google", "gemini-3-pro");
    let out = small_options(&m);
    assert_eq!(out["thinkingConfig"]["thinkingLevel"], "minimal");
}

#[test]
fn sampling_params_for_qwen() {
    let m = model_with("foo", "@ai-sdk/openai", "qwen-x");
    let mut q = m;
    q.id = ModelId::new("foo/qwen-something");
    let s = sampling_params(&q);
    assert_eq!(s.temperature, Some(0.55));
}

#[test]
fn max_output_tokens_caps_at_constant() {
    let mut m = base_model();
    m.limit.output = 64_000.0;
    assert_eq!(max_output_tokens(&m), 32_000);
}

#[test]
fn unsupported_modality_replaced_with_error_text() {
    let mut m = base_model();
    m.capabilities.input = ProviderModalities {
        text: true,
        audio: false,
        image: false,
        video: false,
        pdf: false,
    };
    let msgs = vec![json!({
        "role": "user",
        "content": [
            { "type": "image", "image": "data:image/png;base64,aaaa" }
        ]
    })];
    let opts = Map::new();
    let out = message(msgs, &m, &opts);
    let part = &out[0]["content"][0];
    assert_eq!(part["type"], "text");
    let t = part["text"].as_str().unwrap();
    assert!(t.contains("ERROR: Cannot read"));
    assert!(t.contains("image"));
}

#[test]
fn empty_image_becomes_error_text() {
    let m = base_model();
    let msgs = vec![json!({
        "role": "user",
        "content": [
            { "type": "image", "image": "data:image/png;base64," }
        ]
    })];
    let opts = Map::new();
    let out = message(msgs, &m, &opts);
    assert_eq!(out[0]["content"][0]["type"], "text");
    assert!(out[0]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("empty or corrupted"));
}

#[test]
fn interleaved_reasoning_field_writes_provider_options() {
    let mut m = base_model();
    m.provider_id = ProviderId::new("foo");
    m.api.npm = "@ai-sdk/openai-compatible".into();
    m.capabilities.interleaved = ProviderInterleaved::Field {
        field: InterleavedField::ReasoningContent,
    };
    let msgs = vec![json!({
        "role": "assistant",
        "content": [
            { "type": "text", "text": "hello" },
            { "type": "reasoning", "text": "thinking" }
        ]
    })];
    let opts = Map::new();
    let out = message(msgs, &m, &opts);
    // reasoning part should be removed, content collapsed to text only.
    let arr = out[0]["content"].as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["type"], "text");
    let po = &out[0]["providerOptions"]["openaiCompatible"];
    assert_eq!(po["reasoning_content"], "thinking");
}
