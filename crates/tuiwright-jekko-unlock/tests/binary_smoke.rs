use std::process::Command;

use anyhow::{anyhow, Context, Result};

mod test_helpers;
use test_helpers::jekko_bin;

fn run(args: &[&str]) -> Result<String> {
    let Some(jekko) = jekko_bin() else {
        eprintln!("skipped: set JEKKO_BIN to the jekko binary path");
        return Ok(String::new());
    };
    let output = Command::new(&jekko)
        .args(args)
        .env("JEKKO_DISABLE_AUTOUPDATE", "1")
        .env("JEKKO_DISABLE_LSP_DOWNLOAD", "1")
        .env("JEKKO_DISABLE_MODELS_FETCH", "1")
        .env("JEKKO_DISABLE_PRUNE", "1")
        .output()
        .with_context(|| format!("run {:?} {:?}", jekko, args))?;
    if !output.status.success() {
        return Err(anyhow!(
            "jekko {:?} failed: {}\nstdout:\n{}\nstderr:\n{}",
            args,
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

#[test]
fn jekko_binary_reports_version() -> Result<()> {
    let output = run(&["--version"])?;
    if output.is_empty() {
        return Ok(());
    }
    assert!(
        output.trim().contains("codex") || output.trim().contains("jekko") || output.trim().contains('.'),
        "unexpected version output: {output:?}"
    );
    Ok(())
}

#[test]
fn jekko_binary_help_is_tui_only() -> Result<()> {
    let output = run(&["--help"])?;
    if output.is_empty() {
        return Ok(());
    }
    assert!(
        output.contains("start jekko tui"),
        "help output did not expose the default TUI command:\n{output}"
    );
    let command_lines = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    assert!(
        !command_lines
            .iter()
            .any(|line| *line == "web" || line.starts_with("web ")),
        "help output still exposes a web command:\n{output}"
    );
    assert!(
        !output.to_lowercase().contains("browser ui"),
        "help output still advertises browser UI:\n{output}"
    );
    Ok(())
}
