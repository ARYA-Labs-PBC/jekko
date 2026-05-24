use serde::{Deserialize, Serialize};

/// Row in `daemon_reasoning_artifact`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningArtifactRow {
    /// Artifact id.
    pub id: String,
    /// Owning run id.
    pub run_id: String,
    /// Producer role.
    pub role: String,
    /// Artifact kind.
    pub kind: String,
    /// Title.
    pub title: String,
    /// Stored summary.
    pub summary: String,
    /// Evidence level.
    pub evidence_level: String,
    /// Calibrated confidence.
    pub confidence: f64,
    /// Structured payload.
    pub payload_json: Option<serde_json::Value>,
    /// Stable content hash.
    pub content_hash: String,
    /// Artifact status.
    pub status: String,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_reasoning_edge`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningEdgeRow {
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
    pub payload_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
}

/// Row in `daemon_reasoning_lane`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningLaneRow {
    /// Lane id.
    pub id: String,
    /// Owning run id.
    pub run_id: String,
    /// Lane role.
    pub role: String,
    /// Diversity strategy.
    pub strategy: String,
    /// Lane status.
    pub status: String,
    /// Produced artifact ids.
    pub artifact_ids: Vec<String>,
    /// Declared write scope.
    pub write_scope: Vec<String>,
    /// Worker id.
    pub worker_id: Option<String>,
    /// Lane confidence.
    pub confidence: f64,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_memory_capsule`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryCapsuleRow {
    /// Capsule id.
    pub id: String,
    /// Owning run id.
    pub run_id: String,
    /// Source artifact id.
    pub artifact_id: String,
    /// Memory scope.
    pub scope: String,
    /// Capsule status.
    pub status: String,
    /// Stored summary.
    pub summary: String,
    /// Evidence level.
    pub evidence_level: String,
    /// Confidence.
    pub confidence: f64,
    /// Structured payload.
    pub payload_json: Option<serde_json::Value>,
    /// Stable content hash.
    pub content_hash: String,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_model_reliability`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelReliabilityRow {
    /// Model id.
    pub model_id: String,
    /// Role.
    pub role: String,
    /// Task kind.
    pub task_kind: String,
    /// Success count.
    pub success_count: i64,
    /// Failure count.
    pub failure_count: i64,
    /// Winner count.
    pub winner_count: i64,
    /// Total latency.
    pub total_latency_ms: i64,
    /// Total cost.
    pub total_cost_usd: f64,
    /// Reliability score.
    pub score: f64,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}
