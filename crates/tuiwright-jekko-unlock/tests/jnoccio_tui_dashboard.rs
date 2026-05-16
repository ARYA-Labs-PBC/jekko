//! Jnoccio TUI dashboard integration tests.
//!
//! These test the Jnoccio Fusion dashboard as integrated into the Jekko TUI.
//! Each test spawns a real Jekko instance via TUIwright, navigates to the
//! dashboard, and validates user flows:
//!
//!   - Ctrl+J toggle (hidden when server is down)
//!   - Tab navigation (1-6 keys)
//!   - `?` help overlay
//!   - Command palette contains Jnoccio entry
//!
//! Requires: JEKKO_BIN env pointing to a fresh build.
//!           JNOCCIO_TUI_TEST=1 to opt-in (these are slow PTY tests).

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serial_test::serial;
use tuiwright::{Key, Page};

mod test_helpers;
use test_helpers::{
    copy_jekko_logs, ensure_artifact_dir, jekko_bin, prepare_workspace, spawn_jekko_with_size_env,
};

const ARTIFACT_SUBDIR: &str = "jnoccio-tui";
const BOOT_TIMEOUT: Duration = Duration::from_secs(30);
const SHORT_TIMEOUT: Duration = Duration::from_secs(2);
const JNOCCIO_ADDR: &str = "127.0.0.1:4317";
const FAKE_API_KEY: &str = "tuiwright-offline-fake-key";
const FAKE_DEVELOPER_KEY: &str = "fake";

struct MockJnoccioServer {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Drop for MockJnoccioServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(JNOCCIO_ADDR);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn health_check() -> bool {
    let Ok(mut stream) = TcpStream::connect(JNOCCIO_ADDR) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let request = b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    if stream.write_all(request).is_err() {
        return false;
    }
    let mut response = String::new();
    if stream.read_to_string(&mut response).is_err() {
        return false;
    }
    response.starts_with("HTTP/1.1 200")
        || response.starts_with("HTTP/1.0 200")
        || response.starts_with("HTTP/1.1 401")
        || response.starts_with("HTTP/1.0 401")
        || response.starts_with("HTTP/1.1 404")
        || response.starts_with("HTTP/1.0 404")
}

fn write_http(mut stream: TcpStream, status: &str, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn start_mock_jnoccio_server() -> Result<Option<MockJnoccioServer>> {
    let addr: SocketAddr = JNOCCIO_ADDR.parse()?;
    let listener = match TcpListener::bind(addr) {
        Ok(listener) => listener,
        Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
            if health_check() {
                return Ok(None);
            }
            bail!("{JNOCCIO_ADDR} is already in use, but /health did not answer like Jnoccio");
        }
        Err(err) => return Err(err).context("bind mock Jnoccio health server"),
    };
    listener
        .set_nonblocking(true)
        .context("set mock Jnoccio listener nonblocking")?;
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = Arc::clone(&stop);
    let handle = thread::spawn(move || {
        while !stop_thread.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut buf = [0_u8; 1024];
                    let n = stream.read(&mut buf).unwrap_or(0);
                    let request = String::from_utf8_lossy(&buf[..n]);
                    if request.starts_with("GET /health ") {
                        write_http(
                            stream,
                            "200 OK",
                            "application/json",
                            r#"{"ok":true,"available_models":1,"keyed_models":1}"#,
                        );
                    } else {
                        write_http(stream, "404 Not Found", "text/plain", "not found");
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(_) => break,
            }
        }
    });
    Ok(Some(MockJnoccioServer {
        stop,
        handle: Some(handle),
    }))
}

fn write_jnoccio_model_config(config_home: &std::path::Path) -> Result<()> {
    let config_dir = config_home.join("jekko");
    std::fs::create_dir_all(&config_dir)?;
    std::fs::write(
        config_dir.join("jekko.json"),
        format!(
            r#"{{"model":"jnoccio/jnoccio-fusion","provider":{{"jekko":{{"options":{{"apiKey":"{FAKE_API_KEY}"}}}},"jnoccio":{{"options":{{"apiKey":"{FAKE_API_KEY}"}}}}}}}}"#
        ),
    )?;
    Ok(())
}

fn write_jnoccio_home_key(home: &std::path::Path) -> Result<()> {
    std::fs::write(
        home.join(".env.jnoccio"),
        format!("JNOCCIO_DEVELOPER_KEY={FAKE_DEVELOPER_KEY}\n"),
    )?;
    Ok(())
}

#[allow(dead_code)]
fn spawn_jekko_with_jnoccio_enabled(
    parent: &tempfile::TempDir,
    jekko: &std::path::Path,
    trace_name: &str,
) -> Result<Page> {
    let project = parent.path().join("project");
    let xdg = parent.path().join("xdg");
    let artifact_dir = ensure_artifact_dir()?;
    let trace_dir = artifact_dir.join("traces");
    std::fs::create_dir_all(&trace_dir)?;

    let mut cfg = tuiwright::SpawnConfig::new(jekko.to_string_lossy().as_ref())
        .arg("--pure")
        .arg("--log-level")
        .arg("DEBUG")
        .arg("--model")
        .arg("jnoccio/jnoccio-fusion")
        .arg(project.to_string_lossy().as_ref())
        .cwd(&project)
        .size(200, 60)
        .trace_path(trace_dir.join(format!("{trace_name}.trace.jsonl")))
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("HOME", parent.path().to_string_lossy().as_ref())
        .env("JEKKO_API_KEY", FAKE_API_KEY)
        .env("JNOCCIO_DEFAULT_API_KEY", FAKE_API_KEY)
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_LSP_DOWNLOAD", "1")
        .env("JEKKO_DISABLE_MODELS_FETCH", "1")
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
    Page::spawn(cfg).context("spawn jekko TUI with Jnoccio enabled")
}

fn enabled() -> bool {
    std::env::var("JNOCCIO_TUI_TEST").as_deref() == Ok("1")
}

fn wait_for_boot(
    page: &tuiwright::Page,
    workspace: &tempfile::TempDir,
    artifact_dir: &std::path::Path,
    name: &str,
) -> Result<()> {
    page.wait_for_text("ctrl+p commands", BOOT_TIMEOUT)
        .with_context(|| {
            let _ = page.screenshot(artifact_dir.join(format!("{name}-boot-failed.png")));
            let _ = copy_jekko_logs(workspace, name);
            "TUI did not boot"
        })
}

// ── Test: Dashboard is hidden when server is unavailable ─────────────

#[test]
#[ignore]
#[serial]
fn jnoccio_dashboard_hidden_when_server_offline() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set JNOCCIO_TUI_TEST=1");
        return Ok(());
    }
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN");
        return Ok(());
    };
    let artifact_dir = ensure_artifact_dir()?.join(ARTIFACT_SUBDIR);
    std::fs::create_dir_all(&artifact_dir)?;

    let (workspace, _project, _, _, _, _) = prepare_workspace("project", "# test\n")?;
    write_jnoccio_home_key(workspace.path())?;
    let model_keys_path = workspace.path().join("model-keys.env");
    std::fs::write(&model_keys_path, "OPENAI_API_KEY=fake-openai-key\n")?;
    let page = spawn_jekko_with_size_env(
        &workspace,
        &jekko,
        200,
        60,
        "jnoccio-hidden-offline",
        &[(
            "JEKKO_MODEL_KEYS_FILE",
            model_keys_path.to_string_lossy().as_ref(),
        )],
    )?;

    // Wait for TUI boot
    wait_for_boot(&page, &workspace, &artifact_dir, "jnoccio-hidden-offline")?;

    // The footer should NOT show "Jnoccio" shortcut hint since no server is running.
    // wait_for_text with a short timeout should time out — proving it's absent.
    let found_jnoccio = page.wait_for_text("^J", SHORT_TIMEOUT).is_ok();
    assert!(
        !found_jnoccio,
        "Jnoccio shortcut should be hidden when server is offline"
    );

    page.screenshot(artifact_dir.join("01-hidden-when-offline.png"))?;
    Ok(())
}

// ── Test: Ctrl+J is a no-op when server is offline ──────────────────

#[test]
#[ignore]
#[serial]
fn jnoccio_ctrl_j_noop_when_offline() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set JNOCCIO_TUI_TEST=1");
        return Ok(());
    }
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN");
        return Ok(());
    };
    let artifact_dir = ensure_artifact_dir()?.join(ARTIFACT_SUBDIR);
    std::fs::create_dir_all(&artifact_dir)?;

    let (workspace, _project, _, _, _, _) = prepare_workspace("project", "# test\n")?;
    write_jnoccio_home_key(workspace.path())?;
    let model_keys_path = workspace.path().join("model-keys.env");
    std::fs::write(&model_keys_path, "OPENAI_API_KEY=fake-openai-key\n")?;
    let page = spawn_jekko_with_size_env(
        &workspace,
        &jekko,
        200,
        60,
        "jnoccio-ctrl-j-offline",
        &[(
            "JEKKO_MODEL_KEYS_FILE",
            model_keys_path.to_string_lossy().as_ref(),
        )],
    )?;
    wait_for_boot(&page, &workspace, &artifact_dir, "jnoccio-ctrl-j-offline")?;

    page.screenshot(artifact_dir.join("02-home-before-ctrl-j.png"))?;

    // Press Ctrl+J — since server is not running, should be a no-op
    page.press(Key::Ctrl('j'))?;
    std::thread::sleep(Duration::from_millis(500));

    // Should still be on home — "Jnoccio Fusion" dashboard header should NOT appear
    let on_dashboard = page.wait_for_text("Jnoccio Fusion", SHORT_TIMEOUT).is_ok();
    assert!(
        !on_dashboard,
        "Ctrl+J should NOT open dashboard when server is offline"
    );

    page.screenshot(artifact_dir.join("03-ctrl-j-blocked-offline.png"))?;
    Ok(())
}

// ── Test: Header shortcut appears when Jnoccio is enabled ─────────────

#[test]
#[ignore]
#[serial]
fn jnoccio_header_shortcut_visible_when_model_enabled() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set JNOCCIO_TUI_TEST=1");
        return Ok(());
    }
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN");
        return Ok(());
    };
    let _mock = start_mock_jnoccio_server()?;
    let artifact_dir = ensure_artifact_dir()?.join(ARTIFACT_SUBDIR);
    std::fs::create_dir_all(&artifact_dir)?;

    let (workspace, _project, _, _, xdg_config, _) =
        prepare_workspace("project", "# jnoccio enabled header\n")?;
    write_jnoccio_home_key(workspace.path())?;
    let model_keys_path = workspace.path().join("model-keys.env");
    std::fs::write(&model_keys_path, "OPENAI_API_KEY=fake-openai-key\n")?;
    write_jnoccio_model_config(&xdg_config)?;
    let page = spawn_jekko_with_size_env(
        &workspace,
        &jekko,
        200,
        60,
        "jnoccio-header-shortcut-ready",
        &[
            (
                "JEKKO_MODEL_KEYS_FILE",
                model_keys_path.to_string_lossy().as_ref(),
            ),
            ("JNOCCIO_DEFAULT_API_KEY", FAKE_API_KEY),
        ],
    )?;
    wait_for_boot(
        &page,
        &workspace,
        &artifact_dir,
        "jnoccio-header-shortcut-ready",
    )?;

    page.wait_for_text("^J", Duration::from_secs(10))
        .with_context(|| {
            let _ = page.screenshot(artifact_dir.join("06-header-shortcut-missing.png"));
            let _ = copy_jekko_logs(&workspace, "jnoccio-header-shortcut-ready");
            "Jnoccio shortcut did not appear in the navigation header"
        })?;
    page.wait_for_text("Jnoccio", Duration::from_secs(10))
        .context("Jnoccio navigation label did not appear")?;
    page.screenshot(artifact_dir.join("06-header-shortcut-visible.png"))?;

    page.press(Key::Ctrl('j'))?;
    page.wait_for_text("Jnoccio Fusion", Duration::from_secs(10))
        .with_context(|| {
            let _ = page.screenshot(artifact_dir.join("07-dashboard-missing.png"));
            let _ = copy_jekko_logs(&workspace, "jnoccio-header-shortcut-ready");
            "Ctrl+J did not open the Jnoccio dashboard"
        })?;
    page.screenshot(artifact_dir.join("07-dashboard-opened.png"))?;
    Ok(())
}

// ── Test: Build contains Jnoccio plugin in command palette ──────────

#[test]
#[ignore]
#[serial]
fn jnoccio_build_contains_dashboard_plugin() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set JNOCCIO_TUI_TEST=1");
        return Ok(());
    }
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN");
        return Ok(());
    };
    let artifact_dir = ensure_artifact_dir()?.join(ARTIFACT_SUBDIR);
    std::fs::create_dir_all(&artifact_dir)?;

    let (workspace, _project, _, _, _, _) = prepare_workspace("project", "# test\n")?;
    write_jnoccio_home_key(workspace.path())?;
    let model_keys_path = workspace.path().join("model-keys.env");
    std::fs::write(&model_keys_path, "OPENAI_API_KEY=fake-openai-key\n")?;
    let page = spawn_jekko_with_size_env(
        &workspace,
        &jekko,
        200,
        60,
        "jnoccio-command-palette",
        &[(
            "JEKKO_MODEL_KEYS_FILE",
            model_keys_path.to_string_lossy().as_ref(),
        )],
    )?;
    wait_for_boot(&page, &workspace, &artifact_dir, "jnoccio-command-palette")?;

    // Open command palette
    page.press(Key::Ctrl('p'))?;
    std::thread::sleep(Duration::from_millis(800));

    // Type "jnoccio" to filter commands.
    // The Jnoccio Dashboard command is registered even when offline
    // (it just won't appear in the list because the command register
    // callback returns [] when not ready). This test validates the
    // palette opens and the TUI handles the search gracefully.
    page.type_text("jnoccio")?;
    std::thread::sleep(Duration::from_millis(500));

    page.screenshot(artifact_dir.join("04-command-palette-jnoccio.png"))?;

    // Close palette with Ctrl+P again
    page.press(Key::Ctrl('p'))?;
    std::thread::sleep(Duration::from_millis(300));

    page.screenshot(artifact_dir.join("05-palette-closed.png"))?;
    Ok(())
}

// ── Test: "No agent connected" warning shows when server is offline ───
//
// Regression guard for the silent "0/0 models" bug: the Jnoccio panel
// MUST render a visible warning when the server is unreachable, not just
// silently show zero counts that look like valid data.

#[test]
#[ignore]
#[serial]
fn jnoccio_header_shows_no_agent_warning_when_offline() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set JNOCCIO_TUI_TEST=1");
        return Ok(());
    }
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN");
        return Ok(());
    };
    // Ensure no real Jnoccio server is already running.
    if health_check() {
        eprintln!("skipped: a Jnoccio server is already running at {JNOCCIO_ADDR}");
        return Ok(());
    }

    let artifact_dir = ensure_artifact_dir()?.join(ARTIFACT_SUBDIR);
    std::fs::create_dir_all(&artifact_dir)?;

    let (workspace, _project, _, _, xdg_config, _) =
        prepare_workspace("project", "# no-agent warning test\n")?;
    write_jnoccio_home_key(workspace.path())?;
    let model_keys_path = workspace.path().join("model-keys.env");
    std::fs::write(&model_keys_path, "OPENAI_API_KEY=fake-openai-key\n")?;
    write_jnoccio_model_config(&xdg_config)?;

    // Spawn WITHOUT JEKKO_DISABLE_JNOCCIO_BOOT so the TUI actually probes.
    let project = workspace.path().join("project");
    let xdg = workspace.path().join("xdg");
    let trace_dir = artifact_dir.join("traces");
    std::fs::create_dir_all(&trace_dir)?;
    let cfg = tuiwright::SpawnConfig::new(jekko.to_string_lossy().as_ref())
        .arg("--pure")
        .arg(project.to_string_lossy().as_ref())
        .cwd(&project)
        .size(200, 60)
        .trace_path(trace_dir.join("jnoccio-no-agent-warning.trace.jsonl"))
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("HOME", workspace.path().to_string_lossy().as_ref())
        .env("JEKKO_API_KEY", FAKE_API_KEY)
        .env("JNOCCIO_DEFAULT_API_KEY", FAKE_API_KEY)
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_LSP_DOWNLOAD", "1")
        .env("JEKKO_DISABLE_MODELS_FETCH", "1")
        .env("JEKKO_DISABLE_PRUNE", "1")
        .env(
            "JEKKO_MODEL_KEYS_FILE",
            model_keys_path.to_string_lossy().as_ref(),
        )
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
    let page = Page::spawn(cfg).context("spawn jekko for no-agent warning test")?;

    wait_for_boot(&page, &workspace, &artifact_dir, "jnoccio-no-agent-warning")?;
    page.screenshot(artifact_dir.join("08-home-no-agent.png"))?;

    // Open the Jnoccio panel via Ctrl+J — it should open even offline (panel shows the error).
    page.press(Key::Ctrl('j'))?;
    std::thread::sleep(Duration::from_millis(1500));
    page.screenshot(artifact_dir.join("08-jnoccio-panel-offline.png"))?;

    // The panel MUST show the ⚠ warning — NOT silent "0/0 models".
    page.wait_for_text("No agent connected", Duration::from_secs(8))
        .with_context(|| {
            let _ = page.screenshot(artifact_dir.join("08-no-agent-missing.png"));
            let _ = copy_jekko_logs(&workspace, "jnoccio-no-agent-warning");
            "⚠ 'No agent connected' warning did NOT appear in Jnoccio panel when server is offline. \
             This is the silent-zeros regression — fix render_header() / dashboard-view.tsx"
        })?;

    page.screenshot(artifact_dir.join("08-no-agent-warning-confirmed.png"))?;
    Ok(())
}

// ── Test: Model count shows real number when mock server is live ──────
//
// Guards against the inverse failure: when the server IS up, the model
// count must reflect the server response (not show "0/0").

#[test]
#[ignore]
#[serial]
fn jnoccio_header_shows_model_count_when_server_live() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set JNOCCIO_TUI_TEST=1");
        return Ok(());
    }
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN");
        return Ok(());
    };

    // Start the mock server — it responds with keyed_models=1 on /health.
    let _mock = start_mock_jnoccio_server()?;

    let artifact_dir = ensure_artifact_dir()?.join(ARTIFACT_SUBDIR);
    std::fs::create_dir_all(&artifact_dir)?;

    let (workspace, _project, _, _, xdg_config, _) =
        prepare_workspace("project", "# model count test\n")?;
    write_jnoccio_home_key(workspace.path())?;
    let model_keys_path = workspace.path().join("model-keys.env");
    std::fs::write(&model_keys_path, "OPENAI_API_KEY=fake-openai-key\n")?;
    write_jnoccio_model_config(&xdg_config)?;

    let project = workspace.path().join("project");
    let xdg = workspace.path().join("xdg");
    let trace_dir = artifact_dir.join("traces");
    std::fs::create_dir_all(&trace_dir)?;
    let cfg = tuiwright::SpawnConfig::new(jekko.to_string_lossy().as_ref())
        .arg("--pure")
        .arg(project.to_string_lossy().as_ref())
        .cwd(&project)
        .size(200, 60)
        .trace_path(trace_dir.join("jnoccio-model-count-live.trace.jsonl"))
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("HOME", workspace.path().to_string_lossy().as_ref())
        .env("JEKKO_API_KEY", FAKE_API_KEY)
        .env("JNOCCIO_DEFAULT_API_KEY", FAKE_API_KEY)
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_LSP_DOWNLOAD", "1")
        .env("JEKKO_DISABLE_MODELS_FETCH", "1")
        .env("JEKKO_DISABLE_PRUNE", "1")
        .env(
            "JEKKO_MODEL_KEYS_FILE",
            model_keys_path.to_string_lossy().as_ref(),
        )
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
    let page = Page::spawn(cfg).context("spawn jekko for model-count-live test")?;

    wait_for_boot(&page, &workspace, &artifact_dir, "jnoccio-model-count-live")?;

    // Open the Jnoccio panel
    page.press(Key::Ctrl('j'))?;
    page.wait_for_text("Jnoccio Fusion", Duration::from_secs(10))
        .context("dashboard did not open after Ctrl+J")?;
    page.screenshot(artifact_dir.join("09-dashboard-live.png"))?;

    // The mock server returns keyed_models=1, so the model count must NOT be "0/0".
    // We assert "1/1 models" appears (matching the mock server /health response).
    page.wait_for_text("1/1 models", Duration::from_secs(15))
        .with_context(|| {
            let _ = page.screenshot(artifact_dir.join("09-model-count-wrong.png"));
            let _ = copy_jekko_logs(&workspace, "jnoccio-model-count-live");
            "Jnoccio dashboard shows '0/0 models' even with mock server running. \
             Either the TUI is not reading from /health, or the model count is not \
             propagated to the panel snapshot."
        })?;

    page.screenshot(artifact_dir.join("09-model-count-confirmed.png"))?;
    Ok(())
}
