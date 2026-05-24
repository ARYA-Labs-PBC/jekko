use std::collections::BTreeMap;

use super::*;
use crate::permission::{PermissionAction, PermissionInput};

#[test]
fn defaults_are_empty() {
    let cfg = Config::defaults();
    assert!(cfg.model.is_none());
    assert!(cfg.permission.is_none());
}

#[test]
fn merge_overrides_scalars() {
    let base = Config {
        model: Some("anthropic/claude-sonnet-4".to_string()),
        ..Config::default()
    };
    let overlay = Config {
        model: Some("openai/gpt-5".to_string()),
        ..Config::default()
    };
    let merged = base.merge(overlay);
    assert_eq!(merged.model.as_deref(), Some("openai/gpt-5"));
}

#[test]
fn merge_preserves_base_when_overlay_missing() {
    let base = Config {
        shell: Some("zsh".to_string()),
        ..Config::default()
    };
    let merged = base.clone().merge(Config::default());
    assert_eq!(merged.shell, base.shell);
}

#[test]
fn merge_unions_maps() {
    let mut base_map: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    base_map.insert("anthropic".to_string(), serde_json::json!({"a": 1}));
    let mut overlay_map: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    overlay_map.insert("openai".to_string(), serde_json::json!({"b": 2}));

    let base = Config {
        provider: Some(base_map),
        ..Config::default()
    };
    let overlay = Config {
        provider: Some(overlay_map),
        ..Config::default()
    };
    let merged = base.merge(overlay);
    let providers = merged.provider.unwrap();
    assert!(providers.contains_key("anthropic"));
    assert!(providers.contains_key("openai"));
}

#[test]
fn permission_shorthand_round_trips() {
    let json = serde_json::json!({"permission": "allow"});
    let cfg: Config = serde_json::from_value(json).unwrap();
    assert_eq!(
        cfg.permission,
        Some(PermissionInput::Action(PermissionAction::Allow))
    );
}
