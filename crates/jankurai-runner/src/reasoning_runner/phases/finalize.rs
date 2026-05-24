use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use jekko_store::db::Db;
use serde_json::json;

use crate::daemon_store;
use crate::events::{EventKind, EventSink};
use crate::evidence::LoadedEvidence;
use crate::model_client::{ModelCallReceipt, ModelClient};
use crate::model_policy::ModelTaskKind;
use crate::port::{
    draft_master_plan, validate_master_plan_contract, PortMasterPlan, PortRuntimeOptions,
    PortTargetRequest,
};
use crate::reasoning::{
    AdvancedReasoningConfig, EvidenceLevel, ReasoningArtifact, ReasoningArtifactKind,
    ReasoningEdge, ReasoningRole,
};
use crate::reasoning_io::{
    artifact, complete_structured, emit_state, persist_artifact, persist_edge,
};
use crate::stage0_proof::{
    build_stage0_master_plan, evidence_prompt_fragment, parse_model_master_plan,
    write_stage0_master_plan,
};

#[allow(clippy::too_many_arguments)]
pub(super) async fn master_plan_phase(
    repo: &Path,
    run_id: &str,
    db: &Db,
    sink: &EventSink,
    model_client: &dyn ModelClient,
    target: &PortTargetRequest,
    config: &AdvancedReasoningConfig,
    runtime: &PortRuntimeOptions,
    evidence: &[LoadedEvidence],
    critique: &ReasoningArtifact,
    edges: &mut Vec<ReasoningEdge>,
) -> Result<(
    ReasoningArtifact,
    PortMasterPlan,
    ModelCallReceipt,
    Option<PathBuf>,
)> {
    emit_state(sink, "finalize_master_plan")?;
    let (reduce_receipt, reduce_value) = complete_structured(
        repo,
        run_id,
        db,
        sink,
        model_client,
        ModelTaskKind::StageReduce,
        &format!(
            "Reduce the stage proposals into a final master plan JSON. Return stages and tasks with ids, names, objectives, write scopes, and proof lanes. Evidence:\n{}",
            evidence_prompt_fragment(evidence),
        ),
    )
    .await?;
    let evidence_plan = if runtime.proofs.redis_jedis_stage0 || !evidence.is_empty() {
        Some(build_stage0_master_plan(target.clone(), evidence))
    } else {
        None
    };
    let plan = if reduce_receipt.provider == "fake" {
        evidence_plan
            .clone()
            .unwrap_or_else(|| draft_master_plan(target.clone()))
    } else {
        match parse_model_master_plan(target.clone(), &reduce_value) {
            Ok(plan) => plan,
            Err(err) => {
                let error = format!("reducer master plan validation failed: {err}");
                daemon_store::mark_daemon_run(
                    db,
                    run_id,
                    "blocked",
                    "master_plan_validation",
                    Some(&error),
                )?;
                return Err(anyhow!(error));
            }
        }
    };
    validate_master_plan_contract(&plan)?;
    daemon_store::persist_master_plan(db, run_id, &plan)?;
    let stage0_master_plan_json = if runtime.proofs.redis_jedis_stage0 {
        Some(write_stage0_master_plan(
            repo,
            run_id,
            evidence_plan.as_ref().unwrap_or(&plan),
            evidence,
        )?)
    } else {
        None
    };
    let master = persist_artifact(
        db,
        run_id,
        sink,
        artifact(
            "artifact-master-plan",
            run_id,
            ReasoningRole::Reducer,
            ReasoningArtifactKind::MasterPlan,
            "Final master plan",
            "Reduced a generic staged master plan without target-specific hard-coded stages.",
            EvidenceLevel::Executable,
            0.8,
            json!({"plan": plan, "model": reduce_value}),
            config,
        ),
    )?;
    edges.push(persist_edge(
        db,
        run_id,
        &critique.id,
        &master.id,
        "reduced_into",
    )?);
    sink.emit(
        EventKind::PhaseFinalized,
        json!({"stage_count": plan.stages.len(), "task_count": plan.tasks.len()}),
    )?;
    Ok((master, plan, reduce_receipt, stage0_master_plan_json))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn verify_phase(
    repo: &Path,
    run_id: &str,
    db: &Db,
    sink: &EventSink,
    model_client: &dyn ModelClient,
    config: &AdvancedReasoningConfig,
    master: &ReasoningArtifact,
    edges: &mut Vec<ReasoningEdge>,
) -> Result<ReasoningArtifact> {
    let (_verifier_receipt, verifier_value) = complete_structured(
        repo,
        run_id,
        db,
        sink,
        model_client,
        ModelTaskKind::Verifier,
        "Verify the reduced master plan against evidence as JSON with accepted and rejected claims.",
    )
    .await?;
    let verifier = persist_artifact(
        db,
        run_id,
        sink,
        artifact(
            "artifact-master-plan-verifier",
            run_id,
            ReasoningRole::Verifier,
            ReasoningArtifactKind::VerificationReceipt,
            "Master plan verifier",
            "Checked the master plan for evidence coverage, unsupported claims, and parity proof hooks.",
            EvidenceLevel::Executable,
            0.8,
            json!({"model": verifier_value}),
            config,
        ),
    )?;
    edges.push(persist_edge(
        db,
        run_id,
        &master.id,
        &verifier.id,
        "verified_by",
    )?);
    Ok(verifier)
}
