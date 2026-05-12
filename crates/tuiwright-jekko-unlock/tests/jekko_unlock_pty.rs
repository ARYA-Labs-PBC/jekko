//! Real-PTY end-to-end proof that the Jekko TUI unlock flow works.
//!
//! Spawns the installed `/opt/homebrew/bin/jekko` (or `JEKKO_BIN`) inside a
//! pseudo-terminal via tuiwright, drives the user-visible flow:
//!
//!   1. open the model picker  (`/models` + Enter)
//!   2. select the locked Jnoccio Fusion entry
//!   3. paste the 128-character secret from `~/jnoccio-fusion.unlock`
//!   4. press Enter
//!   5. assert the success toast appears.
//!
//! Skipped unless `JNOCCIO_TUIWRIGHT_E2E=1` is set and the secret + binary
//! exist locally, so CI without git-crypt access just no-ops.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use tempfile::TempDir;
use tuiwright::{Key, Page, SpawnConfig};

const SECRET_LEN: usize = 128;
const SCREEN_COLS: u16 = 160;
const SCREEN_ROWS: u16 = 48;
const ARTIFACT_DIR: &str = "target/tuiwright-jekko";

fn enabled() -> bool {
    std::env::var("JNOCCIO_TUIWRIGHT_E2E").as_deref() == Ok("1")
}

fn jekko_bin() -> Option<PathBuf> {
    std::env::var("JEKKO_BIN").ok().map(PathBuf::from)
}

fn secret_path() -> PathBuf {
    if let Ok(p) = std::env::var("JNOCCIO_UNLOCK_SECRET_PATH") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").expect("HOME must be set");
    PathBuf::from(home).join("jnoccio-fusion.unlock")
}

fn read_secret() -> Result<String> {
    let raw = std::fs::read_to_string(secret_path())
        .with_context(|| format!("read secret from {:?}", secret_path()))?;
    let trimmed = raw.trim();
    if trimmed.len() != SECRET_LEN
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(anyhow!(
            "secret at {:?} is not exactly {} chars [A-Za-z0-9_-]",
            secret_path(),
            SECRET_LEN
        ));
    }
    Ok(trimmed.to_string())
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root above crates/<name>")
        .to_path_buf()
}

fn clone_repo(target: &std::path::Path) -> Result<()> {
    let status = Command::new("git")
        .args(["clone", "--quiet"])
        .arg(repo_root())
        .arg(target)
        .status()
        .context("spawn git clone")?;
    if !status.success() {
        return Err(anyhow!("git clone failed: exit {}", status));
    }
    Ok(())
}

fn ensure_artifact_dir() -> Result<PathBuf> {
    let dir = repo_root().join(ARTIFACT_DIR);
    std::fs::create_dir_all(&dir).with_context(|| format!("create {dir:?}"))?;
    Ok(dir)
}

#[test]
fn jekko_tui_paste_unlocks_jnoccio_fusion() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set JNOCCIO_TUIWRIGHT_E2E=1 to run");
        return Ok(());
    }
    let Some(jekko_bin) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN to the jekko binary path");
        return Ok(());
    };
    if !jekko_bin.exists() {
        eprintln!("skipped: jekko binary {:?} missing", jekko_bin);
        return Ok(());
    }
    let secret = match read_secret() {
        Ok(s) => s,
        Err(err) => {
            eprintln!("skipped: {err:#}");
            return Ok(());
        }
    };

    let parent = TempDir::new().context("tempdir")?;
    let clone = parent.path().join("repo");
    clone_repo(&clone)?;

    let xdg = parent.path().join("xdg");
    let secret_cache = parent.path().join("tui-paste.unlock");
    std::fs::create_dir_all(xdg.join("data"))?;
    std::fs::create_dir_all(xdg.join("cache"))?;
    std::fs::create_dir_all(xdg.join("config"))?;
    std::fs::create_dir_all(xdg.join("state"))?;

    let artifact_dir = ensure_artifact_dir()?;

    let mut cfg = SpawnConfig::new(jekko_bin.to_string_lossy().as_ref())
        .arg("--pure")
        .arg(clone.to_string_lossy().as_ref())
        .cwd(&clone)
        .size(SCREEN_COLS, SCREEN_ROWS)
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
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
        .env("JNOCCIO_REPO_ROOT", clone.to_string_lossy().as_ref())
        .env(
            "JNOCCIO_UNLOCK_SECRET_PATH",
            secret_cache.to_string_lossy().as_ref(),
        )
        .timeout(Duration::from_secs(60));
    for (k, v) in std::env::vars() {
        match k.as_str() {
            "HOME" | "USER" | "LOGNAME" | "PATH" | "SHELL" | "LANG" | "LC_ALL" | "LC_CTYPE" => {
                cfg = cfg.env(k, v);
            }
            _ => {}
        }
    }

    let page = Page::spawn(cfg).context("spawn jekko in pty")?;

    page.wait_for_text("ctrl+p commands", Duration::from_secs(30))
        .context("wait for jekko prompt UI")?;
    page.screenshot(artifact_dir.join("01-boot.png"))?;

    page.press(Key::Ctrl('p'))?;
    page.wait_for_text("Switch model", Duration::from_secs(15))
        .context("command palette did not open")?;
    page.type_text("Switch model")?;
    page.press(Key::Enter)?;

    page.wait_for_text("Jnoccio (recommended)", Duration::from_secs(15))
        .context("provider picker did not list Jnoccio")?;
    page.screenshot(artifact_dir.join("02-provider.png"))?;
    page.press(Key::Enter)?;

    page.wait_for_text("Jnoccio Fusion", Duration::from_secs(15))
        .context("model picker did not show Jnoccio Fusion")?;
    page.screenshot(artifact_dir.join("03-model.png"))?;
    page.press(Key::Enter)?;
    page.wait_for_text(
        "Paste your 128-character unlock secret",
        Duration::from_secs(15),
    )
    .context("unlock dialog did not open")?;
    page.screenshot(artifact_dir.join("04-dialog.png"))?;

    page.paste(&secret).context("send bracketed paste")?;
    page.wait_for_text("128/128", Duration::from_secs(10))
        .context("paste did not register 128/128 in mask")?;
    page.screenshot(artifact_dir.join("05-pasted.png"))?;

    page.press(Key::Enter)?;
    page.wait_for_text("Jnoccio Fusion unlocked", Duration::from_secs(60))
        .context("unlock toast did not appear")?;
    page.screenshot(artifact_dir.join("06-unlocked.png"))?;

    let cargo_toml = clone.join("jnoccio-fusion").join("Cargo.toml");
    let head = std::fs::read(&cargo_toml).with_context(|| format!("read {cargo_toml:?}"))?;
    assert!(
        head.starts_with(b"[package]"),
        "jnoccio-fusion/Cargo.toml not plaintext after unlock; first bytes: {:?}",
        &head[..head.len().min(16)]
    );

    let cached = std::fs::read_to_string(&secret_cache).context("read secret cache")?;
    assert_eq!(cached.trim(), secret, "cached secret mismatch");

    Ok(())
}
