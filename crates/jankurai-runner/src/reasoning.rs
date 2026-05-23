//! Advanced reasoning contract for ZYAL port workflows.
//!
//! The runtime stores structured summaries and evidence, never raw private
//! chain-of-thought. Confidence is intentionally capped unless an artifact has
//! executable or stronger evidence.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hashing::sha256_json;
use crate::port::MAX_PORT_WORKERS;

/// Default confidence cap for unsupported or non-executable reasoning.
pub const DEFAULT_CONFIDENCE_CAP: f64 = 0.35;

/// Advanced reasoning runtime options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvancedReasoningConfig {
    /// Enable the advanced state machine.
    #[serde(default)]
    pub enabled: bool,
    /// Requested worker lanes. Clamped to ten.
    #[serde(default = "default_worker_cap")]
    pub worker_cap: usize,
    /// Maximum confidence without executable evidence.
    #[serde(default = "default_confidence_cap")]
    pub confidence_cap: f64,
    /// Store raw model reasoning text. Defaults false and should stay false.
    #[serde(default)]
    pub store_raw_reasoning: bool,
    /// Permit power models outside reducer/critic/escalation routes.
    #[serde(default)]
    pub allow_power_for_routine_roles: bool,
}

impl Default for AdvancedReasoningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            worker_cap: default_worker_cap(),
            confidence_cap: DEFAULT_CONFIDENCE_CAP,
            store_raw_reasoning: false,
            allow_power_for_routine_roles: false,
        }
    }
}

impl AdvancedReasoningConfig {
    /// Return the effective worker cap enforced by the runtime.
    pub fn effective_worker_cap(&self) -> usize {
        self.worker_cap.clamp(1, MAX_PORT_WORKERS)
    }

    /// Return the effective confidence cap.
    pub fn effective_confidence_cap(&self) -> f64 {
        if self.confidence_cap.is_finite() {
            self.confidence_cap.clamp(0.0, DEFAULT_CONFIDENCE_CAP)
        } else {
            DEFAULT_CONFIDENCE_CAP
        }
    }
}

/// Role that produced a reasoning artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningRole {
    /// Frames the request and success criteria.
    Framer,
    /// Retrieves code, docs, memory, and parity context.
    Retriever,
    /// Proposes stages or phase slices.
    Planner,
    /// Builds one bounded implementation lane.
    Builder,
    /// Tries to falsify a candidate.
    Critic,
    /// Runs executable or source-grounded checks.
    Verifier,
    /// Reduces multiple candidates into a host-owned decision.
    Reducer,
    /// Curates durable memory after verification.
    MemoryCurator,
}

/// Reasoning artifact kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningArtifactKind {
    /// Crystallized request contract.
    TaskContract,
    /// Retrieved evidence/context pack.
    ContextPack,
    /// Candidate stage plan.
    StageProposal,
    /// Critique or objection set.
    Critique,
    /// Final master plan decision.
    MasterPlan,
    /// Phase-level plan.
    PhasePlan,
    /// Worker build receipt.
    BuildReceipt,
    /// Verification receipt.
    VerificationReceipt,
    /// Parity gap report.
    ParityGap,
    /// Baseline-vs-tournament reasoning benchmark.
    ReasoningBenchmark,
    /// Durable memory candidate.
    MemoryCapsule,
}

/// Evidence strength. E4+ is executable enough to lift confidence caps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLevel {
    /// E0: unsupported model claim.
    Unsupported,
    /// E1: internally consistent only.
    InternalConsistency,
    /// E2: independent agreement.
    IndependentAgreement,
    /// E3: source/log/code grounded.
    ExternalGrounding,
    /// E4: executable verification.
    Executable,
    /// E5: survived adversarial review.
    AdversarialSurvival,
    /// E6: durable historical support.
    HistoricalDurability,
}

impl EvidenceLevel {
    /// Whether this level is executable or stronger.
    pub fn has_executable_evidence(self) -> bool {
        self >= Self::Executable
    }
}

/// One structured reasoning artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningArtifact {
    /// Artifact id.
    pub id: String,
    /// Owning run id.
    pub run_id: String,
    /// Producer role.
    pub role: ReasoningRole,
    /// Artifact kind.
    pub kind: ReasoningArtifactKind,
    /// Short title.
    pub title: String,
    /// Stored summary, not chain-of-thought.
    pub summary: String,
    /// Structured payload.
    #[serde(default)]
    pub payload_json: Value,
    /// Evidence strength.
    pub evidence_level: EvidenceLevel,
    /// Calibrated confidence.
    pub confidence: f64,
    /// Source artifact ids.
    #[serde(default)]
    pub source_artifact_ids: Vec<String>,
    /// Verification receipt ids.
    #[serde(default)]
    pub verifier_receipt_ids: Vec<String>,
    /// Raw model reasoning. Redacted before storage unless explicitly allowed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_reasoning: Option<String>,
    /// Stable SHA-256 over the storage-safe content.
    pub content_hash: String,
    /// Artifact status.
    pub status: String,
}

impl ReasoningArtifact {
    /// Construct a storage-safe artifact and apply confidence/redaction rules.
    pub fn new(
        id: impl Into<String>,
        run_id: impl Into<String>,
        role: ReasoningRole,
        kind: ReasoningArtifactKind,
        title: impl Into<String>,
        summary: impl Into<String>,
        evidence_level: EvidenceLevel,
        confidence: f64,
        payload_json: Value,
    ) -> Self {
        let mut artifact = Self {
            id: id.into(),
            run_id: run_id.into(),
            role,
            kind,
            title: title.into(),
            summary: summary.into(),
            payload_json,
            evidence_level,
            confidence,
            source_artifact_ids: Vec::new(),
            verifier_receipt_ids: Vec::new(),
            raw_reasoning: None,
            content_hash: String::new(),
            status: "candidate".to_string(),
        };
        artifact.refresh_hash();
        artifact
    }

    /// Apply config policy before durable storage.
    pub fn prepare_for_storage(&mut self, config: &AdvancedReasoningConfig) {
        if !config.store_raw_reasoning {
            self.raw_reasoning = None;
        }
        if !self.evidence_level.has_executable_evidence() {
            self.confidence = self
                .confidence
                .min(config.effective_confidence_cap())
                .max(0.0);
        } else if self.confidence.is_finite() {
            self.confidence = self.confidence.clamp(0.0, 1.0);
        } else {
            self.confidence = 0.0;
        }
        self.refresh_hash();
    }

    /// Recompute the storage-safe hash.
    pub fn refresh_hash(&mut self) {
        self.content_hash = stable_reasoning_hash(&ReasoningHashPayload {
            id: &self.id,
            run_id: &self.run_id,
            role: self.role,
            kind: self.kind,
            title: &self.title,
            summary: &self.summary,
            payload_json: &self.payload_json,
            evidence_level: self.evidence_level,
            confidence: self.confidence,
            source_artifact_ids: &self.source_artifact_ids,
            verifier_receipt_ids: &self.verifier_receipt_ids,
            status: &self.status,
        });
    }
}

#[derive(Serialize)]
struct ReasoningHashPayload<'a> {
    id: &'a str,
    run_id: &'a str,
    role: ReasoningRole,
    kind: ReasoningArtifactKind,
    title: &'a str,
    summary: &'a str,
    payload_json: &'a Value,
    evidence_level: EvidenceLevel,
    confidence: f64,
    source_artifact_ids: &'a [String],
    verifier_receipt_ids: &'a [String],
    status: &'a str,
}

/// Edge between artifacts in the reasoning graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningEdge {
    /// Owning run id.
    pub run_id: String,
    /// Source artifact id.
    pub src_artifact_id: String,
    /// Destination artifact id.
    pub dst_artifact_id: String,
    /// Edge kind.
    pub kind: String,
    /// Optional weight.
    pub weight: Option<f64>,
    /// Structured payload.
    #[serde(default)]
    pub payload_json: Value,
}

impl ReasoningEdge {
    /// Validate graph edge invariants.
    pub fn validate(&self) -> Result<()> {
        if self.src_artifact_id == self.dst_artifact_id {
            return Err(anyhow!("reasoning edge cannot point to itself"));
        }
        if self.kind.trim().is_empty() {
            return Err(anyhow!("reasoning edge kind cannot be empty"));
        }
        if let Some(weight) = self.weight {
            if !weight.is_finite() {
                return Err(anyhow!("reasoning edge weight must be finite"));
            }
        }
        Ok(())
    }
}

/// One independent reasoning lane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningLane {
    /// Lane id.
    pub id: String,
    /// Owning run id.
    pub run_id: String,
    /// Lane role.
    pub role: ReasoningRole,
    /// Diversity strategy.
    pub strategy: String,
    /// Lane status.
    pub status: String,
    /// Artifacts produced by this lane.
    #[serde(default)]
    pub artifact_ids: Vec<String>,
    /// Declared write scope.
    #[serde(default)]
    pub write_scope: Vec<String>,
    /// Worker id if assigned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    /// Lane confidence after reduction.
    pub confidence: f64,
}

/// Reasoning tournament metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningTournament {
    /// Tournament id.
    pub id: String,
    /// Owning run id.
    pub run_id: String,
    /// Objective.
    pub objective: String,
    /// Lane ids.
    #[serde(default)]
    pub lane_ids: Vec<String>,
    /// Reducer artifact id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reducer_artifact_id: Option<String>,
    /// Status.
    pub status: String,
}

/// Durable memory write candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryCapsule {
    /// Capsule id.
    pub id: String,
    /// Owning run id.
    pub run_id: String,
    /// Source artifact id.
    pub artifact_id: String,
    /// Memory scope.
    pub scope: String,
    /// `verified` or `rejected`.
    pub status: String,
    /// Stored summary.
    pub summary: String,
    /// Evidence strength.
    pub evidence_level: EvidenceLevel,
    /// Confidence.
    pub confidence: f64,
    /// Structured payload.
    #[serde(default)]
    pub payload_json: Value,
    /// Stable content hash.
    pub content_hash: String,
}

impl MemoryCapsule {
    /// Return true if this capsule is eligible for permanent memory.
    pub fn can_write_permanent(&self) -> bool {
        matches!(self.status.as_str(), "verified" | "rejected")
            && self.evidence_level >= EvidenceLevel::ExternalGrounding
            && !self.artifact_id.trim().is_empty()
    }
}

/// Per-model reliability accumulator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelReliability {
    /// Model id.
    pub model_id: String,
    /// Role or task kind.
    pub role: String,
    /// Task kind.
    pub task_kind: String,
    /// Success count.
    pub success_count: u64,
    /// Failure count.
    pub failure_count: u64,
    /// Winner count.
    pub winner_count: u64,
    /// Total latency.
    pub total_latency_ms: u64,
    /// Total cost.
    pub total_cost_usd: f64,
    /// Derived score.
    pub score: f64,
}

impl ModelReliability {
    /// Construct an empty accumulator.
    pub fn new(
        model_id: impl Into<String>,
        role: impl Into<String>,
        task_kind: impl Into<String>,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            role: role.into(),
            task_kind: task_kind.into(),
            success_count: 0,
            failure_count: 0,
            winner_count: 0,
            total_latency_ms: 0,
            total_cost_usd: 0.0,
            score: 0.0,
        }
    }

    /// Update counts from one outcome.
    pub fn record(&mut self, success: bool, winner: bool, latency_ms: u64, cost_usd: f64) {
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }
        if winner {
            self.winner_count += 1;
        }
        self.total_latency_ms = self.total_latency_ms.saturating_add(latency_ms);
        self.total_cost_usd += cost_usd.max(0.0);
        self.score = self.compute_score();
    }

    fn compute_score(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            return 0.0;
        }
        let success_rate = self.success_count as f64 / total as f64;
        let winner_bonus = self.winner_count as f64 / total as f64 * 0.15;
        (success_rate + winner_bonus).clamp(0.0, 1.0)
    }
}

/// Stable SHA-256 over a serializable payload.
pub fn stable_reasoning_hash<T: Serialize>(value: &T) -> String {
    sha256_json(value, "reasoning_hash")
}

fn default_worker_cap() -> usize {
    3
}

fn default_confidence_cap() -> f64 {
    DEFAULT_CONFIDENCE_CAP
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn worker_cap_is_clamped() {
        let config = AdvancedReasoningConfig {
            worker_cap: 99,
            ..AdvancedReasoningConfig::default()
        };
        assert_eq!(config.effective_worker_cap(), MAX_PORT_WORKERS);
    }

    #[test]
    fn confidence_caps_without_executable_evidence() {
        let config = AdvancedReasoningConfig::default();
        let mut artifact = ReasoningArtifact::new(
            "a1",
            "run",
            ReasoningRole::Planner,
            ReasoningArtifactKind::StageProposal,
            "plan",
            "summary",
            EvidenceLevel::IndependentAgreement,
            0.92,
            json!({"claim": "x"}),
        );
        artifact.prepare_for_storage(&config);
        assert_eq!(artifact.confidence, DEFAULT_CONFIDENCE_CAP);

        artifact.evidence_level = EvidenceLevel::Executable;
        artifact.confidence = 0.92;
        artifact.prepare_for_storage(&config);
        assert_eq!(artifact.confidence, 0.92);
    }

    #[test]
    fn raw_reasoning_is_redacted_by_default() {
        let config = AdvancedReasoningConfig::default();
        let mut artifact = ReasoningArtifact::new(
            "a1",
            "run",
            ReasoningRole::Planner,
            ReasoningArtifactKind::StageProposal,
            "plan",
            "summary",
            EvidenceLevel::Unsupported,
            0.5,
            json!({}),
        );
        artifact.raw_reasoning = Some("private reasoning".into());
        artifact.prepare_for_storage(&config);
        assert_eq!(artifact.raw_reasoning, None);
    }

    #[test]
    fn stable_hash_is_repeatable() {
        let one = stable_reasoning_hash(&json!({"a": 1, "b": ["x"]}));
        let two = stable_reasoning_hash(&json!({"a": 1, "b": ["x"]}));
        assert_eq!(one, two);
        assert_eq!(one.len(), 64);
    }

    #[test]
    fn edge_validation_rejects_self_edges() {
        let edge = ReasoningEdge {
            run_id: "run".into(),
            src_artifact_id: "a".into(),
            dst_artifact_id: "a".into(),
            kind: "supports".into(),
            weight: Some(1.0),
            payload_json: json!({}),
        };
        assert!(edge.validate().is_err());
    }

    #[test]
    fn permanent_memory_requires_verified_or_rejected_evidence() {
        let capsule = MemoryCapsule {
            id: "m1".into(),
            run_id: "run".into(),
            artifact_id: "a1".into(),
            scope: "repo".into(),
            status: "verified".into(),
            summary: "lesson".into(),
            evidence_level: EvidenceLevel::ExternalGrounding,
            confidence: 0.8,
            payload_json: json!({}),
            content_hash: "hash".into(),
        };
        assert!(capsule.can_write_permanent());
    }
}
