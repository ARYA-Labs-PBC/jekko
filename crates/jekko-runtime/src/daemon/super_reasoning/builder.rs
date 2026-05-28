//! Constructor helpers for [`SuperReasoningPlan`].
//!
//! Default plans suitable for very large ZYAL missions (database rewrites,
//! compiler ports, protocol-compatible replacements, research programs, large
//! refactors). The target-specific assumptions live in the host runbook —
//! these constructors only seed shape and policy.

use super::{
    canonical_phases, HardeningPolicy, IsolationMode, MemoryCompoundingPolicy, ParityClosurePolicy,
    PersistentSandboxPolicy, PhaseCountTarget, RepoGraphPolicy, SignoffPolicy, SuperReasoningPlan,
    SwarmPolicy, DEFAULT_MAX_PHASES, DEFAULT_MAX_WORKERS, DEFAULT_MIN_PHASES,
    SUPER_REASONING_SCHEMA_VERSION,
};

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
}
