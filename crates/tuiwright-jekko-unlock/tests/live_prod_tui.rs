use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serial_test::serial;
use tuiwright::{Page, SpawnConfig};

mod test_helpers;
use test_helpers::{ensure_artifact_dir, jekko_bin, prepare_workspace};

const SCREEN_COLS: u16 = 200;
const SCREEN_ROWS: u16 = 60;
const LIVE_TIMEOUT: Duration = Duration::from_secs(180);

fn enabled() -> bool {
    std::env::var("JEKKO_TUI_LIVE_PROD").as_deref() == Ok("1")
}

fn require_env(key: &str) -> Result<String> {
    let value = std::env::var(key).unwrap_or_default();
    if value.trim().is_empty() {
        return Err(anyhow!("{key} is required for live production TUI testing"));
    }
    Ok(value)
}

fn spawn_live_tui(parent: &tempfile::TempDir, jekko: &PathBuf, prompt: &str, model: &str) -> Result<Page> {
    let project = parent.path().join("project");
    let xdg = parent.path().join("xdg");
    let mut cfg = SpawnConfig::new(jekko.to_string_lossy().as_ref())
        .arg("--pure")
        .arg("--model")
        .arg(model)
        .arg("--prompt")
        .arg(prompt)
        .arg(project.to_string_lossy().as_ref())
        .cwd(&project)
        .size(SCREEN_COLS, SCREEN_ROWS)
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("HOME", parent.path().to_string_lossy().as_ref())
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_LSP_DOWNLOAD", "1")
        .env("JEKKO_DISABLE_PRUNE", "1")
        .env("XDG_DATA_HOME", xdg.join("data").to_string_lossy().as_ref())
        .env("XDG_CACHE_HOME", xdg.join("cache").to_string_lossy().as_ref())
        .env("XDG_CONFIG_HOME", xdg.join("config").to_string_lossy().as_ref())
        .env("XDG_STATE_HOME", xdg.join("state").to_string_lossy().as_ref())
        .timeout(Duration::from_secs(90));
    for (key, value) in std::env::vars() {
        match key.as_str() {
            "HOME" | "USER" | "LOGNAME" | "PATH" | "SHELL" | "LANG" | "LC_ALL" | "LC_CTYPE"
            | "JEKKO_API_KEY" | "JNOCCIO_DEFAULT_API_KEY" | "JNOCCIO_DEFAULT_BASE_URL" => {
                cfg = cfg.env(key, value);
            }
            _ => {}
        }
    }
    Page::spawn(cfg).context("spawn live production Jekko TUI")
}

#[test]
#[ignore]
#[serial]
fn live_jekko_prompt_round_trips_through_tui() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set JEKKO_TUI_LIVE_PROD=1");
        return Ok(());
    }
    if std::env::var("CI").as_deref() == Ok("true") {
        return Err(anyhow!("live production TUI tests must not run in CI"));
    }
    require_env("JEKKO_API_KEY")?;
    let Some(jekko) = jekko_bin() else {
        return Err(anyhow!("JEKKO_BIN is required for live production TUI testing"));
    };
    let model = std::env::var("JEKKO_LIVE_MODEL").unwrap_or_else(|_| "jekko/gpt-5-nano".to_string());
    let prompt = "Reply exactly with JEKKO_TUI_LIVE_OK and no other text.";

    let (workspace, _project, _, _, _, _) = prepare_workspace("project", "# live production TUI proof\n")?;
    let artifact_dir = ensure_artifact_dir()?.join("live-prod");
    std::fs::create_dir_all(&artifact_dir)?;
    let page = spawn_live_tui(&workspace, &jekko, prompt, &model)?;

    page.wait_for_text("ctrl+p commands", Duration::from_secs(45))
        .context("live TUI did not boot")?;
    page.screenshot(artifact_dir.join("01-live-boot.png"))?;

    page.wait_for_text("JEKKO_TUI_LIVE_OK", LIVE_TIMEOUT)
        .context("live model response did not contain JEKKO_TUI_LIVE_OK")?;
    page.screenshot(artifact_dir.join("02-live-response.png"))?;

    Ok(())
}
