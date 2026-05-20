use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use semver::Version;

pub fn package(repo_root: &Path, target: &str, dry_run: bool) -> Result<()> {
    let version = workspace_version(repo_root)?;
    assert_major_release(&version)?;
    let artifact = artifact_name(&version, target);
    if dry_run {
        println!("release package dry-run: {artifact}");
        println!("release package dry-run: {artifact}.sha256");
        return Ok(());
    }
    bail!("non-dry-run release packaging is intentionally not wired in xtask yet")
}

pub fn attach(version: &str, dry_run: bool) -> Result<()> {
    let version = Version::parse(version.trim_start_matches('v')).context("parse version")?;
    assert_major_release(&version)?;
    if dry_run {
        println!("release attach dry-run: v{version}");
        return Ok(());
    }
    bail!("non-dry-run release attach is intentionally not wired in xtask yet")
}

fn assert_major_release(version: &Version) -> Result<()> {
    if version.major == 0 || version.minor != 0 || version.patch != 0 {
        bail!("major binary releases must use vMAJOR.0.0");
    }
    Ok(())
}

fn workspace_version(repo_root: &Path) -> Result<Version> {
    let manifest = fs::read_to_string(repo_root.join("Cargo.toml")).context("read Cargo.toml")?;
    let mut in_workspace_package = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_workspace_package = trimmed == "[workspace.package]";
            continue;
        }
        if !in_workspace_package {
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("version = ") {
            return Version::parse(value.trim().trim_matches('"'))
                .context("parse workspace version");
        }
    }
    bail!("workspace.package.version not found")
}

fn artifact_name(version: &Version, target: &str) -> String {
    format!("jekko-v{version}-{target}.tar.gz")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn major_release_accepts_v1_0_0() {
        assert!(assert_major_release(&Version::parse("1.0.0").unwrap()).is_ok());
    }

    #[test]
    fn major_release_rejects_minor_release() {
        assert!(assert_major_release(&Version::parse("1.1.0").unwrap()).is_err());
    }
}
