use serde::{Deserialize, Serialize};

/// Row in `daemon_finding`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonFindingRow {
    /// Finding id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Iteration that captured this finding.
    pub iteration: i64,
    /// Jankurai rule id.
    pub rule_id: String,
    /// Stable finding fingerprint.
    pub fingerprint: String,
    /// Severity label.
    pub severity: String,
    /// Paths touched by this finding.
    pub paths: Vec<String>,
    /// Cap id when the finding represents a cap.
    pub cap: Option<String>,
    /// Queue status.
    pub status: String,
    /// Attempt count.
    pub attempt_count: i64,
    /// Assigned batch id.
    pub batch_id: Option<String>,
    /// Last error, if any.
    pub last_error: Option<String>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_finding_batch`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonFindingBatchRow {
    /// Batch id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Wave index.
    pub wave_index: i64,
    /// Dispatch lane.
    pub lane: String,
    /// Assigned worker id.
    pub worker_id: Option<String>,
    /// Batch status.
    pub status: String,
    /// Start timestamp.
    pub started_at: Option<i64>,
    /// End timestamp.
    pub ended_at: Option<i64>,
    /// Batch result JSON.
    pub result_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_finding_edge`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonFindingEdgeRow {
    /// Owning daemon run.
    pub run_id: String,
    /// Parent finding id.
    pub parent_id: String,
    /// Child finding id.
    pub child_id: String,
    /// Edge kind.
    pub kind: String,
    /// Creation timestamp.
    pub time_created: i64,
}

/// Row in `daemon_concept`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonConceptRow {
    /// Row id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Stable concept id.
    pub concept_id: String,
    /// Human-readable definition.
    pub definition: String,
    /// Source concept or artifact refs.
    pub derived_from_json: Option<serde_json::Value>,
    /// Proof references.
    pub proof_refs_json: Option<serde_json::Value>,
    /// Confidence score.
    pub confidence: f64,
    /// Invalidation timestamp.
    pub invalidated_at: Option<i64>,
    /// Invalidation reason.
    pub invalidated_reason: Option<String>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_concept_link`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonConceptLinkRow {
    /// Owning daemon run.
    pub run_id: String,
    /// Parent concept id.
    pub parent_concept: String,
    /// Child concept id.
    pub child_concept: String,
    /// Link relation.
    pub relation: String,
    /// Creation timestamp.
    pub time_created: i64,
}

/// Row in `daemon_regression_cycle`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonRegressionCycleRow {
    /// Cycle id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Iteration number.
    pub iteration: i64,
    /// Baseline audit score.
    pub baseline_score: Option<f64>,
    /// Current audit score.
    pub current_score: Option<f64>,
    /// Hard finding delta.
    pub hard_delta: i64,
    /// Soft finding delta.
    pub soft_delta: i64,
    /// Cap delta.
    pub caps_delta: i64,
    /// Cycle status.
    pub status: String,
    /// Result payload.
    pub result_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}
