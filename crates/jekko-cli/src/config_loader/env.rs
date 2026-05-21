use jekko_core::config::ui::{AnimationLevel, UiConfig};

/// Apply supported environment-variable overrides on top of `cfg`.
pub(super) fn merge_env(mut cfg: UiConfig) -> UiConfig {
    if let Ok(value) = std::env::var("JEKKO_UI_THEME") {
        if !value.is_empty() {
            cfg.ui.theme = Some(value);
        }
    }
    if let Ok(value) = std::env::var("JEKKO_UI_ANIMATIONS") {
        if let Some(level) = parse_animation_level(&value) {
            cfg.ui.animations = Some(level);
        }
    }
    if env_flag("JEKKO_REDUCED_MOTION") {
        cfg.accessibility.reduced_motion = Some(true);
    }
    if env_flag("JEKKO_NO_ALT_SCREEN") {
        cfg.ui.alternate_screen = Some(false);
    }
    if env_flag("JEKKO_NO_MOUSE") {
        cfg.ui.mouse = Some(false);
    }
    if let Ok(value) = std::env::var("JEKKO_HISTORY_SIZE") {
        if let Ok(parsed) = value.parse::<usize>() {
            cfg.input.history_limit = Some(parsed);
        }
    }
    cfg
}

pub(super) fn parse_animation_level(value: &str) -> Option<AnimationLevel> {
    match value.trim().to_ascii_lowercase().as_str() {
        "off" => Some(AnimationLevel::Off),
        "subtle" => Some(AnimationLevel::Subtle),
        "full" => Some(AnimationLevel::Full),
        _ => None,
    }
}

pub(super) fn env_flag(key: &str) -> bool {
    matches!(
        std::env::var(key).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}
