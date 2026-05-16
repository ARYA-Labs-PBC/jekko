use anyhow::Result;
use std::path::Path;
use std::process::Command;

use super::commands::{conflicts, run_status};
use super::paths::host_path;
use super::smoke::run_check;
use super::types::{lines, Pr, MODEL};

pub(super) fn fix(
    root: &Path,
    host_bin: &str,
    pr: &Pr,
    files: &[String],
    prs: &[Pr],
    applied: &[u64],
    idx: usize,
) -> Result<bool> {
    println!(
        "  Trying to auto-resolve {} conflict(s) with jekko...",
        files.len()
    );

    let done = lines(
        prs.iter()
            .filter(|x| applied.contains(&x.number))
            .cloned()
            .collect(),
    );
    let next = lines(prs.iter().skip(idx + 1).cloned().collect());

    let prompt = [
        format!(
            "Resolve the current git merge conflicts while merging PR #{} into the beta branch.",
            pr.number
        ),
        format!("PR #{}: {}", pr.number, pr.title),
        format!("Start with these conflicted files: {}.", files.join(", ")),
        format!("Merged PRs on HEAD:\n{done}"),
        format!("Pending PRs after this one (context only):\n{next}"),
        "IMPORTANT: The conflict resolution must be consistent with already-merged PRs.".into(),
        "Pending PRs are context only; do not introduce their changes unless they are already present on HEAD.".into(),
        "Prefer already-merged PRs over the base branch when resolving stacked conflicts.".into(),
        "If Cargo.lock is conflicted, do not hand-merge it. Delete Cargo.lock and run `cargo generate-lockfile` after the code conflicts are resolved.".into(),
        "If a PR already deleted a file/directory, do not re-add it, instead apply changes in the new semantic location.".into(),
        "If a PR already changed an import, keep that change.".into(),
        "After resolving the conflicts, run `cargo check --workspace --locked --offline` at the repo root.".into(),
        "If cargo check fails, you may also update any files reported by cargo.".into(),
        "Keep any non-conflict edits narrowly scoped to restoring a valid merged state for the current PR batch.".into(),
        "Fix any merge-caused check errors before finishing.".into(),
        "Keep the merge in progress, do not abort the merge, and do not create a commit.".into(),
        "When done, leave the working tree with no unmerged files and a passing cargo check.".into(),
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
        println!("  jekko failed: {err}");
        return Ok(false);
    }

    let left = conflicts()?;
    if !left.is_empty() {
        println!("  Conflicts remain: {}", left.join(", "));
        return Ok(false);
    }

    if !run_check(root)? {
        return Ok(false);
    }

    println!("  Conflicts resolved with jekko");
    Ok(true)
}
