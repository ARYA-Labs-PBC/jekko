//! PTY tests for the slash-command popup (`/` trigger in the prompt).
//!
//! Verifies that:
//! - Typing `/` opens the popup and shows the command list.
//! - Typing `/jan` filters to jankurai-related commands.
//! - `/jankurai` appears in the filtered list.
//! - `Esc` closes the popup.
//!
//! Guards: identical to `rust_dialog_keys.rs` — `JEKKO_BIN` set, binary
//! larger than the stub threshold, and `JEKKO_RUST_MATRIX=1`.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serial_test::serial;
use tuiwright::{Key, Page, SpawnConfig};

mod test_helpers;
use test_helpers::{copy_jekko_logs, ensure_artifact_dir, jekko_bin, prepare_workspace};

const STUB_BINARY_THRESHOLD: u64 = 5 * 1024 * 1024;
const BOOT_TIMEOUT: Duration = Duration::from_secs(20);
const STABILIZE: Duration = Duration::from_millis(400);

fn rust_jekko_bin() -> Option<PathBuf> {
    if std::env::var("JEKKO_RUST_MATRIX").as_deref() != Ok("1") {
        eprintln!("skipped: set JEKKO_RUST_MATRIX=1 to engage the Rust slash-popup matrix");
        return None;
    }
    let path = jekko_bin()?;
    let size = std::fs::metadata(&path).ok()?.len();
    if size < STUB_BINARY_THRESHOLD {
        eprintln!(
            "skipped: {path:?} is {size} bytes (< stub threshold {STUB_BINARY_THRESHOLD}); \
             rebuild jekko-cli in release mode"
        );
        return None;
    }
    Some(path)
}

fn artifact_root() -> Result<PathBuf> {
    let dir = ensure_artifact_dir()?.join("slash-popup");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn capture(page: &Page, name: &str) -> Result<()> {
    let dir = artifact_root()?;
    page.screenshot(dir.join(format!("{name}.png")))?;
    let plain = page.screen().plain_text();
    std::fs::write(dir.join(format!("{name}.txt")), plain)?;
    Ok(())
}

fn spawn_offline(
    parent: &tempfile::TempDir,
    jekko: &Path,
    cols: u16,
    rows: u16,
    trace: &str,
) -> Result<Page> {
    let project = parent.path().join("project");
    let xdg = parent.path().join("xdg");
    let trace_dir = ensure_artifact_dir()?.join("traces");
    std::fs::create_dir_all(&trace_dir)?;
    let mut cfg = SpawnConfig::new(jekko.to_string_lossy().as_ref())
        .arg("--pure")
        .arg("--log-level")
        .arg("WARN")
        .arg(project.to_string_lossy().as_ref())
        .cwd(&project)
        .size(cols, rows)
        .trace_path(trace_dir.join(format!("slash-popup-{trace}.trace.jsonl")))
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("HOME", parent.path().to_string_lossy().as_ref())
        .env("JEKKO_API_KEY", "tuiwright-offline-fake-key")
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_LSP_DOWNLOAD", "1")
        .env("JEKKO_DISABLE_MODELS_FETCH", "1")
        .env("JEKKO_DISABLE_JNOCCIO_BOOT", "1")
        .env("JEKKO_DISABLE_PRUNE", "1")
        .env("XDG_DATA_HOME", xdg.join("data").to_string_lossy().as_ref())
        .env(
            "XDG_CACHE_HOME",
            xdg.join("cache").to_string_lossy().as_ref(),
        )
        .env(
            "XDG_CONFIG_HOME",
            xdg.join("config").to_string_lossy().as_ref(),
        )
        .env(
            "XDG_STATE_HOME",
            xdg.join("state").to_string_lossy().as_ref(),
        )
        .timeout(Duration::from_secs(45));
    for (k, v) in std::env::vars() {
        if matches!(
            k.as_str(),
            "USER" | "LOGNAME" | "PATH" | "SHELL" | "LANG" | "LC_ALL" | "LC_CTYPE"
        ) {
            cfg = cfg.env(k, v);
        }
    }
    Page::spawn(cfg).context("spawn jekko TUI for slash-popup tests")
}

fn wait_for_home(page: &Page, workspace: &tempfile::TempDir, label: &str) -> Result<()> {
    page.wait_for_text("commands", BOOT_TIMEOUT)
        .with_context(|| {
            let _ = capture(page, &format!("{label}-boot-timeout"));
            let _ = copy_jekko_logs(workspace, label);
            format!("home sentinel 'commands' not reached for {label}")
        })?;
    Ok(())
}

/// Typing `/` opens the slash popup; the full command list is visible.
#[test]
#[serial]
fn rust_slash_popup_opens_on_slash() -> Result<()> {
    let Some(jekko) = rust_jekko_bin() else {
        return Ok(());
    };
    let (workspace, _, _, _, _, _) = prepare_workspace("project", "# slash-popup\n")?;
    std::fs::write(
        workspace.path().join(".env.jnoccio"),
        "JNOCCIO_DEVELOPER_KEY=fake\n",
    )?;

    let page = spawn_offline(&workspace, &jekko, 120, 35, "popup-open")?;
    wait_for_home(&page, &workspace, "popup-open")?;

    // Type `/` — popup should open showing command list.
    page.type_text("/")?;
    std::thread::sleep(STABILIZE);
    capture(&page, "01-slash-popup-open-120x35")?;

    let after_slash = page.screen().plain_text();
    // The popup lists built-in commands; "/help" and "/quit" must be present.
    let lower = after_slash.to_lowercase();
    if !lower.contains("help") || !lower.contains("quit") {
        bail!("slash popup did not open or missing expected entries; screen:\n{after_slash}");
    }

    // Clean up.
    page.press(Key::Char('\u{1b}'))?;
    Ok(())
}

/// Typing `/jan` filters the list to jankurai-related entries.
#[test]
#[serial]
fn rust_slash_popup_filters_on_type() -> Result<()> {
    let Some(jekko) = rust_jekko_bin() else {
        return Ok(());
    };
    let (workspace, _, _, _, _, _) = prepare_workspace("project", "# slash-popup\n")?;
    std::fs::write(
        workspace.path().join(".env.jnoccio"),
        "JNOCCIO_DEVELOPER_KEY=fake\n",
    )?;

    let page = spawn_offline(&workspace, &jekko, 120, 35, "popup-filter")?;
    wait_for_home(&page, &workspace, "popup-filter")?;

    page.type_text("/jan")?;
    std::thread::sleep(STABILIZE);
    capture(&page, "10-slash-filtered-jan-120x35")?;

    let filtered = page.screen().plain_text();
    let lower = filtered.to_lowercase();
    if !lower.contains("jankurai") {
        bail!("expected 'jankurai' visible after /jan filter; screen:\n{filtered}");
    }
    // Commands that don't start with "jan" should be gone.
    if lower.contains("/help") || lower.contains("/quit") {
        bail!("unexpected un-filtered entries visible; screen:\n{filtered}");
    }

    page.press(Key::Char('\u{1b}'))?;
    Ok(())
}

/// `/jankurai` (the full-cycle command) appears in the filtered list.
#[test]
#[serial]
fn rust_slash_popup_jankurai_visible() -> Result<()> {
    let Some(jekko) = rust_jekko_bin() else {
        return Ok(());
    };
    let (workspace, _, _, _, _, _) = prepare_workspace("project", "# slash-popup\n")?;
    std::fs::write(
        workspace.path().join(".env.jnoccio"),
        "JNOCCIO_DEVELOPER_KEY=fake\n",
    )?;

    let page = spawn_offline(&workspace, &jekko, 160, 40, "popup-jankurai")?;
    wait_for_home(&page, &workspace, "popup-jankurai")?;

    page.type_text("/jankurai")?;
    std::thread::sleep(STABILIZE);
    capture(&page, "20-slash-jankurai-160x40")?;

    let screen = page.screen().plain_text();
    if !screen.to_lowercase().contains("jankurai") {
        bail!("'/jankurai' not visible in popup; screen:\n{screen}");
    }

    page.press(Key::Char('\u{1b}'))?;
    Ok(())
}

/// `Esc` closes the slash popup and returns to a clean prompt.
#[test]
#[serial]
fn rust_slash_popup_closes_on_esc() -> Result<()> {
    let Some(jekko) = rust_jekko_bin() else {
        return Ok(());
    };
    let (workspace, _, _, _, _, _) = prepare_workspace("project", "# slash-popup\n")?;
    std::fs::write(
        workspace.path().join(".env.jnoccio"),
        "JNOCCIO_DEVELOPER_KEY=fake\n",
    )?;

    let page = spawn_offline(&workspace, &jekko, 120, 35, "popup-esc")?;
    wait_for_home(&page, &workspace, "popup-esc")?;

    // Open popup.
    page.type_text("/")?;
    std::thread::sleep(STABILIZE);
    capture(&page, "30-slash-open-120x35")?;

    // Close with Esc.
    page.press(Key::Char('\u{1b}'))?;
    std::thread::sleep(STABILIZE);
    capture(&page, "31-slash-closed-120x35")?;

    let after_esc = page.screen().plain_text();
    // After Esc, the popup entries should no longer dominate the screen.
    // The footer with key hints should be visible again.
    if !after_esc.to_lowercase().contains("commands") {
        bail!("expected shell footer visible after popup closed; screen:\n{after_esc}");
    }

    Ok(())
}
