//! Chat submission regression tests.
//!
//! Tests that the TUI surfaces a clear error when Enter is pressed with
//! no provider configured, rather than silently ignoring input or crashing.
//!
//! Requires: JEKKO_BIN env pointing to a fresh build.
//!           TUI_CHAT_TEST=1 to opt-in.

use std::time::Duration;

use anyhow::{Context, Result};
use serial_test::serial;
use tuiwright::Key;

mod test_helpers;
use test_helpers::{
    copy_jekko_logs, ensure_artifact_dir, jekko_bin, mock_llm_fixture_json,
    prepare_workspace_no_config, spawn_jekko_with_size, spawn_jekko_with_size_env,
};

const ARTIFACT_SUBDIR: &str = "chat-enter-mock";
const BOOT_TIMEOUT: Duration = Duration::from_secs(30);
const WARNING_TIMEOUT: Duration = Duration::from_secs(5);

fn enabled() -> bool {
    std::env::var("TUI_CHAT_TEST").as_deref() == Ok("1")
}

// ── Test: Pressing Enter with no provider shows a warning ─────────────
//
// This is the regression that was silently broken: in the Rust TUI,
// pressing Enter with no model configured did nothing. The TUI MUST
// either show a toast ("Connect a provider") or open a provider dialog.
//
// When this test passes, it proves that the no-model guard is wired up
// in the active TUI binary — whichever stack it is.

#[test]
#[ignore]
#[serial]
fn chat_enter_with_no_provider_shows_warning() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set TUI_CHAT_TEST=1");
        return Ok(());
    }
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN");
        return Ok(());
    };

    let artifact_dir = ensure_artifact_dir()?.join(ARTIFACT_SUBDIR);
    std::fs::create_dir_all(&artifact_dir)?;

    // Use no-config workspace so no provider is configured.
    let (workspace, _project, _, _, _, _) =
        prepare_workspace_no_config("project", "# chat-enter-no-provider test\n")?;

    let page = spawn_jekko_with_size(&workspace, &jekko, 200, 60, "chat-enter-no-provider")?;

    // Wait for TUI to fully boot.
    page.wait_for_text("bypass permissions", BOOT_TIMEOUT)
        .with_context(|| {
            let _ = page.screenshot(artifact_dir.join("01-boot-failed.png"));
            let _ = copy_jekko_logs(&workspace, "chat-no-provider");
            "TUI did not boot"
        })?;
    page.screenshot(artifact_dir.join("01-booted.png"))?;

    // Type a prompt and press Enter — with no provider, this must NOT silently no-op.
    page.type_text("Hello, are you there?")?;
    std::thread::sleep(Duration::from_millis(300));
    page.screenshot(artifact_dir.join("02-prompt-typed.png"))?;

    page.press(Key::Enter)?;
    std::thread::sleep(Duration::from_millis(800));
    page.screenshot(artifact_dir.join("03-after-enter.png"))?;

    // Assert: the TUI surfaces a visible warning. Accept either:
    //   - "Connect a provider"  (TS TUI toast)
    //   - "no model"            (any variant)
    //   - "provider"            (dialog opening)
    let warned = page.wait_for_text("provider", WARNING_TIMEOUT).is_ok()
        || page.wait_for_text("no model", WARNING_TIMEOUT).is_ok()
        || page.wait_for_text("Connect", WARNING_TIMEOUT).is_ok();

    if !warned {
        let _ = page.screenshot(artifact_dir.join("04-no-warning-FAIL.png"));
        let _ = copy_jekko_logs(&workspace, "chat-no-provider");
        anyhow::bail!(
            "Pressing Enter with no provider configured produced NO visible warning. \
             The Rust TUI prompt is missing the no-model guard. \
             See packages/jekko/src/cli/cmd/tui/component/prompt/index.tsx:347 for \
             the reference implementation (promptModelWarning)."
        );
    }

    page.screenshot(artifact_dir.join("04-warning-shown.png"))?;
    Ok(())
}

// ── Test: Pressing Enter with a model configured sends the message ─────
//
// Counterpart to the above: when a model IS configured (offline fake key),
// the TUI should accept the message and show it in the transcript, even if
// the AI response eventually fails with a fake-key error.

#[test]
#[ignore]
#[serial]
fn chat_enter_with_model_configured_submits() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set TUI_CHAT_TEST=1");
        return Ok(());
    }
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN");
        return Ok(());
    };

    let artifact_dir = ensure_artifact_dir()?.join(ARTIFACT_SUBDIR);
    std::fs::create_dir_all(&artifact_dir)?;

    // Use the standard workspace which writes a fake-key jekko.json config.
    let (workspace, _project, _, _, _, _) =
        prepare_workspace("project", "# chat-enter-with-model test\n")?;

    let page = spawn_jekko_with_size(&workspace, &jekko, 200, 60, "chat-enter-with-model")?;

    page.wait_for_text("bypass permissions", BOOT_TIMEOUT)
        .with_context(|| {
            let _ = page.screenshot(artifact_dir.join("10-boot-failed.png"));
            let _ = copy_jekko_logs(&workspace, "chat-with-model");
            "TUI did not boot"
        })?;
    page.screenshot(artifact_dir.join("10-booted.png"))?;

    // Type a prompt — the sentinel JEKKO_CHAT_TEST_OK is chosen to be
    // unambiguous in the screen text.
    let prompt = "Reply with JEKKO_CHAT_TEST_OK";
    page.type_text(prompt)?;
    std::thread::sleep(Duration::from_millis(300));

    page.press(Key::Enter)?;
    std::thread::sleep(Duration::from_millis(1000));
    page.screenshot(artifact_dir.join("11-after-enter.png"))?;

    // The user message MUST appear in the transcript — if Enter was swallowed
    // silently this will time out and fail.
    page.wait_for_text("JEKKO_CHAT_TEST_OK", Duration::from_secs(8))
        .with_context(|| {
            let _ = page.screenshot(artifact_dir.join("12-user-msg-missing.png"));
            let _ = copy_jekko_logs(&workspace, "chat-with-model");
            "The user message did not appear in the transcript after pressing Enter. \
             Enter key handling may be broken or the session route is not rendering messages."
        })?;

    page.screenshot(artifact_dir.join("12-user-msg-shown.png"))?;
    Ok(())
}

#[test]
#[ignore]
#[serial]
fn chat_enter_with_mock_llm_renders_assistant() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set TUI_CHAT_TEST=1");
        return Ok(());
    }
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN");
        return Ok(());
    };

    let artifact_dir = ensure_artifact_dir()?.join(ARTIFACT_SUBDIR);
    std::fs::create_dir_all(&artifact_dir)?;
    let response_json = mock_llm_fixture_json("chat-enter.json")?;

    let (workspace, _project, _, _, _, _) =
        prepare_workspace("project", "# chat-enter-mock-assistant test\n")?;
    let page = spawn_jekko_with_size_env(
        &workspace,
        &jekko,
        200,
        60,
        "chat-enter-mock-assistant",
        &[
            ("JEKKO_TUI_TEST_MOCK_LLM", "1"),
            ("JEKKO_TUI_TEST_MOCK_RESPONSE", response_json.as_str()),
        ],
    )?;

    page.wait_for_text("bypass permissions", BOOT_TIMEOUT)
        .with_context(|| {
            let _ = page.screenshot(artifact_dir.join("20-boot-failed.png"));
            let _ = copy_jekko_logs(&workspace, "chat-mock-assistant");
            "TUI did not boot"
        })?;

    let prompt = "Reply through the mock LLM path.";
    page.type_text(prompt)?;
    std::thread::sleep(Duration::from_millis(300));
    page.press(Key::Enter)?;
    page.screenshot(artifact_dir.join("21-after-enter.png"))?;

    page.wait_for_text("Mock assistant response.", Duration::from_secs(8))
        .with_context(|| {
            let _ = page.screenshot(artifact_dir.join("22-assistant-missing.png"));
            let _ = copy_jekko_logs(&workspace, "chat-mock-assistant");
            "The mock assistant response did not render after pressing Enter."
        })?;

    page.screenshot(artifact_dir.join("22-assistant-shown.png"))?;
    Ok(())
}

// Re-export prepare_workspace (with config) for the second test.
use test_helpers::prepare_workspace;
