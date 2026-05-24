use std::env;
use std::sync::Mutex;

use ratatui::style::{Color, Modifier};

use super::*;

// Env mutation is process-global, and tests run in parallel by default.
static ENV_GUARD: Mutex<()> = Mutex::new(());

const ENV_VARS: &[&str] = &["NO_COLOR", "CLICOLOR", "CLICOLOR_FORCE", "FORCE_COLOR"];

struct EnvSnapshot {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvSnapshot {
    fn capture() -> Self {
        let saved = ENV_VARS
            .iter()
            .map(|name| (*name, env::var(name).ok()))
            .collect();
        for name in ENV_VARS {
            env::remove_var(name);
        }
        Self { saved }
    }
}

impl Drop for EnvSnapshot {
    fn drop(&mut self) {
        for (name, value) in &self.saved {
            match value {
                Some(v) => env::set_var(name, v),
                None => env::remove_var(name),
            }
        }
    }
}

#[test]
fn no_color_disables_color() {
    let _g = ENV_GUARD.lock().unwrap();
    let _env = EnvSnapshot::capture();
    env::set_var("NO_COLOR", "1");
    assert_eq!(compute_color_mode_for_tests(), ColorMode::Monochrome);
}

#[test]
fn force_color_overrides_no_color() {
    let _g = ENV_GUARD.lock().unwrap();
    let _env = EnvSnapshot::capture();
    env::set_var("NO_COLOR", "1");
    env::set_var("FORCE_COLOR", "1");
    assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
}

#[test]
fn clicolor_force_overrides_no_color() {
    let _g = ENV_GUARD.lock().unwrap();
    let _env = EnvSnapshot::capture();
    env::set_var("NO_COLOR", "1");
    env::set_var("CLICOLOR_FORCE", "1");
    assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
}

#[test]
fn empty_no_color_does_not_disable() {
    let _g = ENV_GUARD.lock().unwrap();
    let _env = EnvSnapshot::capture();
    env::set_var("NO_COLOR", "");
    assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
}

#[test]
fn default_is_full_with_no_env_signals() {
    let _g = ENV_GUARD.lock().unwrap();
    let _env = EnvSnapshot::capture();
    assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
}

#[test]
fn clicolor_zero_without_tty_keeps_full() {
    let _g = ENV_GUARD.lock().unwrap();
    let _env = EnvSnapshot::capture();
    env::set_var("CLICOLOR", "0");
    assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
}

#[test]
fn force_color_zero_is_treated_as_off() {
    let _g = ENV_GUARD.lock().unwrap();
    let _env = EnvSnapshot::capture();
    env::set_var("FORCE_COLOR", "0");
    assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
}

#[test]
fn force_color_false_is_treated_as_off() {
    let _g = ENV_GUARD.lock().unwrap();
    let _env = EnvSnapshot::capture();
    env::set_var("FORCE_COLOR", "false");
    assert_eq!(compute_color_mode_for_tests(), ColorMode::Full);
}

#[test]
fn no_color_takes_priority_over_clicolor_zero() {
    let _g = ENV_GUARD.lock().unwrap();
    let _env = EnvSnapshot::capture();
    env::set_var("NO_COLOR", "1");
    env::set_var("CLICOLOR", "0");
    assert_eq!(compute_color_mode_for_tests(), ColorMode::Monochrome);
}

#[test]
fn monochrome_palette_uses_reset_color() {
    let p = monochrome_palette();
    assert_eq!(p.text, Color::Reset);
    assert_eq!(p.text_muted, Color::Reset);
    assert_eq!(p.accent, Color::Reset);
    assert_eq!(p.border, Color::Reset);
}

#[test]
fn mono_modifiers_are_emphasis_only() {
    assert!(mono_strong().contains(Modifier::BOLD));
    assert!(mono_muted().contains(Modifier::ITALIC));
    assert!(mono_dim().contains(Modifier::DIM));
}

#[test]
fn codex_accessor_returns_const_in_full_mode() {
    let _g = ENV_GUARD.lock().unwrap();
    let _env = EnvSnapshot::capture();
    if color_mode() == ColorMode::Full {
        assert_eq!(codex_fg(), codex::FG);
        assert_eq!(codex_blue_path(), codex::BLUE_PATH);
    }
}
