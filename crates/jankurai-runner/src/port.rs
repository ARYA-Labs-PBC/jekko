//! Generic ZYAL port workflow contract and resume-safe state tags.

use serde::{Deserialize, Serialize};

use crate::model_policy::ModelPolicy;

/// Maximum worker cap for autonomous port runs.
pub const MAX_PORT_WORKERS: usize = 10;

/// Port target request captured from a ZYAL file or CLI prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortTargetRequest {
    /// Reference system name.
    pub target: String,
    /// Replacement system name.
    pub replacement: String,
    /// Reference repository path or URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_repo: Option<String>,
    /// Candidate repository path or URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement_repo: Option<String>,
    /// Original user request.
    pub request: String,
    /// Requested worker cap. Clamped to [`MAX_PORT_WORKERS`].
    pub worker_cap: usize,
}

/// Evidence input kind for live-proof planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceInputKind {
    /// One local file.
    File,
    /// Local files matched by a bounded glob.
    Glob,
    /// External URL. Disabled unless the runtime explicitly enables URL evidence.
    Url,
}

/// One file, glob, or URL input used to ground Stage-0 planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceInput {
    /// Stable evidence id.
    pub id: String,
    /// Evidence source kind.
    pub kind: EvidenceInputKind,
    /// Role in the proof prompt, such as `target_plan` or `workflow_doc`.
    pub role: String,
    /// Local path, glob, or URL.
    pub path_or_url: String,
    /// Maximum bytes read from each expanded source.
    #[serde(default = "default_evidence_max_bytes")]
    pub max_bytes: usize,
}

/// Live model call budget for a port run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveCallBudget {
    /// Maximum successful or attempted model calls.
    #[serde(default = "default_live_max_calls")]
    pub max_calls: usize,
    /// Maximum calls allowed to run at once.
    #[serde(default = "default_live_max_parallel")]
    pub max_parallel: usize,
    /// Require live receipts and reject deterministic model substitutions.
    #[serde(default)]
    pub require_live: bool,
}

impl Default for LiveCallBudget {
    fn default() -> Self {
        Self {
            max_calls: default_live_max_calls(),
            max_parallel: default_live_max_parallel(),
            require_live: false,
        }
    }
}

/// Proofs requested for a port run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortProofs {
    /// Produce a target-derived Stage-0 master plan.
    #[serde(default)]
    pub redis_jedis_stage0: bool,
    /// Produce a deterministic baseline-vs-tournament reasoning benchmark.
    #[serde(default)]
    pub reasoning_benchmark: bool,
}

/// Runtime proof options shared by generic and advanced port runners.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortRuntimeOptions {
    /// Evidence inputs for proof generation.
    #[serde(default)]
    pub evidence_inputs: Vec<EvidenceInput>,
    /// Live model call budget.
    #[serde(default)]
    pub live_call_budget: LiveCallBudget,
    /// Requested proof artifacts.
    #[serde(default)]
    pub proofs: PortProofs,
    /// Model routing policy.
    #[serde(default)]
    pub model_policy: ModelPolicy,
}

impl Default for PortRuntimeOptions {
    fn default() -> Self {
        Self {
            evidence_inputs: Vec::new(),
            live_call_budget: LiveCallBudget::default(),
            proofs: PortProofs::default(),
            model_policy: ModelPolicy::default(),
        }
    }
}

impl PortTargetRequest {
    /// Return the effective worker cap enforced by the runner.
    pub fn effective_worker_cap(&self) -> usize {
        self.worker_cap.clamp(1, MAX_PORT_WORKERS)
    }
}

/// Phase state persisted for crash-safe resume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    /// Stage or phase is being drafted.
    Drafting,
    /// Ordered plan exists.
    Planned,
    /// Workers are building task slices.
    Building,
    /// Proof lanes are running.
    Verifying,
    /// Cross-phase integration is being repaired.
    Healing,
    /// Parity lab is running.
    Parity,
    /// Phase is complete.
    Complete,
    /// Human or budget blocker.
    Blocked,
    /// Repeated failure parked the phase.
    Quarantined,
}

impl PhaseStatus {
    /// Whether this status is terminal for autonomous progression.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Blocked | Self::Quarantined)
    }
}

/// Master task state persisted for crash-safe resume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MasterTaskStatus {
    /// Task is queued.
    Queued,
    /// Task has a worker assignment.
    Assigned,
    /// Worker is running.
    Running,
    /// Proof lane failed.
    ProofFailed,
    /// Jankurai audit failed.
    AuditFailed,
    /// Worker branch merged.
    Merged,
    /// Worker changes rolled back.
    RolledBack,
    /// Repeated failure parked the task.
    Quarantined,
    /// Task is complete.
    Done,
}

impl MasterTaskStatus {
    /// Whether this task may be assigned to a worker.
    pub fn is_assignable(self) -> bool {
        matches!(
            self,
            Self::Queued | Self::ProofFailed | Self::AuditFailed | Self::RolledBack
        )
    }
}

/// One stage in the generated master plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortStage {
    /// Stage id.
    pub id: String,
    /// Stage order.
    pub ordinal: usize,
    /// Human-readable name.
    pub name: String,
    /// Stage objective.
    pub objective: String,
    /// Current status.
    pub status: PhaseStatus,
}

/// One task in the generated master plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortMasterTask {
    /// Task id.
    pub id: String,
    /// Owning stage id.
    pub stage_id: String,
    /// Task title.
    pub title: String,
    /// Declared write scope.
    pub write_scope: Vec<String>,
    /// Proof command or lane.
    pub proof_lane: String,
    /// Current task status.
    pub status: MasterTaskStatus,
}

/// Deterministic starter plan used before model-backed phase finalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortMasterPlan {
    /// Captured target request.
    pub target: PortTargetRequest,
    /// Ordered stages.
    pub stages: Vec<PortStage>,
    /// Ordered master tasks.
    pub tasks: Vec<PortMasterTask>,
}

fn default_evidence_max_bytes() -> usize {
    64 * 1024
}

fn default_live_max_calls() -> usize {
    20
}

fn default_live_max_parallel() -> usize {
    10
}

/// Build a generic starter plan without target-specific hard-coding.
pub fn draft_master_plan(target: PortTargetRequest) -> PortMasterPlan {
    let names = [
        (
            "discover",
            "Discover target behavior, docs, tests, and public contracts.",
        ),
        (
            "skeleton",
            "Create the replacement project skeleton and compatibility adapters.",
        ),
        (
            "correctness",
            "Implement required behavior behind target-switched parity cases.",
        ),
        (
            "integration",
            "Fuse phases and heal cross-phase regressions.",
        ),
        (
            "parity",
            "Close exhaustive correctness and performance parity gaps.",
        ),
    ];
    let stages: Vec<PortStage> = names
        .iter()
        .enumerate()
        .map(|(idx, (id, objective))| PortStage {
            id: format!("stage-{}", id),
            ordinal: idx + 1,
            name: id.to_string(),
            objective: (*objective).to_string(),
            status: if idx == 0 {
                PhaseStatus::Drafting
            } else {
                PhaseStatus::Planned
            },
        })
        .collect();
    let tasks = stages
        .iter()
        .map(|stage| PortMasterTask {
            id: format!("task-{}", stage.name),
            stage_id: stage.id.clone(),
            title: format!("{}: {}", target.replacement, stage.objective),
            write_scope: vec!["src/**".to_string(), "tests/**".to_string()],
            proof_lane: "just zyal-port-fast".to_string(),
            status: MasterTaskStatus::Queued,
        })
        .collect();
    PortMasterPlan {
        target,
        stages,
        tasks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_cap_is_clamped_to_ten() {
        let req = PortTargetRequest {
            target: "Reference".into(),
            replacement: "Candidate".into(),
            target_repo: None,
            replacement_repo: None,
            request: "port it".into(),
            worker_cap: 20,
        };
        assert_eq!(req.effective_worker_cap(), MAX_PORT_WORKERS);
    }

    #[test]
    fn starter_plan_is_generic_and_ordered() {
        let req = PortTargetRequest {
            target: "MiniKV".into(),
            replacement: "MiniKV Rust".into(),
            target_repo: None,
            replacement_repo: None,
            request: "port MiniKV".into(),
            worker_cap: 4,
        };
        let plan = draft_master_plan(req);
        assert_eq!(plan.stages.first().unwrap().name, "discover");
        assert_eq!(plan.stages.last().unwrap().name, "parity");
        assert_eq!(plan.tasks.len(), plan.stages.len());
        assert!(plan
            .tasks
            .iter()
            .all(|t| t.status == MasterTaskStatus::Queued));
    }
}
