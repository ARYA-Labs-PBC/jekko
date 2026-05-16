//! Config merge tests.
use std::collections::BTreeMap;

use jekko_core::config::{Config, LogLevel, SharePolicy};
use jekko_core::permission::{PermissionAction, PermissionInput};

#[test]
fn merge_prefers_overlay_over_base() {
    let base = Config {
        model: Some("anthropic/claude-sonnet-4".to_string()),
        log_level: Some(LogLevel::Info),
        ..Config::default()
    };
    let overlay = Config {
        model: Some("openai/gpt-5".to_string()),
        log_level: Some(LogLevel::Debug),
        share: Some(SharePolicy::Manual),
        ..Config::default()
    };
    let merged = base.merge(overlay);
    assert_eq!(merged.model.as_deref(), Some("openai/gpt-5"));
    assert_eq!(merged.log_level, Some(LogLevel::Debug));
    assert_eq!(merged.share, Some(SharePolicy::Manual));
}

#[test]
fn merge_preserves_base_when_overlay_missing() {
    let base = Config {
        shell: Some("bash".to_string()),
        ..Config::default()
    };
    let merged = base.clone().merge(Config::default());
    assert_eq!(merged.shell, base.shell);
}

#[test]
fn merge_unions_provider_maps() {
    let mut base_providers = BTreeMap::<String, serde_json::Value>::new();
    base_providers.insert("anthropic".to_string(), serde_json::json!({"a": 1}));

    let mut overlay_providers = BTreeMap::<String, serde_json::Value>::new();
    overlay_providers.insert("openai".to_string(), serde_json::json!({"b": 2}));

    let base = Config {
        provider: Some(base_providers),
        ..Config::default()
    };
    let overlay = Config {
        provider: Some(overlay_providers),
        ..Config::default()
    };
    let merged = base.merge(overlay);
    let providers = merged.provider.unwrap();
    assert!(providers.contains_key("anthropic"));
    assert!(providers.contains_key("openai"));
}

#[test]
fn merge_overlay_overrides_same_key() {
    let mut base_providers = BTreeMap::<String, serde_json::Value>::new();
    base_providers.insert("anthropic".to_string(), serde_json::json!({"a": 1}));

    let mut overlay_providers = BTreeMap::<String, serde_json::Value>::new();
    overlay_providers.insert("anthropic".to_string(), serde_json::json!({"a": 2}));

    let merged = Config {
        provider: Some(base_providers),
        ..Config::default()
    }
    .merge(Config {
        provider: Some(overlay_providers),
        ..Config::default()
    });
    let providers = merged.provider.unwrap();
    assert_eq!(providers["anthropic"], serde_json::json!({"a": 2}));
}

#[test]
fn parses_permission_shorthand() {
    let raw = r#"{ "permission": "deny" }"#;
    let cfg: Config = serde_json::from_str(raw).unwrap();
    assert_eq!(
        cfg.permission,
        Some(PermissionInput::Action(PermissionAction::Deny))
    );
}
