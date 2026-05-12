//! Shared helpers used by every backend implementation.
//!
//! Pulled into a dedicated module so the dispatch backends (`bubblewrap`,
//! `docker`) can delegate workspace setup to the cross-platform `worktree`
//! backend without duplicating the call shape.

use std::fs::File;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use anyhow::{anyhow, Context, Result};

use super::{BackendImpl, ExecOutcome, Workspace};
use crate::spec::Lane;

/// Delegate workspace setup (git worktree + private HOME/TMPDIR/cache) to the
/// `worktree` backend. Every alternative backend reuses the same host-side
/// workspace; only `run_argv` differs per backend.
pub fn delegate_workspace_setup(lane: &Lane, workspace: &Workspace) -> Result<()> {
    super::worktree::WorktreeBackend.create(lane, workspace)
}

pub fn run_argv_with_output(
    mut cmd: Command,
    stdout_path: &Path,
    stderr_path: &Path,
    context: impl FnOnce() -> String,
) -> Result<ExecOutcome> {
    let stdout = File::create(stdout_path)?;
    let stderr = File::create(stderr_path)?;
    cmd.stdout(Stdio::from(stdout));
    cmd.stderr(Stdio::from(stderr));
    let status = cmd.status().with_context(context)?;
    Ok(ExecOutcome {
        exit_code: exit_code(status),
    })
}

pub fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(127)
}

/// Shared no-op-style defaults for the `BackendImpl` lifecycle methods that
/// don't differ between backends. Both `docker` and `bubblewrap` reuse the
/// `worktree` workspace setup + teardown — pulling those into a single named
/// boundary keeps the trait impls below the duplicate-block threshold.
pub struct BackendDefaults;

impl BackendDefaults {
    pub fn default_create(lane: &Lane, workspace: &Workspace) -> Result<()> {
        delegate_workspace_setup(lane, workspace)
    }

    pub fn default_destroy(workspace: &Workspace, keep_logs: bool) -> Result<()> {
        super::worktree::WorktreeBackend.destroy(workspace, keep_logs)
    }
}

/// Probe that `binary` is on PATH AND that running `binary <subcommand>`
/// returns a successful status. Used by docker/podman to confirm both the
/// CLI and the daemon are reachable.
pub fn probe_binary_subcommand(binary: &str, subcommand: &str) -> Result<()> {
    which::which(binary).map_err(|_| {
        anyhow!("{binary} not on PATH; install or start it before using this backend")
    })?;
    let status = Command::new(binary)
        .arg(subcommand)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(anyhow!(
            "{binary} {subcommand} returned exit {} — daemon not running?",
            s.code().unwrap_or(-1)
        )),
        Err(err) => Err(anyhow!("{binary} {subcommand} failed: {err}")),
    }
}
