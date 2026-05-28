//! Typed control-plane support for very large ZYAL reasoning missions.
//!
//! The existing ZYAL contract already has the required authoring primitives
//! (`workflow`, `fleet`, `memory`, `evidence`, `repo_intelligence`,
//! `approvals`, `sandbox`, `checkpoint`, `done`). This module provides the
//! runtime-normalised shape that binds those blocks into a durable phase DAG
//! for multi-agent, multi-day work without adding target-specific schema such
//! as "Redis" or "SQLite" to the contract.
//!
//! Sub-modules:
//! * [`canonical`] — the seed 12-phase plan.
//! * [`phase`] — phase / lane / task value-types.
//! * [`policies`] — swarm / memory / sandbox / parity policy types.
//! * [`builder`] — [`SuperReasoningPlan::default_megaproject_plan`] +
//!   [`SuperReasoningPlan::required_zyal_blocks`].
//! * [`validate`] — [`SuperReasoningPlan::validate`].
//! * [`graph`] — phase-DAG queries (topo order, parallel waves, ready set).

use serde::{Deserialize, Serialize};

mod builder;
mod canonical;
mod graph;
mod phase;
mod policies;
mod validate;
pub use canonical::*;
pub use phase::*;
pub use policies::*;

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
