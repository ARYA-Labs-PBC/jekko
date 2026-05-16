use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serial_test::serial;

mod test_helpers;
use test_helpers::{
    copy_jekko_logs, ensure_artifact_dir, jekko_bin, prepare_workspace_no_config,
    spawn_jekko_with_size_env,
};

const FIRST_VISIBLE_TIMEOUT: Duration = Duration::from_secs(8);

#[test]
#[serial]
#[ignore = "Rust port does not yet render the no-keys setup screen; TS-era flow. \
            Phase A landed app directly on Route::Shell — Home flow no longer applies."]
fn no_keys_opens_setup_screen_and_creates_template() -> Result<()> {
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN to the jekko binary path");
        return Ok(());
    };

    let artifact_dir = ensure_artifact_dir()?.join("new-user");
    std::fs::create_dir_all(&artifact_dir)?;

    let (workspace, _project, _, _, _, _) =
        prepare_workspace_no_config("project", "# new-user-setup\n")?;
    let page = spawn_jekko_with_size_env(
        &workspace,
        &jekko,
        120,
        30,
        "no-keys",
        &[
            ("JEKKO_API_KEY", ""),
            ("JNOCCIO_DEVELOPER_KEY", ""),
            ("JEKKO_IGNORE_DEVELOPER_KEY", "1"),
        ],
    )?;

    let deadline = Instant::now() + FIRST_VISIBLE_TIMEOUT;
    loop {
        let screen = page.screen().plain_text();
        if screen.contains("No model keys found") && screen.contains("~/.jekko/jekko.env") {
            break;
        }
        if Instant::now() >= deadline {
            let _ = page.screenshot(artifact_dir.join("no-keys-timeout.png"));
            let _ = copy_jekko_logs(&workspace, "no-keys");
            bail!(
                "setup screen did not appear within {}ms",
                FIRST_VISIBLE_TIMEOUT.as_millis()
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    page.expect_screen()
        .to_contain_text("platform.openai.com/api-keys")?;
    page.expect_screen()
        .to_contain_text("console.anthropic.com/settings/keys")?;

    let template = workspace.path().join(".jekko").join("jekko.env");
    assert!(
        template.exists(),
        "expected canonical key file to be created"
    );
    let contents = std::fs::read_to_string(&template).context("read canonical key file")?;
    assert!(contents.contains("OPENAI_API_KEY="));
    assert!(contents.contains("JNOCCIO_DEVELOPER_KEY="));

    Ok(())
}

#[test]
#[serial]
#[ignore = "Rust port does not yet render the auto-routing setup hint; TS-era flow. \
            Phase A landed app directly on Route::Shell — auto-routing hint moved with engagement."]
fn one_key_reaches_prompt_and_shows_auto() -> Result<()> {
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN to the jekko binary path");
        return Ok(());
    };

    let artifact_dir = ensure_artifact_dir()?.join("new-user");
    std::fs::create_dir_all(&artifact_dir)?;

    let (workspace, _project, _, _, _, _) =
        prepare_workspace_no_config("project", "# one-key-setup\n")?;
    let page = spawn_jekko_with_size_env(
        &workspace,
        &jekko,
        120,
        30,
        "one-key",
        &[
            ("JEKKO_API_KEY", ""),
            (
                "JEKKO_MODEL_KEYS_CONTENT",
                "OPENAI_API_KEY=one-key-openai\n",
            ),
        ],
    )?;

    page.wait_for_text("Auto", FIRST_VISIBLE_TIMEOUT)
        .with_context(|| {
            let _ = page.screenshot(artifact_dir.join("one-key-timeout.png"));
            let _ = copy_jekko_logs(&workspace, "one-key");
            "prompt did not show Auto routing"
        })?;

    page.expect_screen()
        .not_to_contain_text("No model keys found")?;
    Ok(())
}
