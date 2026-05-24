use std::path::Path;

use anyhow::Result;
use jekko_store::db::Db;
use serde_json::json;

use crate::daemon_store;
use crate::events::{EventKind, EventSink};
use crate::evidence::LoadedEvidence;
use crate::model_client::ModelClient;
use crate::model_policy::ModelTaskKind;
use crate::reasoning::{
    AdvancedReasoningConfig, EvidenceLevel, ReasoningArtifact, ReasoningArtifactKind,
    ReasoningEdge, ReasoningLane, ReasoningRole,
};
use crate::reasoning_io::{
    artifact, complete_structured, emit_state, persist_artifact, persist_edge,
};
use crate::stage0_proof::evidence_prompt_fragment;

#[allow(clippy::too_many_arguments)]
pub(super) async fn brainstorm_phase(
    repo: &Path,
    run_id: &str,
    db: &Db,
    sink: &EventSink,
    model_client: &dyn ModelClient,
    config: &AdvancedReasoningConfig,
    evidence: &[LoadedEvidence],
    context: &ReasoningArtifact,
    artifacts: &mut Vec<ReasoningArtifact>,
    edges: &mut Vec<ReasoningEdge>,
    lanes: &mut Vec<ReasoningLane>,
) -> Result<()> {
    emit_state(sink, "brainstorm_stages")?;
    let strategies = [
        "minimal_contract",
        "test_first",
        "protocol_surface",
        "perf_first",
        "integration_healing",
        "adversarial_gap",
        "docs_examples",
        "compatibility_matrix",
        "rollback_safety",
        "parity_lab",
    ];
    for idx in 0..config.effective_worker_cap() {
        let strategy = strategies[idx % strategies.len()];
        let (_brainstorm_receipt, brainstorm_value) = complete_structured(
            repo,
            run_id,
            db,
            sink,
            model_client,
            ModelTaskKind::StageBrainstorm,
            &format!(
                "Blind lane {lane}: brainstorm target-derived port stages as JSON. Strategy: {strategy}. Evidence:\n{evidence}",
                lane = idx + 1,
                evidence = evidence_prompt_fragment(evidence),
            ),
        )
        .await?;
        let proposal = persist_artifact(
            db,
            run_id,
            sink,
            artifact(
                format!("artifact-stage-proposal-{}", idx + 1),
                run_id,
                ReasoningRole::Planner,
                ReasoningArtifactKind::StageProposal,
                format!("Stage proposal {}", idx + 1),
                format!("Blind lane using {strategy} strategy."),
                EvidenceLevel::IndependentAgreement,
                0.5,
                json!({"strategy": strategy, "model": brainstorm_value}),
                config,
            ),
        )?;
        edges.push(persist_edge(
            db,
            run_id,
            &context.id,
            &proposal.id,
            "derived_from",
        )?);
        let lane = ReasoningLane {
            id: format!("lane-{}", idx + 1),
            run_id: run_id.to_string(),
            role: ReasoningRole::Planner,
            strategy: strategy.to_string(),
            status: "complete".to_string(),
            artifact_ids: vec![proposal.id.clone()],
            write_scope: vec!["src/**".to_string(), "tests/**".to_string()],
            worker_id: Some(format!("reasoner-{}", idx + 1)),
            confidence: proposal.confidence,
        };
        daemon_store::persist_reasoning_lane(db, run_id, &lane)?;
        sink.emit(
            EventKind::ReasoningLane,
            json!({"id": lane.id, "role": "planner", "status": "complete"}),
        )?;
        lanes.push(lane);
        artifacts.push(proposal);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn critique_phase(
    repo: &Path,
    run_id: &str,
    db: &Db,
    sink: &EventSink,
    model_client: &dyn ModelClient,
    config: &AdvancedReasoningConfig,
    lanes: &[ReasoningLane],
    edges: &mut Vec<ReasoningEdge>,
) -> Result<ReasoningArtifact> {
    emit_state(sink, "critique_stages")?;
    let (_critique_receipt, critique_value) = complete_structured(
        repo,
        run_id,
        db,
        sink,
        model_client,
        ModelTaskKind::StageCritique,
        "Critique the generic stage proposals as JSON.",
    )
    .await?;
    let critique = persist_artifact(
        db,
        run_id,
        sink,
        artifact(
            "artifact-stage-critique",
            run_id,
            ReasoningRole::Critic,
            ReasoningArtifactKind::Critique,
            "Stage critique",
            "Critiqued stage proposals for missing evidence, overlap, and target hard-coding.",
            EvidenceLevel::IndependentAgreement,
            0.45,
            json!({"model": critique_value}),
            config,
        ),
    )?;
    for lane in lanes {
        if let Some(source) = lane.artifact_ids.first() {
            edges.push(persist_edge(
                db,
                run_id,
                source,
                &critique.id,
                "critiqued_by",
            )?);
        }
    }
    Ok(critique)
}
