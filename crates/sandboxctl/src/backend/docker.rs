// jankurai:allow HLT-000-SCORE-DIMENSION reason=parallel-trait-impl-with-bubblewrap-by-design-shared-defaults-hoisted-to-BackendDefaults expires=2027-12-01
//! Docker / Podman backend. Cross-platform via Docker Desktop or Podman.
//!
//! Implementation calls the `docker`/`podman` CLI directly (no bollard
//! dependency) so the crate stays light and works with either engine.

use anyhow::{Context, Result};
use crate::spec::{Lane, NetworkPolicy};
use std::path::Path;
use std::process::Command;

use crate::wrapper::ExecArgs;

use super::{BackendImpl, ExecOutcome, Workspace};

pub struct DockerBackend {
    binary: &'static str,
}

impl DockerBackend {
    pub fn new(binary: &'static str) -> Self {
        Self { binary }
    }
}

impl BackendImpl for DockerBackend {
    fn name(&self) -> &'static str {
        // Static name for index registration; the binary is still tracked
        // internally so we can dispatch to docker vs podman.
        if self.binary == "podman" {
            "podman"
        } else {
            "docker"
        }
    }

    fn probe(&self) -> Result<()> {
        super::common::probe_binary_subcommand(self.binary, "info")
    }

    fn create(&self, lane: &Lane, workspace: &Workspace) -> Result<()> {
        super::common::BackendDefaults::default_create(lane, workspace)
    }

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
        let image = lane
            .runtime
            .image
            .clone()
            .context("runtime.image is required for docker/podman backends")?;
        let container_name = format!("sandboxctl-{}", workspace.run_id);
        let mut cmd = Command::new(self.binary);
        cmd.args([
            "run",
            "--rm",
            "--read-only",
            "--init",
            "--label",
            "sandboxctl=1",
            "--name",
            &container_name,
        ]);
        cmd.arg("--volume")
            .arg(format!("{}:/work:rw", repo.display()));
        cmd.arg("--workdir").arg("/work");
        cmd.arg("--memory").arg(&lane.runtime.memory_limit);
        cmd.arg("--cpus").arg(&lane.runtime.cpu_limit);
        cmd.args(["--pids-limit", "256"]);
        cmd.args(["--tmpfs", "/tmp"]);
        match lane.runtime.network {
            NetworkPolicy::None => {
                cmd.args(["--network", "none"]);
            }
            NetworkPolicy::Bridge => {
                cmd.args(["--network", "bridge"]);
            }
            NetworkPolicy::Host => {
                cmd.args(["--network", "host"]);
            }
        }
        cmd.args(["--env", "HOME=/work/.agent/home"]);
        cmd.args(["--env", "TMPDIR=/work/.agent/tmp"]);
        cmd.args(["--env", "XDG_CACHE_HOME=/work/.agent/cache"]);
        cmd.args(["--env", "LANG=C.UTF-8"]);
        for key in &lane.commands.allowed_env {
            if let Some(v) = std::env::var_os(key) {
                cmd.arg("--env")
                    .arg(format!("{key}={}", v.to_string_lossy()));
            }
        }
        cmd.arg(image);
        cmd.args(&args.argv);
        super::common::run_argv_with_output(cmd, stdout_path, stderr_path, || {
            format!("{} run_argv for {:?}", self.binary, args.argv)
        })
    }

    fn destroy(&self, workspace: &Workspace, keep_logs: bool) -> Result<()> {
        let container_name = format!("sandboxctl-{}", workspace.run_id);
        let _ = Command::new(self.binary)
            .args(["rm", "-f", "-v", &container_name])
            .status();
        super::common::BackendDefaults::default_destroy(workspace, keep_logs)
    }
}
