//! `xtask publish-build-script` — Rust orchestrator for the publish-lane
//! cross-target CLI build matrix.
//!
//! Workflow:
//!
//! 1. Resolve the build matrix by shelling to `xtask publish-build-plan`
//!    (parsing the JSON output through `publish_build_plan::PublishBuildPlan`).
//! 2. For each target:
//!    - Map the `<runtime>-<os>-<arch>[-baseline][-musl]` token to a Rust
//!      target triple.
//!    - Run `cargo build -p jekko-cli --release --locked --target <triple>`
//!      (preferring `cross` when available for foreign targets). Baseline
//!      variants set `RUSTFLAGS=-C target-feature=-avx2`.
//!    - Stage the produced artifact into `dist/<target.name>/bin/jekko[.exe]`
//!      and write `dist/<target.name>/checksum.txt` (sha256).
//!    - For the current host target, also run `<staged-binary> --version` as
//!      a smoke test. Smoke test failure is fatal.
//! 3. Call `xtask publish-stage-cli-assets --dist-root ./dist --version
//!    <JEKKO_VERSION> [--release]` to finish manifest rewriting and (when
//!    `JEKKO_RELEASE` is set) `gh release upload`.
//!
//! Accepted flags from the previous publish wrapper:
//! - `--single` restricts to the current host's native target.
//! - `--baseline` includes the AVX2-disabled baseline variant.
//! - `--skip-install` is a no-op because Cargo needs no parallel install
//!   step here.
//! - `--sourcemaps` is a no-op because `--release` already emits debug
//!   info via the release profile.
//!
//! Flags can be passed either as positional CLI args after the subcommand or
//! via `BUILD_ARGS` (whitespace-separated), matching the previous shell caller
//! contract used by the publish workflow.

use anyhow::{bail, Context, Result};
use std::env;
use std::fs;
use std::path::Path;

mod build;
mod env_info;
mod flags;
mod plan;
mod stage;
#[cfg(test)]
mod tests;
mod types;

use build::build_one;
use env_info::{installed_rustup_targets, is_command_available, resolve_version};
use flags::parse_flags;
use plan::{fetch_build_plan, resolve_target};
use stage::{run_stage_cli_assets, smoke_test_host, stage_binary};
use types::{BuildOutcome, ResolvedTarget};

const DEFAULT_BUILD_ARGS: &str = "";

pub fn run(root: &Path, cli_args: &[String]) -> Result<()> {
    let env_args = match env::var("BUILD_ARGS") {
        Ok(value) => value,
        Err(_) => DEFAULT_BUILD_ARGS.to_string(),
    };
    let env_tokens: Vec<String> = env_args
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    let combined: Vec<&str> = cli_args
        .iter()
        .chain(env_tokens.iter())
        .map(String::as_str)
        .collect();
    let flags = parse_flags(&combined);

    let plan = fetch_build_plan(root, flags.single, flags.baseline)
        .context("fetch publish build plan via `xtask publish-build-plan`")?;
    if plan.targets.is_empty() {
        bail!("publish build plan returned zero targets");
    }

    let resolved: Vec<ResolvedTarget> = plan
        .targets
        .into_iter()
        .map(resolve_target)
        .collect::<Result<Vec<_>>>()?;

    let dist_root = root.join("dist");
    if dist_root.exists() {
        fs::remove_dir_all(&dist_root)
            .with_context(|| format!("clear dist root {}", dist_root.display()))?;
    }

    let cross_available = is_command_available("cross");
    let installed_targets = installed_rustup_targets();

    let mut host_built = false;
    let mut skipped: Vec<String> = Vec::new();

    for target in &resolved {
        println!("building {}", target.plan.name);
        match build_one(root, target, cross_available, &installed_targets)? {
            BuildOutcome::Built => {
                stage_binary(root, target, &dist_root)?;
                if target.is_host {
                    smoke_test_host(&dist_root, &target.plan.name)?;
                    host_built = true;
                }
            }
            BuildOutcome::Skipped(reason) => {
                let line = format!("{}: SKIPPED ({reason})", target.plan.name);
                println!("{line}");
                skipped.push(line);
            }
        }
    }

    if !host_built {
        bail!(
            "host target build did not run (no resolved target matched the current platform); \
             at least the host build must succeed"
        );
    }

    let version = resolve_version();
    let release = env::var("JEKKO_RELEASE")
        .ok()
        .filter(|v| !v.is_empty())
        .is_some();
    run_stage_cli_assets(root, &version, release)?;

    if !skipped.is_empty() {
        println!();
        println!("publish-build-script: {} target(s) skipped:", skipped.len());
        for line in &skipped {
            println!("  - {line}");
        }
    }
    println!(
        "publish-build-script: done (host build OK; {} skipped)",
        skipped.len()
    );

    Ok(())
}
