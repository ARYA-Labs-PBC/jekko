//! Real-PTY proof that pasting a valid ZYAL block inside the **session** view
//! lights up the right-hand `∞ ZYAL MODE` sidebar (not just the home overlay).
//!
//! Flow:
//!   1. Spin up `jekko serve` against an isolated XDG so we own the SQLite store.
//!   2. POST {} to `/session` to mint a fresh session record. No model is set,
//!      no provider is required — we only need the row to exist so the TUI
//!      enters session route.
//!   3. Stop the headless server.
//!   4. Spawn `jekko -s <sessionID>` in a PTY against the same XDG. The TUI
//!      hydrates the session from SQLite and renders the session sidebar.
//!   5. Bracketed-paste a minimal valid ZYAL daemon block.
//!   6. Assert `✓ ZYAL` (footer detection) **and** `∞ ZYAL MODE` (sidebar
//!      panel) surface within a few seconds.
//!
//! Skipped only when `JEKKO_BIN` is unset — bun source-run is unreliable for
//! the headless serve flow because the dev server expects packaged assets.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use serial_test::serial;
use tempfile::TempDir;
use tuiwright::{Page, SpawnConfig};

const SCREEN_COLS: u16 = 200;
const SCREEN_ROWS: u16 = 60;
const ARTIFACT_DIR: &str = "target/tuiwright-jekko";
const RECOGNITION_TIMEOUT: Duration = Duration::from_secs(8);
const SESSION_BOOT_TIMEOUT: Duration = Duration::from_secs(45);

const ZYAL_BLOCK: &str = "<<<ZYAL v1:daemon id=tuiwright-session>>>\n\
version: v1\n\
intent: daemon\n\
confirm: RUN_FOREVER\n\
job:\n  name: \"tuiwright session paste proof\"\n  objective: \"prove sidebar panel\"\n\
stop:\n  all:\n    - git_clean: {}\n\
<<<END_ZYAL id=tuiwright-session>>>\n\
ZYAL_ARM RUN_FOREVER id=tuiwright-session";

fn enabled() -> bool {
    std::env::var("JEKKO_TUIWRIGHT_PTY").as_deref() == Ok("1")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root above crates/<name>")
        .to_path_buf()
}

fn jekko_bin() -> Option<PathBuf> {
    let bin = std::env::var("JEKKO_BIN").ok()?;
    let path = PathBuf::from(bin);
    if !path.exists() {
        eprintln!("JEKKO_BIN points to a missing file: {path:?}");
        return None;
    }
    Some(path)
}

fn ensure_artifact_dir() -> Result<PathBuf> {
    let dir = repo_root().join(ARTIFACT_DIR);
    std::fs::create_dir_all(&dir).with_context(|| format!("create {dir:?}"))?;
    Ok(dir)
}

struct Workspace {
    parent: TempDir,
    project: PathBuf,
    xdg_data: PathBuf,
    xdg_cache: PathBuf,
    xdg_config: PathBuf,
    xdg_state: PathBuf,
}

fn prepare_workspace() -> Result<Workspace> {
    let parent = TempDir::new().context("tempdir")?;
    let project = parent.path().join("project");
    std::fs::create_dir_all(&project)?;
    std::fs::write(project.join("README.md"), "# zyal session paste proof\n")?;
    let xdg = parent.path().join("xdg");
    let xdg_data = xdg.join("data");
    let xdg_cache = xdg.join("cache");
    let xdg_config = xdg.join("config");
    let xdg_state = xdg.join("state");
    for dir in [&xdg_data, &xdg_cache, &xdg_config, &xdg_state] {
        std::fs::create_dir_all(dir)?;
    }
    let config_dir = xdg_config.join("jekko");
    std::fs::create_dir_all(&config_dir)?;
    std::fs::write(
        config_dir.join("jekko.json"),
        r#"{"provider":{"jekko":{"options":{"apiKey":"tuiwright-offline-fake-key"}}}}"#,
    )?;
    Ok(Workspace {
        parent,
        project,
        xdg_data,
        xdg_cache,
        xdg_config,
        xdg_state,
    })
}

fn home_str(ws: &Workspace) -> String {
    ws.parent.path().to_string_lossy().into_owned()
}

struct Server {
    child: Child,
    base_url: String,
}

impl Server {
    fn shutdown(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start_serve(jekko: &PathBuf, ws: &Workspace) -> Result<Server> {
    let mut cmd = Command::new(jekko);
    cmd.arg("serve")
        .arg("--port")
        .arg("0")
        .arg("--hostname")
        .arg("127.0.0.1")
        .arg("--pure")
        .current_dir(&ws.project)
        .env("HOME", home_str(ws))
        .env("XDG_DATA_HOME", &ws.xdg_data)
        .env("XDG_CACHE_HOME", &ws.xdg_cache)
        .env("XDG_CONFIG_HOME", &ws.xdg_config)
        .env("XDG_STATE_HOME", &ws.xdg_state)
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_LSP_DOWNLOAD", "1")
        .env("JEKKO_DISABLE_PRUNE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().context("spawn jekko serve")?;

    // Drain stdout for the listening URL. Server prints a single line:
    //   "jekko server listening on http://127.0.0.1:PORT"
    let stdout = match child.stdout.take() {
        Some(stream) => stream,
        None => {
            eprintln!("WARNING: jekko serve process has no stdout - this indicates a potential issue with the jekko binary or invocation");
            return Err(anyhow!("jekko serve has no stdout - check that JEKKO_BIN points to a valid jekko binary and that the server can start successfully"));
        }
    };
    let stderr = match child.stderr.take() {
        Some(stream) => stream,
        None => {
            eprintln!("WARNING: jekko serve process has no stderr - diagnostic information will be unavailable if the server fails");
            return Err(anyhow!("jekko serve has no stderr - this may impede debugging if the server fails to start properly"));
        }
    };
    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let tx_out = tx.clone();
    std::thread::spawn(move || {
        for line in stdout_reader.lines().flatten() {
            let _ = tx_out.send(line);
        }
    });
    let tx_err = tx.clone();
    std::thread::spawn(move || {
        for line in stderr_reader.lines().flatten() {
            let _ = tx_err.send(line);
        }
    });

    let deadline = Instant::now() + Duration::from_secs(45);
    let mut transcript: Vec<String> = Vec::new();
    while Instant::now() < deadline {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or(Duration::from_millis(100));
        match rx.recv_timeout(remaining.min(Duration::from_secs(1))) {
            Ok(line) => {
                transcript.push(line.clone());
                if let Some(rest) = line.split("listening on ").nth(1) {
                    let url = rest.trim().trim_end_matches('.').to_string();
                    if url.starts_with("http://") {
                        return Ok(Server {
                            child,
                            base_url: url,
                        });
                    }
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = child.kill();
    Err(anyhow!(
        "jekko serve never reported a listening URL within 45s. transcript:\n{}",
        transcript.join("\n")
    ))
}

fn create_session(base_url: &str) -> Result<String> {
    let endpoint = format!("{base_url}/session");
    let output = Command::new("curl")
        .arg("-sf")
        .arg("-X")
        .arg("POST")
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("--data-binary")
        .arg("{}")
        .arg(&endpoint)
        .output()
        .with_context(|| format!("curl POST {endpoint}"))?;
    if !output.status.success() {
        return Err(anyhow!(
            "POST {endpoint} failed: {}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let body = String::from_utf8(output.stdout).context("decode session JSON")?;
    match extract_session_id(&body) {
        Some(id) => Ok(id),
        None => Err(anyhow!("no session id in response: JSON={}", body)),
    }
}

fn extract_session_id(body: &str) -> Option<String> {
    let needle = "\"id\":\"";
    let start = body.find(needle)? + needle.len();
    let tail = &body[start..];
    let end = tail.find('"')?;
    Some(tail[..end].to_string())
}

#[test]
fn extract_session_id_round_trips_common_shapes() {
    for id in ["alpha", "alpha-1", "alpha_2", "alpha.3", "Alpha123_-"] {
        let body = format!(r#"{{"id":"{id}","status":"ok"}}"#);
        assert_eq!(extract_session_id(&body).as_deref(), Some(id));
    }
}

fn spawn_session_tui(jekko: &PathBuf, ws: &Workspace, session_id: &str) -> Result<Page> {
    let mut cfg = SpawnConfig::new(jekko.to_string_lossy().as_ref())
        .arg("-s")
        .arg(session_id)
        .arg("--pure")
        .arg(ws.project.to_string_lossy().as_ref())
        .cwd(&ws.project)
        .size(SCREEN_COLS, SCREEN_ROWS)
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("HOME", home_str(ws))
        .env("JEKKO_API_KEY", "tuiwright-offline-fake-key")
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_LSP_DOWNLOAD", "1")
        .env("JEKKO_DISABLE_PRUNE", "1")
        .env("JEKKO_FAST_BOOT", "1")
        .env("XDG_DATA_HOME", ws.xdg_data.to_string_lossy().as_ref())
        .env("XDG_CACHE_HOME", ws.xdg_cache.to_string_lossy().as_ref())
        .env("XDG_CONFIG_HOME", ws.xdg_config.to_string_lossy().as_ref())
        .env("XDG_STATE_HOME", ws.xdg_state.to_string_lossy().as_ref())
        .timeout(Duration::from_secs(60));

    for (k, v) in std::env::vars() {
        match k.as_str() {
            "USER" | "LOGNAME" | "PATH" | "SHELL" | "LANG" | "LC_ALL" | "LC_CTYPE" => {
                cfg = cfg.env(k, v);
            }
            _ => {}
        }
    }

    Page::spawn(cfg).context("spawn jekko TUI in session view")
}

#[test]
#[serial]
fn pasting_zyal_inside_a_session_lights_up_the_right_sidebar() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set JEKKO_TUIWRIGHT_PTY=1 to run prompt PTY tests");
        return Ok(());
    }
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN to the jekko binary path");
        return Ok(());
    };

    let ws = prepare_workspace()?;
    let artifact_dir = ensure_artifact_dir()?;

    // 1-3: serve → POST /session → keep the server alive until the TUI has
    // hydrated the session view.
    let server = start_serve(&jekko, &ws).context("start jekko serve")?;
    let session_id =
        create_session(&server.base_url).context("create session via POST /session")?;

    // 4: spawn TUI directly into the session route.
    let page = spawn_session_tui(&jekko, &ws, &session_id)
        .with_context(|| format!("spawn jekko -s {session_id}"))?;

    // The session view shows the prompt footer keybinds — same as home — so we
    // wait on the same boot sentinel. The presence of the sidebar (right rail)
    // is what proves we're in session route, and that's verified by the panel
    // assertion below.
    page.wait_for_text("ctrl+p commands", SESSION_BOOT_TIMEOUT)
        .context("session TUI did not finish booting")?;
    page.screenshot(artifact_dir.join("zyal-session-01-boot.png"))?;

    // 5: bracketed paste of a valid ZYAL block.
    page.paste(ZYAL_BLOCK)
        .context("send bracketed paste of ZYAL block")?;

    // 6a: footer detection.
    page.wait_for_text("✓ ZYAL", RECOGNITION_TIMEOUT)
        .context("session paste was not recognised as ZYAL — daemonDraft never flipped")?;
    page.screenshot(artifact_dir.join("zyal-session-02-detected.png"))?;

    // 6b: right-rail panel — the proof the user asked for.
    page.wait_for_text("∞ ZYAL MODE", Duration::from_secs(3))
        .context("session sidebar `∞ ZYAL MODE` panel did not surface")?;
    page.screenshot(artifact_dir.join("zyal-session-03-sidebar.png"))?;

    server.shutdown();

    Ok(())
}
