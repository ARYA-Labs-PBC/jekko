//! Typed control-plane support for very large ZYAL reasoning missions.
//!
//! The existing ZYAL contract already has the required authoring primitives
//! (`workflow`, `fleet`, `memory`, `evidence`, `repo_intelligence`,
//! `approvals`, `sandbox`, `checkpoint`, `done`). This module provides the
//! runtime-normalised shape that binds those blocks into a durable phase DAG
//! for multi-agent, multi-day work without adding target-specific schema such
//! as "Redis" or "SQLite" to the contract.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeResult};

mod types;
pub use types::*;

/// Metadata key used on [`crate::daemon::DaemonRecord::metadata`].
pub const SUPER_REASONING_METADATA_KEY: &str = "super_reasoning";

/// Schema version for the runtime-normalised plan payload.
pub const SUPER_REASONING_SCHEMA_VERSION: &str = "super_reasoning/v1";

/// Default maximum worker cap aligned with the current flagship fleet examples.
pub const DEFAULT_MAX_WORKERS: u16 = 20;

/// Default lower bound for macro-phase plans.
pub const DEFAULT_MIN_PHASES: usize = 9;

/// Default upper bound for macro-phase plans.
pub const DEFAULT_MAX_PHASES: usize = 12;

/// A host-normalised mission plan for ambitious ZYAL workflows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuperReasoningPlan {
    /// Plan schema version.
    pub schema_version: String,
    /// Stable mission id.
    pub mission_id: String,
    /// Human objective for the whole mission.
    pub objective: String,
    /// Required macro-phase envelope.
    pub phase_count: PhaseCountTarget,
    /// Swarm/fleet behavior.
    pub swarm: SwarmPolicy,
    /// Memory compounding policy.
    pub memory: MemoryCompoundingPolicy,
    /// Repository graph/indexing policy.
    pub repo_graph: RepoGraphPolicy,
    /// Persistent sandbox policy.
    pub sandbox: PersistentSandboxPolicy,
    /// Generic parity closure policy.
    pub parity: Option<ParityClosurePolicy>,
    /// Hardening and safety proof policy.
    pub hardening: HardeningPolicy,
    /// Final mission signoff policy.
    pub signoff: SignoffPolicy,
    /// Macro phases that make up the mission.
    pub phases: Vec<MacroPhase>,
}

impl SuperReasoningPlan {
    /// Create the canonical 12-stage mega-project plan.
    ///
    /// This intentionally avoids target-specific assumptions. It can represent
    /// a database rewrite, compiler port, protocol-compatible replacement,
    /// research program, or large refactor as long as parity and performance
    /// policies are supplied by the host/runbook.
    pub fn default_megaproject_plan(
        mission_id: impl Into<String>,
        objective: impl Into<String>,
    ) -> Self {
        let mission_id = mission_id.into();
        let objective = objective.into();
        Self {
            schema_version: SUPER_REASONING_SCHEMA_VERSION.to_string(),
            mission_id,
            objective,
            phase_count: PhaseCountTarget {
                min: DEFAULT_MIN_PHASES,
                max: DEFAULT_MAX_PHASES,
            },
            swarm: SwarmPolicy {
                max_workers: DEFAULT_MAX_WORKERS,
                isolation: IsolationMode::GitWorktree,
                parallel_phase_mode: true,
                weak_agent_redundancy: 3,
                critic_ratio_percent: 25,
                reducer_quorum: 2,
                worktree_branch_prefix: "zyal/super".to_string(),
                integration_branch: "zyal_super_integration".to_string(),
            },
            memory: MemoryCompoundingPolicy::default(),
            repo_graph: RepoGraphPolicy::default(),
            sandbox: PersistentSandboxPolicy::default(),
            parity: Some(ParityClosurePolicy::default()),
            hardening: HardeningPolicy::default(),
            signoff: SignoffPolicy::default(),
            phases: canonical_phases(),
        }
    }

    /// Validate the plan before daemon registration or execution.
    pub fn validate(&self) -> RuntimeResult<()> {
        if self.schema_version != SUPER_REASONING_SCHEMA_VERSION {
            return Err(RuntimeError::invalid(format!(
                "unsupported super reasoning schema '{}', expected '{}'",
                self.schema_version, SUPER_REASONING_SCHEMA_VERSION
            )));
        }
        if self.mission_id.trim().is_empty() {
            return Err(RuntimeError::invalid(
                "super reasoning mission_id is required",
            ));
        }
        if self.objective.trim().is_empty() {
            return Err(RuntimeError::invalid(
                "super reasoning objective is required",
            ));
        }
        if self.phase_count.min == 0 || self.phase_count.min > self.phase_count.max {
            return Err(RuntimeError::invalid(
                "phase_count must have min > 0 and min <= max",
            ));
        }
        let phase_len = self.phases.len();
        if phase_len < self.phase_count.min || phase_len > self.phase_count.max {
            return Err(RuntimeError::invalid(format!(
                "super reasoning plans require {}-{} phases; got {}",
                self.phase_count.min, self.phase_count.max, phase_len
            )));
        }
        if self.swarm.max_workers == 0 || self.swarm.max_workers > DEFAULT_MAX_WORKERS {
            return Err(RuntimeError::invalid(format!(
                "swarm.max_workers must be between 1 and {DEFAULT_MAX_WORKERS}"
            )));
        }
        if self.swarm.weak_agent_redundancy == 0 {
            return Err(RuntimeError::invalid(
                "swarm.weak_agent_redundancy must be at least 1",
            ));
        }
        if self.swarm.critic_ratio_percent > 100 {
            return Err(RuntimeError::invalid(
                "swarm.critic_ratio_percent cannot exceed 100",
            ));
        }

        let mut ids = BTreeSet::new();
        for phase in &self.phases {
            if phase.id.trim().is_empty() {
                return Err(RuntimeError::invalid("phase id is required"));
            }
            if !ids.insert(phase.id.clone()) {
                return Err(RuntimeError::invalid(format!(
                    "duplicate phase id '{}'",
                    phase.id
                )));
            }
            if phase.name.trim().is_empty() || phase.objective.trim().is_empty() {
                return Err(RuntimeError::invalid(format!(
                    "phase '{}' requires name and objective",
                    phase.id
                )));
            }
            if phase.workers == 0 || phase.workers > self.swarm.max_workers {
                return Err(RuntimeError::invalid(format!(
                    "phase '{}' workers must be between 1 and swarm.max_workers ({})",
                    phase.id, self.swarm.max_workers
                )));
            }
            if phase.tasks.is_empty() {
                return Err(RuntimeError::invalid(format!(
                    "phase '{}' must declare at least one task",
                    phase.id
                )));
            }
            if phase.lanes.is_empty() {
                return Err(RuntimeError::invalid(format!(
                    "phase '{}' must declare at least one reasoning lane",
                    phase.id
                )));
            }
            if phase.acceptance.is_empty() {
                return Err(RuntimeError::invalid(format!(
                    "phase '{}' must declare at least one acceptance gate",
                    phase.id
                )));
            }
            for dep in &phase.depends_on {
                if dep == &phase.id {
                    return Err(RuntimeError::invalid(format!(
                        "phase '{}' cannot depend on itself",
                        phase.id
                    )));
                }
            }
        }

        let _ = self.topological_phase_ids()?;

        if let Some(parity) = &self.parity {
            if parity.enabled {
                if parity.workflows.is_empty() {
                    return Err(RuntimeError::invalid(
                        "enabled parity policy requires at least one workflow",
                    ));
                }
                if parity.reference_command.trim().is_empty()
                    || parity.candidate_command.trim().is_empty()
                    || parity.manifest.trim().is_empty()
                    || parity.oracle.trim().is_empty()
                {
                    return Err(RuntimeError::invalid(
                        "enabled parity policy requires reference_command, candidate_command, manifest, and oracle",
                    ));
                }
            }
        }

        Ok(())
    }

    /// Return phase ids in deterministic topological order.
    pub fn topological_phase_ids(&self) -> RuntimeResult<Vec<String>> {
        let (mut indegree, mut children) = self.dependency_maps()?;
        let mut queue: VecDeque<String> = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then(|| id.clone()))
            .collect();
        let mut out = Vec::with_capacity(self.phases.len());
        while let Some(id) = queue.pop_front() {
            out.push(id.clone());
            if let Some(next) = children.remove(&id) {
                for child in next {
                    let degree = indegree
                        .get_mut(&child)
                        .ok_or_else(|| RuntimeError::invalid("dependency map corrupt"))?;
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }
        if out.len() != self.phases.len() {
            return Err(RuntimeError::invalid(
                "super reasoning phase dependency graph contains a cycle",
            ));
        }
        Ok(out)
    }

    /// Return deterministic parallel waves from the phase DAG.
    pub fn parallel_waves(&self) -> RuntimeResult<Vec<Vec<String>>> {
        let mut remaining: BTreeMap<String, BTreeSet<String>> = self
            .phases
            .iter()
            .map(|phase| {
                (
                    phase.id.clone(),
                    phase.depends_on.iter().cloned().collect::<BTreeSet<_>>(),
                )
            })
            .collect();
        let valid_ids: BTreeSet<String> = remaining.keys().cloned().collect();
        for (id, deps) in &remaining {
            for dep in deps {
                if !valid_ids.contains(dep) {
                    return Err(RuntimeError::invalid(format!(
                        "phase '{id}' depends on unknown phase '{dep}'"
                    )));
                }
            }
        }

        let mut waves = Vec::new();
        while !remaining.is_empty() {
            let wave: Vec<String> = remaining
                .iter()
                .filter_map(|(id, deps)| deps.is_empty().then(|| id.clone()))
                .collect();
            if wave.is_empty() {
                return Err(RuntimeError::invalid(
                    "super reasoning phase dependency graph contains a cycle",
                ));
            }
            for id in &wave {
                remaining.remove(id);
            }
            for deps in remaining.values_mut() {
                for id in &wave {
                    deps.remove(id);
                }
            }
            waves.push(wave);
        }
        Ok(waves)
    }

    /// Return phases that are runnable given a set of completed phase ids.
    pub fn ready_phase_ids<I, S>(&self, completed_phase_ids: I) -> RuntimeResult<Vec<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let completed: BTreeSet<String> = completed_phase_ids
            .into_iter()
            .map(|s| s.as_ref().to_string())
            .collect();
        let valid_ids: BTreeSet<String> = self.phases.iter().map(|p| p.id.clone()).collect();
        for id in &completed {
            if !valid_ids.contains(id) {
                return Err(RuntimeError::invalid(format!(
                    "completed phase '{id}' is not in this plan"
                )));
            }
        }
        Ok(self
            .phases
            .iter()
            .filter(|phase| {
                !completed.contains(&phase.id)
                    && phase.depends_on.iter().all(|dep| completed.contains(dep))
            })
            .map(|phase| phase.id.clone())
            .collect())
    }

    /// Return the ZYAL blocks that should be present in the source runbook.
    pub fn required_zyal_blocks(&self) -> Vec<&'static str> {
        let mut blocks = vec![
            "workflow",
            "fleet",
            "memory",
            "evidence",
            "approvals",
            "budgets",
            "sandbox",
            "observability",
            "checkpoint",
            "done",
        ];
        if self.repo_graph.enabled {
            blocks.push("repo_intelligence");
        }
        if self.parity.as_ref().is_some_and(|p| p.enabled) {
            blocks.push("hooks");
        }
        blocks
    }

    fn dependency_maps(
        &self,
    ) -> RuntimeResult<(BTreeMap<String, usize>, BTreeMap<String, Vec<String>>)> {
        let mut ids = BTreeSet::new();
        for phase in &self.phases {
            if !ids.insert(phase.id.clone()) {
                return Err(RuntimeError::invalid(format!(
                    "duplicate phase id '{}'",
                    phase.id
                )));
            }
        }
        let mut indegree: BTreeMap<String, usize> = ids.iter().map(|id| (id.clone(), 0)).collect();
        let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for phase in &self.phases {
            // Dedupe deps per phase — a duplicate entry (e.g. `depends_on:
            // ["p01", "p01"]`) would inflate indegree past what the topo
            // walk can decrement, falsely reporting a cycle.
            let unique_deps: std::collections::BTreeSet<&String> =
                phase.depends_on.iter().collect();
            for dep in unique_deps {
                if !ids.contains(dep) {
                    return Err(RuntimeError::invalid(format!(
                        "phase '{}' depends on unknown phase '{}'",
                        phase.id, dep
                    )));
                }
                *indegree
                    .get_mut(&phase.id)
                    .ok_or_else(|| RuntimeError::invalid("dependency map corrupt"))? += 1;
                children
                    .entry(dep.clone())
                    .or_default()
                    .push(phase.id.clone());
            }
        }
        for child_list in children.values_mut() {
            child_list.sort();
        }
        Ok((indegree, children))
    }
}

fn canonical_phases() -> Vec<MacroPhase> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_plan_validates() {
        let plan = SuperReasoningPlan::default_megaproject_plan(
            "mission_1",
            "Build a reference-compatible replacement and close parity/perf gaps.",
        );
        plan.validate().unwrap();
        assert_eq!(plan.phases.len(), 12);
        assert_eq!(
            plan.topological_phase_ids().unwrap().first().unwrap(),
            "source_of_truth"
        );
    }

    #[test]
    fn ready_phases_respect_dependencies() {
        let plan = SuperReasoningPlan::default_megaproject_plan("mission_1", "objective");
        assert_eq!(
            plan.ready_phase_ids(std::iter::empty::<String>()).unwrap(),
            vec!["source_of_truth"]
        );

        let ready = plan
            .ready_phase_ids(["source_of_truth".to_string()])
            .unwrap();
        assert!(ready.contains(&"architecture_blueprint".to_string()));
        assert!(ready.contains(&"repo_graph_bootstrap".to_string()));
    }

    #[test]
    fn rejects_cycles() {
        let mut plan = SuperReasoningPlan::default_megaproject_plan("mission_1", "objective");
        plan.phases[0]
            .depends_on
            .push("architecture_blueprint".to_string());
        assert!(plan.validate().is_err());
    }

    #[test]
    fn rejects_over_cap_worker_plan() {
        let mut plan = SuperReasoningPlan::default_megaproject_plan("mission_1", "objective");
        plan.swarm.max_workers = DEFAULT_MAX_WORKERS + 1;
        assert!(plan.validate().is_err());
    }

    #[test]
    fn rejects_incomplete_parity_policy() {
        let mut plan = SuperReasoningPlan::default_megaproject_plan("mission_1", "objective");
        let parity = plan.parity.as_mut().unwrap();
        parity.reference_command.clear();
        assert!(plan.validate().is_err());
    }
}
