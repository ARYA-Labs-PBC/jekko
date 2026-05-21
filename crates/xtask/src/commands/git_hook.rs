use std::process::Command;

use anyhow::{bail, Context, Result};

pub fn run(kind: &str) -> Result<()> {
    match kind {
        "pre-commit" | "pre-push" => block_main_commit(),
        other => bail!("unsupported git hook `{other}`"),
    }
}

fn block_main_commit() -> Result<()> {
    block_main_commit_in(None)
}

fn block_main_commit_in(repo: Option<&std::path::Path>) -> Result<()> {
    let branch = git_stdout(repo, ["branch", "--show-current"])?;
    if branch.trim() == "main" {
        bail!("direct commits to main are blocked; create a feature branch");
    }
    Ok(())
}

fn git_stdout<const N: usize>(repo: Option<&std::path::Path>, args: [&str; N]) -> Result<String> {
    let mut command = Command::new("git");
    command.args(args);
    if let Some(repo) = repo {
        command.current_dir(repo);
    }
    let output = command.output().context("spawn git")?;
    if !output.status.success() {
        bail!("git failed with status {}", output.status);
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn commit_on_main_fails() {
        let repo = init_repo();
        run_git(repo.path(), ["checkout", "-B", "main"]);
        let result = block_main_commit_in(Some(repo.path()));
        assert!(result.is_err());
    }

    #[test]
    fn commit_on_feature_branch_passes() {
        let repo = init_repo();
        run_git(repo.path(), ["checkout", "-B", "feature/test"]);
        let result = block_main_commit_in(Some(repo.path()));
        assert!(result.is_ok());
    }

    fn init_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        run_git(dir.path(), ["init"]);
        run_git(dir.path(), ["config", "user.email", "test@example.com"]);
        run_git(dir.path(), ["config", "user.name", "Test User"]);
        fs::write(dir.path().join("README.md"), "test\n").unwrap();
        run_git(dir.path(), ["add", "README.md"]);
        run_git(dir.path(), ["commit", "-m", "init"]);
        dir
    }

    fn run_git<const N: usize>(dir: &Path, args: [&str; N]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(status.success());
    }
}
