use super::*;

#[test]
fn defaults_match_tip8_contract() {
    let cfg = UiConfig::defaults();
    assert_eq!(cfg.ui.theme.as_deref(), Some("codex-dark"));
    assert_eq!(cfg.ui.animations, Some(AnimationLevel::Full));
    assert_eq!(cfg.ui.active_fps, Some(30));
    assert_eq!(cfg.input.history_limit, Some(500));
    assert_eq!(cfg.execution.chunk_max_bytes, Some(8192));
    assert_eq!(cfg.status.show_branch, Some(true));
    assert_eq!(cfg.accessibility.reduced_motion, Some(false));
}

#[test]
fn empty_deserialize_is_overlay_not_defaults() {
    let cfg: UiConfig = serde_json::from_value(serde_json::json!({})).unwrap();
    assert_eq!(cfg, UiConfig::default());
    assert!(cfg.ui.theme.is_none());
}

#[test]
fn deserialize_tip8_shape_from_raw_values() {
    let cfg: UiConfig = serde_json::from_value(serde_json::json!({
        "ui": {
            "theme": "codex-light",
            "animations": "subtle",
            "active_fps": 24,
            "soft_wrap": false
        },
        "input": {
            "history_limit": 42
        },
        "execution": {
            "prefer_pty": false,
            "chunk_max_bytes": 4096
        },
        "status": {
            "show_model": false
        },
        "accessibility": {
            "reduced_motion": true
        }
    }))
    .unwrap();

    assert_eq!(cfg.ui.theme.as_deref(), Some("codex-light"));
    assert_eq!(cfg.ui.animations, Some(AnimationLevel::Subtle));
    assert_eq!(cfg.ui.soft_wrap, Some(false));
    assert_eq!(cfg.input.history_limit, Some(42));
    assert_eq!(cfg.execution.prefer_pty, Some(false));
    assert_eq!(cfg.status.show_model, Some(false));
    assert_eq!(cfg.accessibility.reduced_motion, Some(true));
}

#[test]
fn merge_prefers_overlay_values_and_preserves_missing_defaults() {
    let base = UiConfig::defaults();
    let overlay = UiConfig {
        ui: UiSection {
            active_fps: Some(12),
            show_scrollbar: Some(true),
            ..UiSection::default()
        },
        accessibility: AccessibilitySection {
            reduced_motion: Some(true),
            ..AccessibilitySection::default()
        },
        ..UiConfig::default()
    };

    let merged = base.merge(overlay);
    assert_eq!(merged.ui.theme.as_deref(), Some("codex-dark"));
    assert_eq!(merged.ui.active_fps, Some(12));
    assert_eq!(merged.ui.show_scrollbar, Some(true));
    assert_eq!(merged.accessibility.reduced_motion, Some(true));
    assert_eq!(merged.execution.prefer_pty, Some(true));
}
