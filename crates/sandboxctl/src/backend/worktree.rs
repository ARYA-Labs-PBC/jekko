//! Worktree backend — cross-platform.
//!
//! Strategy: `git worktree add --detach <root>/workspace` from the parent repo,
//! private HOME/TMPDIR/XDG_CACHE_HOME, no kernel-level isolation. Permission
//! enforcement happens upstream in `permission::Matcher`; this backend is
//! responsible for the workspace + env, not for sandboxing syscalls.

use std::{
    ffi::OsString,
    fs::{self, File},
    io::Write,
    path::Path,
    process::Command,
};

use anyhow::{anyhow, Context, Result};

use super::{BackendImpl, ExecOutcome, Workspace};
use crate::spec::Lane;
use crate::wrapper::ExecArgs;

pub struct WorktreeBackend;

impl BackendImpl for WorktreeBackend {
    fn name(&self) -> &'static str {
        "worktree"
    }

    fn probe(&self) -> Result<()> {
        // `git --version` must succeed.
        let out = Command::new("git")
            .arg("--version")
            .output()
            .context("invoke `git --version`")?;
        if !out.status.success() {
            return Err(anyhow!("`git --version` failed"));
        }
        Ok(())
    }

    fn create(&self, _lane: &Lane, workspace: &Workspace) -> Result<()> {
        fs::create_dir_all(&workspace.root)
            .with_context(|| format!("mkdir {}", workspace.root.display()))?;
        for sub in ["home", "tmp", "cache", "logs"] {
            fs::create_dir_all(workspace.root.join(sub)).ok();
        }
        let repo = workspace.root.join(&workspace.repo_subdir);
        if repo.exists() {
            return Ok(());
        }
        let source = std::env::current_dir()?;
        let status = Command::new("git")
            .args(["worktree", "add", "--detach"])
            .arg(&repo)
            .current_dir(&source)
            .status()
            .with_context(|| format!("git worktree add → {}", repo.display()))?;
        if !status.success() {
            return Err(anyhow!("git worktree add returned {:?}", status.code()));
        }
        fs::create_dir_all(repo.join(".agent/runs")).ok();
        // Write a marker file the destroy path uses to recognise the worktree.
        let marker = workspace.root.join(".sandbox-meta");
        let mut f = File::create(&marker)
            .with_context(|| format!("create {}", marker.display()))?;
        writeln!(f, "run_id={}", workspace.run_id)?;
        writeln!(f, "backend=worktree")?;
        writeln!(f, "source={}", source.display())?;
        Ok(())
    }

    fn run_argv(
        &self,
        lane: &Lane,
        workspace: &Workspace,
        args: &ExecArgs,
        stdout_path: &Path,
        stderr_path: &Path,
    ) -> Result<ExecOutcome> {
        let repo = workspace.root.join(&workspace.repo_subdir);
        if args.argv.is_empty() {
            return Err(anyhow!("argv is empty"));
        }
        let mut cmd = Command::new(&args.argv[0]);
        cmd.args(&args.argv[1..]);
        cmd.current_dir(&repo);
        cmd.env_clear();
        for (k, v) in curated_env(lane, workspace) {
            cmd.env(k, v);
        }
        super::common::run_argv_with_output(cmd, stdout_path, stderr_path, || {
            format!("worktree run_argv for {:?}", args.argv)
        })
    }

    fn destroy(&self, workspace: &Workspace, keep_logs: bool) -> Result<()> {
        let repo = workspace.root.join(&workspace.repo_subdir);
        if repo.exists() {
            // Remove via git so the worktree registry stays consistent.
            let cwd = resolve_source_root(&workspace.root)?;
            let _ = Command::new("git")
                .args(["worktree", "remove", "--force"])
                .arg(&repo)
                .current_dir(&cwd)
                .status();
            if repo.exists() {
                fs::remove_dir_all(&repo).ok();
            }
        }
        if keep_logs {
            // leave logs/, .agent/runs/, patch.diff intact
            for sub in ["home", "tmp", "cache"] {
                let p = workspace.root.join(sub);
                if p.exists() {
                    fs::remove_dir_all(p).ok();
                }
            }
        } else {
            fs::remove_dir_all(&workspace.root).ok();
        }
        // `git worktree prune` reconciles abandoned registry entries.
        let _ = Command::new("git")
            .args(["worktree", "prune"])
            .status();
        Ok(())
    }
}

/// Resolve the source repo that owns this worktree. Reads the marker written
/// at `create` time; if absent (e.g. external invocation), falls through to
/// `cwd`. Returns a typed error only when both lookups fail.
fn resolve_source_root(root: &std::path::Path) -> Result<std::path::PathBuf> {
    if let Some(source) = read_source_line(root)? {
        return Ok(source);
    }
    std::env::current_dir().context("read current working directory for git worktree remove")
}

fn curated_env(lane: &Lane, workspace: &Workspace) -> Vec<(OsString, OsString)> {
    let home = expand_path(&lane.environment.home, &workspace.run_id, &workspace.root);
    let tmp = expand_path(&lane.environment.tmpdir, &workspace.run_id, &workspace.root);
    let cache = expand_path(&lane.environment.cache_home, &workspace.run_id, &workspace.root);
    fs::create_dir_all(&home).ok();
    fs::create_dir_all(&tmp).ok();
    fs::create_dir_all(&cache).ok();
    let host_path: OsString = match std::env::var_os("PATH") {
        Some(value) => value,
        None => safe_path(),
    };
    let mut out: Vec<(OsString, OsString)> = vec![
        ("HOME".into(), home.into_os_string()),
        ("TMPDIR".into(), tmp.into_os_string()),
        ("XDG_CACHE_HOME".into(), cache.into_os_string()),
        ("LANG".into(), "C.UTF-8".into()),
        ("PATH".into(), host_path),
        ("SANDBOXCTL_RUN_ID".into(), workspace.run_id.clone().into()),
    ];
    for key in &lane.commands.allowed_env {
        if let Some(v) = std::env::var_os(key) {
            out.push((key.into(), v));
        }
    }
    out
}

/// Minimal POSIX PATH used when the host hasn't set one. Matches the standard
/// `getconf PATH` baseline; documented here so the sandbox env stays
/// predictable across shells.
fn safe_path() -> OsString {
    "/usr/bin:/bin".into()
}

fn expand_path(template: &str, run_id: &str, sandbox_root: &Path) -> std::path::PathBuf {
    let parent = match sandbox_root.parent() {
        Some(p) => p.display().to_string(),
        None => String::new(),
    };
    let replaced = template
        .replace("{run_id}", run_id)
        .replace("{sandbox_root}", &parent);
    let resolved = expand_tilde(&replaced);
    std::path::PathBuf::from(resolved)
}

fn expand_tilde(input: &str) -> String {
    let Some(rest) = input.strip_prefix("~/") else {
        return input.to_string();
    };
    let Some(home) = std::env::var_os("HOME") else {
        return input.to_string();
    };
    std::path::PathBuf::from(home)
        .join(rest)
        .to_string_lossy()
        .into_owned()
}

fn read_source_line(root: &Path) -> Result<Option<std::path::PathBuf>> {
    let marker = root.join(".sandbox-meta");
    if !marker.exists() {
        return Ok(None);
    }
    let bytes = fs::read_to_string(&marker)?;
    for line in bytes.lines() {
        if let Some(rest) = line.strip_prefix("source=") {
            return Ok(Some(std::path::PathBuf::from(rest)));
        }
    }
    Ok(None)
}
