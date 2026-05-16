use anyhow::Result;
use std::path::Path;
use std::process::Command;

use super::commands::{git_output, run_status};
use super::paths::host_path;
use super::types::{lines, Pr, MODEL};

pub(super) fn smoke(root: &Path, host_bin: &str, prs: &[Pr], applied: &[u64]) -> Result<bool> {
    println!("\nRunning final smoke check...");

    if run_check(root)? {
        return commit_smoke_changes(root);
    }

    println!("\nTrying to fix final smoke check with jekko...");

    let done = lines(
        prs.iter()
            .filter(|x| applied.contains(&x.number))
            .cloned()
            .collect(),
    );
    let prompt = [
        "The beta merge batch is complete, but the deterministic final smoke check failed."
            .to_string(),
        format!("Merged PRs on HEAD:\n{done}"),
        "Run `cargo check --workspace --locked --offline` at the repo root.".to_string(),
        "Run `cargo build -p jekko-cli --release --locked` at the repo root.".to_string(),
        "Fix any merge-caused issues until both commands pass.".to_string(),
        "Do not create a commit.".to_string(),
    ]
    .join("\n");

    let mut command = Command::new(host_bin);
    command
        .arg("run")
        .arg("--model")
        .arg(MODEL)
        .arg(prompt)
        .env("PATH", host_path(host_bin)?)
        .current_dir(root);
    if let Err(err) = run_status(&mut command) {
        println!("Smoke fix failed: {err}");
        return Ok(false);
    }

    if !run_check(root)? {
        return Ok(false);
    }
    commit_smoke_changes(root)
}

fn commit_smoke_changes(root: &Path) -> Result<bool> {
    let out = git_output(Command::new("git").arg("status").arg("--porcelain"))?;
    if out.trim().is_empty() {
        println!("Smoke check passed");
        return Ok(true);
    }

    run_status(Command::new("git").arg("add").arg("-A"))?;
    run_status(
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("Fix beta integration"),
    )?;

    if !run_check(root)? {
        return Ok(false);
    }

    let left = git_output(Command::new("git").arg("status").arg("--porcelain"))?;
    if left.trim().is_empty() {
        println!("Smoke check passed");
        Ok(true)
    } else {
        println!("Smoke check left uncommitted changes:\n{left}");
        Ok(false)
    }
}

pub(super) fn run_check(root: &Path) -> Result<bool> {
    println!("  Running cargo check...");
    if let Err(err) = run_status(
        Command::new("cargo")
            .arg("check")
            .arg("--workspace")
            .arg("--locked")
            .arg("--offline")
            .current_dir(root),
    ) {
        println!("cargo check failed: {err}");
        return Ok(false);
    }

    println!("  Running release build smoke check...");
    if let Err(err) = run_status(
        Command::new("cargo")
            .arg("build")
            .arg("-p")
            .arg("jekko-cli")
            .arg("--release")
            .arg("--locked")
            .current_dir(root),
    ) {
        println!("cargo build failed: {err}");
        return Ok(false);
    }

    Ok(true)
}
