use anyhow::{bail, Context, Result};
use std::process::Command;

pub(super) fn comment_on_pr(pr_number: u64, reason: &str) {
    let body = format!(
        "WARNING: Blocking Beta Release\n\nThis PR cannot be merged into the beta branch due to: **{reason}**\n\nPlease resolve this issue to include this PR in the next beta release."
    );

    match run_status(
        Command::new("gh")
            .arg("pr")
            .arg("comment")
            .arg(pr_number.to_string())
            .arg("--body")
            .arg(&body),
    ) {
        Ok(()) => println!("  Posted comment on PR #{pr_number}"),
        Err(err) => println!("  Failed to post comment on PR #{pr_number}: {err}"),
    }
}

pub(super) fn conflicts() -> Result<Vec<String>> {
    let stdout = git_output(
        Command::new("git")
            .arg("diff")
            .arg("--name-only")
            .arg("--diff-filter=U"),
    )?;
    Ok(split_lines(stdout))
}

pub(super) fn cleanup() {
    let _ = run_status(Command::new("git").arg("merge").arg("--abort"));
    let _ = run_status(Command::new("git").arg("checkout").arg("--").arg("."));
    let _ = run_status(Command::new("git").arg("clean").arg("-fd"));
}

pub(super) fn merge_in_progress() -> Result<bool> {
    match run_text(
        Command::new("git")
            .arg("rev-parse")
            .arg("-q")
            .arg("--verify")
            .arg("MERGE_HEAD"),
    ) {
        Ok(stdout) => Ok(!stdout.trim().is_empty()),
        Err(_) => Ok(false),
    }
}

pub(super) fn git_output(cmd: &mut Command) -> Result<String> {
    let output = cmd.output().with_context(|| format!("running {:?}", cmd))?;
    if !output.status.success() {
        bail!(
            "{}",
            String::from_utf8_lossy(&output.stderr).trim().to_owned()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub(super) fn run_text(cmd: &mut Command) -> Result<String> {
    git_output(cmd)
}

pub(super) fn run_status(cmd: &mut Command) -> Result<()> {
    let output = cmd.output().with_context(|| format!("running {:?}", cmd))?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let mut msg = format!("command failed: {:?}", cmd);
    if !stderr.is_empty() {
        msg.push_str(&format!("\nstderr: {stderr}"));
    }
    if !stdout.is_empty() {
        msg.push_str(&format!("\nstdout: {stdout}"));
    }
    bail!(msg);
}

pub(super) fn split_lines(stdout: String) -> Vec<String> {
    stdout
        .lines()
        .map(|x| x.trim().to_owned())
        .filter(|x| !x.is_empty())
        .collect()
}
