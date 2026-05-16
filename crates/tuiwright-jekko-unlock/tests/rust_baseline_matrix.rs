//! Packet M-prep: Rust-render parity capture matrix.
//!
//! Mirrors `baseline_matrix.rs` but drives the Rust `jekko` binary instead of
//! the captured reference binary. Captures PNG + plain-text snapshots into
//! `target/tuiwright-jekko/rust/<screen>/<WxH>.{png,txt}` so the eventual
//! `xtask baseline-diff` command can diff Rust output against the captured
//! Phase 0 baselines.
//!
//! Set `JEKKO_BIN` to a Rust-built `jekko` binary. Every test in this file
//! returns `Ok(())` immediately when any of the guards trip:
//!
//! - `JEKKO_BIN` is unset.
//! - The binary at `JEKKO_BIN` is smaller than `STUB_BINARY_THRESHOLD` bytes
//!   (treated as a scaffold stub).
//! - `JEKKO_RUST_MATRIX` is not set to `1`. Deliberate opt-in so the matrix
//!   only engages once the Rust TUI is feature-complete enough to stand up
//!   the boot sentinels.
//!
//! Canonical invocation:
//!
//! ```text
//! JEKKO_BIN=$(cargo run -p xtask -- host-binary-path) \
//! JEKKO_RUST_MATRIX=1 \
//!   cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml \
//!   --test rust_baseline_matrix -- --test-threads=1 --nocapture
//!
//! cargo run -p xtask -- baseline-diff --threshold 5
//! ```
//!
//! Shared matrix logic lives in `tests/common/mod.rs`; this file only supplies
//! the binary resolver and artifact-subdirectory naming.

#![allow(dead_code)]

use anyhow::Result;
use serial_test::serial;

mod common;
mod test_helpers;
use common::{
    recipe_command_dialog, recipe_home, recipe_model_dialog, recipe_prompt_autocomplete,
    recipe_provider_dialog, recipe_session_empty, recipe_shell, recipe_theme_dialog, rust_binary,
    MatrixConfig,
};

const RUST: MatrixConfig = MatrixConfig {
    artifact_subdir: "rust",
    trace_prefix: "rust-",
    log_level: "WARN",
    error_prefix: "rust ",
    resolve_binary: rust_binary,
};

/// Captures the home (default landing) screen at all 5 resolutions.
#[test]
#[serial]
fn rust_home_matrix() -> Result<()> {
    RUST.run_screen("home", "rust matrix", "home", recipe_home)
}

/// Captures the command palette (Ctrl+P) at all 5 resolutions.
#[test]
#[serial]
fn rust_command_dialog_matrix() -> Result<()> {
    RUST.run_screen(
        "command-dialog",
        "rust matrix",
        "cmddlg",
        recipe_command_dialog,
    )
}

/// Captures the model picker dialog at all 5 resolutions. Leader key is Ctrl+X
/// per default keybinds (`packages/jekko/src/config/keybinds.ts`); model_list
/// is `<leader>m`.
#[test]
#[serial]
fn rust_model_dialog_matrix() -> Result<()> {
    RUST.run_screen(
        "model-dialog",
        "rust matrix",
        "modeldlg",
        recipe_model_dialog,
    )
}

/// Captures the provider dialog (Ctrl+A from inside the model dialog).
#[test]
#[serial]
fn rust_provider_dialog_matrix() -> Result<()> {
    RUST.run_screen(
        "provider-dialog",
        "rust matrix",
        "providerdlg",
        recipe_provider_dialog,
    )
}

/// Captures the theme picker dialog (`<leader>t` → Ctrl+X then T).
#[test]
#[serial]
fn rust_theme_dialog_matrix() -> Result<()> {
    RUST.run_screen(
        "theme-dialog",
        "rust matrix",
        "themedlg",
        recipe_theme_dialog,
    )
}

/// Captures session-empty: `<leader>n` creates a new session, screen shows the
/// session route with no messages.
#[test]
#[serial]
fn rust_session_empty_matrix() -> Result<()> {
    RUST.run_screen(
        "session-empty",
        "rust matrix",
        "sessempty",
        recipe_session_empty,
    )
}

/// Captures shell route (`<leader>b` toggles sidebar; shell is a top-level
/// route reached via command palette → "Shell").
#[test]
#[serial]
fn rust_shell_matrix() -> Result<()> {
    RUST.run_screen("shell", "rust matrix", "shell", recipe_shell)
}

/// Captures the prompt-autocomplete screen: type a slash to trigger
/// slash-command completion in the prompt.
#[test]
#[serial]
fn rust_prompt_autocomplete_matrix() -> Result<()> {
    RUST.run_screen(
        "prompt-autocomplete",
        "rust matrix",
        "promptac",
        recipe_prompt_autocomplete,
    )
}

/// Captures the early splash frame. Splash is brief so capture at the first
/// non-blank screen before the home sentinel arrives.
#[test]
#[serial]
fn rust_splash_matrix() -> Result<()> {
    RUST.run_splash("splash")
}

/// Captures the Jnoccio dashboard (Ctrl+J after Jnoccio model is configured).
/// Spins up a tiny mock `/health` server at `127.0.0.1:4317` so the binary
/// believes Jnoccio is reachable. Failures are advisory.
#[test]
#[serial]
fn rust_jnoccio_panel_matrix() -> Result<()> {
    RUST.run_jnoccio_panel("jnoccio")
}

/// Captures the ZYAL paste indicator in the home prompt. Pastes a known ZYAL
/// runbook snippet and waits for the `✓ ZYAL` sigil. Failures are advisory.
#[test]
#[serial]
fn rust_zyal_panel_matrix() -> Result<()> {
    let example_path =
        test_helpers::repo_root().join("docs/ZYAL/examples/13-advanced-research-loop.zyal");
    RUST.run_zyal_panel("zyal", &example_path)
}
