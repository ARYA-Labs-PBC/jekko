use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

use super::env_info::{current_host_triple, resolve_target_dir, sha256_of_file};
use super::types::ResolvedTarget;

pub(super) fn stage_binary(root: &Path, target: &ResolvedTarget, dist_root: &Path) -> Result<()> {
    let target_dir = resolve_target_dir(root);
    let triple = target.rust_triple;
    let host_triple = current_host_triple();
    let exe_name = if triple.contains("windows") {
        "jekko.exe"
    } else {
        "jekko"
    };
    // When we built without --target (host fast path), the artifact lives at
    // `<target-dir>/release/jekko`; otherwise under `<target-dir>/<triple>/release/jekko`.
    let release_dir = if target.is_host && triple == host_triple {
        target_dir.join("release")
    } else {
        target_dir.join(triple).join("release")
    };
    let built = release_dir.join(exe_name);
    if !built.exists() {
        bail!(
            "expected release artifact at {} after build (target {})",
            built.display(),
            target.plan.name
        );
    }

    let staged_dir = dist_root.join(&target.plan.name).join("bin");
    fs::create_dir_all(&staged_dir)
        .with_context(|| format!("create staging dir {}", staged_dir.display()))?;
    let staged = staged_dir.join(exe_name);
    fs::copy(&built, &staged)
        .with_context(|| format!("copy {} -> {}", built.display(), staged.display()))?;

    let sha = sha256_of_file(&staged)?;
    let checksum_path = dist_root.join(&target.plan.name).join("checksum.txt");
    let checksum_body = format!("{sha}  bin/{exe_name}\n");
    fs::write(&checksum_path, &checksum_body)
        .with_context(|| format!("write checksum {}", checksum_path.display()))?;

    println!(
        "  staged {} sha256={}",
        staged.display(),
        &sha[..16.min(sha.len())]
    );
    Ok(())
}

pub(super) fn smoke_test_host(dist_root: &Path, target_name: &str) -> Result<()> {
    let exe_name = if cfg!(windows) { "jekko.exe" } else { "jekko" };
    let binary = dist_root.join(target_name).join("bin").join(exe_name);
    if !binary.exists() {
        bail!(
            "smoke test: staged host binary missing at {}",
            binary.display()
        );
    }
    println!("  smoke test: {} --version", binary.display());
    let output = Command::new(&binary)
        .arg("--version")
        .output()
        .with_context(|| format!("run {} --version", binary.display()))?;
    if !output.status.success() {
        bail!(
            "smoke test failed for {}: status {} stderr={}",
            target_name,
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    println!("  smoke test passed: {trimmed}");
    Ok(())
}

pub(super) fn run_stage_cli_assets(root: &Path, version: &str, release: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "-q",
        "-p",
        "xtask",
        "--",
        "publish-stage-cli-assets",
        "--dist-root",
        "./dist",
        "--version",
        version,
    ]);
    if release {
        cmd.arg("--release");
    }
    cmd.current_dir(root);
    let status = cmd.status().context("spawn publish-stage-cli-assets")?;
    if !status.success() {
        bail!("publish-stage-cli-assets failed with status {status}");
    }
    Ok(())
}
