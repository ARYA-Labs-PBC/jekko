use anyhow::{bail, Result};
use std::env;
use std::process::Command;

mod commands;
mod conflict_fix;
mod group;
mod paths;
mod pr_loop;
mod smoke;
mod types;

use commands::{git_output, run_status, run_text, split_lines};
use paths::host_binary_path;
use pr_loop::process_pr;
use smoke::smoke;
use types::{FailedPr, Pr};

pub fn run() -> Result<()> {
    let root =
        env::current_dir().map_err(|err| anyhow::anyhow!("reading current directory: {err}"))?;
    let host_bin = host_binary_path(&root)?;

    println!("Fetching open PRs with beta label...");
    let stdout = run_text(
        Command::new("gh")
            .arg("pr")
            .arg("list")
            .arg("--state")
            .arg("open")
            .arg("--draft=false")
            .arg("--label")
            .arg("beta")
            .arg("--json")
            .arg("number,title,author,labels")
            .arg("--limit")
            .arg("100"),
    )?;
    let prs: Vec<Pr> = serde_json::from_str(&stdout)
        .map_err(|err| anyhow::anyhow!("parse gh pr list response: {err}"))?;
    let mut prs = prs;
    prs.sort_by_key(|pr| pr.number);

    println!("Found {} open PRs with beta label", prs.len());
    if prs.is_empty() {
        println!("No team PRs to merge");
        return Ok(());
    }

    println!("Fetching latest dev branch...");
    run_status(Command::new("git").arg("fetch").arg("origin").arg("dev"))?;

    println!("Checking out beta branch...");
    run_status(
        Command::new("git")
            .arg("checkout")
            .arg("-B")
            .arg("beta")
            .arg("origin/dev"),
    )?;

    let mut applied: Vec<u64> = Vec::new();
    let mut failed: Vec<FailedPr> = Vec::new();

    for (idx, pr) in prs.iter().enumerate() {
        println!();
        process_pr(&root, &host_bin, pr, idx, &prs, &mut applied, &mut failed)?;
    }

    println!("\n--- Summary ---");
    println!("Applied: {} PRs", applied.len());
    for number in &applied {
        println!("  - PR #{number}");
    }

    if !failed.is_empty() {
        println!("Failed: {} PRs", failed.len());
        for pr in &failed {
            println!("  - PR #{}: {} ({})", pr.number, pr.title, pr.reason);
        }
        bail!("{} PR(s) failed to merge", failed.len());
    }

    println!("\nChecking if beta branch has changes...");
    run_status(Command::new("git").arg("fetch").arg("origin").arg("beta"))?;

    let local_tree = git_output(Command::new("git").arg("rev-parse").arg("beta^{tree}"))?;
    let remote_trees = git_output(
        Command::new("git")
            .arg("log")
            .arg("origin/dev..origin/beta")
            .arg("--format=%T"),
    )?;
    let remote_trees = split_lines(remote_trees);

    if let Some(idx) = remote_trees.iter().position(|tree| tree == &local_tree) {
        if idx != 0 {
            println!(
                "Beta branch contains this sync, but additional commits exist after it. Leaving beta branch as is."
            );
        } else {
            println!("Beta branch has identical contents, no push needed");
        }
        return Ok(());
    }

    if !smoke(&root, &host_bin, &prs, &applied)? {
        bail!("Final smoke check failed");
    }

    run_status(Command::new("git").arg("fetch").arg("origin").arg("beta"))?;

    let validated_tree = git_output(Command::new("git").arg("rev-parse").arg("beta^{tree}"))?;
    let remote_trees_after_smoke = git_output(
        Command::new("git")
            .arg("log")
            .arg("origin/dev..origin/beta")
            .arg("--format=%T"),
    )?;
    let remote_trees_after_smoke = split_lines(remote_trees_after_smoke);

    if let Some(idx) = remote_trees_after_smoke
        .iter()
        .position(|tree| tree == &validated_tree)
    {
        if idx != 0 {
            println!(
                "Beta branch contains this validated sync, but additional commits exist after it. Leaving beta branch as is."
            );
        } else {
            println!("Validated beta branch now matches remote contents, no push needed");
        }
        return Ok(());
    }

    println!("Force pushing validated beta branch...");
    run_status(
        Command::new("git")
            .arg("push")
            .arg("origin")
            .arg("beta")
            .arg("--force")
            .arg("--no-verify"),
    )?;

    println!("Successfully synced beta branch");
    Ok(())
}
