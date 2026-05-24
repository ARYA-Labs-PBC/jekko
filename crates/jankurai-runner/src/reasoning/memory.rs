use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::EvidenceLevel;

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
