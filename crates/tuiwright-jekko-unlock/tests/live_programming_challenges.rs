//! Ignored local-only live battle tests for real-provider TUI round trips.
//!
//! Gates:
//! - `CI != true`
//! - `JEKKO_TUI_BATTLE=1`
//! - `JEKKO_TUI_BATTLE_LIVE=1`
//! - `JEKKO_BIN` points at a built host binary
//! - `JEKKO_API_KEY` is present
//!
//! These tests intentionally use disposable temp projects only.

use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serial_test::serial;
use tempfile::TempDir;
use tuiwright::{Key, Page, SpawnConfig};

mod test_helpers;
use test_helpers::{copy_jekko_logs, ensure_artifact_dir, jekko_bin};

const SCREEN_COLS: u16 = 200;
const SCREEN_ROWS: u16 = 60;
const BOOT_TIMEOUT: Duration = Duration::from_secs(60);
const LIVE_TIMEOUT: Duration = Duration::from_secs(240);
const DEFAULT_MODEL: &str = "jekko/gpt-5-nano";

struct LiveWorkspace {
    parent: TempDir,
    project: PathBuf,
    xdg_data: PathBuf,
    xdg_cache: PathBuf,
    xdg_config: PathBuf,
    xdg_state: PathBuf,
}

fn require_live_battle_bin() -> Result<Option<PathBuf>> {
    if std::env::var("JEKKO_TUI_BATTLE").as_deref() != Ok("1") {
        eprintln!("skipped: set JEKKO_TUI_BATTLE=1");
        return Ok(None);
    }
    if std::env::var("JEKKO_TUI_BATTLE_LIVE").as_deref() != Ok("1") {
        eprintln!("skipped: set JEKKO_TUI_BATTLE_LIVE=1");
        return Ok(None);
    }
    if std::env::var("CI").as_deref() == Ok("true") {
        bail!("live battle TUI tests must not run in CI");
    }
    if std::env::var("JEKKO_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        bail!("JEKKO_API_KEY is required for live battle TUI tests");
    }
    let Some(path) = jekko_bin() else {
        bail!("JEKKO_BIN must point to a built jekko binary for live battle TUI tests");
    };
    Ok(Some(path))
}

fn live_model() -> String {
    std::env::var("JEKKO_LIVE_MODEL")
        .or_else(|_| std::env::var("JEKKO_CHAT_MODEL"))
        .unwrap_or_else(|_| DEFAULT_MODEL.to_string())
}

fn prepare_live_workspace(name: &str, readme: &str) -> Result<LiveWorkspace> {
    let parent = TempDir::new().context("tempdir")?;
    let project = parent.path().join(name);
    std::fs::create_dir_all(&project)?;
    std::fs::write(project.join("README.md"), readme)?;

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
    let api_key = std::env::var("JEKKO_API_KEY").unwrap_or_default();
    std::fs::write(
        config_dir.join("jekko.json"),
        format!(
            r#"{{"model":"{}","provider":{{"jekko":{{"options":{{"apiKey":"{}"}}}}}}}}"#,
            live_model(),
            api_key
        ),
    )?;

    Ok(LiveWorkspace {
        parent,
        project,
        xdg_data,
        xdg_cache,
        xdg_config,
        xdg_state,
    })
}

fn artifact_dir(test_name: &str) -> Result<PathBuf> {
    let dir = ensure_artifact_dir()?
        .join("live-programming")
        .join(test_name);
    std::fs::create_dir_all(&dir).with_context(|| format!("create {dir:?}"))?;
    Ok(dir)
}

fn capture(page: &Page, dir: &Path, name: &str) -> Result<()> {
    page.screenshot(dir.join(format!("{name}.png")))?;
    std::fs::write(dir.join(format!("{name}.txt")), page.screen().plain_text())?;
    let text = page.screen().plain_text();
    if text.trim().is_empty() {
        bail!("{name}: terminal frame is blank");
    }
    let lower = text.to_lowercase();
    if lower.contains("thread '") || lower.contains("panicked at") || lower.contains("panic:") {
        bail!("{name}: visible panic-like text in frame:\n{text}");
    }
    Ok(())
}

fn spawn_live_tui(ws: &LiveWorkspace, jekko: &Path, trace_name: &str) -> Result<Page> {
    let trace_dir = ensure_artifact_dir()?.join("traces");
    std::fs::create_dir_all(&trace_dir)?;
    let mut cfg = SpawnConfig::new(jekko.to_string_lossy().as_ref())
        .arg("--pure")
        .arg("--log-level")
        .arg("DEBUG")
        .arg(ws.project.to_string_lossy().as_ref())
        .cwd(&ws.project)
        .size(SCREEN_COLS, SCREEN_ROWS)
        .trace_path(trace_dir.join(format!("{trace_name}.trace.jsonl")))
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("HOME", ws.parent.path().to_string_lossy().as_ref())
        .env("JEKKO_CHAT_MODEL", live_model())
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_LSP_DOWNLOAD", "1")
        .env("JEKKO_DISABLE_PRUNE", "1")
        .env("XDG_DATA_HOME", ws.xdg_data.to_string_lossy().as_ref())
        .env("XDG_CACHE_HOME", ws.xdg_cache.to_string_lossy().as_ref())
        .env("XDG_CONFIG_HOME", ws.xdg_config.to_string_lossy().as_ref())
        .env("XDG_STATE_HOME", ws.xdg_state.to_string_lossy().as_ref())
        .timeout(Duration::from_secs(120));

    for (key, value) in std::env::vars() {
        match key.as_str() {
            "USER"
            | "LOGNAME"
            | "PATH"
            | "SHELL"
            | "LANG"
            | "LC_ALL"
            | "LC_CTYPE"
            | "JEKKO_API_KEY"
            | "OPENAI_API_KEY"
            | "ANTHROPIC_API_KEY"
            | "JNOCCIO_DEVELOPER_KEY"
            | "JNOCCIO_DEFAULT_API_KEY"
            | "JNOCCIO_DEFAULT_BASE_URL"
            | "JNOCCIO_UNLOCK_SECRET_PATH"
            | "JEKKO_CHAT_GATEWAY_URL" => {
                cfg = cfg.env(key, value);
            }
            _ => {}
        }
    }

    Page::spawn(cfg).context("spawn live battle Jekko TUI")
}

fn wait_for_boot_and_gateway(
    page: &Page,
    ws: &LiveWorkspace,
    artifact_dir: &Path,
    label: &str,
) -> Result<()> {
    page.wait_for_text("bypass permissions", BOOT_TIMEOUT)
        .with_context(|| {
            let _ = capture(page, artifact_dir, &format!("{label}-boot-timeout"));
            let _ = copy_jekko_logs(&ws.parent, label);
            "live TUI did not boot"
        })?;
    wait_for_gateway(Duration::from_secs(90))
        .context("local live gateway 127.0.0.1:4317 did not become reachable")
}

fn wait_for_gateway(timeout: Duration) -> Result<()> {
    let addr: SocketAddr = "127.0.0.1:4317".parse()?;
    let deadline = Instant::now() + timeout;
    loop {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(anyhow!("timed out after {timeout:?} waiting for {addr}"));
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

fn submit_prompt(page: &Page, prompt: &str) -> Result<()> {
    page.paste(prompt)?;
    std::thread::sleep(Duration::from_millis(300));
    page.press(Key::Enter)?;
    Ok(())
}

fn run_cargo_test(project: &Path) -> Result<std::process::Output> {
    Command::new("cargo")
        .arg("test")
        .current_dir(project)
        .output()
        .context("run cargo test")
}

#[test]
#[ignore]
#[serial]
fn live_battle_prompt_roundtrip() -> Result<()> {
    let Some(jekko) = require_live_battle_bin()? else {
        return Ok(());
    };
    let ws = prepare_live_workspace("project", "# live battle prompt\n")?;
    let dir = artifact_dir("prompt-roundtrip")?;
    let page = spawn_live_tui(&ws, &jekko, "battle-live-roundtrip")?;
    wait_for_boot_and_gateway(&page, &ws, &dir, "prompt-roundtrip")?;
    capture(&page, &dir, "01-boot")?;

    submit_prompt(
        &page,
        "Reply exactly with JEKKO_TUI_LIVE_OK and no other text.",
    )?;
    page.wait_for_text("JEKKO_TUI_LIVE_OK", LIVE_TIMEOUT)
        .context("live model response did not contain JEKKO_TUI_LIVE_OK")?;
    capture(&page, &dir, "02-response")?;
    Ok(())
}

#[test]
#[ignore]
#[serial]
fn live_programming_challenge_adds_rust_function_and_test() -> Result<()> {
    let Some(jekko) = require_live_battle_bin()? else {
        return Ok(());
    };
    let ws = prepare_live_workspace("rust-challenge-a", "# Rust challenge A\n")?;
    std::fs::write(
        ws.project.join("Cargo.toml"),
        "[package]\nname = \"battle_challenge_a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
    )?;
    std::fs::create_dir_all(ws.project.join("src"))?;
    std::fs::write(
        ws.project.join("src/lib.rs"),
        "pub fn identity(n: i32) -> i32 {\n    n\n}\n",
    )?;

    let dir = artifact_dir("challenge-a")?;
    let page = spawn_live_tui(&ws, &jekko, "battle-live-challenge-a")?;
    wait_for_boot_and_gateway(&page, &ws, &dir, "challenge-a")?;
    capture(&page, &dir, "01-boot")?;

    let prompt = "In this temporary Rust library, edit src/lib.rs directly. Add a public pure function `double(n: i32) -> i32` that returns n * 2. Add a #[cfg(test)] unit test named `battle_double_works` that asserts double(21) == 42. Run `cargo test`. When complete, reply exactly with BATTLE_CHALLENGE_A_DONE.";
    submit_prompt(&page, prompt)?;
    page.wait_for_text("BATTLE_CHALLENGE_A_DONE", LIVE_TIMEOUT)
        .context("challenge A did not report completion")?;
    capture(&page, &dir, "02-complete")?;

    let lib = std::fs::read_to_string(ws.project.join("src/lib.rs"))?;
    if !lib.contains("pub fn double") || !lib.contains("battle_double_works") {
        bail!("challenge A did not edit src/lib.rs as requested:\n{lib}");
    }
    let output = run_cargo_test(&ws.project)?;
    if !output.status.success() {
        bail!(
            "challenge A cargo test failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
#[ignore]
#[serial]
fn live_programming_challenge_debugs_failing_test() -> Result<()> {
    let Some(jekko) = require_live_battle_bin()? else {
        return Ok(());
    };
    let ws = prepare_live_workspace("rust-challenge-b", "# Rust challenge B\n")?;
    std::fs::write(
        ws.project.join("Cargo.toml"),
        "[package]\nname = \"battle_challenge_b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
    )?;
    std::fs::create_dir_all(ws.project.join("src"))?;
    let original = "pub fn normalize_name(input: &str) -> String {\n    input.to_string()\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn trims_and_lowercases_names() {\n        assert_eq!(normalize_name(\"  Ada LOVELACE  \"), \"ada lovelace\");\n    }\n}\n";
    std::fs::write(ws.project.join("src/lib.rs"), original)?;
    let before = run_cargo_test(&ws.project)?;
    if before.status.success() {
        bail!("challenge B fixture unexpectedly passed before the TUI interaction");
    }

    let dir = artifact_dir("challenge-b")?;
    let page = spawn_live_tui(&ws, &jekko, "battle-live-challenge-b")?;
    wait_for_boot_and_gateway(&page, &ws, &dir, "challenge-b")?;
    capture(&page, &dir, "01-boot")?;

    let prompt = "This temporary Rust library has a failing test. Inspect and edit src/lib.rs directly so `cargo test` passes. Keep the public function name `normalize_name`. Run `cargo test`. When complete, reply exactly with BATTLE_CHALLENGE_B_DONE.";
    submit_prompt(&page, prompt)?;
    page.wait_for_text("BATTLE_CHALLENGE_B_DONE", LIVE_TIMEOUT)
        .context("challenge B did not report completion")?;
    capture(&page, &dir, "02-complete")?;

    let lib = std::fs::read_to_string(ws.project.join("src/lib.rs"))?;
    if lib == original {
        bail!("challenge B left src/lib.rs unchanged");
    }
    let output = run_cargo_test(&ws.project)?;
    if !output.status.success() {
        bail!(
            "challenge B cargo test failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
