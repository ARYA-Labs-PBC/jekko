//! Canonical 12-stage mega-project phase catalog backing
//! [`super::SuperReasoningPlan::default_megaproject_plan`].
//!
//! Split out of `super_reasoning.rs` to keep that file under the 500-LOC
//! shape threshold (jankurai HLT-001:shape). All items are re-exported by
//! the parent module via `pub use canonical::*;`, so callers continue to
//! reach `canonical_phases()` etc. through `super::*` within the module.

use super::phase::{
    AcceptanceGate, AcceptanceGateKind, FuseStrategy, MacroPhase, PhaseBudget, PhaseSignoffMode,
    ReasoningLane, ReasoningTask, WriteScope,
};

/// Build the canonical 12-stage mega-project plan phases.
pub fn canonical_phases() -> Vec<MacroPhase> {
    vec![
        phase(
            "source_of_truth",
            "Source of truth",
            "Build the non-negotiable behavior, API, compatibility, and evidence ledger.",
            vec![],
            WriteScope::ScratchOnly,
            PhaseSignoffMode::Reviewer,
            FuseStrategy::SynthesizeBest,
            AcceptanceGateKind::EvidenceBundle,
        ),
        phase(
            "architecture_blueprint",
            "Architecture blueprint",
            "Convert requirements into module boundaries, invariants, risk register, and implementation slices.",
            vec!["source_of_truth"],
            WriteScope::ScratchOnly,
            PhaseSignoffMode::Reviewer,
            FuseStrategy::SynthesizeBest,
            AcceptanceGateKind::PlanReceipt,
        ),
        phase(
            "repo_graph_bootstrap",
            "Repo graph bootstrap",
            "Index functions, symbols, tests, call edges, dataflow hints, ownership, and blast radius.",
            vec!["source_of_truth"],
            WriteScope::ScratchOnly,
            PhaseSignoffMode::EvidenceOnly,
            FuseStrategy::BestVerified,
            AcceptanceGateKind::RepoGraphFresh,
        ),
        phase(
            "contracts_and_slices",
            "Contracts and slices",
            "Produce task contracts, proof commands, fixture strategy, and independent slices.",
            vec!["architecture_blueprint", "repo_graph_bootstrap"],
            WriteScope::ScratchOnly,
            PhaseSignoffMode::Reviewer,
            FuseStrategy::SynthesizeBest,
            AcceptanceGateKind::PlanReceipt,
        ),
        phase(
            "parallel_subsystems",
            "Parallel subsystems",
            "Implement independent verified slices in isolated worktrees with critic lanes.",
            vec!["contracts_and_slices"],
            WriteScope::IsolatedWorktree,
            PhaseSignoffMode::Reviewer,
            FuseStrategy::MergeNonConflicting,
            AcceptanceGateKind::TestsGreen,
        ),
        phase(
            "integration_fusion",
            "Integration fusion",
            "Fuse verified subsystem work, resolve interface drift, and run integration proof lanes.",
            vec!["parallel_subsystems"],
            WriteScope::IntegrationBranch,
            PhaseSignoffMode::Reviewer,
            FuseStrategy::SequentialIntegration,
            AcceptanceGateKind::TestsGreen,
        ),
        phase(
            "parity_lab",
            "Parity lab",
            "Create differential, golden, metamorphic, and fuzz parity harnesses.",
            vec!["source_of_truth", "contracts_and_slices"],
            WriteScope::IsolatedWorktree,
            PhaseSignoffMode::Reviewer,
            FuseStrategy::SynthesizeBest,
            AcceptanceGateKind::ParitySuite,
        ),
        phase(
            "parity_gap_closure",
            "Parity gap closure",
            "Close blocking parity gaps using the gap ledger and regression proofs.",
            vec!["integration_fusion", "parity_lab"],
            WriteScope::IntegrationBranch,
            PhaseSignoffMode::Reviewer,
            FuseStrategy::SequentialIntegration,
            AcceptanceGateKind::ParitySuite,
        ),
        phase(
            "performance_closure",
            "Performance closure",
            "Benchmark reference versus candidate and close hot-path gaps without weakening parity.",
            vec!["parity_gap_closure"],
            WriteScope::IntegrationBranch,
            PhaseSignoffMode::Reviewer,
            FuseStrategy::SequentialIntegration,
            AcceptanceGateKind::PerformanceBudget,
        ),
        phase(
            "hardening_security",
            "Hardening and security",
            "Run fuzzing, stress, recovery, race, security, and fault-injection proof lanes.",
            vec!["parity_gap_closure"],
            WriteScope::IntegrationBranch,
            PhaseSignoffMode::Reviewer,
            FuseStrategy::BestVerified,
            AcceptanceGateKind::SecurityReview,
        ),
        phase(
            "docs_release_ops",
            "Docs, release, and operations",
            "Produce docs, CI gates, migration notes, release checklist, and operational runbooks.",
            vec!["performance_closure", "hardening_security"],
            WriteScope::IsolatedWorktree,
            PhaseSignoffMode::Reviewer,
            FuseStrategy::MergeNonConflicting,
            AcceptanceGateKind::EvidenceBundle,
        ),
        phase(
            "final_signoff",
            "Final signoff",
            "Aggregate receipts, rerun full proofs, ensure clean tree, and require final approval.",
            vec!["docs_release_ops"],
            WriteScope::ReadOnly,
            PhaseSignoffMode::Human,
            FuseStrategy::BlockingReview,
            AcceptanceGateKind::HumanApproval,
        ),
    ]
}

// Builder for canonical phase rows. clippy::too_many_arguments fires at 8
// vs the default 7; introducing a config struct just shifts the noise
// (every call site would still pass the same 8 fields). The arity is the
// minimum for the canonical-phases catalog; merging fields would create
// false correlations (e.g. WriteScope and PhaseSignoffMode aren't paired
// 1:1 across the 12 phases).
#[allow(clippy::too_many_arguments)]
fn phase(
    id: &str,
    name: &str,
    objective: &str,
    depends_on: Vec<&str>,
    writes: WriteScope,
    signoff: PhaseSignoffMode,
    fuse_strategy: FuseStrategy,
    primary_gate: AcceptanceGateKind,
) -> MacroPhase {
    MacroPhase {
        id: id.to_string(),
        name: name.to_string(),
        objective: objective.to_string(),
        depends_on: depends_on.into_iter().map(str::to_string).collect(),
        can_run_parallel_with: Vec::new(),
        tasks: vec![ReasoningTask {
            id: format!("{id}.plan"),
            description: format!("Produce and verify the {name} phase receipt."),
            owner_role: "phase_lead".to_string(),
            produces: vec![format!("target/zyal/super-reasoning/{id}/receipt.json")],
            blocked_by: Vec::new(),
            test_first: matches!(
                primary_gate,
                AcceptanceGateKind::TestsGreen
                    | AcceptanceGateKind::ParitySuite
                    | AcceptanceGateKind::PerformanceBudget
            ),
        }],
        workers: match writes {
            WriteScope::ReadOnly | WriteScope::ScratchOnly => 4,
            WriteScope::IsolatedWorktree => 8,
            WriteScope::IntegrationBranch => 6,
            WriteScope::MainWorktree => 1,
        },
        lanes: default_lanes(writes),
        acceptance: vec![
            AcceptanceGate {
                kind: primary_gate,
                command: None,
                required_artifacts: vec![format!("target/zyal/super-reasoning/{id}/receipt.json")],
                min_reviewer_quorum: Some(2),
                blocks_promotion: true,
            },
            AcceptanceGate {
                kind: AcceptanceGateKind::NoOpenCriticalObjections,
                command: None,
                required_artifacts: Vec::new(),
                min_reviewer_quorum: Some(1),
                blocks_promotion: true,
            },
        ],
        budget: PhaseBudget {
            max_iterations: 12,
            max_wall_clock: "24h".to_string(),
            max_diff_lines: 5_000,
            max_cost: None,
        },
        signoff,
        fuse_strategy,
        writes,
        artifacts: vec![format!("target/zyal/super-reasoning/{id}/receipt.json")],
    }
}

fn default_lanes(writes: WriteScope) -> Vec<ReasoningLane> {
    vec![
        ReasoningLane {
            id: "blind_scout".to_string(),
            role: "scout".to_string(),
            count: 2,
            context_policy: "blind_minimal".to_string(),
            blind: true,
            writes: WriteScope::ScratchOnly,
        },
        ReasoningLane {
            id: "builder".to_string(),
            role: "implementer".to_string(),
            count: 3,
            context_policy: "graph_and_phase_memory".to_string(),
            blind: false,
            writes,
        },
        ReasoningLane {
            id: "critic".to_string(),
            role: "critic".to_string(),
            count: 2,
            context_policy: "evidence_and_diff_only".to_string(),
            blind: false,
            writes: WriteScope::ScratchOnly,
        },
        ReasoningLane {
            id: "reducer".to_string(),
            role: "reducer".to_string(),
            count: 1,
            context_policy: "candidate_pool".to_string(),
            blind: false,
            writes: WriteScope::ScratchOnly,
        },
    ]
}
