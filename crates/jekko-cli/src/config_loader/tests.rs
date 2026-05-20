use std::path::PathBuf;
use std::sync::Mutex;

use jekko_core::config::ui::{AnimationLevel, UiConfig};

use super::*;

/// Serializes env-touching tests because process env is global state.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// A full TOML example covering every section of the current
/// `jekko_core::config::ui::UiConfig` overlay. The values match the
/// runtime defaults exactly so `parse_str` should round-trip back to
/// `UiConfig::defaults()`.
const FULL_EXAMPLE_TOML: &str = r#"
[ui]
theme = "codex-dark"
alternate_screen = true
mouse = true
animations = "full"
active_fps = 30
idle_fps = 5
timer_tick_ms = 1000
timeline_overscan = 6
stick_to_bottom = true
compact_transcripts = true
max_compact_transcript_lines = 5
show_scrollbar = false
soft_wrap = true
diff_context_lines = 3

[input]
single_line_default = true
history_limit = 500
at_file_completion = true

[execution]
prefer_pty = true
chunk_latency_ms = 16
chunk_max_bytes = 8192
kill_grace_ms = 2000
inherit_env = true
force_color = true

[status]
show_model = true
show_pwd = true
show_branch = true
show_profile = true

[accessibility]
reduced_motion = false
respect_no_color = true
high_contrast = false
"#;

#[test]
fn full_example_round_trips_to_defaults() {
    let cfg = UiConfigLoader::parse_str(FULL_EXAMPLE_TOML).expect("parse ok");
    assert_eq!(cfg, UiConfig::defaults());
}

#[test]
fn parse_empty_string_yields_defaults() {
    let cfg = UiConfigLoader::parse_str("").expect("parse ok");
    assert_eq!(cfg, UiConfig::defaults());
}

#[test]
fn parse_partial_toml_fills_missing_from_defaults() {
    let cfg = UiConfigLoader::parse_str("[ui]\ntheme = \"light\"\n").expect("parse ok");
    assert_eq!(cfg.ui.theme.as_deref(), Some("light"));
    assert_eq!(cfg.ui.animations, Some(AnimationLevel::Full));
    assert_eq!(cfg.ui.active_fps, Some(30));
    assert_eq!(cfg.input.history_limit, Some(500));
    assert_eq!(cfg.execution.prefer_pty, Some(true));
    assert_eq!(cfg.status.show_branch, Some(true));
    assert_eq!(cfg.accessibility.reduced_motion, Some(false));
}

#[test]
fn parse_propagates_toml_errors() {
    let err = UiConfigLoader::parse_str("[ui]\ntheme = \"oops\n").unwrap_err();
    match err {
        LoadError::Parse(_) => {}
        other => panic!("expected Parse error, got {other:?}"),
    }
}

#[test]
fn invalid_animations_value_errors() {
    let err = UiConfigLoader::parse_str("[ui]\nanimations = \"wat\"\n").unwrap_err();
    match err {
        LoadError::Parse(_) => {}
        other => panic!("expected Parse error, got {other:?}"),
    }
}

#[test]
fn animation_levels_round_trip() {
    let cfg = UiConfigLoader::parse_str("[ui]\nanimations = \"off\"\n").expect("parse");
    assert_eq!(cfg.ui.animations, Some(AnimationLevel::Off));
    let cfg = UiConfigLoader::parse_str("[ui]\nanimations = \"subtle\"\n").expect("parse");
    assert_eq!(cfg.ui.animations, Some(AnimationLevel::Subtle));
}

#[test]
fn load_from_path_reads_file() {
    let dir = tempdir();
    let path = dir.join("ui.toml");
    std::fs::write(&path, "[ui]\ntheme = \"slate\"\n").unwrap();
    let cfg = UiConfigLoader::load_from_path(&path).expect("load ok");
    assert_eq!(cfg.ui.theme.as_deref(), Some("slate"));
}

#[test]
fn load_default_path_returns_defaults_when_missing() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempdir();
    std::env::set_var("XDG_CONFIG_HOME", &dir);
    std::env::set_var("HOME", &dir);
    let cfg = UiConfigLoader::load_default_path().expect("load ok");
    assert_eq!(cfg, UiConfig::defaults());
    std::env::remove_var("XDG_CONFIG_HOME");
    std::env::remove_var("HOME");
}

#[test]
fn env_override_jekko_ui_theme() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("JEKKO_UI_THEME", "amber");
    let cfg = UiConfigLoader::merge_env(UiConfig::defaults());
    assert_eq!(cfg.ui.theme.as_deref(), Some("amber"));
    std::env::remove_var("JEKKO_UI_THEME");
}

#[test]
fn env_override_jekko_ui_theme_empty_string_ignored() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("JEKKO_UI_THEME", "");
    let cfg = UiConfigLoader::merge_env(UiConfig::defaults());
    assert_eq!(cfg.ui.theme.as_deref(), Some("codex-dark"));
    std::env::remove_var("JEKKO_UI_THEME");
}

#[test]
fn env_override_jekko_ui_animations() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("JEKKO_UI_ANIMATIONS", "subtle");
    let cfg = UiConfigLoader::merge_env(UiConfig::defaults());
    assert_eq!(cfg.ui.animations, Some(AnimationLevel::Subtle));
    std::env::set_var("JEKKO_UI_ANIMATIONS", "garbage");
    let cfg = UiConfigLoader::merge_env(UiConfig::defaults());
    assert_eq!(cfg.ui.animations, Some(AnimationLevel::Full));
    std::env::remove_var("JEKKO_UI_ANIMATIONS");
}

#[test]
fn env_override_reduced_motion_targets_accessibility_section() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("JEKKO_REDUCED_MOTION", "1");
    let cfg = UiConfigLoader::merge_env(UiConfig::defaults());
    assert_eq!(cfg.accessibility.reduced_motion, Some(true));
    std::env::remove_var("JEKKO_REDUCED_MOTION");
}

#[test]
fn env_override_no_alt_screen_and_no_mouse() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("JEKKO_NO_ALT_SCREEN", "1");
    std::env::set_var("JEKKO_NO_MOUSE", "1");
    let cfg = UiConfigLoader::merge_env(UiConfig::defaults());
    assert_eq!(cfg.ui.alternate_screen, Some(false));
    assert_eq!(cfg.ui.mouse, Some(false));
    std::env::remove_var("JEKKO_NO_ALT_SCREEN");
    std::env::remove_var("JEKKO_NO_MOUSE");
}

#[test]
fn env_override_history_size_targets_history_limit_field() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("JEKKO_HISTORY_SIZE", "1234");
    let cfg = UiConfigLoader::merge_env(UiConfig::defaults());
    assert_eq!(cfg.input.history_limit, Some(1234));
    std::env::set_var("JEKKO_HISTORY_SIZE", "not-a-number");
    let cfg = UiConfigLoader::merge_env(UiConfig::defaults());
    assert_eq!(cfg.input.history_limit, Some(500));
    std::env::remove_var("JEKKO_HISTORY_SIZE");
}

#[test]
fn resolved_path_uses_xdg_config_home() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", "/tmp/foo");
    std::env::remove_var("HOME");
    let path = UiConfigLoader::resolved_path().expect("path");
    assert_eq!(path, PathBuf::from("/tmp/foo/jekko/ui.toml"));
    std::env::remove_var("XDG_CONFIG_HOME");
}

#[test]
fn resolved_path_falls_back_to_home() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var("XDG_CONFIG_HOME");
    std::env::set_var("HOME", "/tmp/baruser");
    let path = UiConfigLoader::resolved_path().expect("path");
    assert_eq!(path, PathBuf::from("/tmp/baruser/.config/jekko/ui.toml"));
    std::env::remove_var("HOME");
}

#[test]
fn resolved_path_none_when_no_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var("XDG_CONFIG_HOME");
    std::env::remove_var("HOME");
    assert!(UiConfigLoader::resolved_path().is_none());
}

#[test]
fn parse_unknown_keys_are_tolerated() {
    let cfg = UiConfigLoader::parse_str("[ui]\nfuture_knob = true\n").expect("parse ok");
    assert_eq!(cfg, UiConfig::defaults());
}

fn scrub_env() {
    for var in [
        "JEKKO_UI_THEME",
        "JEKKO_UI_ANIMATIONS",
        "JEKKO_REDUCED_MOTION",
        "JEKKO_NO_ALT_SCREEN",
        "JEKKO_NO_MOUSE",
        "JEKKO_HISTORY_SIZE",
    ] {
        std::env::remove_var(var);
    }
}

#[test]
fn resolve_with_provenance_marks_defaults_when_no_overlay() {
    let _guard = ENV_LOCK.lock().unwrap();
    scrub_env();
    let dir = tempdir();
    std::env::set_var("XDG_CONFIG_HOME", &dir);
    std::env::set_var("HOME", &dir);
    let resolved = UiConfigLoader::resolve_with_provenance(None).expect("resolve");
    assert_eq!(resolved.config, UiConfig::defaults());
    assert!(resolved.toml_path.is_none());
    for field in TRACKED_FIELDS {
        assert_eq!(
            resolved.source_for(field),
            Source::Default,
            "{field} should still be default"
        );
    }
    std::env::remove_var("XDG_CONFIG_HOME");
    std::env::remove_var("HOME");
}

#[test]
fn resolve_with_provenance_marks_toml_overlay_fields() {
    let _guard = ENV_LOCK.lock().unwrap();
    scrub_env();
    let dir = tempdir();
    let path = dir.join("ui.toml");
    std::fs::write(
        &path,
        "[ui]\ntheme = \"slate\"\nanimations = \"subtle\"\n\
         [input]\nhistory_limit = 999\n",
    )
    .unwrap();
    let resolved = UiConfigLoader::resolve_with_provenance(Some(&path)).expect("resolve");
    assert_eq!(resolved.config.ui.theme.as_deref(), Some("slate"));
    assert_eq!(resolved.toml_path.as_deref(), Some(path.as_path()));
    assert_eq!(
        resolved.source_for("ui.theme"),
        Source::TomlPath(path.clone())
    );
    assert_eq!(
        resolved.source_for("ui.animations"),
        Source::TomlPath(path.clone())
    );
    assert_eq!(
        resolved.source_for("input.history_limit"),
        Source::TomlPath(path.clone())
    );
    assert_eq!(resolved.source_for("ui.mouse"), Source::Default);
    assert_eq!(
        resolved.source_for("accessibility.reduced_motion"),
        Source::Default
    );
}

#[test]
fn resolve_with_provenance_records_env_overrides() {
    let _guard = ENV_LOCK.lock().unwrap();
    scrub_env();
    let dir = tempdir();
    std::env::set_var("XDG_CONFIG_HOME", &dir);
    std::env::set_var("HOME", &dir);
    std::env::set_var("JEKKO_REDUCED_MOTION", "1");
    std::env::set_var("JEKKO_UI_THEME", "amber");
    std::env::set_var("JEKKO_UI_ANIMATIONS", "off");
    let resolved = UiConfigLoader::resolve_with_provenance(None).expect("resolve");
    assert_eq!(
        resolved.source_for("accessibility.reduced_motion"),
        Source::Env("JEKKO_REDUCED_MOTION")
    );
    assert_eq!(
        resolved.source_for("ui.theme"),
        Source::Env("JEKKO_UI_THEME")
    );
    assert_eq!(
        resolved.source_for("ui.animations"),
        Source::Env("JEKKO_UI_ANIMATIONS")
    );
    assert_eq!(resolved.config.ui.theme.as_deref(), Some("amber"));
    assert_eq!(resolved.config.ui.animations, Some(AnimationLevel::Off));
    assert_eq!(resolved.config.accessibility.reduced_motion, Some(true));
    scrub_env();
    std::env::remove_var("XDG_CONFIG_HOME");
    std::env::remove_var("HOME");
}

#[test]
fn resolve_with_provenance_env_beats_toml_for_same_field() {
    let _guard = ENV_LOCK.lock().unwrap();
    scrub_env();
    let dir = tempdir();
    let path = dir.join("ui.toml");
    std::fs::write(&path, "[ui]\ntheme = \"slate\"\n").unwrap();
    std::env::set_var("JEKKO_UI_THEME", "amber");
    let resolved = UiConfigLoader::resolve_with_provenance(Some(&path)).expect("resolve");
    assert_eq!(resolved.config.ui.theme.as_deref(), Some("amber"));
    assert_eq!(
        resolved.source_for("ui.theme"),
        Source::Env("JEKKO_UI_THEME")
    );
    scrub_env();
}

#[test]
fn record_cli_override_overrides_existing_source() {
    let _guard = ENV_LOCK.lock().unwrap();
    scrub_env();
    let dir = tempdir();
    std::env::set_var("XDG_CONFIG_HOME", &dir);
    std::env::set_var("HOME", &dir);
    let mut resolved = UiConfigLoader::resolve_with_provenance(None).expect("resolve");
    resolved.record_cli_override("accessibility.reduced_motion", "--reduced-motion");
    resolved.config.accessibility.reduced_motion = Some(true);
    assert_eq!(
        resolved.source_for("accessibility.reduced_motion"),
        Source::Cli("--reduced-motion")
    );
    std::env::remove_var("XDG_CONFIG_HOME");
    std::env::remove_var("HOME");
}

#[test]
fn source_label_formats_each_variant() {
    assert_eq!(Source::Default.label(), "default");
    assert_eq!(
        Source::TomlPath(PathBuf::from("/tmp/x.toml")).label(),
        "toml(/tmp/x.toml)"
    );
    assert_eq!(Source::Env("JEKKO_UI_THEME").label(), "env(JEKKO_UI_THEME)");
    assert_eq!(
        Source::Cli("--reduced-motion").label(),
        "cli(--reduced-motion)"
    );
}

#[test]
fn tracked_fields_covers_every_section() {
    let names: Vec<&&str> = TRACKED_FIELDS.iter().collect();
    assert!(names.iter().any(|n| n.starts_with("ui.")));
    assert!(names.iter().any(|n| n.starts_with("input.")));
    assert!(names.iter().any(|n| n.starts_with("execution.")));
    assert!(names.iter().any(|n| n.starts_with("status.")));
    assert!(names.iter().any(|n| n.starts_with("accessibility.")));
}

fn tempdir() -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "jekko-cli-config-loader-test-{}-{}",
        std::process::id(),
        next_id()
    ));
    std::fs::create_dir_all(&base).expect("mkdir tempdir");
    base
}

fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}
