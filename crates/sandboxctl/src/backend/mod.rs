//! Backend trait + dispatch. Each backend lives in its own module:
//! - `worktree` — cross-platform; git worktree + fake env
//! - `bubblewrap` — Linux-only kernel-namespace isolation
//! - `docker` — cross-platform container isolation
//!
//! `Resolver` picks one based on the lane's `runtime.backend` field (or a
//! caller override) and exposes a unified `create / run_argv / destroy / probe`
//! surface.

pub mod bubblewrap;
pub mod common;
pub mod docker;
pub mod worktree;

use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::spec::{Backend, Lane};
use crate::wrapper::ExecArgs;

#[derive(Debug, Clone)]
pub struct Workspace {
    pub run_id: String,
    pub root: PathBuf,
    pub repo_subdir: PathBuf,
}

pub struct ExecOutcome {
    pub exit_code: i32,
}

pub trait BackendImpl {
    fn name(&self) -> &'static str;
    fn probe(&self) -> Result<()>;
    fn create(&self, lane: &Lane, workspace: &Workspace) -> Result<()>;
    fn run_argv(
        &self,
        lane: &Lane,
        workspace: &Workspace,
        args: &ExecArgs,
        stdout_path: &std::path::Path,
        stderr_path: &std::path::Path,
    ) -> Result<ExecOutcome>;
    fn destroy(&self, workspace: &Workspace, keep_logs: bool) -> Result<()>;
}

pub struct Resolver {
    inner: Box<dyn BackendImpl>,
}

impl Resolver {
    pub fn for_lane(lane: &Lane, override_name: Option<&str>) -> Result<Self> {
        let name = override_name.unwrap_or(match lane.runtime.backend {
            Backend::Worktree => "worktree",
            Backend::Bubblewrap => "bubblewrap",
            Backend::Docker => "docker",
            Backend::Podman => "podman",
        });
        Self::for_backend(name)
    }

    pub fn for_backend(name: &str) -> Result<Self> {
        let inner: Box<dyn BackendImpl> = match name {
            "worktree" => Box::new(worktree::WorktreeBackend),
            "bubblewrap" => Box::new(bubblewrap::BubblewrapBackend),
            "docker" => Box::new(docker::DockerBackend::new("docker")),
            "podman" => Box::new(docker::DockerBackend::new("podman")),
            other => return Err(anyhow!("unknown backend '{other}'")),
        };
        Ok(Self { inner })
    }

    pub fn name(&self) -> &'static str {
        self.inner.name()
    }

    pub fn probe(&self) -> Result<()> {
        self.inner.probe()
    }

    pub fn create(&self, lane: &Lane, workspace: &Workspace) -> Result<()> {
        self.inner.create(lane, workspace)
    }

    pub fn run_argv(
        &self,
        lane: &Lane,
        workspace: &Workspace,
        args: &ExecArgs,
        stdout_path: &std::path::Path,
        stderr_path: &std::path::Path,
    ) -> Result<ExecOutcome> {
        self.inner.run_argv(lane, workspace, args, stdout_path, stderr_path)
    }

    pub fn destroy(&self, workspace: &Workspace, keep_logs: bool) -> Result<()> {
        self.inner.destroy(workspace, keep_logs)
    }
}
