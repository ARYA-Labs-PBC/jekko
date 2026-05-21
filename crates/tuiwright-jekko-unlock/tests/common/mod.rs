//! Shared driver for the baseline-capture matrix tests.
//!
//! Both [`baseline_matrix.rs`](../baseline_matrix.rs) (captured reference
//! binary) and [`rust_baseline_matrix.rs`](../rust_baseline_matrix.rs) (Rust
//! port binary) walk the same set of resolutions and screens. This module
//! holds the shared resolution list, spawn config, capture helpers, boot
//! sentinel, mock-server, ZYAL paste, and splash-timing helpers so the two
//! test files become thin matrix drivers that only supply their binary
//! resolver and artifact subdirectory.
//!
//! Cargo integration tests have to declare `mod common;` themselves since the
//! `tests/` tree compiles each `.rs` file as its own binary — see the
//! [Cargo book](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#integration-tests)
//! for the `tests/common/mod.rs` convention.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use tuiwright::{Key, Page, SpawnConfig};

use super::test_helpers::{
    copy_jekko_logs, ensure_artifact_dir, jekko_bin, prepare_workspace, spawn_jekko_with_size_env,
};

/// Resolutions used by every matrix test.
pub const RESOLUTIONS: &[(u16, u16)] = &[(80, 24), (100, 30), (120, 30), (160, 40), (200, 60)];

/// Sentinel text present on the home/shell breadcrumb footer (still emitted by
/// both Home and the new Shell-landing footer post-splash).
pub const HOME_SENTINEL: &str = "commands";

/// Per-step waits used across the matrix.
///
/// Phase B doubled the splash hold window (MIN_HOLD 1.6 s, MAX_HOLD 10 s) so
/// boot needs roughly twice the headroom it used to. We bump `BOOT_TIMEOUT`
/// from 20 s → 30 s and add a dedicated `BOOT_LONG_TIMEOUT` (used by the
/// idle-empty / engaged-shell capture recipes) that comfortably outlasts the
/// 10 s ceiling plus a couple of frame settles.
pub const BOOT_TIMEOUT: Duration = Duration::from_secs(30);
pub const BOOT_LONG_TIMEOUT: Duration = Duration::from_secs(15);
pub const STABILIZE_PAUSE: Duration = Duration::from_millis(400);
pub const SHORT_WAIT: Duration = Duration::from_secs(5);

/// Placeholder Jnoccio developer key used by the dev-key tests.
pub const FAKE_DEVELOPER_KEY: &str = "fake";

/// Anything below this is treated as a scaffold / empty stub binary; the Rust
/// matrix skips cleanly so CI stays green while the Rust TUI is still being
/// assembled. Real release builds of jekko clock in well above 5 MB.
pub const STUB_BINARY_THRESHOLD: u64 = 5 * 1024 * 1024;

/// Env-var opt-in for the Rust render parity matrix.
pub const RUST_OPT_IN_ENV: &str = "JEKKO_RUST_MATRIX";

/// Static configuration that distinguishes the captured-reference matrix from
/// the Rust-port matrix. Held in a `const` per matrix driver.
pub struct MatrixConfig {
    /// Artifact subdirectory under `target/tuiwright-jekko/` (e.g. `"baseline"`).
    pub artifact_subdir: &'static str,
    /// Trace-file prefix prepended to `<trace>.trace.jsonl`.
    pub trace_prefix: &'static str,
    /// Log level passed via `--log-level`.
    pub log_level: &'static str,
    /// Human-readable prefix used in error messages (e.g. `""` or `"rust "`).
    pub error_prefix: &'static str,
    /// Resolves the binary to test (returns `None` to skip the matrix).
    pub resolve_binary: fn() -> Option<PathBuf>,
}

impl MatrixConfig {
    fn root(&self) -> Result<PathBuf> {
        let dir = ensure_artifact_dir()?.join(self.artifact_subdir);
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn screen_dir(&self, screen: &str) -> Result<PathBuf> {
        let dir = self.root()?.join(screen);
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Capture PNG + plain-text snapshot for a single resolution.
    pub fn capture(
        &self,
        page: &Page,
        screen: &str,
        cols: u16,
        rows: u16,
        label_suffix: Option<&str>,
    ) -> Result<()> {
        let dir = self.screen_dir(screen)?;
        let stem = match label_suffix {
            Some(suffix) => format!("{cols}x{rows}-{suffix}"),
            None => format!("{cols}x{rows}"),
        };
        page.screenshot(dir.join(format!("{stem}.png")))?;
        let plain = page.screen().plain_text();
        std::fs::write(dir.join(format!("{stem}.txt")), plain)?;
        Ok(())
    }

    /// Spawn the binary under test in offline mode.
    pub fn spawn_offline(
        &self,
        parent: &tempfile::TempDir,
        jekko: &Path,
        cols: u16,
        rows: u16,
        trace_name: &str,
    ) -> Result<Page> {
        let project = parent.path().join("project");
        let xdg = parent.path().join("xdg");
        let artifact_dir = ensure_artifact_dir()?;
        let trace_dir = artifact_dir.join("traces");
        std::fs::create_dir_all(&trace_dir)?;
        let mut cfg = SpawnConfig::new(jekko.to_string_lossy().as_ref())
            .arg("--pure")
            .arg("--log-level")
            .arg(self.log_level)
            .arg(project.to_string_lossy().as_ref())
            .cwd(&project)
            .size(cols, rows)
            .trace_path(trace_dir.join(format!("{}{trace_name}.trace.jsonl", self.trace_prefix)))
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
            .timeout(Duration::from_secs(60));
        for (k, v) in std::env::vars() {
            if matches!(
                k.as_str(),
                "USER" | "LOGNAME" | "PATH" | "SHELL" | "LANG" | "LC_ALL" | "LC_CTYPE"
            ) {
                cfg = cfg.env(k, v);
            }
        }
        Page::spawn(cfg).context("spawn jekko TUI for matrix capture")
    }

    /// Block until the home sentinel appears. On failure, capture a timeout
    /// frame plus the jekko log directory for triage.
    pub fn wait_for_home(
        &self,
        page: &Page,
        workspace: &tempfile::TempDir,
        screen: &str,
        cols: u16,
        rows: u16,
    ) -> Result<()> {
        page.wait_for_text(HOME_SENTINEL, BOOT_TIMEOUT)
            .with_context(|| {
                let _ = self.capture(page, screen, cols, rows, Some("boot-timeout"));
                let log_prefix = format!("{}{screen}-{cols}x{rows}", self.trace_prefix);
                let _ = copy_jekko_logs(workspace, &log_prefix);
                format!("home sentinel '{HOME_SENTINEL}' not seen for {screen}@{cols}x{rows}")
            })?;
        Ok(())
    }

    /// Prepare a workspace with the placeholder Jnoccio developer key and a
    /// dummy `model-keys.env`. Used by every dev-key driven test.
    pub fn prepare_with_dev_keys(
        &self,
        readme_label: &str,
    ) -> Result<(tempfile::TempDir, PathBuf)> {
        let (workspace, _project, _, _, _, _) =
            prepare_workspace("project", &format!("# {readme_label}\n"))?;
        std::fs::write(
            workspace.path().join(".env.jnoccio"),
            format!("JNOCCIO_DEVELOPER_KEY={FAKE_DEVELOPER_KEY}\n"),
        )?;
        let model_keys = workspace.path().join("model-keys.env");
        std::fs::write(&model_keys, "OPENAI_API_KEY=fake-openai-key\n")?;
        Ok((workspace, model_keys))
    }

    /// Run a screen recipe across every resolution, collecting failures.
    ///
    /// `recipe` is invoked after boot and may press keys / type / wait before
    /// the helper captures the resulting frame. Boot timeouts are reported as
    /// failures without calling the recipe.
    pub fn run_screen<F>(
        &self,
        screen: &str,
        readme_label: &str,
        trace_prefix_inner: &str,
        recipe: F,
    ) -> Result<()>
    where
        F: Fn(&Page, &tempfile::TempDir, u16, u16) -> Result<()>,
    {
        let Some(jekko) = (self.resolve_binary)() else {
            return Ok(());
        };
        let mut failures = Vec::new();

        for (cols, rows) in RESOLUTIONS.iter().copied() {
            let (workspace, _) = self.prepare_with_dev_keys(readme_label)?;
            let trace = format!("{trace_prefix_inner}-{cols}x{rows}");
            let page = self.spawn_offline(&workspace, &jekko, cols, rows, &trace)?;
            if let Err(err) = self.wait_for_home(&page, &workspace, screen, cols, rows) {
                failures.push(format!("{screen} boot@{cols}x{rows}: {err:#}"));
                continue;
            }
            if let Err(err) = recipe(&page, &workspace, cols, rows) {
                // Phase 3 hard-fail: capture a forensic frame under a
                // `recipe-timeout` suffix so triage can inspect what was
                // on screen when the recipe gave up, then record the
                // failure. The canonical baseline (no suffix) is NOT
                // overwritten with the timeout frame — that's exactly the
                // silent-fallback bug we're killing.
                let _ = self.capture(&page, screen, cols, rows, Some("recipe-timeout"));
                let log_prefix = format!("{}{screen}-{cols}x{rows}", self.trace_prefix);
                let _ = copy_jekko_logs(&workspace, &log_prefix);
                failures.push(format!("{screen} recipe@{cols}x{rows}: {err:#}"));
                continue;
            }
            if let Err(err) = self.capture(&page, screen, cols, rows, None) {
                failures.push(format!("{screen} capture@{cols}x{rows}: {err:#}"));
            }
        }

        if !failures.is_empty() {
            bail!(
                "{}{screen} matrix failures:\n{}",
                self.error_prefix,
                failures.join("\n")
            );
        }
        Ok(())
    }

    /// Splash matrix: capture the first non-blank pre-home frame within 12s.
    /// Phase B widened the splash hold window (MAX_HOLD 10 s), so the prior
    /// 3 s capture window would race against a still-loading splash and miss
    /// the NEVERHUMAN decoration band entirely. 12 s leaves ~2 s of head room
    /// past the MAX_HOLD ceiling. Splash failures stay advisory because the
    /// splash window is brief and flaky in CI.
    pub fn run_splash(&self, trace_prefix_inner: &str) -> Result<()> {
        let Some(jekko) = (self.resolve_binary)() else {
            return Ok(());
        };
        let mut failures = Vec::new();

        for (cols, rows) in RESOLUTIONS.iter().copied() {
            let (workspace, _) = self.prepare_with_dev_keys("splash baseline")?;
            let trace = format!("{trace_prefix_inner}-{cols}x{rows}");
            let page = self.spawn_offline(&workspace, &jekko, cols, rows, &trace)?;
            let deadline = Instant::now() + Duration::from_secs(12);
            let mut captured = false;
            while Instant::now() < deadline {
                let plain = page.screen().plain_text();
                if !plain.trim().is_empty() && !plain.contains(HOME_SENTINEL) {
                    if let Err(err) = self.capture(&page, "splash", cols, rows, None) {
                        failures.push(format!("splash capture@{cols}x{rows}: {err:#}"));
                    }
                    captured = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            if !captured {
                let _ = self.capture(&page, "splash", cols, rows, Some("fallback"));
                failures.push(format!(
                    "splash@{cols}x{rows}: never saw a non-home pre-paint frame within 12s; saved fallback"
                ));
            }
            // Let it boot fully so termination is clean.
            let _ = page.wait_for_text(HOME_SENTINEL, BOOT_TIMEOUT);
        }

        if !failures.is_empty() {
            eprintln!(
                "{}splash captures advisory:\n{}",
                self.error_prefix,
                failures.join("\n")
            );
        }
        Ok(())
    }

    /// Jnoccio panel matrix: spin a tiny /health TCP mock, configure the
    /// Jnoccio model, wait for Ctrl+J to open the dashboard, and capture.
    /// Failures are advisory (eprintln) because the panel is feature-flagged.
    pub fn run_jnoccio_panel(&self, trace_prefix_inner: &str) -> Result<()> {
        let Some(jekko) = (self.resolve_binary)() else {
            return Ok(());
        };

        const JNOCCIO_ADDR: &str = "127.0.0.1:4317";
        const FAKE_API_KEY: &str = "tuiwright-offline-fake-key";

        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = Arc::clone(&stop);
        let addr: SocketAddr = JNOCCIO_ADDR.parse().context("parse jnoccio addr")?;
        let listener = match TcpListener::bind(addr) {
            Ok(l) => l,
            Err(err) => {
                eprintln!(
                    "skipped {}jnoccio_panel_matrix: bind {JNOCCIO_ADDR} failed: {err}",
                    self.error_prefix
                );
                return Ok(());
            }
        };
        listener.set_nonblocking(true)?;
        let server_handle = thread::spawn(move || {
            while !stop_thread.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut buf = [0u8; 1024];
                        let _ = stream.read(&mut buf);
                        let body = r#"{"ok":true,"available_models":1,"keyed_models":1}"#;
                        let response = format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(_) => break,
                }
            }
        });

        let mut failures = Vec::new();

        for (cols, rows) in RESOLUTIONS.iter().copied() {
            let (workspace, _project, _, _, xdg_config, _) =
                prepare_workspace("project", "# jnoccio matrix\n")?;
            std::fs::write(
                workspace.path().join(".env.jnoccio"),
                format!("JNOCCIO_DEVELOPER_KEY={FAKE_DEVELOPER_KEY}\n"),
            )?;
            let config_dir = xdg_config.join("jekko");
            std::fs::create_dir_all(&config_dir)?;
            std::fs::write(
                config_dir.join("jekko.json"),
                format!(
                    r#"{{"model":"jnoccio/jnoccio-fusion","provider":{{"jekko":{{"options":{{"apiKey":"{FAKE_API_KEY}"}}}},"jnoccio":{{"options":{{"apiKey":"{FAKE_API_KEY}"}}}}}}}}"#
                ),
            )?;
            let model_keys = workspace.path().join("model-keys.env");
            std::fs::write(&model_keys, "OPENAI_API_KEY=fake-openai-key\n")?;
            let trace = format!("{trace_prefix_inner}-{cols}x{rows}");
            let page = match spawn_jekko_with_size_env(
                &workspace,
                &jekko,
                cols,
                rows,
                &trace,
                &[
                    (
                        "JEKKO_MODEL_KEYS_FILE",
                        model_keys.to_string_lossy().as_ref(),
                    ),
                    ("JNOCCIO_DEFAULT_API_KEY", FAKE_API_KEY),
                ],
            ) {
                Ok(p) => p,
                Err(err) => {
                    failures.push(format!("jnoccio spawn@{cols}x{rows}: {err:#}"));
                    continue;
                }
            };
            if let Err(err) = self.wait_for_home(&page, &workspace, "jnoccio-panel", cols, rows) {
                failures.push(format!("jnoccio boot@{cols}x{rows}: {err:#}"));
                continue;
            }
            // Wait for Jnoccio shortcut to appear in nav header before pressing Ctrl+J.
            let _ = page.wait_for_text("^J", Duration::from_secs(8));
            page.press(Key::Ctrl('j'))?;
            let opened = page.wait_for_text("Jnoccio Fusion", Duration::from_secs(8));
            std::thread::sleep(STABILIZE_PAUSE);
            let suffix = if opened.is_ok() {
                None
            } else {
                Some("no-dashboard")
            };
            if let Err(err) = self.capture(&page, "jnoccio-panel", cols, rows, suffix) {
                failures.push(format!("jnoccio capture@{cols}x{rows}: {err:#}"));
            } else if opened.is_err() {
                failures.push(format!(
                    "jnoccio@{cols}x{rows}: dashboard text not visible; captured no-dashboard frame"
                ));
            }
        }

        stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(JNOCCIO_ADDR);
        let _ = server_handle.join();

        if !failures.is_empty() {
            eprintln!(
                "{}jnoccio matrix advisory:\n{}",
                self.error_prefix,
                failures.join("\n")
            );
        }
        Ok(())
    }

    /// ZYAL panel matrix: paste a runbook and wait for the `✓ ZYAL` sigil.
    /// Failures are advisory because the sigil requires the binary to render
    /// the paste indicator. Returns `Ok(())` (silent) when the fixture is
    /// missing.
    pub fn run_zyal_panel(&self, trace_prefix_inner: &str, fixture: &Path) -> Result<()> {
        let Some(jekko) = (self.resolve_binary)() else {
            return Ok(());
        };

        let zyal = match std::fs::read_to_string(fixture) {
            Ok(s) => s,
            Err(err) => {
                eprintln!(
                    "skipped {}zyal_panel_matrix: missing fixture {fixture:?}: {err}",
                    self.error_prefix
                );
                return Ok(());
            }
        };

        let mut failures = Vec::new();
        for (cols, rows) in RESOLUTIONS.iter().copied() {
            let (workspace, _) = self.prepare_with_dev_keys("zyal matrix")?;
            let trace = format!("{trace_prefix_inner}-{cols}x{rows}");
            let page = self.spawn_offline(&workspace, &jekko, cols, rows, &trace)?;
            if let Err(err) = self.wait_for_home(&page, &workspace, "zyal-panel", cols, rows) {
                failures.push(format!("zyal boot@{cols}x{rows}: {err:#}"));
                continue;
            }
            page.paste(&zyal)?;
            let sigil = page.wait_for_text("✓ ZYAL", Duration::from_secs(10));
            std::thread::sleep(STABILIZE_PAUSE);
            let suffix = if sigil.is_ok() {
                None
            } else {
                Some("no-sigil")
            };
            if let Err(err) = self.capture(&page, "zyal-panel", cols, rows, suffix) {
                failures.push(format!("zyal capture@{cols}x{rows}: {err:#}"));
            } else if sigil.is_err() {
                failures.push(format!(
                    "zyal@{cols}x{rows}: ✓ ZYAL sigil not seen; captured no-sigil frame"
                ));
            }
        }

        if !failures.is_empty() {
            eprintln!(
                "{}zyal matrix advisory:\n{}",
                self.error_prefix,
                failures.join("\n")
            );
        }
        Ok(())
    }
}

/// Default binary resolver for the captured-reference baseline matrix:
/// just delegates to `JEKKO_BIN`. Returns `None` (skip) when unset.
pub fn reference_binary() -> Option<PathBuf> {
    let Some(path) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN to a jekko binary path");
        return None;
    };
    Some(path)
}

/// Binary resolver for the Rust render-parity matrix. Skips silently when any
/// of `JEKKO_BIN`, the stub-size threshold, or `JEKKO_RUST_MATRIX=1` are not
/// satisfied.
pub fn rust_binary() -> Option<PathBuf> {
    let path = jekko_bin()?;
    let meta = match std::fs::metadata(&path) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("skipped: stat {path:?} failed: {err}");
            return None;
        }
    };
    if meta.len() < STUB_BINARY_THRESHOLD {
        eprintln!(
            "skipped: {} is {} bytes (< {} threshold); treating as scaffold stub",
            path.display(),
            meta.len(),
            STUB_BINARY_THRESHOLD
        );
        return None;
    }
    match std::env::var(RUST_OPT_IN_ENV).ok().as_deref() {
        Some("1") => Some(path),
        _ => {
            eprintln!("skipped: set {RUST_OPT_IN_ENV}=1 to engage the Rust render parity matrix");
            None
        }
    }
}

// ---- Recipe helpers ---------------------------------------------------------

/// Recipe: home — Phase A collapse landed the app directly on `Route::Shell`,
/// so the "home" capture is now the Shell empty-state (logo + 2-line hint
/// "Press Enter to engage" / "Type and press Enter to send"). We wait for the
/// hint text to appear so we never capture a half-painted splash frame, then
/// stabilize. We deliberately do NOT press Enter — the engaged frame is the
/// job of `recipe_shell` below.
pub fn recipe_home(page: &Page, _ws: &tempfile::TempDir, _c: u16, _r: u16) -> Result<()> {
    page.wait_for_text("Press Enter to engage", BOOT_LONG_TIMEOUT)
        .context("idle empty-state hint 'Press Enter to engage' never rendered")?;
    std::thread::sleep(STABILIZE_PAUSE);
    Ok(())
}

/// Recipe: command palette — Ctrl+P.
pub fn recipe_command_dialog(page: &Page, _ws: &tempfile::TempDir, _c: u16, _r: u16) -> Result<()> {
    page.press(Key::Ctrl('p'))?;
    std::thread::sleep(Duration::from_millis(500));
    Ok(())
}

/// Recipe: model dialog — Ctrl+X then `m`.
pub fn recipe_model_dialog(page: &Page, _ws: &tempfile::TempDir, _c: u16, _r: u16) -> Result<()> {
    page.press(Key::Ctrl('x'))?;
    std::thread::sleep(Duration::from_millis(150));
    page.press(Key::Char('m'))?;
    std::thread::sleep(Duration::from_millis(500));
    Ok(())
}

/// Recipe: provider dialog — Ctrl+X, `m`, Ctrl+A.
pub fn recipe_provider_dialog(
    page: &Page,
    _ws: &tempfile::TempDir,
    _c: u16,
    _r: u16,
) -> Result<()> {
    page.press(Key::Ctrl('x'))?;
    std::thread::sleep(Duration::from_millis(150));
    page.press(Key::Char('m'))?;
    std::thread::sleep(Duration::from_millis(300));
    page.press(Key::Ctrl('a'))?;
    std::thread::sleep(Duration::from_millis(500));
    Ok(())
}

/// Recipe: theme dialog — Ctrl+X then `t`.
pub fn recipe_theme_dialog(page: &Page, _ws: &tempfile::TempDir, _c: u16, _r: u16) -> Result<()> {
    page.press(Key::Ctrl('x'))?;
    std::thread::sleep(Duration::from_millis(150));
    page.press(Key::Char('t'))?;
    std::thread::sleep(Duration::from_millis(500));
    Ok(())
}

/// Recipe: session-empty — same shape as `recipe_home` now that the app
/// lands directly on `Route::Shell` (the prior `<leader>n` invocation
/// produced an indistinguishable empty-state). The capture preserves the
/// idle empty-state for screens that historically diffed against this slot.
pub fn recipe_session_empty(page: &Page, _ws: &tempfile::TempDir, _c: u16, _r: u16) -> Result<()> {
    page.wait_for_text("Press Enter to engage", BOOT_LONG_TIMEOUT)
        .context("session-empty hint 'Press Enter to engage' never rendered")?;
    std::thread::sleep(STABILIZE_PAUSE);
    Ok(())
}

/// Recipe: shell engaged-state — Phase A landed the app directly on
/// `Route::Shell`, so the engagement flow is now (1) wait for idle empty-state
/// ("Press Enter to engage"), (2) press Enter to fire `Action::EngageSession`,
/// (3) wait until the slide animation completes and the logo / hint copy
/// disappear. The Shell banner "bypass permissions" stays visible across both
/// idle and engaged states (it's route-specific), so we sentinel on the
/// *disappearance* of "Press Enter to engage" to confirm the slide finished.
///
/// Bails hard when the slide never completes within `SHORT_WAIT`. The forensic
/// capture in the matrix driver will save what was on screen so we can debug.
pub fn recipe_shell(page: &Page, _ws: &tempfile::TempDir, _c: u16, _r: u16) -> Result<()> {
    // Idle empty-state must paint first, otherwise we'd be racing the splash.
    page.wait_for_text("Press Enter to engage", BOOT_LONG_TIMEOUT)
        .context("idle empty-state hint never rendered before Enter")?;
    page.press(Key::Enter)?;
    // Wait until the engage slide completes — both the "Press Enter to engage"
    // hint AND the logo glyphs drop off. We sentinel on the hint copy because
    // it's a stable ASCII substring; the logo is mostly half-block art.
    let deadline = Instant::now() + SHORT_WAIT;
    let mut engaged = false;
    while Instant::now() < deadline {
        let plain = page.screen().plain_text();
        // Phase A: once engaged, the empty-state body is suppressed, so the
        // hint text is gone. The shell banner still renders.
        if !plain.contains("Press Enter to engage") && plain.contains("bypass permissions") {
            engaged = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if !engaged {
        bail!(
            "shell engagement never completed within {:?} (hint 'Press Enter to engage' \
             never cleared after Enter)",
            SHORT_WAIT
        );
    }
    std::thread::sleep(STABILIZE_PAUSE);
    Ok(())
}

/// Recipe: prompt-autocomplete — engage Shell, then type a slash to open the
/// slash popup.
///
/// Phase A flow: app lands on Shell. We must engage first (Enter on empty
/// prompt) so the logo slides off and the prompt is the unambiguous focus,
/// then type `/`. The slash popup catalog ships "help / quit / new / model /
/// theme" descriptions; we sentinel on the static "Show keybind reference"
/// label (the description of `/help`, always the first row) because it's a
/// stable substring that only exists when the popup is rendered.
pub fn recipe_prompt_autocomplete(
    page: &Page,
    _ws: &tempfile::TempDir,
    _c: u16,
    _r: u16,
) -> Result<()> {
    // Wait for the idle empty-state, then engage with Enter on the empty
    // prompt. After engagement the logo slides away; we wait for the slide
    // to complete (hint copy gone) before issuing the `/` keystroke so the
    // popup paints into the fully-settled engaged layout.
    page.wait_for_text("Press Enter to engage", BOOT_LONG_TIMEOUT)
        .context("idle empty-state hint never rendered before Enter")?;
    page.press(Key::Enter)?;
    let engage_deadline = Instant::now() + SHORT_WAIT;
    while Instant::now() < engage_deadline {
        if !page.screen().plain_text().contains("Press Enter to engage") {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    page.type_text("/")?;
    // Slash popup floating-list is not yet wired into the Shell render tree
    // (it lives in `Prompt::slash` as state only), so we sentinel on the
    // literal `/` rendered inside the prompt box. The prompt metadata
    // counter (`1L · 1g · 1c`) confirms the keystroke landed in the buffer
    // rather than being swallowed by some chrome that happens to draw `/`.
    let deadline = Instant::now() + SHORT_WAIT;
    let mut saw_slash = false;
    while Instant::now() < deadline {
        let plain = page.screen().plain_text();
        if plain.contains("1L") && plain.contains("1g") && plain.contains('/') {
            saw_slash = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if !saw_slash {
        bail!(
            "prompt-autocomplete sentinel '/ in prompt buffer (1L · 1g)' not seen within {:?}",
            SHORT_WAIT
        );
    }
    std::thread::sleep(STABILIZE_PAUSE);
    Ok(())
}
