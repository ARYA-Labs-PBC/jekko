//! Command wrapper: captures stdout/stderr/exit/meta per `sandboxctl run`.

use std::{
    collections::VecDeque,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::backend::{self, Workspace};
use crate::spec::Lane;

#[derive(Debug, Clone)]
pub struct ExecArgs {
    pub argv: Vec<String>,
    pub timeout_seconds: u64,
    pub tail_lines: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunOutcome {
    pub cmd_id: String,
    pub exit_code: i32,
    pub duration_ms: u128,
    pub changed_files: u32,
    pub stdout_tail: String,
    pub stderr_tail: String,
    pub meta_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunMeta {
    pub cmd_id: String,
    pub ts_start: String,
    pub ts_end: Option<String>,
    pub cwd: PathBuf,
    pub argv: Vec<String>,
    pub lane: String,
    pub backend: String,
    pub run_id: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u128>,
    pub changed_files: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ExportReport {
    pub patch_path: PathBuf,
    pub hunks: u32,
    pub changed_files: u32,
    pub untracked_tar: Option<PathBuf>,
}

pub fn dispatch(
    resolved: &backend::Resolver,
    lane: &Lane,
    workspace: &Workspace,
    args: &ExecArgs,
) -> Result<RunOutcome> {
    let runs_dir = workspace.repo_dir().join(".agent/runs");
    fs::create_dir_all(&runs_dir).with_context(|| format!("mkdir {}", runs_dir.display()))?;
    let cmd_id = next_cmd_id(&runs_dir)?;

    let mut meta = RunMeta {
        cmd_id: cmd_id.clone(),
        ts_start: crate::runid::generate(),
        ts_end: None,
        cwd: workspace.repo_dir(),
        argv: args.argv.clone(),
        lane: lane.name.clone(),
        backend: resolved.name().to_string(),
        run_id: workspace.run_id.clone(),
        exit_code: None,
        duration_ms: None,
        changed_files: None,
    };
    write_meta(&runs_dir, &cmd_id, &meta)?;

    let stdout_path = runs_dir.join(format!("{cmd_id}.stdout"));
    let stderr_path = runs_dir.join(format!("{cmd_id}.stderr"));

    let started = Instant::now();
    let exec_outcome = resolved.run_argv(lane, workspace, args, &stdout_path, &stderr_path)?;
    let elapsed = started.elapsed().as_millis();

    let changed = count_changed_files(&workspace.repo_dir())?;
    meta.ts_end = Some(crate::runid::generate());
    meta.exit_code = Some(exec_outcome.exit_code);
    meta.duration_ms = Some(elapsed);
    meta.changed_files = Some(changed);
    write_meta(&runs_dir, &cmd_id, &meta)?;

    let stdout_tail = tail_lines(&stdout_path, args.tail_lines)?;
    let stderr_tail = tail_lines(&stderr_path, args.tail_lines)?;

    let meta_path = runs_dir.join(format!("{cmd_id}.meta"));
    Ok(RunOutcome {
        cmd_id,
        exit_code: exec_outcome.exit_code,
        duration_ms: elapsed,
        changed_files: changed,
        stdout_tail,
        stderr_tail,
        meta_path,
    })
}

pub fn record_denial(workspace: &Workspace, argv: &[String], pattern: &str) -> Result<()> {
    let runs_dir = workspace.repo_dir().join(".agent/runs");
    fs::create_dir_all(&runs_dir)?;
    let cmd_id = next_cmd_id(&runs_dir)?;
    let path = runs_dir.join(format!("{cmd_id}.denied"));
    let payload = serde_json::json!({
        "cmd_id": cmd_id,
        "ts": crate::runid::generate(),
        "argv": argv,
        "matched_pattern": pattern,
    });
    let mut f = File::create(path)?;
    f.write_all(serde_json::to_string_pretty(&payload)?.as_bytes())?;
    Ok(())
}

pub fn print_outcome(o: &RunOutcome) {
    if !o.stdout_tail.is_empty() {
        print!("{}", o.stdout_tail);
    }
    if !o.stderr_tail.is_empty() {
        eprint!("{}", o.stderr_tail);
    }
    eprintln!(
        "[sandboxctl exit={} cmd_id={} duration_ms={} changed={}]",
        o.exit_code, o.cmd_id, o.duration_ms, o.changed_files
    );
}

#[derive(Debug, Clone, Serialize)]
pub struct RunSummary {
    pub cmd_id: String,
    pub exit_code: i32,
    pub duration_ms: u128,
    pub changed_files: u32,
    pub argv: Vec<String>,
}

pub fn list_runs(workspace_root: &Path, last: usize) -> Result<Vec<RunSummary>> {
    let runs_dir = workspace_root.join("workspace/.agent/runs");
    if !runs_dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<_> = fs::read_dir(&runs_dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    let summaries: Vec<RunSummary> = entries
        .iter()
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("meta") {
                return None;
            }
            let mut buf = String::new();
            File::open(&path).ok()?.read_to_string(&mut buf).ok()?;
            let meta: RunMeta = serde_json::from_str(&buf).ok()?;
            Some(RunSummary {
                cmd_id: meta.cmd_id,
                exit_code: meta.exit_code.unwrap_or(-1),
                duration_ms: meta.duration_ms.unwrap_or(0),
                changed_files: meta.changed_files.unwrap_or(0),
                argv: meta.argv,
            })
        })
        .collect();
    let start = summaries.len().saturating_sub(last);
    Ok(summaries[start..].to_vec())
}

pub fn export_patch(
    workspace_root: &Path,
    patch_path: &Path,
    include_untracked: bool,
) -> Result<ExportReport> {
    let repo = workspace_root.join("workspace");
    if let Some(parent) = patch_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let out = std::process::Command::new("git")
        .args(["diff", "HEAD", "--patch", "--no-color"])
        .current_dir(&repo)
        .output()
        .with_context(|| format!("git diff in {}", repo.display()))?;
    let bytes = out.stdout;
    let mut f = File::create(patch_path).with_context(|| format!("create {}", patch_path.display()))?;
    f.write_all(&bytes)?;
    let hunks = bytes.iter().filter(|b| **b == b'@').count() / 2;
    let changed_files = count_changed_files(&repo)?;
    let untracked_tar = if include_untracked {
        let tar_path = patch_path.with_extension("untracked.tar");
        let status = std::process::Command::new("bash")
            .arg("-c")
            .arg(format!(
                "git -C {} ls-files --others --exclude-standard -z | xargs -0 -I {{}} tar --append --file {} {{}}",
                repo.display(),
                tar_path.display()
            ))
            .status();
        match status {
            Ok(s) if s.success() && tar_path.exists() => Some(tar_path),
            _ => None,
        }
    } else {
        None
    };
    Ok(ExportReport {
        patch_path: patch_path.to_path_buf(),
        hunks: hunks as u32,
        changed_files,
        untracked_tar,
    })
}

fn next_cmd_id(runs_dir: &Path) -> Result<String> {
    let mut max = 0u32;
    if let Ok(entries) = fs::read_dir(runs_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if let Some(stem) = name.split('.').next() {
                    if let Ok(n) = stem.parse::<u32>() {
                        if n > max {
                            max = n;
                        }
                    }
                }
            }
        }
    }
    Ok(format!("{:05}", max + 1))
}

fn write_meta(runs_dir: &Path, cmd_id: &str, meta: &RunMeta) -> Result<()> {
    let path = runs_dir.join(format!("{cmd_id}.meta"));
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    f.write_all(serde_json::to_string_pretty(meta)?.as_bytes())?;
    Ok(())
}

fn tail_lines(path: &Path, n: u32) -> Result<String> {
    if !path.exists() || n == 0 {
        return Ok(String::new());
    }
    let mut buf = String::new();
    File::open(path)?.read_to_string(&mut buf)?;
    let mut ring: VecDeque<&str> = VecDeque::with_capacity(n as usize);
    for line in buf.lines() {
        if ring.len() == n as usize {
            ring.pop_front();
        }
        ring.push_back(line);
    }
    let mut out = String::new();
    for line in ring {
        out.push_str(line);
        out.push('\n');
    }
    Ok(out)
}

fn count_changed_files(repo: &Path) -> Result<u32> {
    let out = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output();
    let Ok(out) = out else {
        return Ok(0);
    };
    if !out.status.success() {
        return Ok(0);
    }
    let s = String::from_utf8_lossy(&out.stdout);
    Ok(s.lines().count() as u32)
}

pub trait WorkspaceLayout {
    fn repo_dir(&self) -> PathBuf;
}

impl WorkspaceLayout for Workspace {
    fn repo_dir(&self) -> PathBuf {
        self.root.join(&self.repo_subdir)
    }
}
