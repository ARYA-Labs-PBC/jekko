use anyhow::{bail, Context, Result};
use std::env;
use std::path::Path;
use std::process::Command;

use super::types::{PublishBuildPlanJson, PublishBuildTarget, ResolvedTarget};

pub(super) fn fetch_build_plan(
    root: &Path,
    single: bool,
    baseline: bool,
) -> Result<PublishBuildPlanJson> {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "-q",
        "-p",
        "xtask",
        "--",
        "publish-build-plan",
        "--package-name",
        "jekko",
    ]);
    if single {
        cmd.arg("--single");
    }
    if baseline {
        cmd.arg("--baseline");
    }
    cmd.current_dir(root);
    let output = cmd
        .output()
        .context("spawn `cargo run -p xtask -- publish-build-plan`")?;
    if !output.status.success() {
        bail!(
            "publish-build-plan failed (status {}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let stdout = String::from_utf8(output.stdout).context("decode publish-build-plan stdout")?;
    serde_json::from_str::<PublishBuildPlanJson>(stdout.trim())
        .with_context(|| format!("parse publish-build-plan JSON: {stdout}"))
}

pub(super) fn resolve_target(plan: PublishBuildTarget) -> Result<ResolvedTarget> {
    let baseline = plan.avx2 == Some(false);
    let rust_triple = bun_target_to_rust_triple(&plan.bun_target).with_context(|| {
        format!(
            "no Rust target triple mapping for target token {}",
            plan.bun_target
        )
    })?;
    let is_host = is_host_target(&plan);
    Ok(ResolvedTarget {
        plan,
        rust_triple,
        baseline,
        is_host,
    })
}

/// Maps the target token emitted by `publish-build-plan` to a Rust target
/// triple. Baseline variants share the same triple as the standard variant
/// for the same os/arch; they only differ via `RUSTFLAGS`.
pub(super) fn bun_target_to_rust_triple(bun_target: &str) -> Option<&'static str> {
    match bun_target {
        "bun-darwin-arm64" | "bun-darwin-arm64-baseline" => Some("aarch64-apple-darwin"),
        "bun-darwin-x64" | "bun-darwin-x64-baseline" => Some("x86_64-apple-darwin"),
        "bun-linux-arm64" | "bun-linux-arm64-baseline" => Some("aarch64-unknown-linux-gnu"),
        "bun-linux-x64" | "bun-linux-x64-baseline" => Some("x86_64-unknown-linux-gnu"),
        "bun-linux-arm64-musl" | "bun-linux-arm64-baseline-musl" => {
            Some("aarch64-unknown-linux-musl")
        }
        "bun-linux-x64-musl" | "bun-linux-x64-baseline-musl" => Some("x86_64-unknown-linux-musl"),
        "bun-windows-arm64" | "bun-windows-arm64-baseline" => Some("aarch64-pc-windows-msvc"),
        "bun-windows-x64" | "bun-windows-x64-baseline" => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}

pub(super) fn is_host_target(plan: &PublishBuildTarget) -> bool {
    if plan.avx2 == Some(false) {
        // Baseline variants are never treated as the "host" smoke-test target.
        return false;
    }
    if plan.abi.as_deref() == Some("musl") {
        return false;
    }
    let host_os = match env::consts::OS {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "win32",
        other => other,
    };
    let host_arch = match env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    plan.os == host_os && plan.arch == host_arch
}
