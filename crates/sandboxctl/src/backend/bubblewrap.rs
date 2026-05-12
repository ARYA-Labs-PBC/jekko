// jankurai:allow HLT-000-SCORE-DIMENSION reason=parallel-trait-impl-with-docker-by-design-shared-defaults-hoisted-to-BackendDefaults expires=2027-12-01
//! Bubblewrap backend — Linux only.
//!
//! Wraps `bwrap --unshare-pid --unshare-net --die-with-parent --new-session
//! --ro-bind /usr /usr --bind <ws> /work --proc /proc --dev /dev --tmpfs /tmp
//! --setenv HOME /home/sandbox --chdir /work -- <cmd>`. On non-Linux hosts
//! `probe()` returns a clear "use worktree" error before any side effects.
//!
//! This file complements `docker.rs` — both alternative backends delegate
//! workspace setup to `worktree`, but their `run_argv` wiring is
//! backend-specific.

use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;

use super::{BackendImpl, ExecOutcome, Workspace};
use crate::spec::Lane;
use crate::wrapper::ExecArgs;

pub struct BubblewrapBackend;

impl BackendImpl for BubblewrapBackend {
    fn name(&self) -> &'static str {
        "bubblewrap"
    }

    fn probe(&self) -> Result<()> {
        if !cfg!(target_os = "linux") {
            return Err(anyhow!(
                "bubblewrap backend requires Linux; use `backend = \"worktree\"` or `\"docker\"` on this host"
            ));
        }
        which::which("bwrap").map_err(|_| {
            anyhow!("bwrap not on PATH; install bubblewrap (`apt-get install bubblewrap`)")
        })?;
        Ok(())
    }

    fn create(&self, lane: &Lane, workspace: &Workspace) -> Result<()> {
        super::common::BackendDefaults::default_create(lane, workspace)
    }

    /// Assemble and run a `bwrap` invocation. Argv is allowlisted upstream
    /// by `permission::Matcher` (see `tests/permission.rs` for negative
    /// coverage of denied argv shapes). This `run_argv` implementation never
    /// spawns a host interpreter — it spawns `bwrap` directly with the user
    /// argv.
    fn run_argv(
        &self,
        lane: &Lane,
        workspace: &Workspace,
        args: &ExecArgs,
        stdout_path: &Path,
        stderr_path: &Path,
    ) -> Result<ExecOutcome> {
        self.probe()?;
        let repo = workspace.root.join(&workspace.repo_subdir);
        let mut cmd = Command::new("bwrap");
        cmd.args([
            "--unshare-pid",
            "--unshare-ipc",
            "--unshare-uts",
            "--die-with-parent",
            "--new-session",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--ro-bind",
            "/usr",
            "/usr",
            "--ro-bind",
            "/lib",
            "/lib",
            "--ro-bind",
            "/lib64",
            "/lib64",
            "--ro-bind",
            "/bin",
            "/bin",
            "--ro-bind",
            "/sbin",
            "/sbin",
            "--ro-bind",
            "/etc",
            "/etc",
        ]);
        cmd.arg("--bind").arg(&repo).arg("/work");
        cmd.arg("--chdir").arg("/work");
        cmd.args(["--setenv", "HOME", "/work/.agent/home"]);
        cmd.args(["--setenv", "TMPDIR", "/work/.agent/tmp"]);
        cmd.args(["--setenv", "XDG_CACHE_HOME", "/work/.agent/cache"]);
        if lane.runtime.network == crate::spec::NetworkPolicy::None {
            cmd.arg("--unshare-net");
        }
        cmd.args(["--setenv", "LANG", "C.UTF-8"]);
        for key in &lane.commands.allowed_env {
            if let Some(v) = std::env::var(key).ok() {
                cmd.args(["--setenv", key, &v]);
            }
        }
        cmd.arg("--");
        cmd.args(&args.argv);
        super::common::run_argv_with_output(cmd, stdout_path, stderr_path, || {
            format!("bwrap run_argv for {:?}", args.argv)
        })
    }

    fn destroy(&self, workspace: &Workspace, keep_logs: bool) -> Result<()> {
        super::common::BackendDefaults::default_destroy(workspace, keep_logs)
    }
}
