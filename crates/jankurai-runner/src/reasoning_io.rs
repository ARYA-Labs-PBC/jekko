//! I/O helpers for advanced reasoning runs.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use jekko_store::db::Db;
use serde_json::json;

use crate::daemon_store;
use crate::events::{EventKind, EventSink};
use crate::model_client::{ModelCallReceipt, ModelClient};
use crate::model_policy::ModelTaskKind;
use crate::reasoning::{
    AdvancedReasoningConfig, EvidenceLevel, MemoryCapsule, ReasoningArtifact,
    ReasoningArtifactKind, ReasoningEdge, ReasoningLane, ReasoningRole,
};
use crate::repo_graph::RepoGraph;

pub(crate) async fn complete_structured(
    repo: &Path,
    run_id: &str,
    db: &Db,
    sink: &EventSink,
    model_client: &dyn ModelClient,
    kind: ModelTaskKind,
    prompt: &str,
) -> Result<(ModelCallReceipt, serde_json::Value)> {
    let mut last_error = None;
    for attempt in 1..=3 {
        sink.emit(
            EventKind::ModelAttempt,
            json!({
                "kind": crate::model_client::kind_label(kind),
                "attempt": attempt,
            }),
        )?;
        let receipt = model_client.complete(kind, prompt, repo).await?;
        daemon_store::persist_model_receipt(db, run_id, &receipt)?;
        if receipt.budget_used.is_some() || receipt.budget_remaining.is_some() {
            sink.emit(
                EventKind::LiveBudget,
                json!({
                    "used": receipt.budget_used.unwrap_or(0),
                    "remaining": receipt.budget_remaining.unwrap_or(0),
                }),
            )?;
        }
        if !receipt.success {
            let error = match receipt.error.clone() {
                Some(error) => error,
                None => "unknown model failure".to_string(),
            };
            emit_model_outcome(sink, &receipt, attempt, "model_failure")?;
            daemon_store::mark_daemon_run(db, run_id, "blocked", &receipt.kind, Some(&error))?;
            return Err(anyhow!("model call failed: {error}"));
        }
        let Some(text) = receipt.response.as_deref() else {
            if receipt.provider == "fake" {
                emit_model_outcome(sink, &receipt, attempt, "fake_provider_synthetic_response")?;
                return Ok((receipt, synthetic_structured_value(kind)));
            }
            emit_model_outcome(sink, &receipt, attempt, "missing_response")?;
            last_error = Some("model response missing".to_string());
            continue;
        };
        match serde_json::from_str::<serde_json::Value>(text) {
            Ok(value) => {
                emit_model_outcome(sink, &receipt, attempt, "parsed")?;
                return Ok((receipt, value));
            }
            Err(_err) if receipt.provider == "fake" => {
                emit_model_outcome(sink, &receipt, attempt, "fake_provider_synthetic_response")?;
                return Ok((receipt, synthetic_structured_value(kind)));
            }
            Err(err) => {
                emit_model_outcome(sink, &receipt, attempt, "retryable_failure")?;
                last_error = Some(err.to_string());
            }
        }
    }
    let error = match last_error {
        Some(error) => error,
        None => "invalid model JSON".to_string(),
    };
    mark_blocked_for_parse_error(db, run_id, &error)?;
    Err(anyhow!(
        "advanced reasoning model JSON parse failed: {error}"
    ))
}

fn emit_model_outcome(
    sink: &EventSink,
    receipt: &ModelCallReceipt,
    attempt: usize,
    state: &str,
) -> Result<()> {
    let response_bytes = match receipt.response.as_deref() {
        Some(response) => response.len(),
        None => 0,
    };
    let retry_count = match receipt.retry_count {
        Some(retry_count) => retry_count,
        None => attempt.saturating_sub(1),
    };
    let budget_used = match receipt.budget_used {
        Some(budget_used) => budget_used,
        None => 0,
    };
    let budget_remaining = match receipt.budget_remaining {
        Some(budget_remaining) => budget_remaining,
        None => 0,
    };
    sink.emit(
        EventKind::ModelOutcome,
        json!({
            "kind": receipt.kind,
            "provider": receipt.provider,
            "model": receipt.model,
            "success": receipt.success,
            "attempt": attempt,
            "state": state,
            "latency_ms": receipt.latency_ms,
            "response_bytes": response_bytes,
            "credential_policy": receipt.credential_policy,
            "credential_user_id": receipt.credential_user_id,
            "retry_count": retry_count,
            "budget_used": budget_used,
            "budget_remaining": budget_remaining,
        }),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn artifact(
    id: impl Into<String>,
    run_id: &str,
    role: ReasoningRole,
    kind: ReasoningArtifactKind,
    title: impl Into<String>,
    summary: impl Into<String>,
    evidence_level: EvidenceLevel,
    confidence: f64,
    payload_json: serde_json::Value,
    config: &AdvancedReasoningConfig,
) -> ReasoningArtifact {
    let mut artifact = ReasoningArtifact::new(
        id,
        run_id,
        role,
        kind,
        title,
        summary,
        evidence_level,
        confidence,
        payload_json,
    );
    artifact.prepare_for_storage(config);
    artifact
}

pub(crate) fn persist_artifact(
    db: &Db,
    run_id: &str,
    sink: &EventSink,
    artifact: ReasoningArtifact,
) -> Result<ReasoningArtifact> {
    daemon_store::persist_reasoning_artifact(db, run_id, &artifact)?;
    sink.emit(
        EventKind::ReasoningArtifact,
        json!({"id": artifact.id, "kind": artifact.kind, "status": artifact.status}),
    )?;
    Ok(artifact)
}

pub(crate) fn persist_edge(
    db: &Db,
    run_id: &str,
    src: &str,
    dst: &str,
    kind: &str,
) -> Result<ReasoningEdge> {
    let edge = ReasoningEdge {
        run_id: run_id.to_string(),
        src_artifact_id: src.to_string(),
        dst_artifact_id: dst.to_string(),
        kind: kind.to_string(),
        weight: Some(1.0),
        payload_json: json!({}),
    };
    daemon_store::persist_reasoning_edge(db, run_id, &edge)?;
    Ok(edge)
}

pub(crate) fn export_reasoning_graph(
    repo: &Path,
    run_id: &str,
    repo_graph: &RepoGraph,
    artifacts: &[ReasoningArtifact],
    edges: &[ReasoningEdge],
    lanes: &[ReasoningLane],
    memory_capsules: &[MemoryCapsule],
) -> Result<PathBuf> {
    let path = repo
        .join("target/zyal/reasoning")
        .join(run_id)
        .join("reasoning-graph.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let payload = json!({
        "schema_version": "zyal.reasoning.graph.v1",
        "run_id": run_id,
        "repo_graph_summary": repo_graph.summary(),
        "artifacts": artifacts,
        "edges": edges,
        "lanes": lanes,
        "memory_capsules": memory_capsules,
    });
    fs::write(&path, serde_json::to_string_pretty(&payload)?)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn emit_state(sink: &EventSink, state: &str) -> Result<()> {
    sink.emit(EventKind::ReasoningState, json!({"state": state}))
}

fn synthetic_structured_value(kind: ModelTaskKind) -> serde_json::Value {
    json!({
        "kind": format!("{kind:?}"),
        "summary": "deterministic fake structured response",
    })
}

fn mark_blocked_for_parse_error(db: &Db, run_id: &str, error: &str) -> Result<()> {
    daemon_store::mark_daemon_run(db, run_id, "blocked", "model_json_parse", Some(error))?;
    Ok(())
}
