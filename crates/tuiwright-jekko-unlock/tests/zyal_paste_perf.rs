//! Perf regression guard for ZYAL paste detection.
//!
//! Pastes a large real ZYAL example (~170 lines, ~600 highlighter tokens) and
//! asserts the `✓ ZYAL` indicator surfaces in well under one second. The
//! original implementation took multiple seconds because opentui's
//! ExtmarksController.updateHighlights rebuilds every highlight per
//! create()/delete() call — O(N² × text). This test fails loudly if that
//! quadratic behaviour creeps back.

use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use serial_test::serial;

mod test_helpers;
use test_helpers::{ensure_artifact_dir, jekko_bin, prepare_workspace, spawn_jekko, repo_root};

const RECOGNITION_BUDGET: Duration = Duration::from_millis(1500);

fn enabled() -> bool {
    std::env::var("JEKKO_TUIWRIGHT_PTY").as_deref() == Ok("1")
}

#[test]
#[serial]
fn large_zyal_paste_recognised_under_one_and_a_half_seconds() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set JEKKO_TUIWRIGHT_PTY=1 to run prompt PTY tests");
        return Ok(());
    }
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN to the jekko binary path");
        return Ok(());
    };

    let (parent, _project, _xdg_data, _xdg_cache, _xdg_config, _xdg_state) =
        prepare_workspace("project", "# zyal paste perf\n")?;
    let artifact_dir = ensure_artifact_dir()?;
    let page = spawn_jekko(&parent, &jekko)?;

    page.wait_for_text("ctrl+p commands", Duration::from_secs(30))
        .context("jekko prompt UI did not boot")?;

    let zyal = std::fs::read_to_string(repo_root().join("docs/ZYAL/examples/12-jankurai-min-loop.zyal"))
        .context("read 12-jankurai-min-loop example")?;
    if zyal.len() < 5000 {
        return Err(anyhow!(
            "perf test expects a sizeable example; got {} bytes",
            zyal.len()
        ));
    }

    let start = Instant::now();
    page.paste(&zyal).context("send bracketed paste")?;
    page.wait_for_text("✓ ZYAL", RECOGNITION_BUDGET)
        .with_context(|| {
            format!(
                "✓ ZYAL did not surface within {}ms — extmark batching likely regressed",
                RECOGNITION_BUDGET.as_millis()
            )
        })?;
    let elapsed = start.elapsed();
    page.screenshot(artifact_dir.join("zyal-perf-ok.png"))?;

    eprintln!(
        "ZYAL paste recognised in {}ms (budget {}ms)",
        elapsed.as_millis(),
        RECOGNITION_BUDGET.as_millis()
    );

    if elapsed > RECOGNITION_BUDGET {
        return Err(anyhow!(
            "ZYAL paste detection took {}ms (>{}ms budget)",
            elapsed.as_millis(),
            RECOGNITION_BUDGET.as_millis()
        ));
    }

    Ok(())
}
