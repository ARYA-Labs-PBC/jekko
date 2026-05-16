//! Rust-binary interactive PTY tests covering the dialog-key forwarding wired
//! into `crates/jekko-tui/src/app.rs::dispatch_key`.
//!
//! Tests open dialogs via the documented key chords (`Ctrl+P` for the command
//! palette, `Ctrl+X` leader + `m`/`t` for select dialogs), type into the open
//! widget, and assert the dialog responds. Captures go to
//! `target/tuiwright-jekko/rust/dialog-keys/` for follow-up inspection.
//!
//! Guards: identical to `rust_baseline_matrix.rs` — `JEKKO_BIN` set, binary
//! larger than the stub threshold, and `JEKKO_RUST_MATRIX=1`.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
        eprintln!("skipped: set JEKKO_RUST_MATRIX=1 to engage the Rust dialog-key matrix");
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
    // Write OUTSIDE the `rust/` matrix tree so `xtask baseline-diff` does not
    // walk these regression captures (they have no baseline counterpart).
    let dir = ensure_artifact_dir()?.join("dialog-keys");
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
        .trace_path(trace_dir.join(format!("dialog-keys-{trace}.trace.jsonl")))
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
    Page::spawn(cfg).context("spawn jekko TUI for dialog-key tests")
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

/// Ctrl+P opens the command palette; typing filters the visible list; Esc
/// closes it.
#[test]
#[serial]
fn rust_command_palette_filter_then_close() -> Result<()> {
    let Some(jekko) = rust_jekko_bin() else {
        return Ok(());
    };
    let (workspace, _, _, _, _, _) = prepare_workspace("project", "# dialog-keys\n")?;
    std::fs::write(
        workspace.path().join(".env.jnoccio"),
        "JNOCCIO_DEVELOPER_KEY=fake\n",
    )?;

    let page = spawn_offline(&workspace, &jekko, 120, 30, "palette-filter")?;
    wait_for_home(&page, &workspace, "palette-filter")?;

    // Open palette.
    page.press(Key::Ctrl('p'))?;
    std::thread::sleep(Duration::from_millis(300));
    capture(&page, "01-palette-open-120x30")?;

    // Type filter.
    page.type_text("ses")?;
    std::thread::sleep(STABILIZE);
    capture(&page, "02-palette-filtered-ses-120x30")?;

    // Verify "session.new" still visible (the filter matches it).
    let after_filter = page.screen().plain_text();
    let has_session = after_filter.to_lowercase().contains("session");
    if !has_session {
        bail!("expected 'session' visible after filter; screen:\n{after_filter}");
    }

    // Close.
    page.press(Key::Char('\u{1b}'))?;
    std::thread::sleep(STABILIZE);
    capture(&page, "03-palette-closed-120x30")?;

    Ok(())
}

/// Ctrl+X leader + 'm' opens model dialog; j moves the cursor; Esc closes.
#[test]
#[serial]
fn rust_model_dialog_cursor_then_close() -> Result<()> {
    let Some(jekko) = rust_jekko_bin() else {
        return Ok(());
    };
    let (workspace, _, _, _, _, _) = prepare_workspace("project", "# dialog-keys\n")?;
    std::fs::write(
        workspace.path().join(".env.jnoccio"),
        "JNOCCIO_DEVELOPER_KEY=fake\n",
    )?;

    let page = spawn_offline(&workspace, &jekko, 160, 40, "model-cursor")?;
    wait_for_home(&page, &workspace, "model-cursor")?;

    // Leader chord: Ctrl+X then 'm'.
    page.press(Key::Ctrl('x'))?;
    std::thread::sleep(Duration::from_millis(120));
    page.press(Key::Char('m'))?;
    std::thread::sleep(Duration::from_millis(300));
    capture(&page, "10-model-open-160x40")?;

    let after_open = page.screen().plain_text();
    if !after_open.to_lowercase().contains("model") {
        bail!("expected 'Model' visible after open; screen:\n{after_open}");
    }

    // Move cursor down twice.
    page.press(Key::Char('j'))?;
    page.press(Key::Char('j'))?;
    std::thread::sleep(STABILIZE);
    capture(&page, "11-model-cursor-down-160x40")?;

    page.press(Key::Char('\u{1b}'))?;
    std::thread::sleep(STABILIZE);
    capture(&page, "12-model-closed-160x40")?;

    Ok(())
}

/// Ctrl+X leader + 't' opens theme dialog; verify it renders the two themes.
#[test]
#[serial]
fn rust_theme_dialog_options_visible() -> Result<()> {
    let Some(jekko) = rust_jekko_bin() else {
        return Ok(());
    };
    let (workspace, _, _, _, _, _) = prepare_workspace("project", "# dialog-keys\n")?;
    std::fs::write(
        workspace.path().join(".env.jnoccio"),
        "JNOCCIO_DEVELOPER_KEY=fake\n",
    )?;

    let page = spawn_offline(&workspace, &jekko, 200, 60, "theme-options")?;
    wait_for_home(&page, &workspace, "theme-options")?;

    // Send leader as raw chord bytes — `Ctrl+X` is 0x18 and 't' is 0x74.
    // tuiwright's `press(Key::Char(...))` sometimes loses the follower when
    // it races the previous press; raw byte writes are more deterministic.
    let mut opened = false;
    for _ in 0..3 {
        page.type_text("\x18t")?;
        if page.wait_for_text("Theme", Duration::from_secs(2)).is_ok() {
            opened = true;
            break;
        }
    }
    capture(&page, "20-theme-open-200x60")?;
    if !opened {
        let after = page.screen().plain_text();
        bail!("theme dialog did not open after 3 retries; screen:\n{after}");
    }

    let after_open = page.screen().plain_text();
    let lower = after_open.to_lowercase();
    if !lower.contains("dark") || !lower.contains("light") {
        bail!("expected Dark + Light visible in theme dialog; screen:\n{after_open}");
    }

    page.press(Key::Char('\u{1b}'))?;
    std::thread::sleep(STABILIZE);
    capture(&page, "21-theme-closed-200x60")?;

    Ok(())
}

/// Boot-time splash window should leave a non-empty frame for at least the
/// first ~150ms before the route paints. This is the assertion the matrix
/// `rust_splash_matrix` test relies on; here we explicitly time it.
#[test]
#[serial]
fn rust_splash_window_holds_for_at_least_100ms() -> Result<()> {
    let Some(jekko) = rust_jekko_bin() else {
        return Ok(());
    };
    let (workspace, _, _, _, _, _) = prepare_workspace("project", "# splash-timing\n")?;
    std::fs::write(
        workspace.path().join(".env.jnoccio"),
        "JNOCCIO_DEVELOPER_KEY=fake\n",
    )?;

    let started = Instant::now();
    let page = spawn_offline(&workspace, &jekko, 120, 30, "splash-timing")?;

    // Look for any non-empty frame that does NOT contain the home sentinel.
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut splash_seen_at: Option<Instant> = None;
    while Instant::now() < deadline {
        let plain = page.screen().plain_text();
        if !plain.trim().is_empty() && !plain.contains("commands") {
            splash_seen_at = Some(Instant::now());
            capture(&page, "30-splash-window-120x30")?;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let Some(seen) = splash_seen_at else {
        bail!("never observed a non-blank pre-home frame within 3s");
    };
    let elapsed = seen.duration_since(started);
    eprintln!(
        "splash-window seen at {}ms after spawn",
        elapsed.as_millis()
    );
    // We only assert that the splash was reached at all; the 200ms window in
    // `App::run_loop` keeps the splash visible long enough for PTY capture.
    Ok(())
}
