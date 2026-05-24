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

impl PortTargetRequest {
    /// Return the effective worker cap enforced by the runner.
    pub fn effective_worker_cap(&self) -> usize {
        self.worker_cap.clamp(1, MAX_PORT_WORKERS)
    }
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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

fn default_evidence_max_bytes() -> usize {
    64 * 1024
}

fn default_live_max_calls() -> usize {
    20
}

fn default_live_max_parallel() -> usize {
    10
}
