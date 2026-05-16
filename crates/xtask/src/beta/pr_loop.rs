use anyhow::Result;
use std::path::Path;
use std::process::Command;

use super::commands::{cleanup, comment_on_pr, conflicts, merge_in_progress, run_status};
use super::conflict_fix::fix;
use super::group::Group;
use super::types::{FailedPr, Pr};

pub(super) fn process_pr(
    root: &Path,
    host_bin: &str,
    pr: &Pr,
    idx: usize,
    prs: &[Pr],
    applied: &mut Vec<u64>,
    failed: &mut Vec<FailedPr>,
) -> Result<()> {
    let _group = Group::new(format!(
        "Processing PR {}/{} #{}: {}",
        idx + 1,
        prs.len(),
        pr.number,
        pr.title
    ));
    println!("  Fetching PR head...");
    if let Err(err) = run_status(
        Command::new("git")
            .arg("fetch")
            .arg("origin")
            .arg(format!("pull/{}/head:pr/{}", pr.number, pr.number)),
    ) {
        println!("  Failed to fetch: {err}");
        failed.push(FailedPr {
            number: pr.number,
            title: pr.title.clone(),
            reason: "Fetch failed".into(),
        });
        comment_on_pr(pr.number, "Fetch failed");
        return Ok(());
    }

    println!("  Merging...");
    if let Err(_err) = run_status(
        Command::new("git")
            .arg("merge")
            .arg("--no-commit")
            .arg("--no-ff")
            .arg(format!("pr/{}", pr.number)),
    ) {
        let files = conflicts()?;
        if files.is_empty() {
            println!("  Failed to merge");
            cleanup();
            failed.push(FailedPr {
                number: pr.number,
                title: pr.title.clone(),
                reason: "Merge failed".into(),
            });
            comment_on_pr(pr.number, "Merge failed");
            return Ok(());
        }

        println!("  Failed to merge (conflicts)");
        if !fix(root, host_bin, pr, &files, prs, applied, idx)? {
            cleanup();
            failed.push(FailedPr {
                number: pr.number,
                title: pr.title.clone(),
                reason: "Merge conflicts".into(),
            });
            comment_on_pr(pr.number, "Merge conflicts with dev branch");
            return Ok(());
        }
    }

    if !merge_in_progress()? {
        println!("  No changes, skipping");
        return Ok(());
    }

    if let Err(err) = run_status(Command::new("git").arg("add").arg("-A")) {
        println!("  Failed to stage changes");
        failed.push(FailedPr {
            number: pr.number,
            title: pr.title.clone(),
            reason: "Staging failed".into(),
        });
        comment_on_pr(pr.number, "Failed to stage changes");
        println!("  git add failed: {err}");
        return Ok(());
    }

    let commit_msg = format!("Apply PR #{}: {}", pr.number, pr.title);
    if let Err(err) = run_status(Command::new("git").arg("commit").arg("-m").arg(&commit_msg)) {
        println!("  Failed to commit: {err}");
        failed.push(FailedPr {
            number: pr.number,
            title: pr.title.clone(),
            reason: "Commit failed".into(),
        });
        comment_on_pr(pr.number, "Failed to commit changes");
        return Ok(());
    }

    println!("  Applied successfully");
    applied.push(pr.number);
    Ok(())
}
