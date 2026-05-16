use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use zyalc::compile::inspect;

mod test_helpers;
use test_helpers::repo_root;

fn tracked_zyal_files(root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-files", "--", "*.zyal"])
        .output()
        .context("run git ls-files for ZYAL examples")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git ls-files failed: {}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let mut files = String::from_utf8(output.stdout).context("decode git ls-files output")?;
    if files.ends_with('\n') {
        files.pop();
    }
    let mut paths = files
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| root.join(line))
        .collect::<Vec<_>>();
    paths.retain(|path| !is_declarative_zyal(path));
    paths.sort();
    Ok(paths)
}

fn is_declarative_zyal(path: &Path) -> bool {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| text.lines().next().map(str::to_owned))
        .is_some_and(|line| line.trim_start().starts_with("# zyal: declarative"))
}

#[test]
fn all_tracked_zyal_files_parse_and_preview() -> Result<()> {
    let root = repo_root();
    let files = tracked_zyal_files(&root)?;
    assert!(
        !files.is_empty(),
        "expected at least one tracked ZYAL file under the repository"
    );

    for path in &files {
        let info = inspect(path).with_context(|| format!("inspect ZYAL file {path:?}"))?;
        assert!(
            !info.profile.is_empty(),
            "expected a detectable ZYAL profile for {path:?}"
        );
        println!(
            "previewed {} as {} -> {}",
            path.display(),
            info.profile,
            info.target
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<none>".to_string())
        );
    }

    println!("validated {} ZYAL files", files.len());
    Ok(())
}
