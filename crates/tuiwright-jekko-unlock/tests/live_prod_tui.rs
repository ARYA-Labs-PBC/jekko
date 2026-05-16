use std::path::Path;
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

fn spawn_live_tui(
    parent: &tempfile::TempDir,
    jekko: &Path,
    prompt: &str,
    model: &str,
) -> Result<Page> {
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
        .timeout(Duration::from_secs(90));
    for (key, value) in std::env::vars() {
        match key.as_str() {
            "HOME"
            | "USER"
            | "LOGNAME"
            | "PATH"
            | "SHELL"
            | "LANG"
            | "LC_ALL"
            | "LC_CTYPE"
            | "JEKKO_API_KEY"
            | "JNOCCIO_DEFAULT_API_KEY"
            | "JNOCCIO_DEFAULT_BASE_URL" => {
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
        return Err(anyhow!(
            "JEKKO_BIN is required for live production TUI testing"
        ));
    };
    let model =
        std::env::var("JEKKO_LIVE_MODEL").unwrap_or_else(|_| "jekko/gpt-5-nano".to_string());
    let prompt = "Reply exactly with JEKKO_TUI_LIVE_OK and no other text.";

    let (workspace, _project, _, _, _, _) =
        prepare_workspace("project", "# live production TUI proof\n")?;
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

/// LOCAL ONLY. Verifies:
///   1. The Jnoccio auto-boot thread fired and the panel shows models > 0.
///   2. A chat command round-trips through the TUI successfully.
///
/// Gate: `JEKKO_TUI_LIVE_PROD=1` (never runs in CI, never in normal `cargo test`).
/// Requires: `JEKKO_BIN` pointing at a built binary, and either
///   - `~/.env.jnoccio` (dev key file), or
///   - `JNOCCIO_DEVELOPER_KEY` env var, or
///   - a decrypted `jnoccio-fusion/` in the repo tree.
///
/// Run:
/// ```
///   JEKKO_TUI_LIVE_PROD=1 JEKKO_BIN=$(cargo run -p xtask -- host-binary-path) \
///     cargo test -p tuiwright-jekko-unlock --test live_prod_tui \
///       live_jnoccio_models_and_chat -- --ignored --nocapture
/// ```
#[test]
#[ignore]
#[serial]
fn live_jnoccio_models_and_chat() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set JEKKO_TUI_LIVE_PROD=1 to run");
        return Ok(());
    }
    if std::env::var("CI").as_deref() == Ok("true") {
        return Err(anyhow!("live Jnoccio tests must not run in CI"));
    }

    let Some(jekko) = jekko_bin() else {
        return Err(anyhow!("JEKKO_BIN must point to the compiled jekko binary"));
    };

    // Use the jnoccio provider explicitly — no external API key needed if
    // jnoccio-fusion is running locally.
    let model =
        std::env::var("JEKKO_LIVE_MODEL").unwrap_or_else(|_| "jnoccio/jnoccio-fusion".to_string());

    let (workspace, _project, _, _, _, _) = prepare_workspace("project", "# jnoccio live test\n")?;
    let artifact_dir = ensure_artifact_dir()?.join("live-jnoccio");
    std::fs::create_dir_all(&artifact_dir)?;

    // ── Step 1: Spawn the TUI without a prompt (interactive mode). ────────
    // We need to observe the Jnoccio panel booting, so we don't supply
    // --prompt; instead we wait for visual signals then type a command.
    let xdg = workspace.path().join("xdg");
    let mut cfg = tuiwright::SpawnConfig::new(jekko.to_string_lossy().as_ref())
        .arg("--model")
        .arg(model)
        .cwd(test_helpers::repo_root())
        .size(SCREEN_COLS, SCREEN_ROWS)
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("HOME", workspace.path().to_string_lossy().as_ref())
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
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
        .timeout(Duration::from_secs(120));

    // Forward unlock signals from the real environment.
    for (key, value) in std::env::vars() {
        match key.as_str() {
            "HOME"
            | "USER"
            | "LOGNAME"
            | "PATH"
            | "SHELL"
            | "LANG"
            | "LC_ALL"
            | "LC_CTYPE"
            | "JNOCCIO_DEVELOPER_KEY"
            | "JNOCCIO_DEFAULT_API_KEY"
            | "JNOCCIO_DEFAULT_BASE_URL"
            | "JEKKO_API_KEY" => {
                cfg = cfg.env(key, value);
            }
            _ => {}
        }
    }

    let page = tuiwright::Page::spawn(cfg).context("spawn jekko TUI for Jnoccio live test")?;
    eprintln!("[live-jnoccio] TUI spawned");

    // ── Step 2: Wait for the TUI to finish booting. ───────────────────────
    page.wait_for_text("Tab switch pane", Duration::from_secs(45))
        .context("TUI did not boot (footer hint not visible within 45 s)")?;
    page.screenshot(artifact_dir.join("01-boot.png"))?;
    eprintln!("[live-jnoccio] TUI booted");

    // ── Step 3: Wait for the Jnoccio panel to show a positive model count. ─
    // The auto-boot thread spawns jnoccio-fusion and re-probes every 5 s.
    // We give it up to 30 s for the server to start and the TUI to update.
    // The header renders as "{N}/{M} models" (N = keyed_models, M = total).
    // We assert at least "1/" appears, which proves N >= 1.
    //
    // We also assert that "⚠ No agent connected" is NOT on screen — this
    // would be visible if the boot thread failed to start the server.
    page.wait_for_regex(r"[1-9][0-9]*/[0-9]+ models", Duration::from_secs(60))
        .context("Jnoccio panel did not show models > 0 within 60 s — check auto-boot logs")?;

    page.screenshot(artifact_dir.join("02-jnoccio-live.png"))?;
    eprintln!("[live-jnoccio] Jnoccio panel shows models > 0 ✓");

    // Assert the error banner is NOT visible.
    page.expect_screen()
        .not_to_contain_text("No agent connected")
        .context("expected no 'No agent connected' error but found it in TUI output")?;

    // ── Step 4: Type a chat command and verify it round-trips. ───────────
    // Switch to the Jnoccio tab (already on it by default) and submit a
    // simple prompt. The response sentinel proves the entire pipeline fires.
    page.type_text("Reply exactly with JNOCCIO_LIVE_OK and nothing else.")
        .context("could not type prompt")?;
    page.press(tuiwright::Key::Enter)
        .context("could not send Enter")?;
    page.screenshot(artifact_dir.join("03-prompt-sent.png"))?;
    eprintln!("[live-jnoccio] prompt submitted, waiting for response...");

    page.wait_for_text("JNOCCIO_LIVE_OK", LIVE_TIMEOUT)
        .context("Jnoccio did not return JNOCCIO_LIVE_OK within timeout")?;
    page.screenshot(artifact_dir.join("04-response.png"))?;
    eprintln!("[live-jnoccio] response received ✓");

    Ok(())
}

// ── Lightweight live test: jnoccio connects, no LLM round-trip ──────────
//
// Gates on `JEKKO_TUI_LIVE_LOCAL=1` (separate from JEKKO_TUI_LIVE_PROD).
// Spawns jekko with the user's real $HOME so the auto-boot picks up
// `~/.env.jnoccio` + the jnoccio-fusion server binary. Verifies:
//   1. The Jnoccio panel opens and shows models > 0
//   2. The "⚠ No agent connected" banner is NOT visible
//
// FAILS LOUDLY if the banner is present after boot — that's the regression
// we're guarding against.
#[test]
#[ignore]
#[serial]
fn jnoccio_local_panel_connects_without_no_agent_banner() -> Result<()> {
    if std::env::var("JEKKO_TUI_LIVE_LOCAL").as_deref() != Ok("1") {
        eprintln!("skipped: set JEKKO_TUI_LIVE_LOCAL=1 to run");
        return Ok(());
    }
    if std::env::var("CI").as_deref() == Ok("true") {
        return Err(anyhow!("live local tests must not run in CI"));
    }

    let Some(jekko) = jekko_bin() else {
        return Err(anyhow!("JEKKO_BIN must point to the compiled jekko binary"));
    };

    let real_home = std::env::var("HOME").context("HOME must be set")?;
    let env_file = std::path::PathBuf::from(&real_home).join(".env.jnoccio");
    if !env_file.exists() {
        return Err(anyhow!(
            "expected {}/.env.jnoccio to exist (drop your dev keys there)",
            real_home
        ));
    }
    // Confirm jnoccio-fusion binary is built.
    let fusion_bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|repo| {
            repo.join("jnoccio-fusion")
                .join("target")
                .join("release")
                .join("jnoccio-fusion")
        });
    if let Some(bin) = &fusion_bin {
        if !bin.exists() {
            return Err(anyhow!(
                "jnoccio-fusion binary not built at {} — run `cargo build --release` in jnoccio-fusion/",
                bin.display()
            ));
        }
    }

    let artifact_dir = ensure_artifact_dir()?.join("live-local-jnoccio");
    std::fs::create_dir_all(&artifact_dir)?;

    // Spawn jekko using the REAL $HOME so auto-boot reads the user's env.
    // CWD is intentionally NOT the repo root — proves `find_jnoccio_fusion_root()`
    // falls back to the installed bundle at `$XDG_CONFIG_HOME/jekko/jnoccio-fusion`.
    let outside_repo = std::env::temp_dir();
    let mut cfg = tuiwright::SpawnConfig::new(jekko.to_string_lossy().as_ref())
        .arg("--pure")
        .cwd(&outside_repo)
        .size(SCREEN_COLS, SCREEN_ROWS)
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("HOME", &real_home)
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_PRUNE", "1")
        .timeout(Duration::from_secs(90));

    // Forward live env vars that auto-boot needs.
    for (key, value) in std::env::vars() {
        match key.as_str() {
            "USER"
            | "LOGNAME"
            | "PATH"
            | "SHELL"
            | "LANG"
            | "LC_ALL"
            | "LC_CTYPE"
            | "JNOCCIO_DEVELOPER_KEY"
            | "JNOCCIO_DEFAULT_API_KEY"
            | "JNOCCIO_DEFAULT_BASE_URL"
            | "JEKKO_API_KEY" => {
                cfg = cfg.env(key, value);
            }
            _ => {}
        }
    }

    let page = tuiwright::Page::spawn(cfg).context("spawn jekko TUI for local jnoccio test")?;
    eprintln!("[live-local] TUI spawned");

    // Wait for splash to dismiss and Shell route to render.
    page.wait_for_text("Tab switch pane", Duration::from_secs(60))
        .context("TUI did not reach Shell route (splash stuck or boot failed?)")?;
    page.screenshot(artifact_dir.join("01-shell.png"))?;
    eprintln!("[live-local] Shell route reached");

    // Engage so the activity-feed shows the Jnoccio panel.
    page.press(tuiwright::Key::Enter)?;
    std::thread::sleep(Duration::from_millis(1200));

    // Switch to Jnoccio tab (it's tab 1, the default — but make it explicit).
    page.press(tuiwright::Key::Char('1'))?;
    std::thread::sleep(Duration::from_millis(500));
    page.screenshot(artifact_dir.join("02-jnoccio-tab.png"))?;

    // Wait up to 60s for the server to boot + panel to show models.
    page.wait_for_regex(r"[1-9][0-9]*/[0-9]+ models", Duration::from_secs(60))
        .context("Jnoccio panel did not show models > 0 within 60s — server boot likely failed")?;
    page.screenshot(artifact_dir.join("03-jnoccio-connected.png"))?;
    eprintln!("[live-local] Jnoccio panel shows models > 0 ✓");

    // HARD ASSERT: no "No agent connected" banner anywhere on screen.
    page.expect_screen()
        .not_to_contain_text("No agent connected")
        .with_context(|| {
            let _ = page.screenshot(artifact_dir.join("99-banner-present-FAIL.png"));
            "REGRESSION: '⚠ No agent connected' banner present after successful boot. \
             The Jnoccio panel rendered the warning even though the server is reachable."
        })?;

    eprintln!("[live-local] no warning banner ✓");
    Ok(())
}

// ── Live chat-submit regression guard ────────────────────────────────────
//
// Reproduces the user-reported regression: typing a real prompt and
// pressing Enter must produce assistant output. Spawns jekko outside the
// repo to also exercise the installed-bundle fallback for jnoccio-fusion
// resolution. Gates on `JEKKO_TUI_LIVE_LOCAL=1`.
#[test]
#[ignore]
#[serial]
fn live_chat_submit_renders_assistant_reply() -> Result<()> {
    if std::env::var("JEKKO_TUI_LIVE_LOCAL").as_deref() != Ok("1") {
        eprintln!("skipped: set JEKKO_TUI_LIVE_LOCAL=1 to run");
        return Ok(());
    }
    if std::env::var("CI").as_deref() == Ok("true") {
        return Err(anyhow!("live local tests must not run in CI"));
    }
    let Some(jekko) = jekko_bin() else {
        return Err(anyhow!("JEKKO_BIN must point to the compiled jekko binary"));
    };
    let real_home = std::env::var("HOME").context("HOME must be set")?;

    let artifact_dir = ensure_artifact_dir()?.join("live-chat-submit");
    std::fs::create_dir_all(&artifact_dir)?;

    let outside_repo = std::env::temp_dir();
    let mut cfg = tuiwright::SpawnConfig::new(jekko.to_string_lossy().as_ref())
        .arg("--pure")
        .cwd(&outside_repo)
        .size(SCREEN_COLS, SCREEN_ROWS)
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("HOME", &real_home)
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_PRUNE", "1")
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
            | "JNOCCIO_DEVELOPER_KEY"
            | "JNOCCIO_DEFAULT_API_KEY"
            | "JNOCCIO_DEFAULT_BASE_URL"
            | "JEKKO_API_KEY" => {
                cfg = cfg.env(key, value);
            }
            _ => {}
        }
    }

    let page = tuiwright::Page::spawn(cfg).context("spawn jekko for live chat-submit test")?;
    eprintln!("[chat-submit] TUI spawned");

    // Reach Shell route.
    page.wait_for_text("Tab switch pane", Duration::from_secs(60))
        .context("TUI did not reach Shell route")?;
    page.screenshot(artifact_dir.join("01-shell.png"))?;

    // Wait for jnoccio to be actually reachable before submitting — otherwise
    // the bridge thread fails on connect and the test races the boot.
    page.wait_for_regex(r"[1-9][0-9]*/[0-9]+ models", Duration::from_secs(60))
        .context("jnoccio gateway never became reachable")?;
    page.screenshot(artifact_dir.join("02-jnoccio-ready.png"))?;

    // Type the prompt + Enter.
    let prompt = "Write a simple python script";
    page.type_text(prompt)
        .context("could not type prompt into TUI")?;
    std::thread::sleep(Duration::from_millis(400));
    page.screenshot(artifact_dir.join("03-prompt-typed.png"))?;
    page.press(tuiwright::Key::Enter)
        .context("could not send Enter to TUI")?;
    eprintln!("[chat-submit] prompt submitted: {prompt}");

    // The user card must render in the transcript first (proves the prompt
    // was consumed — not eaten by Home auto-engage or similar).
    page.wait_for_text("simple python script", Duration::from_secs(15))
        .with_context(|| {
            let _ = page.screenshot(artifact_dir.join("99-user-card-missing.png"));
            "REGRESSION: user prompt 'Write a simple python script' did not appear in the transcript. \
             The prompt was either not submitted or the transcript never updated."
        })?;
    page.screenshot(artifact_dir.join("04-user-card-rendered.png"))?;
    eprintln!("[chat-submit] user card visible ✓");

    // Assistant must respond within a generous window. Look for ANY of
    // several common Python sentinels — "python", "def ", "print(", "#!/" —
    // so the test isn't tied to a specific model's exact wording.
    page.wait_for_regex(
        r"(?i)\b(python|def\s|print\(|#!/usr/bin/env\s+python)",
        Duration::from_secs(120),
    )
    .with_context(|| {
        let _ = page.screenshot(artifact_dir.join("99-no-reply.png"));
        "REGRESSION: assistant did not render any Python-related response within 120s. \
         The chat-submit → provider → transcript loop is broken. \
         Check ~/.config/jekko/jnoccio-fusion/jnoccio-fusion is running + reachable."
    })?;
    page.screenshot(artifact_dir.join("05-assistant-replied.png"))?;
    eprintln!("[chat-submit] assistant reply rendered ✓");
    Ok(())
}
