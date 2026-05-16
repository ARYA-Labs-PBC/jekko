//! Provider catalog deserialization tests.
use jekko_core::provider::{
    InterleavedField, ModelRef, ModelStatus, ProviderInterleaved, ProviderListResult,
    ProviderSource,
};

const FIXTURE: &str = include_str!("fixtures/provider_catalog.json");

#[test]
fn deserializes_full_catalog_fixture() {
    let result: ProviderListResult = serde_json::from_str(FIXTURE).expect("decode");
    assert_eq!(result.all.len(), 1);
    let anthropic = &result.all[0];
    assert_eq!(anthropic.id.as_str(), "anthropic");
    assert_eq!(anthropic.source, ProviderSource::Config);
    assert_eq!(anthropic.env, vec!["ANTHROPIC_API_KEY".to_string()]);
    let model = &anthropic.models["claude-sonnet-4"];
    assert_eq!(model.status, ModelStatus::Active);
    assert_eq!(model.family.as_deref(), Some("claude-sonnet"));
    assert_eq!(model.limit.context, 200000.0);
    assert_eq!(result.default["anthropic"], "claude-sonnet-4");
    assert_eq!(result.connected, vec!["anthropic".to_string()]);
}

#[test]
fn interleaved_bool_form_decodes() {
    let json = "{\"temperature\":true,\"reasoning\":true,\"attachment\":true,\"toolcall\":true,\"input\":{\"text\":true,\"audio\":false,\"image\":false,\"video\":false,\"pdf\":false},\"output\":{\"text\":true,\"audio\":false,\"image\":false,\"video\":false,\"pdf\":false},\"interleaved\":true}";
    let caps: jekko_core::provider::ProviderCapabilities = serde_json::from_str(json).unwrap();
    match caps.interleaved {
        ProviderInterleaved::Bool(true) => {}
        other => panic!("expected Bool(true), got {other:?}"),
    }
}

#[test]
fn interleaved_object_form_decodes() {
    let raw = r#"{"field": "reasoning_content"}"#;
    let parsed: ProviderInterleaved = serde_json::from_str(raw).unwrap();
    assert_eq!(
        parsed,
        ProviderInterleaved::Field {
            field: InterleavedField::ReasoningContent
        }
    );
}

#[test]
fn model_status_normalises_deprecated() {
    assert_eq!(
        ModelStatus::normalize(Some("deprecated")),
        ModelStatus::Inactive
    );
    assert_eq!(
        ModelStatus::normalize(Some("discouraged")),
        ModelStatus::Inactive
    );
}

#[test]
fn model_ref_parse_splits_on_first_slash() {
    let r = ModelRef::parse("openai/gpt-5/mini").unwrap();
    assert_eq!(r.provider_id.as_str(), "openai");
    assert_eq!(r.id.as_str(), "gpt-5/mini");
}
