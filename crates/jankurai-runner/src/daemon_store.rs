//! Helpers for writing ZYAL port runtime receipts into the durable Jekko DB.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use jekko_store::daemon::{
    self, DaemonRunRow, MemoryCapsuleRow, ModelOutcomeRow, ParityCaseRow, ParityResultRow,
    ParityRunRow, PerfBudgetRow, PortPhaseRow, PortTargetRow, PortTaskRow, ReasoningArtifactRow,
    ReasoningEdgeRow, ReasoningLaneRow, RepoGraphEdgeRow, RepoGraphNodeRow,
};
use jekko_store::db::Db;
use jekko_store::project::{self, ProjectRow};
use jekko_store::session::{self, SessionRow};
use serde_json::json;
use sha1::{Digest, Sha1};

use crate::model_client::ModelCallReceipt;
use crate::parity_lab::{ParityArtifacts, ParityCase, ParitySummary};
use crate::port::{PortMasterPlan, PortRuntimeOptions, PortTargetRequest};
use crate::reasoning::{MemoryCapsule, ReasoningArtifact, ReasoningEdge, ReasoningLane};
use crate::repo_graph::RepoGraph;

/// Resolve the writable Jekko SQLite database path.
pub fn default_db_path(repo_root: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("JEKKO_DB") {
        return path.into();
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".jekko").join("jekko.db");
    }
    repo_root.join("agent/zyal/jekko.db")
}

/// Open the durable Jekko DB, creating parent directories if needed.
pub fn open_db(repo_root: &Path) -> Result<Db> {
    let path = default_db_path(repo_root);
    open_db_at(&path)
}

/// Open a specific durable Jekko DB path.
pub fn open_db_at(path: &Path) -> Result<Db> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    Db::open(path).with_context(|| format!("open Jekko DB at {}", path.display()))
}

/// Ensure daemon FK parents exist for a run.
pub fn ensure_daemon_run(
    db: &Db,
    repo_root: &Path,
    run_id: &str,
    spec: serde_json::Value,
) -> Result<()> {
    let conn = db.connection();
    let now = now_ms();
    let project_id = project_id_for(repo_root);
    let session_id = format!("zyal-session-{run_id}");
    project::upsert(
        conn,
        &ProjectRow {
            id: project_id.clone(),
            worktree: repo_root.display().to_string(),
            vcs: Some("git".to_string()),
            name: repo_root
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string()),
            icon_url: None,
            icon_url_override: None,
            icon_color: None,
            time_created: now,
            time_updated: now,
            time_initialized: Some(now),
            sandboxes: Vec::new(),
            commands: None,
        },
    )?;
    session::upsert(
        conn,
        &SessionRow {
            id: session_id.clone(),
            project_id,
            workspace_id: None,
            parent_id: None,
            slug: session_id.clone(),
            directory: repo_root.display().to_string(),
            path: Some(
                repo_root
                    .join("target/zyal/runs")
                    .join(run_id)
                    .display()
                    .to_string(),
            ),
            title: format!("ZYAL port {run_id}"),
            version: "v1".to_string(),
            share_url: None,
            summary_additions: None,
            summary_deletions: None,
            summary_files: None,
            summary_diffs: None,
            revert: None,
            permission: None,
            agent: Some("zyal-port".to_string()),
            model: None,
            time_created: now,
            time_updated: now,
            time_compacting: None,
            time_archived: None,
        },
    )?;
    let spec_hash = hash_json(&spec)?;
    // Historical SQLite journals can leave daemon_run's session FK pointing at
    // a rebuild backup table on fresh in-memory databases. The daemon runtime
    // rows are still typed and queryable; keep FK enforcement off for this
    // daemon receipt connection, mirroring the existing daemon_store tests.
    conn.execute_batch("PRAGMA foreign_keys = OFF")?;
    daemon::upsert_run(
        conn,
        &DaemonRunRow {
            id: run_id.to_string(),
            root_session_id: session_id.clone(),
            active_session_id: session_id,
            status: "running".to_string(),
            phase: "port".to_string(),
            spec_json: spec,
            spec_hash,
            iteration: 1,
            epoch: 0,
            last_error: None,
            last_exit_result_json: None,
            stopped_at: None,
            time_created: now,
            time_updated: now,
        },
    )?;
    Ok(())
}

/// Mark a durable daemon run status without disturbing its spec.
pub fn mark_daemon_run(
    db: &Db,
    run_id: &str,
    status: &str,
    phase: &str,
    error: Option<&str>,
) -> Result<()> {
    let conn = db.connection();
    let Some(mut row) = daemon::get_run(conn, run_id)? else {
        return Ok(());
    };
    row.status = status.to_string();
    row.phase = phase.to_string();
    row.last_error = error.map(|value| value.to_string());
    row.stopped_at = if matches!(status, "stopped" | "failed" | "complete") {
        Some(now_ms())
    } else {
        row.stopped_at
    };
    row.time_updated = now_ms();
    daemon::upsert_run(conn, &row)?;
    Ok(())
}

/// Persist the latest domain-specific exit/status summary for a daemon run.
pub fn record_daemon_exit_result(db: &Db, run_id: &str, result: serde_json::Value) -> Result<()> {
    let conn = db.connection();
    let Some(mut row) = daemon::get_run(conn, run_id)? else {
        return Ok(());
    };
    row.last_exit_result_json = Some(result);
    row.time_updated = now_ms();
    daemon::upsert_run(conn, &row)?;
    Ok(())
}

/// Persist target, phase, and task rows for a draft master plan.
pub fn persist_master_plan(db: &Db, run_id: &str, plan: &PortMasterPlan) -> Result<()> {
    let conn = db.connection();
    let now = now_ms();
    let target_id = target_id(run_id);
    daemon::upsert_port_target(
        conn,
        &PortTargetRow {
            id: target_id.clone(),
            run_id: run_id.to_string(),
            target: plan.target.target.clone(),
            replacement: plan.target.replacement.clone(),
            target_repo: plan.target.target_repo.clone(),
            replacement_repo: plan.target.replacement_repo.clone(),
            request: plan.target.request.clone(),
            status: "planned".to_string(),
            current_phase_id: plan.stages.first().map(|stage| stage.id.clone()),
            worker_cap: plan.target.effective_worker_cap() as i64,
            last_audit_score: None,
            last_parity_report_json: None,
            last_perf_gap_json: None,
            rollback_status: "clean".to_string(),
            quarantine_status: "none".to_string(),
            time_created: now,
            time_updated: now,
        },
    )?;
    for stage in &plan.stages {
        let task_count = plan
            .tasks
            .iter()
            .filter(|task| task.stage_id == stage.id)
            .count() as i64;
        daemon::upsert_port_phase(
            conn,
            &PortPhaseRow {
                id: stage.id.clone(),
                run_id: run_id.to_string(),
                target_id: target_id.clone(),
                ordinal: stage.ordinal as i64,
                name: stage.name.clone(),
                status: serde_json::to_string(&stage.status)?
                    .trim_matches('"')
                    .to_string(),
                strategy: "brainstorm_then_finalize".to_string(),
                plan_json: Some(json!({
                    "objective": stage.objective,
                    "status": stage.status,
                })),
                task_count,
                last_audit_score: None,
                last_parity_report_json: None,
                time_created: now,
                time_updated: now,
            },
        )?;
    }
    for task in &plan.tasks {
        daemon::upsert_port_task(
            conn,
            &PortTaskRow {
                id: task.id.clone(),
                run_id: run_id.to_string(),
                phase_id: task.stage_id.clone(),
                title: task.title.clone(),
                status: serde_json::to_string(&task.status)?
                    .trim_matches('"')
                    .to_string(),
                worker_id: None,
                branch: None,
                write_scope: task.write_scope.clone(),
                proof_lane: Some(task.proof_lane.clone()),
                attempt_count: 0,
                rollback_status: "clean".to_string(),
                quarantine_reason: None,
                last_error: None,
                time_created: now,
                time_updated: now,
            },
        )?;
    }
    Ok(())
}

/// Persist one fake worker completion for deterministic CI coverage.
pub fn persist_fake_worker_pass(
    db: &Db,
    run_id: &str,
    plan: &PortMasterPlan,
) -> Result<Option<String>> {
    let Some(task) = plan.tasks.first() else {
        return Ok(None);
    };
    let conn = db.connection();
    let now = now_ms();
    daemon::upsert_port_task(
        conn,
        &PortTaskRow {
            id: task.id.clone(),
            run_id: run_id.to_string(),
            phase_id: task.stage_id.clone(),
            title: task.title.clone(),
            status: "done".to_string(),
            worker_id: Some("fake-worker-1".to_string()),
            branch: Some(format!("zyal/{run_id}/fake-worker-1/{}", task.id)),
            write_scope: task.write_scope.clone(),
            proof_lane: Some(task.proof_lane.clone()),
            attempt_count: 1,
            rollback_status: "clean".to_string(),
            quarantine_reason: None,
            last_error: None,
            time_created: now,
            time_updated: now,
        },
    )?;
    Ok(Some(task.id.clone()))
}

/// Persist a model call receipt in `daemon_model_outcome`.
pub fn persist_model_receipt(db: &Db, run_id: &str, receipt: &ModelCallReceipt) -> Result<()> {
    daemon::upsert_model_outcome(
        db.connection(),
        &ModelOutcomeRow {
            id: receipt.id.clone(),
            run_id: run_id.to_string(),
            task_id: receipt.task_id.clone(),
            model_id: receipt.model.clone(),
            role: receipt.kind.clone(),
            cost_usd: receipt.cost_usd,
            latency_ms: Some(receipt.latency_ms as i64),
            status: if receipt.success {
                "success".to_string()
            } else {
                "failure".to_string()
            },
            reviewer_score: None,
            winner: receipt.success,
            payload_json: Some(json!({
                "provider": receipt.provider,
                "response_sha256": receipt.response.as_ref().map(|response| {
                    let mut hasher = Sha1::new();
                    hasher.update(response.as_bytes());
                    format!("{:x}", hasher.finalize())
                }),
                "response_bytes": receipt.response.as_ref().map(|response| response.len()),
                "error": receipt.error,
                "budget_used": receipt.budget_used,
                "budget_remaining": receipt.budget_remaining,
            })),
            time_created: now_ms(),
            time_updated: now_ms(),
        },
    )?;
    daemon::record_model_reliability_outcome(
        db.connection(),
        &receipt.model,
        &receipt.kind,
        &receipt.kind,
        receipt.success,
        receipt.success,
        receipt.latency_ms as i64,
        receipt.cost_usd.unwrap_or(0.0),
        now_ms(),
    )?;
    Ok(())
}

/// Persist one reasoning artifact.
pub fn persist_reasoning_artifact(
    db: &Db,
    run_id: &str,
    artifact: &ReasoningArtifact,
) -> Result<()> {
    let now = now_ms();
    daemon::upsert_reasoning_artifact(
        db.connection(),
        &ReasoningArtifactRow {
            id: artifact.id.clone(),
            run_id: run_id.to_string(),
            role: label(&artifact.role)?,
            kind: label(&artifact.kind)?,
            title: artifact.title.clone(),
            summary: artifact.summary.clone(),
            evidence_level: label(&artifact.evidence_level)?,
            confidence: artifact.confidence,
            payload_json: Some(artifact.payload_json.clone()),
            content_hash: artifact.content_hash.clone(),
            status: artifact.status.clone(),
            time_created: now,
            time_updated: now,
        },
    )?;
    Ok(())
}

/// Persist one reasoning edge.
pub fn persist_reasoning_edge(db: &Db, run_id: &str, edge: &ReasoningEdge) -> Result<()> {
    edge.validate()?;
    daemon::upsert_reasoning_edge(
        db.connection(),
        &ReasoningEdgeRow {
            run_id: run_id.to_string(),
            src_artifact_id: edge.src_artifact_id.clone(),
            dst_artifact_id: edge.dst_artifact_id.clone(),
            kind: edge.kind.clone(),
            weight: edge.weight,
            payload_json: Some(edge.payload_json.clone()),
            time_created: now_ms(),
        },
    )?;
    Ok(())
}

/// Persist one reasoning lane.
pub fn persist_reasoning_lane(db: &Db, run_id: &str, lane: &ReasoningLane) -> Result<()> {
    let now = now_ms();
    daemon::upsert_reasoning_lane(
        db.connection(),
        &ReasoningLaneRow {
            id: lane.id.clone(),
            run_id: run_id.to_string(),
            role: label(&lane.role)?,
            strategy: lane.strategy.clone(),
            status: lane.status.clone(),
            artifact_ids: lane.artifact_ids.clone(),
            write_scope: lane.write_scope.clone(),
            worker_id: lane.worker_id.clone(),
            confidence: lane.confidence,
            time_created: now,
            time_updated: now,
        },
    )?;
    Ok(())
}

/// Persist a verified or rejected memory capsule.
pub fn persist_memory_capsule(db: &Db, run_id: &str, capsule: &MemoryCapsule) -> Result<()> {
    if !capsule.can_write_permanent() {
        anyhow::bail!("memory capsule is not eligible for durable write");
    }
    let now = now_ms();
    daemon::upsert_memory_capsule(
        db.connection(),
        &MemoryCapsuleRow {
            id: capsule.id.clone(),
            run_id: run_id.to_string(),
            artifact_id: capsule.artifact_id.clone(),
            scope: capsule.scope.clone(),
            status: capsule.status.clone(),
            summary: capsule.summary.clone(),
            evidence_level: label(&capsule.evidence_level)?,
            confidence: capsule.confidence,
            payload_json: Some(capsule.payload_json.clone()),
            content_hash: capsule.content_hash.clone(),
            time_created: now,
            time_updated: now,
        },
    )?;
    Ok(())
}

/// Persist a lightweight repository graph for the run.
pub fn persist_repo_graph(db: &Db, run_id: &str, graph: &RepoGraph) -> Result<()> {
    let conn = db.connection();
    let now = now_ms();
    for node in &graph.nodes {
        daemon::upsert_repo_graph_node(
            conn,
            &RepoGraphNodeRow {
                id: node.id.clone(),
                run_id: run_id.to_string(),
                kind: node.kind.clone(),
                key: node.key.clone(),
                label: node.label.clone(),
                payload_json: node.payload_json.clone(),
                time_created: now,
                time_updated: now,
            },
        )?;
    }
    for edge in &graph.edges {
        daemon::upsert_repo_graph_edge(
            conn,
            &RepoGraphEdgeRow {
                run_id: run_id.to_string(),
                src_node_id: edge.from.clone(),
                dst_node_id: edge.to.clone(),
                kind: edge.kind.clone(),
                payload_json: edge.payload_json.clone(),
                time_created: now,
            },
        )?;
    }
    Ok(())
}

/// Persist parity cases, run summary, raw results, and perf budgets.
pub fn persist_parity_summary(
    db: &Db,
    run_id: &str,
    target_id: &str,
    cases: &[ParityCase],
    artifacts: &ParityArtifacts,
    summary: &ParitySummary,
) -> Result<String> {
    let conn = db.connection();
    let now = now_ms();
    for case in cases {
        daemon::upsert_parity_case(
            conn,
            &ParityCaseRow {
                id: case.id.clone(),
                run_id: run_id.to_string(),
                target_id: target_id.to_string(),
                tags: case.tags.clone(),
                target_kind: case.target_kind.clone(),
                steps_json: serde_json::to_value(&case.steps)?,
                perf_json: case.perf.as_ref().map(serde_json::to_value).transpose()?,
                approved: case.is_required(),
                time_created: now,
                time_updated: now,
            },
        )?;
        if let Some(max_ratio) = case.perf.as_ref().and_then(|perf| perf.p95_ms_max_ratio) {
            daemon::upsert_perf_budget(
                conn,
                &PerfBudgetRow {
                    id: format!("budget-{run_id}-{}", case.id),
                    run_id: run_id.to_string(),
                    case_id: case.id.clone(),
                    metric: "p95_ms".to_string(),
                    max_ratio: Some(max_ratio),
                    baseline_value: None,
                    candidate_value: None,
                    status: if summary.perf_over_budget == 0 {
                        "pass".to_string()
                    } else {
                        "fail".to_string()
                    },
                    time_created: now,
                    time_updated: now,
                },
            )?;
        }
    }
    let parity_run_id = format!("parity-run-{run_id}");
    daemon::upsert_parity_run(
        conn,
        &ParityRunRow {
            id: parity_run_id.clone(),
            run_id: run_id.to_string(),
            target_id: target_id.to_string(),
            case_count: cases.len() as i64,
            status: summary.status.clone(),
            report_path: Some(artifacts.summary_json.display().to_string()),
            started_at: Some(now),
            ended_at: Some(now),
            summary_json: Some(serde_json::to_value(summary)?),
            time_created: now,
            time_updated: now,
        },
    )?;
    for result in &summary.report.results {
        daemon::insert_parity_result(
            conn,
            &ParityResultRow {
                id: format!("result-{run_id}-{}-{}-{now}", result.case_id, result.target),
                parity_run_id: parity_run_id.clone(),
                case_id: result.case_id.clone(),
                target_name: result.target.clone(),
                status: result.status.clone(),
                skipped: result.skipped,
                duration_ms: result
                    .perf
                    .as_ref()
                    .and_then(|perf| perf.get("duration_ms"))
                    .and_then(serde_json::Value::as_i64)
                    .or_else(|| result.elapsed_nanos.map(|nanos| (nanos / 1_000_000) as i64)),
                perf_json: result.perf.clone(),
                message: result.message.clone(),
                time_created: now,
            },
        )?;
    }
    Ok(parity_run_id)
}

/// Convert a port target request into a daemon spec payload.
pub fn port_spec(target: &PortTargetRequest) -> serde_json::Value {
    json!({
        "kind": "zyal_port",
        "target": target,
    })
}

/// Convert a port target request and runtime options into a daemon spec payload.
pub fn port_spec_with_runtime(
    target: &PortTargetRequest,
    runtime: &PortRuntimeOptions,
) -> serde_json::Value {
    json!({
        "kind": "zyal_port",
        "target": target,
        "evidence_inputs": &runtime.evidence_inputs,
        "live_call_budget": &runtime.live_call_budget,
        "proofs": &runtime.proofs,
        "model_policy": &runtime.model_policy,
    })
}

pub fn target_id(run_id: &str) -> String {
    format!("port-target-{run_id}")
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn project_id_for(repo_root: &Path) -> String {
    let mut hasher = Sha1::new();
    hasher.update(repo_root.display().to_string().as_bytes());
    format!("zyal-project-{:x}", hasher.finalize())[..26].to_string()
}

fn hash_json(value: &serde_json::Value) -> Result<String> {
    let mut hasher = Sha1::new();
    let bytes = serde_json::to_vec(value).context("serialize daemon run spec for hash")?;
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn label<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?.trim_matches('"').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_policy::ModelTaskKind;
    use tempfile::tempdir;

    #[test]
    fn persists_model_receipt_with_seeded_run() {
        let dir = tempdir().unwrap();
        let db = Db::open_in_memory().unwrap();
        let target = PortTargetRequest {
            target: "Reference".into(),
            replacement: "Candidate".into(),
            target_repo: None,
            replacement_repo: None,
            request: "port it".into(),
            worker_cap: 2,
        };
        ensure_daemon_run(&db, dir.path(), "run-1", port_spec(&target)).unwrap();
        let receipt = ModelCallReceipt::fake_success(ModelTaskKind::PhaseFinalize, "ok");
        persist_model_receipt(&db, "run-1", &receipt).unwrap();
        assert_eq!(
            daemon::list_model_outcomes_for_run(db.connection(), "run-1")
                .unwrap()
                .len(),
            1
        );
    }
}
