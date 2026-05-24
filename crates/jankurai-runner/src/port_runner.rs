//! Durable generic ZYAL port workflow tick.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use jekko_store::db::Db;

use crate::classifier;
use crate::daemon_store;
use crate::events::{EventKind, EventSink};
use crate::jankurai_gate::{self, AuditSnapshot, JankuraiGatePolicy};
use crate::model_client::{ModelCallReceipt, ModelClient};
use crate::model_policy::ModelTaskKind;
use crate::port::{draft_master_plan, PortMasterPlan, PortRuntimeOptions, PortTargetRequest};
use crate::reasoning::AdvancedReasoningConfig;
use crate::reasoning_runner::{run_advanced_reasoning_tick_with_db, AdvancedReasoningSummary};
use crate::repo_graph::{build_repo_graph, RepoGraph};

/// Config accepted by `jankurai-runner port-run --config`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortRunConfig {
    /// Target request.
    #[serde(flatten)]
    pub target: PortTargetRequest,
    /// Whether fake worker receipts should be emitted.
    #[serde(default = "default_fake_worker")]
    pub fake_worker_cycle: bool,
    /// Whether a dirty tree is allowed.
    #[serde(default)]
    pub allow_dirty: bool,
    /// Advanced reasoning runtime config.
    #[serde(default)]
    pub advanced_reasoning: AdvancedReasoningConfig,
    /// Runtime proof options.
    #[serde(flatten)]
    pub runtime: PortRuntimeOptions,
}

/// One durable port tick report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortTickReport {
    /// Run id.
    pub run_id: String,
    /// Target id.
    pub target_id: String,
    /// Draft plan.
    pub plan: PortMasterPlan,
    /// Model receipt.
    pub model_receipt: ModelCallReceipt,
    /// Graph summary by kind.
    pub graph_summary: serde_json::Value,
    /// Fake task completed, if any.
    pub fake_task_completed: Option<String>,
    /// Advanced reasoning summary, when enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advanced_reasoning: Option<AdvancedReasoningSummary>,
}

/// Parse a JSON or TOML port config.
pub fn read_port_run_config(path: &Path) -> Result<PortRunConfig> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => serde_json::from_str(&text).context("parse JSON port run config"),
        Some("toml") => toml::from_str(&text).context("parse TOML port run config"),
        Some(ext) => {
            anyhow::bail!("unsupported port run config extension .{ext}; use .json or .toml")
        }
        None => anyhow::bail!("port run config path must end in .json or .toml"),
    }
}

/// Run one durable port workflow tick.
pub async fn run_port_tick(
    repo: &Path,
    run_id: &str,
    config: PortRunConfig,
    model_client: &dyn ModelClient,
) -> Result<PortTickReport> {
    let db = daemon_store::open_db(repo)?;
    run_port_tick_with_db(repo, run_id, config, model_client, &db).await
}

/// Run one durable port workflow tick with a caller-supplied DB handle.
pub async fn run_port_tick_with_db(
    repo: &Path,
    run_id: &str,
    config: PortRunConfig,
    model_client: &dyn ModelClient,
    db: &Db,
) -> Result<PortTickReport> {
    if !config.allow_dirty {
        assert_clean_tree(repo)?;
    }

    if config.advanced_reasoning.enabled {
        let report = run_advanced_reasoning_tick_with_db(
            repo,
            run_id,
            config.target.clone(),
            config.advanced_reasoning.clone(),
            config.runtime.clone(),
            config.fake_worker_cycle,
            model_client,
            db,
        )
        .await?;
        return Ok(PortTickReport {
            run_id: report.run_id,
            target_id: report.target_id,
            plan: report.plan,
            model_receipt: report.model_receipt,
            graph_summary: report.graph_summary,
            fake_task_completed: report.fake_task_completed,
            advanced_reasoning: Some(report.advanced),
        });
    }

    let sink = EventSink::open(repo, run_id)?;
    daemon_store::ensure_daemon_run(
        db,
        repo,
        run_id,
        daemon_store::port_spec_with_runtime(&config.target, &config.runtime),
    )?;
    sink.emit(
        EventKind::RunStarted,
        json!({
            "workflow": "zyal_port",
            "target": config.target.target,
            "replacement": config.target.replacement,
        }),
    )?;

    let graph = build_repo_graph(repo)?;
    daemon_store::persist_repo_graph(db, run_id, &graph)?;
    let graph_summary = graph_summary_json(&graph)?;

    sink.emit(
        EventKind::BrainstormStarted,
        json!({
            "worker_cap": config.target.effective_worker_cap(),
            "graph": graph_summary,
        }),
    )?;
    let prompt = planning_prompt(&config.target, &graph);
    let model_receipt = model_client
        .complete(ModelTaskKind::PhaseFinalize, &prompt, repo)
        .await?;
    daemon_store::persist_model_receipt(db, run_id, &model_receipt)?;
    sink.emit(
        EventKind::ModelOutcome,
        json!({
            "kind": model_receipt.kind,
            "provider": model_receipt.provider,
            "model": model_receipt.model,
            "success": model_receipt.success,
        }),
    )?;
    if !model_receipt.success {
        daemon_store::mark_daemon_run(
            db,
            run_id,
            "blocked",
            "model_planning",
            model_receipt.error.as_deref(),
        )?;
        return Err(anyhow!(
            "model planning failed: {}",
            model_receipt
                .error
                .as_deref()
                .unwrap_or("unknown model failure")
        ));
    }

    let plan = draft_master_plan(config.target.clone());
    daemon_store::persist_master_plan(db, run_id, &plan)?;
    sink.emit(
        EventKind::PhaseFinalized,
        json!({
            "stage_count": plan.stages.len(),
            "task_count": plan.tasks.len(),
        }),
    )?;

    let fake_task_completed = if config.fake_worker_cycle {
        let completed = daemon_store::persist_fake_worker_pass(db, run_id, &plan)?;
        if let Some(task_id) = &completed {
            sink.emit(
                EventKind::TaskAssigned,
                json!({"task_id": task_id, "worker_id": "fake-worker-1"}),
            )?;
            sink.emit(
                EventKind::WorkerStarted,
                json!({"task_id": task_id, "worker_id": "fake-worker-1"}),
            )?;
            sink.emit(
                EventKind::WorkerPass,
                json!({"task_id": task_id, "worker_id": "fake-worker-1"}),
            )?;
            sink.emit(
                EventKind::ProofPassed,
                json!({"task_id": task_id, "lane": "fake"}),
            )?;
        }
        completed
    } else {
        None
    };

    let audit = current_audit_snapshot(repo)?;
    jankurai_gate::check_gate(audit, audit, JankuraiGatePolicy::default())?;
    sink.emit(
        EventKind::AuditResult,
        json!({
            "score": audit.score,
            "hard_findings": audit.hard_findings,
            "caps": audit.caps,
            "status": "passed",
        }),
    )?;
    daemon_store::mark_daemon_run(db, run_id, "running", "phase_plan", None)?;

    Ok(PortTickReport {
        run_id: run_id.to_string(),
        target_id: daemon_store::target_id(run_id),
        plan,
        model_receipt,
        graph_summary,
        fake_task_completed,
        advanced_reasoning: None,
    })
}

fn planning_prompt(target: &PortTargetRequest, graph: &RepoGraph) -> String {
    format!(
        "Draft a generic port master plan.\nTarget: {}\nReplacement: {}\nRequest: {}\nGraph summary: {:?}",
        target.target,
        target.replacement,
        target.request,
        graph.summary(),
    )
}

fn current_audit_snapshot(repo: &Path) -> Result<AuditSnapshot> {
    let classify = classifier::classify(repo)?;
    Ok(AuditSnapshot {
        score: classify.score,
        hard_findings: classify.hard_total,
        caps: classify.caps_total,
    })
}

fn graph_summary_json(graph: &RepoGraph) -> Result<serde_json::Value> {
    serde_json::to_value(graph.summary()).context("serialize repo graph summary")
}

fn assert_clean_tree(repo: &Path) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
        .with_context(|| format!("git status in {}", repo.display()))?;
    if !output.status.success() {
        return Err(anyhow!("git status failed in {}", repo.display()));
    }
    if !output.stdout.is_empty() {
        return Err(anyhow!("working tree dirty; pass allow_dirty=true"));
    }
    Ok(())
}

fn default_fake_worker() -> bool {
    true
}

#[allow(dead_code)]
fn _pathbuf_send_sync(_: PathBuf) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap_check;
    use crate::model_client::FakeModelClient;
    use tempfile::tempdir;

    fn bootstrap_repo(dir: &Path) {
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(dir)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .status()
            .unwrap();
        for file in bootstrap_check::CANONICAL_FILES {
            let abs = dir.join(file.rel);
            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(abs, "").unwrap();
        }
        fs::create_dir_all(dir.join(".jankurai")).unwrap();
        fs::write(
            dir.join(".jankurai/repo-score.json"),
            r#"{"score": 95.0, "findings": [], "caps_applied": []}"#,
        )
        .unwrap();
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/lib.rs"), "pub fn ping() {}\n").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-q", "-m", "seed"])
            .current_dir(dir)
            .status()
            .unwrap();
    }

    fn config() -> PortRunConfig {
        PortRunConfig {
            target: PortTargetRequest {
                target: "MiniKV".into(),
                replacement: "MiniKV Rust".into(),
                target_repo: None,
                replacement_repo: None,
                request: "port MiniKV".into(),
                worker_cap: 4,
            },
            fake_worker_cycle: true,
            allow_dirty: false,
            advanced_reasoning: AdvancedReasoningConfig::default(),
            runtime: PortRuntimeOptions::default(),
        }
    }

    #[tokio::test]
    async fn fake_port_tick_persists_plan_events_and_worker_pass() {
        let dir = tempdir().unwrap();
        let db_dir = tempdir().unwrap();
        bootstrap_repo(dir.path());
        let db = Db::open(db_dir.path().join("jekko.db")).unwrap();
        let report = run_port_tick_with_db(
            dir.path(),
            "run-port-1",
            config(),
            &FakeModelClient::success("plan"),
            &db,
        )
        .await
        .unwrap();

        assert_eq!(report.plan.stages.len(), 5);
        assert_eq!(report.fake_task_completed.as_deref(), Some("task-discover"));
        let event_path = dir.path().join("target/zyal/runs/run-port-1/events.jsonl");
        let events = fs::read_to_string(event_path).unwrap();
        assert!(events.contains("model_outcome"));
        assert!(events.contains("worker_pass"));
    }

    #[test]
    fn reads_json_port_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("port.json");
        fs::write(
            &path,
            r#"{
              "target": "Reference",
              "replacement": "Candidate",
              "request": "port it",
              "worker_cap": 3
            }"#,
        )
        .unwrap();
        let config = read_port_run_config(&path).unwrap();
        assert_eq!(config.target.effective_worker_cap(), 3);
    }
}
