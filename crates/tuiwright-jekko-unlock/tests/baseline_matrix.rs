//! Phase 0 baseline capture matrix for the captured reference `jekko` binary.
//!
//! Drives the binary across 5 terminal sizes and captures PNG + plain-text
//! snapshots of each reachable screen so the Rust port has a deterministic
//! parity reference.
//!
//! Set `JEKKO_BIN` to the captured reference binary path. When unset the tests
//! skip silently.
//!
//! Captures land under `target/tuiwright-jekko/baseline/<screen>/<WxH>.png`.
//!
//! Shared matrix logic lives in `tests/common/mod.rs`; this file only supplies
//! the binary resolver, artifact-subdirectory naming, and one specialised
//! mock-server / ZYAL test driver each.

#![allow(dead_code)]

use anyhow::Result;
use serial_test::serial;

mod common;
mod test_helpers;
use common::{
    recipe_command_dialog, recipe_home, recipe_model_dialog, recipe_prompt_autocomplete,
    recipe_provider_dialog, recipe_session_empty, recipe_shell, recipe_theme_dialog,
    reference_binary, MatrixConfig,
};

const BASELINE: MatrixConfig = MatrixConfig {
    artifact_subdir: "baseline",
    trace_prefix: "baseline-",
    log_level: "WARN",
    error_prefix: "",
    resolve_binary: reference_binary,
};

/// Captures the home (default landing) screen at all 5 resolutions.
#[test]
#[serial]
fn baseline_home_matrix() -> Result<()> {
    BASELINE.run_screen("home", "baseline", "home", recipe_home)
}

/// Captures the command palette (Ctrl+P) at all 5 resolutions.
#[test]
#[serial]
fn baseline_command_dialog_matrix() -> Result<()> {
    BASELINE.run_screen(
        "command-dialog",
        "baseline",
        "cmddlg",
        recipe_command_dialog,
    )
}

/// Captures the model picker dialog at all 5 resolutions. Leader key is Ctrl+X
/// per default keybinds (`packages/jekko/src/config/keybinds.ts`); model_list
/// is `<leader>m`.
#[test]
#[serial]
fn baseline_model_dialog_matrix() -> Result<()> {
    BASELINE.run_screen("model-dialog", "baseline", "modeldlg", recipe_model_dialog)
}

/// Captures the provider dialog (Ctrl+A from inside the model dialog).
#[test]
#[serial]
fn baseline_provider_dialog_matrix() -> Result<()> {
    BASELINE.run_screen(
        "provider-dialog",
        "baseline",
        "providerdlg",
        recipe_provider_dialog,
    )
}

/// Captures the theme picker dialog (`<leader>t` → Ctrl+X then T).
#[test]
#[serial]
fn baseline_theme_dialog_matrix() -> Result<()> {
    BASELINE.run_screen("theme-dialog", "baseline", "themedlg", recipe_theme_dialog)
}

/// Captures session-empty: `<leader>n` creates a new session, screen shows the
/// session route with no messages.
#[test]
#[serial]
fn baseline_session_empty_matrix() -> Result<()> {
    BASELINE.run_screen(
        "session-empty",
        "baseline",
        "sessempty",
        recipe_session_empty,
    )
}

/// Captures shell route (`<leader>b` toggles sidebar; shell is a top-level
/// route reached via command palette → "Shell").
#[test]
#[serial]
fn baseline_shell_matrix() -> Result<()> {
    BASELINE.run_screen("shell", "baseline", "shell", recipe_shell)
}

/// Captures the prompt-autocomplete screen: type a slash to trigger
/// slash-command completion in the prompt.
#[test]
#[serial]
fn baseline_prompt_autocomplete_matrix() -> Result<()> {
    BASELINE.run_screen(
        "prompt-autocomplete",
        "baseline",
        "promptac",
        recipe_prompt_autocomplete,
    )
}

/// Captures the Jnoccio dashboard (Ctrl+J after Jnoccio model is configured).
/// Spins up a tiny mock `/health` server at `127.0.0.1:4317` so the binary
/// believes Jnoccio is reachable. Failures are advisory.
#[test]
#[serial]
fn baseline_jnoccio_panel_matrix() -> Result<()> {
    BASELINE.run_jnoccio_panel("jnoccio")
}

/// Captures the ZYAL paste indicator in the home prompt. Pastes a known ZYAL
/// runbook snippet and waits for the `✓ ZYAL` sigil. Failures are advisory.
#[test]
#[serial]
fn baseline_zyal_panel_matrix() -> Result<()> {
    let example_path =
        test_helpers::repo_root().join("docs/ZYAL/examples/13-advanced-research-loop.zyal");
    BASELINE.run_zyal_panel("zyal", &example_path)
}

/// Captures the early splash frame. Splash is brief so capture at the first
/// non-blank screen before the home sentinel arrives.
#[test]
#[serial]
fn baseline_splash_matrix() -> Result<()> {
    BASELINE.run_splash("splash")
}
