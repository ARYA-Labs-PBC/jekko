use anyhow::{bail, Context, Result};
use std::env;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

const DEFAULT_REPOSITORY: &str = "neverhuman/jekko";

pub fn init() -> Result<()> {
    run_checked(
        Command::new("git").args(["fetch", "origin", "--tags"]),
        "git fetch origin --tags",
    )?;
    run_checked(
        Command::new("git").args(["switch", "--detach"]),
        "git switch --detach",
    )?;
    Ok(())
}

pub fn finalize(repo_root: &std::path::Path, version: &str, repo: Option<&str>) -> Result<()> {
    let tag = release_tag(version);
    run_checked(
        Command::new("git")
            .args(["commit", "-am", &format!("release: {tag}")])
            .current_dir(repo_root),
        "git commit release",
    )?;

    let _ = Command::new("git")
        .args(["tag", "-d", &tag])
        .current_dir(repo_root)
        .status();

    run_checked(
        Command::new("git")
            .args(["tag", &tag])
            .current_dir(repo_root),
        "git tag",
    )?;
    run_checked(
        Command::new("git")
            .args([
                "push",
                "origin",
                &format!("refs/tags/{tag}"),
                "--force-with-lease",
                "--no-verify",
            ])
            .current_dir(repo_root),
        "git push release tag",
    )?;

    sleep(Duration::from_secs(5));

    run_checked(
        Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(repo_root),
        "git fetch origin",
    )?;
    run_checked(
        Command::new("git")
            .args(["checkout", "-B", "dev", "origin/dev"])
            .current_dir(repo_root),
        "git checkout -B dev origin/dev",
    )?;

    println!("publish-release-finalize: binary-only version sync is handled by xtask release");

    let repo = match repo.map(ToOwned::to_owned) {
        Some(value) => value,
        None => match env::var("GH_REPO").ok() {
            Some(value) => value,
            None => DEFAULT_REPOSITORY.to_string(),
        },
    };
    run_checked(
        Command::new("gh")
            .args(["release", "edit", &tag, "--draft=false", "--repo", &repo])
            .current_dir(repo_root),
        "gh release edit",
    )?;
    Ok(())
}

pub fn release_tag(version: &str) -> String {
    format!("v{version}")
}

fn run_checked(cmd: &mut Command, label: &str) -> Result<()> {
    let status = cmd.status().with_context(|| format!("run {label}"))?;
    if !status.success() {
        bail!("{label} failed with status {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_tag_prefixes_version_with_v() {
        assert_eq!(release_tag("1.2.3"), "v1.2.3");
    }
}
