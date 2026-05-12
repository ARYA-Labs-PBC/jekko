//! Main tick loop. Each tick:
//!   1. Re-run `jankurai audit` (skipped in dry-run mode).
//!   2. Classify findings (`classifier::classify`).
//!   3. Build path-overlap DAG → wave-by-wave schedule (`dag::schedule`).
//!   4. Acquire locks per batch (`locks::try_lock_all`).
//!   5. Dispatch workers in isolated worktrees (out of scope for PR3 — the
//!      worker process itself lands in PR4 when the daemon-TS bridge is wired
//!      in).
//!   6. Commit on green / rollback on red.
//!   7. Emit events to the NDJSON sink + receipts SQLite.
//!
//! PR3 ships the tick orchestration in dry-run mode (no `jankurai audit`
//! spawn, no real workers). The full live mode lands once PR4 owns the
//! worker lifecycle.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use serde_json::json;

use crate::bootstrap_check;
use crate::classifier::{ClassifyResult, classify};
use crate::dag::{Wave, schedule};
use crate::events::{EventKind, EventSink};
use crate::receipts::ReceiptsStore;

#[derive(Debug, Clone)]
pub struct RunnerConfig {
    pub repo: PathBuf,
    pub run_id: String,
    pub pool_size: usize,
    pub integration_branch: Option<String>,
    pub allow_dirty: bool,
    pub dry_run: bool,
}

impl RunnerConfig {
    pub fn integration_branch(&self) -> String {
        self.integration_branch
            .clone()
            .unwrap_or_else(|| format!("zyal/{}/integration", self.run_id))
    }
}

#[derive(Debug, Clone)]
pub struct TickReport {
    pub iteration: u64,
    pub classify: ClassifyResult,
    pub waves: Vec<Wave>,
    pub events_emitted: usize,
}

pub async fn run_once(config: &RunnerConfig) -> Result<i32> {
    let report = run_tick(config, 1).await?;
    if report.classify.findings.is_empty() {
        println!("jankurai-runner: green (no findings)");
        Ok(0)
    } else {
        println!(
            "jankurai-runner: {} finding(s) across {} wave(s) (caps={}, hard={}, soft={})",
            report.classify.findings.len(),
            report.waves.len(),
            report.classify.caps_total,
            report.classify.hard_total,
            report.classify.soft_total,
        );
        Ok(0)
    }
}

pub async fn run_forever(config: &RunnerConfig) -> Result<i32> {
    let mut iteration: u64 = 0;
    loop {
        iteration += 1;
        let report = run_tick(config, iteration).await?;
        if report.classify.findings.is_empty() {
            println!("jankurai-runner: green at iter {iteration}, exiting");
            return Ok(0);
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

pub async fn run_tick(config: &RunnerConfig, iteration: u64) -> Result<TickReport> {
    // Precondition mirrors the CLI check, but we re-validate here in case the
    // canonical surface drifted under us mid-run.
    let readiness = bootstrap_check::is_ready(&config.repo);
    if !readiness.ok {
        let sink = EventSink::open(&config.repo, &config.run_id)?;
        sink.emit(
            EventKind::BootstrapRequired,
            json!({"missing": readiness.missing_required}),
        )?;
        return Err(anyhow!(
            "bootstrap precondition failed: {} required canonical file(s) missing",
            readiness.missing_required.len()
        ));
    }

    if !config.allow_dirty {
        assert_clean_tree(&config.repo)?;
    }

    let sink = EventSink::open(&config.repo, &config.run_id)?;
    let receipts = ReceiptsStore::open(&config.repo)?;
    if iteration == 1 {
        receipts.record_run_started(
            &config.run_id,
            config.pool_size,
            &config.integration_branch(),
            config.dry_run,
        )?;
        sink.emit(
            EventKind::RunStarted,
            json!({
                "pool_size": config.pool_size,
                "integration_branch": config.integration_branch(),
                "dry_run": config.dry_run,
            }),
        )?;
    }

    if !config.dry_run {
        run_jankurai_audit(&config.repo)?;
    }

    let classify = classify(&config.repo)?;
    let waves = schedule(&classify.findings);

    let mut events_emitted = 1usize; // RunStarted on iteration 1 counted already
    // Persist a finding snapshot per tick so the operator can replay
    // classifications offline.
    for finding in &classify.findings {
        receipts.record_finding(
            &config.run_id,
            &finding.fingerprint,
            &finding.rule_id,
            severity_label(finding.severity),
            &finding.paths,
            finding.cap.as_deref(),
        )?;
    }

    // PR3 stops short of actually dispatching workers — the worker lifecycle
    // is owned by PR4's daemon-TS bridge. We log the planned schedule so the
    // daemon can pick it up later.
    if !waves.is_empty() {
        events_emitted += 1;
        sink.emit(
            EventKind::WorkerStarted,
            json!({
                "iteration": iteration,
                "wave_count": waves.len(),
                "batch_count": waves.iter().map(|w| w.batches.len()).sum::<usize>(),
                "caps": classify.caps_total,
            }),
        )?;
    }

    if classify.findings.is_empty() {
        sink.emit(
            EventKind::RunFinished,
            json!({"iteration": iteration, "reason": "no_findings"}),
        )?;
        events_emitted += 1;
        receipts.record_run_finished(&config.run_id, "green")?;
    }

    Ok(TickReport {
        iteration,
        classify,
        waves,
        events_emitted,
    })
}

fn assert_clean_tree(repo: &std::path::Path) -> Result<()> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
        .with_context(|| format!("git status in {}", repo.display()))?;
    if !output.status.success() {
        return Err(anyhow!("git status failed in {}", repo.display()));
    }
    if !output.stdout.is_empty() {
        return Err(anyhow!(
            "working tree dirty; pass --allow-dirty to stash or commit first"
        ));
    }
    Ok(())
}

fn run_jankurai_audit(repo: &std::path::Path) -> Result<()> {
    let status = Command::new("jankurai")
        .args([
            "audit",
            ".",
            "--mode",
            "advisory",
            "--json",
            "target/jankurai/repo-score.json",
            "--no-score-history",
        ])
        .current_dir(repo)
        .status()
        .context("spawn jankurai audit")?;
    if !status.success() {
        return Err(anyhow!("jankurai audit exited with {}", status.code().unwrap_or(-1)));
    }
    Ok(())
}

fn severity_label(s: crate::classifier::Severity) -> &'static str {
    use crate::classifier::Severity::*;
    match s {
        Critical => "critical",
        High => "high",
        Medium => "medium",
        Low => "low",
        Info => "info",
    }
}

/// Random run-id: UTC compact timestamp + 6-byte random suffix. Sortable so
/// `ls -1 .zyal/worktrees/` walks runs in chronological order.
pub fn random_run_id() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = now.as_secs();
    let nanos = now.subsec_nanos();
    // Six hex chars derived from the nano portion give us a low-collision
    // suffix without pulling rand. Two runs started in the same nanosecond
    // would still collide, but that is fine for human-scale operations.
    format!(
        "{:08x}-{:06x}",
        secs as u32,
        nanos & 0x00ff_ffff
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn bootstrap_repo(dir: &std::path::Path) {
        // Initialize a real git repo so `git status` returns clean exit 0.
        Command::new("git").args(["init", "-q"]).current_dir(dir).status().unwrap();
        Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(dir).status().unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(dir).status().unwrap();
        for file in bootstrap_check::CANONICAL_FILES {
            let abs = dir.join(file.rel);
            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(abs, "").unwrap();
        }
        fs::write(
            dir.join("agent/audit-policy.toml"),
            r#"[decision]
min_score = 85
fail_on = ["critical", "high"]
advisory_on = ["medium", "low"]
"#,
        )
        .unwrap();
        fs::write(
            dir.join("agent/repo-score.json"),
            r#"{"score": 95.0, "findings": [], "caps_applied": []}"#,
        )
        .unwrap();
        Command::new("git").args(["add", "."]).current_dir(dir).status().unwrap();
        Command::new("git").args(["commit", "-q", "-m", "seed"]).current_dir(dir).status().unwrap();
    }

    #[tokio::test]
    async fn dry_run_tick_on_green_repo_emits_run_started_and_finished() {
        let dir = tempdir().unwrap();
        bootstrap_repo(dir.path());
        let config = RunnerConfig {
            repo: dir.path().to_path_buf(),
            run_id: "test-run".to_string(),
            pool_size: 2,
            integration_branch: None,
            allow_dirty: false,
            dry_run: true,
        };
        let report = run_tick(&config, 1).await.unwrap();
        assert!(report.classify.findings.is_empty());
        assert!(report.waves.is_empty());
        let events_path = dir.path().join("agent/zyal/runner-events.jsonl");
        let text = fs::read_to_string(&events_path).unwrap();
        assert!(text.contains("run_started"));
        assert!(text.contains("run_finished"));
    }

    #[tokio::test]
    async fn dry_run_tick_schedules_findings_into_waves() {
        let dir = tempdir().unwrap();
        bootstrap_repo(dir.path());
        fs::write(
            dir.path().join("agent/repo-score.json"),
            r#"{
                "score": 60.0,
                "findings": [
                    {"rule_id": "HLT-001", "fingerprint": "fp1", "severity": "low", "path": "src/a.rs"},
                    {"rule_id": "HLT-002", "fingerprint": "fp2", "severity": "low", "path": "src/b.rs"}
                ],
                "caps_applied": []
            }"#,
        )
        .unwrap();
        // Modifying repo-score.json after the bootstrap commit dirties the
        // tree; pass --allow-dirty so the tick proceeds against the new
        // classification instead of refusing to start.
        let config = RunnerConfig {
            repo: dir.path().to_path_buf(),
            run_id: "test-run-2".to_string(),
            pool_size: 2,
            integration_branch: None,
            allow_dirty: true,
            dry_run: true,
        };
        let report = run_tick(&config, 1).await.unwrap();
        assert_eq!(report.classify.findings.len(), 2);
        assert_eq!(report.waves.len(), 1);
        let receipts_path = dir.path().join("agent/zyal/receipts.sqlite");
        assert!(receipts_path.exists());
    }

    #[tokio::test]
    async fn dirty_tree_aborts_unless_allow_dirty() {
        let dir = tempdir().unwrap();
        bootstrap_repo(dir.path());
        fs::write(dir.path().join("untracked.txt"), "junk").unwrap();
        let config = RunnerConfig {
            repo: dir.path().to_path_buf(),
            run_id: "test-run-3".to_string(),
            pool_size: 2,
            integration_branch: None,
            allow_dirty: false,
            dry_run: true,
        };
        let err = run_tick(&config, 1).await.unwrap_err();
        assert!(err.to_string().contains("dirty"));
    }

    #[tokio::test]
    async fn allow_dirty_proceeds_through_tick() {
        let dir = tempdir().unwrap();
        bootstrap_repo(dir.path());
        fs::write(dir.path().join("untracked.txt"), "junk").unwrap();
        let config = RunnerConfig {
            repo: dir.path().to_path_buf(),
            run_id: "test-run-4".to_string(),
            pool_size: 2,
            integration_branch: None,
            allow_dirty: true,
            dry_run: true,
        };
        let report = run_tick(&config, 1).await.unwrap();
        assert!(report.classify.findings.is_empty());
    }

    #[test]
    fn random_run_id_is_lex_sortable_within_a_second() {
        let a = random_run_id();
        std::thread::sleep(Duration::from_millis(5));
        let b = random_run_id();
        // a and b might collide in nanos at high resolution, but should sort
        // by secs prefix which is monotonic.
        let a_secs = &a[..8];
        let b_secs = &b[..8];
        assert!(a_secs.le(b_secs));
    }
}
