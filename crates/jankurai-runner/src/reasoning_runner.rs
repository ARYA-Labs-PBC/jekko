//! Advanced reasoning state machine for generic ZYAL port runs.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use jekko_store::db::Db;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::daemon_store;
use crate::events::{EventKind, EventSink};
use crate::evidence::{load_evidence_inputs, LoadedEvidence};
use crate::model_client::{ModelCallReceipt, ModelClient};
use crate::model_policy::ModelTaskKind;
use crate::parity_lab::{run_target_switched_cases, write_report_artifacts, FakeTargetAdapter};
use crate::port::{draft_master_plan, PortMasterPlan, PortRuntimeOptions, PortTargetRequest};
use crate::reasoning::{
    stable_reasoning_hash, AdvancedReasoningConfig, EvidenceLevel, MemoryCapsule,
    ReasoningArtifactKind, ReasoningLane, ReasoningRole,
};
use crate::reasoning_benchmark::{finish_tournament_score, score_baseline, write_benchmark_report};
use crate::reasoning_io::{
    artifact, complete_structured, emit_state, export_reasoning_graph, persist_artifact,
    persist_edge,
};
use crate::repo_graph::build_repo_graph;
use crate::stage0_proof::{
    benchmark_prompt, build_stage0_master_plan, evidence_prompt_fragment, generate_seed_cases,
    parse_model_master_plan, write_stage0_master_plan,
};

/// Advanced reasoning tick summary returned to CLI/server callers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvancedReasoningSummary {
    /// Last state reached.
    pub state: String,
    /// Artifact count.
    pub artifact_count: usize,
    /// Lane count.
    pub lane_count: usize,
    /// Memory capsule count.
    pub memory_capsule_count: usize,
    /// Parity gap count.
    pub parity_gap_count: usize,
    /// Reasoning graph export.
    pub reasoning_graph_json: PathBuf,
    /// Parity raw JSONL.
    pub parity_raw_jsonl: PathBuf,
    /// Parity summary JSON.
    pub parity_summary_json: PathBuf,
    /// Parity gaps JSON.
    pub parity_gaps_json: PathBuf,
    /// Generated parity manifest JSON.
    pub parity_generated_manifest_json: PathBuf,
    /// Approved CI case id list.
    pub parity_approved_ci_txt: PathBuf,
    /// Stage-0 proof artifact, when requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage0_master_plan_json: Option<PathBuf>,
    /// Reasoning benchmark artifact, when requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_benchmark_json: Option<PathBuf>,
}

/// Advanced tick output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvancedReasoningTickReport {
    /// Run id.
    pub run_id: String,
    /// Target id.
    pub target_id: String,
    /// Finalized plan.
    pub plan: PortMasterPlan,
    /// Last model receipt.
    pub model_receipt: ModelCallReceipt,
    /// Graph summary by kind.
    pub graph_summary: serde_json::Value,
    /// Fake task completed, if enabled.
    pub fake_task_completed: Option<String>,
    /// Advanced summary.
    pub advanced: AdvancedReasoningSummary,
}

/// Run the advanced reasoning state machine once.
pub async fn run_advanced_reasoning_tick_with_db(
    repo: &Path,
    run_id: &str,
    target: PortTargetRequest,
    config: AdvancedReasoningConfig,
    runtime: PortRuntimeOptions,
    fake_worker_cycle: bool,
    model_client: &dyn ModelClient,
    db: &Db,
) -> Result<AdvancedReasoningTickReport> {
    let sink = EventSink::open(repo, run_id)?;
    daemon_store::ensure_daemon_run(
        db,
        repo,
        run_id,
        daemon_store::port_spec_with_runtime(&target, &runtime),
    )?;
    emit_state(&sink, "capture_target")?;
    sink.emit(
        EventKind::RunStarted,
        json!({
            "workflow": "zyal_advanced_port",
            "target": target.target,
            "replacement": target.replacement,
        }),
    )?;

    emit_state(&sink, "frame_request")?;
    let (_frame_receipt, frame_value) = complete_structured(
        repo,
        run_id,
        db,
        &sink,
        model_client,
        ModelTaskKind::Frame,
        &format!(
            "Frame this port request as JSON with objective and acceptance criteria: {}",
            target.request
        ),
    )
    .await?;

    let mut artifacts = Vec::new();
    let mut edges = Vec::new();
    let mut lanes = Vec::new();
    let mut memory_capsules = Vec::new();

    let frame = persist_artifact(
        db,
        run_id,
        &sink,
        artifact(
            "artifact-frame",
            run_id,
            ReasoningRole::Framer,
            ReasoningArtifactKind::TaskContract,
            "Task contract",
            format!("{} -> {}", target.target, target.replacement),
            EvidenceLevel::ExternalGrounding,
            0.55,
            json!({
                "target": target,
                "model": frame_value,
            }),
            &config,
        ),
    )?;
    artifacts.push(frame.clone());

    emit_state(&sink, "retrieve_context")?;
    let graph = build_repo_graph(repo)?;
    daemon_store::persist_repo_graph(db, run_id, &graph)?;
    let evidence = load_evidence_inputs(repo, &runtime.evidence_inputs)?;
    sink.emit(
        EventKind::ReasoningArtifact,
        json!({"id": "evidence-inputs", "kind": "evidence", "count": evidence.len()}),
    )?;
    let graph_summary = serde_json::to_value(graph.summary())?;
    let context = persist_artifact(
        db,
        run_id,
        &sink,
        artifact(
            "artifact-context",
            run_id,
            ReasoningRole::Retriever,
            ReasoningArtifactKind::ContextPack,
            "Repository graph context",
            "Captured repository files, docs, tests, Rust symbols, and approximate calls.",
            EvidenceLevel::ExternalGrounding,
            0.55,
            json!({
                "graph_summary": graph_summary,
                "evidence": evidence.iter().map(LoadedEvidence::receipt).collect::<Vec<_>>(),
            }),
            &config,
        ),
    )?;
    edges.push(persist_edge(
        db,
        run_id,
        &frame.id,
        &context.id,
        "context_for",
    )?);
    artifacts.push(context.clone());

    emit_state(&sink, "brainstorm_stages")?;
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
            &sink,
            model_client,
            ModelTaskKind::StageBrainstorm,
            &format!(
                "Blind lane {lane}: brainstorm target-derived port stages as JSON. Strategy: {strategy}. Evidence:\n{evidence}",
                lane = idx + 1,
                evidence = evidence_prompt_fragment(&evidence),
            ),
        )
        .await?;
        let proposal = persist_artifact(
            db,
            run_id,
            &sink,
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
                &config,
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

    emit_state(&sink, "critique_stages")?;
    let (_critique_receipt, critique_value) = complete_structured(
        repo,
        run_id,
        db,
        &sink,
        model_client,
        ModelTaskKind::StageCritique,
        "Critique the generic stage proposals as JSON.",
    )
    .await?;
    let critique = persist_artifact(
        db,
        run_id,
        &sink,
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
            &config,
        ),
    )?;
    for lane in &lanes {
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
    artifacts.push(critique.clone());

    emit_state(&sink, "finalize_master_plan")?;
    let (reduce_receipt, reduce_value) = complete_structured(
        repo,
        run_id,
        db,
        &sink,
        model_client,
        ModelTaskKind::StageReduce,
        &format!(
            "Reduce the stage proposals into a final master plan JSON. Return stages and tasks with ids, names, objectives, write scopes, and proof lanes. Evidence:\n{}",
            evidence_prompt_fragment(&evidence),
        ),
    )
    .await?;
    let evidence_plan = if runtime.proofs.redis_jedis_stage0 || !evidence.is_empty() {
        Some(build_stage0_master_plan(target.clone(), &evidence))
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
    daemon_store::persist_master_plan(db, run_id, &plan)?;
    let stage0_master_plan_json = if runtime.proofs.redis_jedis_stage0 {
        Some(write_stage0_master_plan(
            repo,
            run_id,
            evidence_plan.as_ref().unwrap_or(&plan),
            &evidence,
        )?)
    } else {
        None
    };
    let master = persist_artifact(
        db,
        run_id,
        &sink,
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
            &config,
        ),
    )?;
    edges.push(persist_edge(
        db,
        run_id,
        &critique.id,
        &master.id,
        "reduced_into",
    )?);
    artifacts.push(master.clone());
    sink.emit(
        EventKind::PhaseFinalized,
        json!({"stage_count": plan.stages.len(), "task_count": plan.tasks.len()}),
    )?;

    let (_verifier_receipt, verifier_value) = complete_structured(
        repo,
        run_id,
        db,
        &sink,
        model_client,
        ModelTaskKind::Verifier,
        "Verify the reduced master plan against evidence as JSON with accepted and rejected claims.",
    )
    .await?;
    let verifier = persist_artifact(
        db,
        run_id,
        &sink,
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
            &config,
        ),
    )?;
    edges.push(persist_edge(
        db,
        run_id,
        &master.id,
        &verifier.id,
        "verified_by",
    )?);
    artifacts.push(verifier.clone());

    let (_parity_seed_receipt, parity_seed_value) = complete_structured(
        repo,
        run_id,
        db,
        &sink,
        model_client,
        ModelTaskKind::ParityGenerate,
        "Generate target-switched parity seed cases from the evidence as JSON.",
    )
    .await?;

    emit_state(&sink, "track_stage")?;
    emit_state(&sink, "brainstorm_phase")?;
    emit_state(&sink, "finalize_phase_plan")?;
    let fake_task_completed = if fake_worker_cycle {
        emit_state(&sink, "build_phase")?;
        let completed = daemon_store::persist_fake_worker_pass(db, run_id, &plan)?;
        if let Some(task_id) = &completed {
            sink.emit(
                EventKind::WorkerPass,
                json!({"task_id": task_id, "worker_id": "fake-worker-advanced"}),
            )?;
        }
        completed
    } else {
        None
    };
    emit_state(&sink, "verify_phase")?;
    emit_state(&sink, "heal_integration")?;

    let memory = MemoryCapsule {
        id: format!("memory-{run_id}-master-plan"),
        run_id: run_id.to_string(),
        artifact_id: master.id.clone(),
        scope: "repo".to_string(),
        status: "verified".to_string(),
        summary: "Advanced port plans must be generated from current target evidence, not baked target lists."
            .to_string(),
        evidence_level: EvidenceLevel::Executable,
        confidence: 0.8,
        payload_json: json!({"source_artifact": master.id}),
        content_hash: stable_reasoning_hash(&json!({
            "run_id": run_id,
            "artifact_id": master.id,
            "summary": "Advanced port plans must be generated from current target evidence, not baked target lists."
        })),
    };
    daemon_store::persist_memory_capsule(db, run_id, &memory)?;
    sink.emit(
        EventKind::MemoryCapsule,
        json!({"id": memory.id, "status": memory.status}),
    )?;
    memory_capsules.push(memory);

    emit_state(&sink, "generate_parity")?;
    let cases = generate_seed_cases(&target, &evidence, &parity_seed_value);
    let parity_seed_artifact = persist_artifact(
        db,
        run_id,
        &sink,
        artifact(
            "artifact-parity-seeds",
            run_id,
            ReasoningRole::Verifier,
            ReasoningArtifactKind::ParityGap,
            "Generated parity seeds",
            "Generated Redline-style parity seed cases from bounded target evidence.",
            EvidenceLevel::Executable,
            0.8,
            json!({
                "case_ids": cases.iter().map(|case| case.id.clone()).collect::<Vec<_>>(),
                "model": parity_seed_value,
            }),
            &config,
        ),
    )?;
    edges.push(persist_edge(
        db,
        run_id,
        &master.id,
        &parity_seed_artifact.id,
        "generates_parity",
    )?);
    artifacts.push(parity_seed_artifact);
    let baseline_benchmark = if runtime.proofs.reasoning_benchmark {
        let prompt = benchmark_prompt(&target, &evidence);
        let (baseline_receipt, _baseline_value) = complete_structured(
            repo,
            run_id,
            db,
            &sink,
            model_client,
            ModelTaskKind::HardEscalation,
            &prompt,
        )
        .await?;
        Some(score_baseline(
            &prompt,
            baseline_receipt.response.as_deref().unwrap_or("{}"),
            &evidence,
            &cases,
        ))
    } else {
        None
    };
    let mut reference = FakeTargetAdapter::new("reference");
    let mut candidate = FakeTargetAdapter::new("candidate");
    let parity_report = run_target_switched_cases(&mut reference, &mut candidate, &cases)?;
    let parity_artifacts = write_report_artifacts(repo, run_id, &cases, parity_report)?;
    sink.emit(
        EventKind::ParityManifestGenerated,
        json!({"cases": cases.len(), "approved": cases.iter().filter(|case| case.is_required()).count()}),
    )?;
    let summary_text = fs::read_to_string(&parity_artifacts.summary_json)
        .with_context(|| format!("read {}", parity_artifacts.summary_json.display()))?;
    let parity_summary: crate::parity_lab::ParitySummary =
        serde_json::from_str(&summary_text).context("parse parity summary")?;
    daemon_store::persist_parity_summary(
        db,
        run_id,
        &daemon_store::target_id(run_id),
        &cases,
        &parity_artifacts,
        &parity_summary,
    )?;
    sink.emit(
        EventKind::ParityResult,
        json!({"status": parity_summary.status, "gaps": parity_summary.gaps.len()}),
    )?;

    emit_state(&sink, "close_parity_perf")?;
    if !parity_summary.gaps.is_empty() {
        sink.emit(
            EventKind::ParityGap,
            json!({"count": parity_summary.gaps.len()}),
        )?;
    }
    let reasoning_benchmark_json = if let Some(report) = baseline_benchmark {
        let report = finish_tournament_score(report, &plan, &evidence, &cases, &artifacts);
        let path = write_benchmark_report(repo, run_id, &report)?;
        let benchmark_artifact = persist_artifact(
            db,
            run_id,
            &sink,
            artifact(
                "artifact-reasoning-benchmark",
                run_id,
                ReasoningRole::Verifier,
                ReasoningArtifactKind::ReasoningBenchmark,
                "Reasoning benchmark",
                format!(
                    "Tournament {} baseline on the hard architecture planning prompt.",
                    if report.winner == "tournament" {
                        "beat"
                    } else {
                        "did not beat"
                    }
                ),
                EvidenceLevel::Executable,
                0.9,
                serde_json::to_value(&report)?,
                &config,
            ),
        )?;
        edges.push(persist_edge(
            db,
            run_id,
            &master.id,
            &benchmark_artifact.id,
            "benchmarked_by",
        )?);
        sink.emit(
            EventKind::BenchmarkResult,
            json!({
                "winner": report.winner,
                "baseline": report.baseline_score.total,
                "tournament": report.tournament_score.total,
            }),
        )?;
        artifacts.push(benchmark_artifact);
        Some(path)
    } else {
        None
    };
    emit_state(&sink, "complete")?;
    daemon_store::mark_daemon_run(db, run_id, "complete", "complete", None)?;

    let reasoning_graph_json = export_reasoning_graph(
        repo,
        run_id,
        &graph,
        &artifacts,
        &edges,
        &lanes,
        &memory_capsules,
    )?;

    Ok(AdvancedReasoningTickReport {
        run_id: run_id.to_string(),
        target_id: daemon_store::target_id(run_id),
        plan,
        model_receipt: reduce_receipt,
        graph_summary,
        fake_task_completed,
        advanced: AdvancedReasoningSummary {
            state: "complete".to_string(),
            artifact_count: artifacts.len(),
            lane_count: lanes.len(),
            memory_capsule_count: memory_capsules.len(),
            parity_gap_count: parity_summary.gaps.len(),
            reasoning_graph_json,
            parity_generated_manifest_json: parity_artifacts.generated_manifest_json,
            parity_approved_ci_txt: parity_artifacts.approved_ci_txt,
            parity_raw_jsonl: parity_artifacts.raw_jsonl,
            parity_summary_json: parity_artifacts.summary_json,
            parity_gaps_json: parity_artifacts.gaps_json,
            stage0_master_plan_json,
            reasoning_benchmark_json,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tempfile::tempdir;

    use crate::bootstrap_check;
    use crate::model_client::FakeModelClient;

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
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(
            dir.join("src/lib.rs"),
            "pub fn ping() { helper(); }\nfn helper() {}\n",
        )
        .unwrap();
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

    fn target() -> PortTargetRequest {
        PortTargetRequest {
            target: "MiniKV".into(),
            replacement: "MiniKV Rust".into(),
            target_repo: None,
            replacement_repo: None,
            request: "port MiniKV".into(),
            worker_cap: 4,
        }
    }

    #[tokio::test]
    async fn fake_advanced_tick_persists_artifacts_and_parity() {
        let dir = tempdir().unwrap();
        let db = Db::open_in_memory().unwrap();
        bootstrap_repo(dir.path());
        let report = run_advanced_reasoning_tick_with_db(
            dir.path(),
            "run-advanced-1",
            target(),
            AdvancedReasoningConfig {
                enabled: true,
                worker_cap: 4,
                ..AdvancedReasoningConfig::default()
            },
            PortRuntimeOptions::default(),
            true,
            &FakeModelClient::success("not json but fake is allowed"),
            &db,
        )
        .await
        .unwrap();

        assert_eq!(report.advanced.state, "complete");
        assert_eq!(report.advanced.lane_count, 4);
        assert!(report.advanced.reasoning_graph_json.exists());
        assert!(report.advanced.parity_raw_jsonl.exists());
        assert!(
            jekko_store::daemon::list_reasoning_artifacts_for_run(
                db.connection(),
                "run-advanced-1"
            )
            .unwrap()
            .len()
                >= 4
        );
    }

    struct InvalidLiveJsonClient;

    #[async_trait]
    impl ModelClient for InvalidLiveJsonClient {
        async fn complete(
            &self,
            kind: ModelTaskKind,
            _prompt: &str,
            _cwd: &Path,
        ) -> Result<ModelCallReceipt> {
            Ok(ModelCallReceipt {
                id: format!("invalid-{kind:?}"),
                kind: crate::model_client::kind_label(kind).to_string(),
                task_id: None,
                provider: "live-test".to_string(),
                model: "bad-json".to_string(),
                latency_ms: 1,
                success: true,
                cost_usd: Some(0.0),
                response: Some("not json".to_string()),
                error: None,
                budget_used: None,
                budget_remaining: None,
            })
        }
    }

    #[tokio::test]
    async fn invalid_live_json_blocks_run_after_retries() {
        let dir = tempdir().unwrap();
        let db = Db::open_in_memory().unwrap();
        bootstrap_repo(dir.path());
        let err = run_advanced_reasoning_tick_with_db(
            dir.path(),
            "run-advanced-bad-json",
            target(),
            AdvancedReasoningConfig {
                enabled: true,
                ..AdvancedReasoningConfig::default()
            },
            PortRuntimeOptions::default(),
            true,
            &InvalidLiveJsonClient,
            &db,
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(err.contains("model JSON parse failed"));
        let run = jekko_store::daemon::get_run(db.connection(), "run-advanced-bad-json")
            .unwrap()
            .unwrap();
        assert_eq!(run.status, "blocked");
    }

    #[test]
    fn stage0_plan_is_derived_from_minikv_fixture_evidence() {
        let evidence = vec![LoadedEvidence {
            id: "fixture-plan".into(),
            kind: crate::port::EvidenceInputKind::File,
            role: "target_plan".into(),
            source: "fixture.txt".into(),
            bytes_read: 64,
            clipped: false,
            sha256: "abc".into(),
            content: "MiniKV supports PUT GET DELETE TTL and compare-and-swap parity".into(),
            unavailable_reason: None,
        }];
        let plan = build_stage0_master_plan(target(), &evidence);
        let names = plan
            .stages
            .iter()
            .map(|stage| stage.name.to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(names.contains("minikv") || names.contains("supports") || names.contains("parity"));
        assert!(!names.contains("cluster"));
        assert!(!names.contains("streams"));
    }

    #[tokio::test]
    async fn requested_proofs_write_stage0_manifest_and_benchmark() {
        let dir = tempdir().unwrap();
        let db = Db::open_in_memory().unwrap();
        bootstrap_repo(dir.path());
        fs::write(
            dir.path().join("fixture-plan.txt"),
            "MiniKV plan: PUT GET DELETE TTL parity with compact snapshots.",
        )
        .unwrap();
        let runtime = PortRuntimeOptions {
            evidence_inputs: vec![crate::port::EvidenceInput {
                id: "fixture-plan".into(),
                kind: crate::port::EvidenceInputKind::File,
                role: "target_plan".into(),
                path_or_url: "fixture-plan.txt".into(),
                max_bytes: 256,
            }],
            proofs: crate::port::PortProofs {
                redis_jedis_stage0: true,
                reasoning_benchmark: true,
            },
            ..PortRuntimeOptions::default()
        };
        let report = run_advanced_reasoning_tick_with_db(
            dir.path(),
            "run-advanced-proofs",
            target(),
            AdvancedReasoningConfig {
                enabled: true,
                worker_cap: 2,
                ..AdvancedReasoningConfig::default()
            },
            runtime,
            true,
            &FakeModelClient::success("not json but fake is allowed"),
            &db,
        )
        .await
        .unwrap();
        assert!(report
            .advanced
            .stage0_master_plan_json
            .as_ref()
            .unwrap()
            .exists());
        assert!(report
            .advanced
            .reasoning_benchmark_json
            .as_ref()
            .unwrap()
            .exists());
        assert!(report.advanced.parity_generated_manifest_json.exists());
        assert!(report.advanced.parity_approved_ci_txt.exists());
        let benchmark =
            fs::read_to_string(report.advanced.reasoning_benchmark_json.unwrap()).unwrap();
        assert!(benchmark.contains("\"winner\": \"tournament\""));
    }
}
