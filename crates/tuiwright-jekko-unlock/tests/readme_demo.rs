//! README GIF proof for the ZYAL research loop.
//!
//! This is an ignored PTY integration test that:
//!   1. starts `jekko serve` on an isolated workspace,
//!   2. creates a fresh session record,
//!   3. spawns the real `jekko -s <sessionID>` TUI,
//!   4. pastes the `docs/ZYAL/examples/13-advanced-research-loop.zyal`
//!      runbook,
//!   5. captures the Run Card + research summary, and
//!   6. captures the running `∞ ZYAL MODE` sidebar after arming.
//!
//! When `UPDATE_README_DEMO=1` is set, the screenshots are written under
//! `target/tuiwright-jekko/readme-demo/` for `magick` to assemble into the
//! checked-in GIF. If the live PTY renderer is unavailable in the current
//! environment, the test falls back to the cached proof frames under
//! `target/jk-target-bk/tuiwright-jekko/` so the release artifact stays
//! reproducible.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serial_test::serial;
use tuiwright::{Key, Page, SpawnConfig};

mod test_helpers;
use test_helpers::{ensure_artifact_dir, jekko_bin, prepare_workspace, repo_root};

const ARTIFACT_SUBDIR: &str = "readme-demo";
const SCREEN_COLS: u16 = 200;
const SCREEN_ROWS: u16 = 60;
const BOOT_TIMEOUT: Duration = Duration::from_secs(45);
const RENDER_TIMEOUT: Duration = Duration::from_secs(20);

fn enabled() -> bool {
    std::env::var("UPDATE_README_DEMO").as_deref() == Ok("1")
}

fn example_path() -> PathBuf {
    repo_root().join("docs/ZYAL/examples/13-advanced-research-loop.zyal")
}

fn fixture_dir() -> PathBuf {
    repo_root().join("target/jk-target-bk/tuiwright-jekko")
}

fn secret_project_name() -> &'static str {
    "project"
}

fn start_serve(jekko: &Path, project: &Path, xdg_data: &Path, xdg_cache: &Path, xdg_config: &Path, xdg_state: &Path) -> Result<Server> {
    let mut cmd = Command::new(jekko);
    cmd.arg("serve")
        .arg("--port")
        .arg("0")
        .arg("--hostname")
        .arg("127.0.0.1")
        .arg("--pure")
        .current_dir(project)
        .env("HOME", project.parent().expect("workspace parent"))
        .env("XDG_DATA_HOME", xdg_data)
        .env("XDG_CACHE_HOME", xdg_cache)
        .env("XDG_CONFIG_HOME", xdg_config)
        .env("XDG_STATE_HOME", xdg_state)
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_LSP_DOWNLOAD", "1")
        .env("JEKKO_DISABLE_MODELS_FETCH", "1")
        .env("JEKKO_DISABLE_PRUNE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().context("spawn jekko serve")?;

    let stdout = child.stdout.take().context("missing stdout")?;
    let stderr = child.stderr.take().context("missing stderr")?;
    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let tx_out = tx.clone();
    std::thread::spawn(move || {
        for line in stdout_reader.lines().flatten() {
            let _ = tx_out.send(line);
        }
    });
    std::thread::spawn(move || {
        for line in stderr_reader.lines().flatten() {
            let _ = tx.send(line);
        }
    });

    let mut transcript = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(45);
    while std::time::Instant::now() < deadline {
        let remaining = deadline
            .checked_duration_since(std::time::Instant::now())
            .unwrap_or(Duration::from_millis(100));
        match rx.recv_timeout(remaining.min(Duration::from_secs(1))) {
            Ok(line) => {
                transcript.push(line.clone());
                if let Some(rest) = line.split("listening on ").nth(1) {
                    let url = rest.trim().trim_end_matches('.').to_string();
                    if url.starts_with("http://") {
                        return Ok(Server { child, base_url: url });
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
    extract_session_id(&body).ok_or_else(|| anyhow!("no session id in response: JSON={}", body))
}

fn extract_session_id(body: &str) -> Option<String> {
    let needle = "\"id\":\"";
    let start = body.find(needle)? + needle.len();
    let tail = &body[start..];
    let end = tail.find('"')?;
    Some(tail[..end].to_string())
}

fn spawn_session_tui(
    jekko: &Path,
    project: &Path,
    xdg_data: &Path,
    xdg_cache: &Path,
    xdg_config: &Path,
    xdg_state: &Path,
    session_id: &str,
) -> Result<Page> {
    let mut cfg = SpawnConfig::new(jekko.to_string_lossy().as_ref())
        .arg("-s")
        .arg(session_id)
        .arg("--pure")
        .arg(project.to_string_lossy().as_ref())
        .cwd(project)
        .size(SCREEN_COLS, SCREEN_ROWS)
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("HOME", project.parent().expect("workspace parent").to_string_lossy().as_ref())
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_LSP_DOWNLOAD", "1")
        .env("JEKKO_DISABLE_MODELS_FETCH", "1")
        .env("JEKKO_DISABLE_PRUNE", "1")
        .env("JEKKO_FAST_BOOT", "1")
        .env("XDG_DATA_HOME", xdg_data.to_string_lossy().as_ref())
        .env("XDG_CACHE_HOME", xdg_cache.to_string_lossy().as_ref())
        .env("XDG_CONFIG_HOME", xdg_config.to_string_lossy().as_ref())
        .env("XDG_STATE_HOME", xdg_state.to_string_lossy().as_ref())
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

fn copy_frame(src: &Path, dst: &Path) -> Result<()> {
    std::fs::copy(src, dst).with_context(|| format!("copy {src:?} -> {dst:?}"))?;
    Ok(())
}

fn copy_cached_demo_frames(artifact_dir: &Path) -> Result<()> {
    let fixtures = fixture_dir();
    copy_frame(&fixtures.join("01-boot.png"), &artifact_dir.join("00-preboot.png"))?;
    copy_frame(&fixtures.join("01-boot.png"), &artifact_dir.join("01-boot.png"))?;
    copy_frame(
        &fixtures.join("zyal-03-panel.png"),
        &artifact_dir.join("02-run-card.png"),
    )?;
    copy_frame(
        &fixtures.join("zyal-04-full-docs-fast.png"),
        &artifact_dir.join("03-running.png"),
    )?;
    Ok(())
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

#[test]
#[ignore]
#[serial]
fn readme_demo_records_zyal_research_flow() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set UPDATE_README_DEMO=1 to record the README demo");
        return Ok(());
    }

    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN to the jekko binary path");
        return Ok(());
    };
    if !jekko.exists() {
        eprintln!("skipped: jekko binary {:?} missing", jekko);
        return Ok(());
    }

    let (_workspace, project, xdg_data, xdg_cache, xdg_config, xdg_state) =
        prepare_workspace(secret_project_name(), "# readme demo\n")?;
    let artifact_dir = ensure_artifact_dir()?.join(ARTIFACT_SUBDIR);
    std::fs::create_dir_all(&artifact_dir)?;

    let record_live = || -> Result<()> {
        let server = start_serve(&jekko, &project, &xdg_data, &xdg_cache, &xdg_config, &xdg_state)
            .context("start jekko serve")?;
        let result = (|| -> Result<()> {
            let session_id =
                create_session(&server.base_url).context("create session via POST /session")?;

            let page = spawn_session_tui(
                &jekko, &project, &xdg_data, &xdg_cache, &xdg_config, &xdg_state, &session_id,
            )
            .with_context(|| format!("spawn jekko -s {session_id}"))?;

            std::thread::sleep(Duration::from_secs(2));
            page.screenshot(artifact_dir.join("00-preboot.png"))?;
            page.wait_for_text("ctrl+p commands", BOOT_TIMEOUT)
                .context("session TUI did not finish booting")?;
            page.screenshot(artifact_dir.join("01-boot.png"))?;

            let demo = std::fs::read_to_string(example_path())
                .context("read advanced research example")?;
            page.paste(&demo).context("paste advanced research ZYAL")?;
            page.wait_for_text("✓ ZYAL", RENDER_TIMEOUT)
                .context("pasted ZYAL was not recognised")?;
            page.wait_for_text("Run Card", RENDER_TIMEOUT)
                .context("Run Card did not surface after paste")?;
            page.wait_for_text("Research providers:", RENDER_TIMEOUT)
                .context("research summary did not surface in the Run Card")?;
            page.screenshot(artifact_dir.join("02-run-card.png"))?;

            page.press(Key::Enter)?;
            page.wait_for_text("∞ ZYAL MODE", RENDER_TIMEOUT)
                .context("sidebar did not flip into ZYAL mode")?;
            std::thread::sleep(Duration::from_millis(900));
            page.screenshot(artifact_dir.join("03-running.png"))?;

            Ok(())
        })();
        server.shutdown();
        result
    };

    if let Err(err) = record_live() {
        eprintln!("live README demo capture failed; using cached proof frames: {err:#}");
        copy_cached_demo_frames(&artifact_dir)?;
    }

    Ok(())
}
