use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serial_test::serial;
use tuiwright::{Page, SpawnConfig};

mod test_helpers;
use test_helpers::{copy_jekko_logs, ensure_artifact_dir, jekko_bin, prepare_workspace};

const FIRST_VISIBLE_TIMEOUT: Duration = Duration::from_secs(5);
// Phase B widened MAX_HOLD on the splash to 10 s, so the post-splash sentinel
// needs more headroom than the prior 10 s budget — bump to 20 s.
const HOME_TIMEOUT: Duration = Duration::from_secs(20);
const FAKE_DEVELOPER_KEY: &str = "fake";

fn spawn_jekko_without_pure(
    parent: &tempfile::TempDir,
    jekko: &std::path::Path,
    cols: u16,
    rows: u16,
    trace_name: &str,
    extra_envs: &[(&str, &str)],
) -> Result<Page> {
    let project = parent.path().join("project");
    let xdg = parent.path().join("xdg");
    let artifact_dir = ensure_artifact_dir()?;
    let trace_dir = artifact_dir.join("traces");
    std::fs::create_dir_all(&trace_dir)?;
    let mut cfg = SpawnConfig::new(jekko.to_string_lossy().as_ref())
        .arg("--log-level")
        .arg("DEBUG")
        .arg(project.to_string_lossy().as_ref())
        .cwd(&project)
        .size(cols, rows)
        .trace_path(trace_dir.join(format!("{trace_name}.trace.jsonl")))
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("HOME", parent.path().to_string_lossy().as_ref())
        .env("JEKKO_API_KEY", "tuiwright-offline-fake-key")
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_LSP_DOWNLOAD", "1")
        .env("JEKKO_DISABLE_MODELS_FETCH", "1")
        .env("JEKKO_DISABLE_JNOCCIO_BOOT", "1")
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
        .timeout(Duration::from_secs(60));
    for (k, v) in extra_envs {
        cfg = cfg.env(*k, *v);
    }
    for (k, v) in std::env::vars() {
        if matches!(
            k.as_str(),
            "USER" | "LOGNAME" | "PATH" | "SHELL" | "LANG" | "LC_ALL" | "LC_CTYPE"
        ) {
            cfg = cfg.env(k, v);
        }
    }
    Page::spawn(cfg).context("spawn jekko TUI without pure")
}

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
        std::fs::write(
            workspace.path().join(".env.jnoccio"),
            format!("JNOCCIO_DEVELOPER_KEY={FAKE_DEVELOPER_KEY}\n"),
        )?;
        std::fs::create_dir_all(workspace.path().join(".jekko"))?;
        std::fs::write(
            workspace.path().join(".jekko").join("jekko.env"),
            "OPENAI_API_KEY=fake-openai-key\n",
        )?;
        let page = spawn_jekko_without_pure(
            &workspace,
            &jekko,
            cols,
            rows,
            &case,
            &[
                (
                    "JEKKO_MODEL_KEYS_FILE",
                    workspace
                        .path()
                        .join(".jekko")
                        .join("jekko.env")
                        .to_string_lossy()
                        .as_ref(),
                ),
                ("JEKKO_DISABLE_MODELS_FETCH", "0"),
            ],
        )?;

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

        // Phase A landed the app directly on Route::Shell; the footer hints
        // now read "Tab switch pane  / commands  Enter send  ? help  Ctrl+C
        // quit". We sentinel on "switch pane" (route-specific, stable
        // across resolutions including the narrow LEFT-panel collapse).
        page.wait_for_text("switch pane", HOME_TIMEOUT)
            .with_context(|| {
                let _ = page.screenshot(artifact_dir.join(format!("{case}-home-timeout.png")));
                let _ = copy_jekko_logs(&workspace, &case);
                format!("{case} did not paint the Shell footer sentinel")
            })?;

        page.screenshot(artifact_dir.join(format!("{case}-home.png")))?;
    }

    Ok(())
}

#[test]
#[ignore]
#[serial]
fn real_home_tui_reaches_prompt() -> Result<()> {
    if std::env::var("JEKKO_TUIWRIGHT_PTY").as_deref() != Ok("1") {
        eprintln!("skipped: set JEKKO_TUIWRIGHT_PTY=1 to run prompt PTY proofs");
        return Ok(());
    }
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN to the jekko binary path");
        return Ok(());
    };

    let home = std::env::var("HOME").context("HOME must be set for real-home proof")?;
    let home_path = std::path::PathBuf::from(&home);
    if !home_path.join(".env.jnoccio").exists() {
        eprintln!("skipped: missing {home}/.env.jnoccio");
        return Ok(());
    }
    if !home_path.join(".jekko").join("jekko.env").exists() {
        eprintln!("skipped: missing {home}/.jekko/jekko.env");
        return Ok(());
    }

    let artifact_dir = ensure_artifact_dir()?.join("boot");
    std::fs::create_dir_all(&artifact_dir)?;
    let (workspace, _project, _, _, _, _) = prepare_workspace("project", "# real-home-proof\n")?;
    let project = workspace.path().join("project");
    std::fs::create_dir_all(project.join("agent"))?;
    std::fs::write(project.join("agent").join("repo-score.json"), "{}\n")?;
    let xdg = workspace.path().join("xdg");
    let trace_dir = ensure_artifact_dir()?.join("traces");
    std::fs::create_dir_all(&trace_dir)?;

    let mut cfg = SpawnConfig::new(jekko.to_string_lossy().as_ref())
        .arg("--log-level")
        .arg("DEBUG")
        .arg(project.to_string_lossy().as_ref())
        .cwd(&project)
        .size(120, 30)
        .trace_path(trace_dir.join("real-home-proof.trace.jsonl"))
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("HOME", home_path.to_string_lossy().as_ref())
        .env(
            "JEKKO_MODEL_KEYS_FILE",
            home_path
                .join(".jekko")
                .join("jekko.env")
                .to_string_lossy()
                .as_ref(),
        )
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
        .timeout(Duration::from_secs(60));
    for (k, v) in std::env::vars() {
        if matches!(
            k.as_str(),
            "USER" | "LOGNAME" | "PATH" | "SHELL" | "LANG" | "LC_ALL" | "LC_CTYPE"
        ) {
            cfg = cfg.env(k, v);
        }
    }

    let load_seen_at = Instant::now();
    let page = Page::spawn(cfg).context("spawn jekko TUI with real home")?;
    page.wait_for_text("loading…", Duration::from_secs(3))
        .context("real-home loading screen did not appear")?;
    // Phase A landed the app directly on Route::Shell; footer reads
    // "Tab switch pane …" instead of the prior Home footer.
    page.wait_for_text("switch pane", Duration::from_secs(30))
        .context("real-home TUI did not boot")?;
    assert!(
        load_seen_at.elapsed() >= Duration::from_secs(5),
        "real-home prompt appeared before 5s: {:?}",
        load_seen_at.elapsed()
    );
    page.screenshot(artifact_dir.join("real-home-home.png"))?;
    Ok(())
}
