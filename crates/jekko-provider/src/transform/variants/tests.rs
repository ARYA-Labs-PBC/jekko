use super::*;
use jekko_core::provider::{
    ModelId, ModelStatus, ProviderApiInfo, ProviderCacheCost, ProviderCapabilities, ProviderCost,
    ProviderId, ProviderInterleaved, ProviderLimit, ProviderModalities,
};
use std::collections::BTreeMap;

fn mock_model(id: &str, provider_id: &str, npm: &str, api_id: &str) -> Model {
    Model {
        id: ModelId::new(id),
        provider_id: ProviderId::new(provider_id),
        api: ProviderApiInfo {
            id: api_id.to_string(),
            url: "https://api.test.com".into(),
            npm: npm.to_string(),
        },
        name: "Test".into(),
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
            output: 64_000.0,
        },
        status: ModelStatus::Active,
        options: BTreeMap::new(),
        headers: BTreeMap::new(),
        release_date: "2024-01-01".into(),
        variants: None,
    }
}

#[test]
fn no_reasoning_no_variants() {
    let mut m = mock_model("test/test-model", "test", "@ai-sdk/openai", "test-model");
    m.capabilities.reasoning = false;
    let v = variants(&m);
    assert!(v.is_empty());
}

#[test]
fn deepseek_no_variants() {
    let m = mock_model(
        "deepseek/deepseek-chat",
        "deepseek",
        "@ai-sdk/openai-compatible",
        "deepseek-chat",
    );
    let v = variants(&m);
    assert!(v.is_empty());
}

#[test]
fn qwen_sampling() {
    let m = mock_model("foo/qwen-2", "foo", "@ai-sdk/openai", "qwen-2");
    let s = sampling_params(&m);
    assert_eq!(s.temperature, Some(0.55));
    assert_eq!(s.top_p, Some(1.0));
    assert!(s.top_k.is_none());
}

#[test]
fn gemini_sampling() {
    let m = mock_model("foo/gemini", "foo", "@ai-sdk/google", "gemini");
    let s = sampling_params(&m);
    assert_eq!(s.temperature, Some(1.0));
    assert_eq!(s.top_p, Some(0.95));
    assert_eq!(s.top_k, Some(64));
}
