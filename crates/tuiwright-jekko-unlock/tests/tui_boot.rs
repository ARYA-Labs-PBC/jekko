use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serial_test::serial;

mod test_helpers;
use test_helpers::{
    copy_jekko_logs, ensure_artifact_dir, jekko_bin, prepare_workspace, spawn_jekko_with_size,
};

const FIRST_VISIBLE_TIMEOUT: Duration = Duration::from_secs(5);
const HOME_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
#[serial]
fn default_tui_paints_first_frame() -> Result<()> {
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN to the jekko binary path");
        return Ok(());
    };

    let artifact_dir = ensure_artifact_dir()?.join("boot");
    std::fs::create_dir_all(&artifact_dir)?;

    for (cols, rows) in [(80, 24), (120, 30), (200, 60)] {
        let case = format!("default-tui-{cols}x{rows}");
        let (workspace, _project, _, _, _, _) =
            prepare_workspace("project", &format!("# {case}\n"))?;
        let page = spawn_jekko_with_size(&workspace, &jekko, cols, rows, &case)?;

        let deadline = Instant::now() + FIRST_VISIBLE_TIMEOUT;
        loop {
            let screen = page.screen().plain_text();
            if !screen.trim().is_empty() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = page.screenshot(artifact_dir.join(format!("{case}-blank.png")));
                let _ = copy_jekko_logs(&workspace, &case);
                bail!(
                    "{case} stayed blank for {}ms",
                    FIRST_VISIBLE_TIMEOUT.as_millis()
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        page.wait_for_text("JEKKO", HOME_TIMEOUT)
            .with_context(|| {
                let _ = page.screenshot(artifact_dir.join(format!("{case}-home-timeout.png")));
                let _ = copy_jekko_logs(&workspace, &case);
                format!("{case} did not paint the home sentinel")
            })?;

        page.screenshot(artifact_dir.join(format!("{case}-home.png")))?;
    }

    Ok(())
}
