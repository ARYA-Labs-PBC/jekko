use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

use super::env_info::current_host_triple;
use super::types::{BuildOutcome, ResolvedTarget};

pub(super) fn build_one(
    root: &Path,
    target: &ResolvedTarget,
    cross_available: bool,
    installed_targets: &[String],
) -> Result<BuildOutcome> {
    let triple = target.rust_triple;

    // Host triple needs no rustup target and no `cross`; just `cargo build`.
    let host_triple = current_host_triple();
    if target.is_host && triple == host_triple {
        run_cargo_build(root, None, target.baseline)?;
        return Ok(BuildOutcome::Built);
    }

    // Foreign target: prefer cross, fall back to cargo if the rustup target
    // is installed, otherwise SKIP.
    if cross_available {
        run_cross_build(root, triple, target.baseline)?;
        return Ok(BuildOutcome::Built);
    }
    if installed_targets.iter().any(|t| t == triple) {
        run_cargo_build(root, Some(triple), target.baseline)?;
        return Ok(BuildOutcome::Built);
    }
    Ok(BuildOutcome::Skipped(format!(
        "cross-compile tooling missing for {triple} (install `cross` or `rustup target add {triple}`)"
    )))
}

fn run_cargo_build(root: &Path, triple: Option<&str>, baseline: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(["build", "-p", "jekko-cli", "--release", "--locked"]);
    if let Some(triple) = triple {
        cmd.args(["--target", triple]);
    }
    if baseline {
        cmd.env("RUSTFLAGS", "-C target-feature=-avx2");
    }
    cmd.current_dir(root);
    let status = cmd.status().context("spawn `cargo build`")?;
    if !status.success() {
        let target_label = match triple {
            Some(t) => format!("--target {t} "),
            None => String::new(),
        };
        bail!("cargo build {} failed with status {}", target_label, status);
    }
    Ok(())
}

fn run_cross_build(root: &Path, triple: &str, baseline: bool) -> Result<()> {
    let mut cmd = Command::new("cross");
    cmd.args([
        "build",
        "-p",
        "jekko-cli",
        "--release",
        "--locked",
        "--target",
        triple,
    ]);
    if baseline {
        cmd.env("RUSTFLAGS", "-C target-feature=-avx2");
    }
    cmd.current_dir(root);
    let status = cmd.status().context("spawn `cross build`")?;
    if !status.success() {
        bail!("cross build --target {triple} failed with status {status}");
    }
    Ok(())
}
