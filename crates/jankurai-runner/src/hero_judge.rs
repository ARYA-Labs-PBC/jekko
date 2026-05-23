//! Typed Hero/Judge prompt-evolution contract for ZYAL runbooks.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Parsed top-level ZYAL runbook subset used by the Hero/Judge runner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroJudgeRunbook {
    /// Optional YAML id. The envelope id is accepted by the parser too.
    #[serde(default)]
    pub id: Option<String>,
    /// Optional job metadata.
    #[serde(default)]
    pub job: Option<HeroJudgeJob>,
    /// Hero/Judge runtime config.
    pub hero_judge: HeroJudgeConfig,
}

/// Minimal job metadata consumed for prompts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroJudgeJob {
    /// Job name.
    pub name: String,
    /// Objective text.
    pub objective: String,
}

/// Runtime configuration for dual hero/judge prompt evolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroJudgeConfig {
    /// Runtime objective override.
    #[serde(default)]
    pub objective: Option<String>,
    /// Generation count requested by the runbook.
    #[serde(default = "default_generations")]
    pub generations: usize,
    /// Lane population counts.
    #[serde(default)]
    pub population: HeroJudgePopulation,
    /// Model/search budgets.
    #[serde(default)]
    pub budgets: HeroJudgeBudgets,
    /// Research behavior.
    #[serde(default)]
    pub research: HeroJudgeResearchConfig,
    /// Local evidence inputs.
    #[serde(default)]
    pub evidence: Vec<HeroJudgeEvidenceInput>,
    /// Promotion gate.
    #[serde(default)]
    pub promotion: HeroJudgePromotionPolicy,
    /// Artifact output root relative to repo root.
    #[serde(default)]
    pub output_root: Option<String>,
}

impl HeroJudgeConfig {
    /// Generation count with a hard defensive cap.
    pub fn effective_generations(&self, override_max: Option<usize>) -> usize {
        let requested = override_max.unwrap_or(self.generations);
        requested.clamp(1, self.generations.max(1)).min(8)
    }

    /// Output root relative to the target repo.
    pub fn output_root(&self) -> PathBuf {
        self.output_root
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target/openqg/hero-judge"))
    }
}

impl Default for HeroJudgeConfig {
    fn default() -> Self {
        Self {
            objective: None,
            generations: default_generations(),
            population: HeroJudgePopulation::default(),
            budgets: HeroJudgeBudgets::default(),
            research: HeroJudgeResearchConfig::default(),
            evidence: Vec::new(),
            promotion: HeroJudgePromotionPolicy::default(),
            output_root: None,
        }
    }
}

/// Lane population counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeroJudgePopulation {
    /// Hero candidate lanes.
    #[serde(default = "default_hero_lanes")]
    pub hero_lanes: usize,
    /// Judge patch lanes.
    #[serde(default = "default_judge_lanes")]
    pub judge_lanes: usize,
    /// Verifier lanes.
    #[serde(default = "default_verifier_lanes")]
    pub verifier_lanes: usize,
    /// Literature synthesis lanes.
    #[serde(default = "default_literature_lanes")]
    pub literature_lanes: usize,
    /// Red-team lanes.
    #[serde(default = "default_red_team_lanes")]
    pub red_team_lanes: usize,
    /// Maximum concurrent model/search lanes.
    #[serde(default = "default_max_parallel")]
    pub max_parallel: usize,
}

impl Default for HeroJudgePopulation {
    fn default() -> Self {
        Self {
            hero_lanes: default_hero_lanes(),
            judge_lanes: default_judge_lanes(),
            verifier_lanes: default_verifier_lanes(),
            literature_lanes: default_literature_lanes(),
            red_team_lanes: default_red_team_lanes(),
            max_parallel: default_max_parallel(),
        }
    }
}

/// Runtime budgets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeroJudgeBudgets {
    /// Maximum model calls.
    #[serde(default = "default_model_calls")]
    pub model_calls: usize,
    /// Maximum search queries.
    #[serde(default = "default_search_queries")]
    pub search_queries: usize,
    /// Maximum searched pages/hits.
    #[serde(default = "default_search_pages")]
    pub search_pages: usize,
}

impl Default for HeroJudgeBudgets {
    fn default() -> Self {
        Self {
            model_calls: default_model_calls(),
            search_queries: default_search_queries(),
            search_pages: default_search_pages(),
        }
    }
}

/// Research config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeroJudgeResearchConfig {
    /// Enable research receipts.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Use live agent-search providers when `--live` and `AGENT_SEARCH_LIVE=1`.
    #[serde(default)]
    pub live_when_available: bool,
    /// Missing provider policy.
    #[serde(default)]
    pub missing_provider: HeroJudgeMissingProviderPolicy,
    /// Explicit query list.
    #[serde(default)]
    pub queries: Vec<String>,
}

impl Default for HeroJudgeResearchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            live_when_available: false,
            missing_provider: HeroJudgeMissingProviderPolicy::SkipWithReceipt,
            queries: Vec::new(),
        }
    }
}

/// Missing search provider policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeroJudgeMissingProviderPolicy {
    /// Write a skipped receipt and continue.
    #[default]
    SkipWithReceipt,
    /// Fail the run.
    Fail,
}

/// Local evidence input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeroJudgeEvidenceInput {
    /// Stable evidence id.
    pub id: String,
    /// Evidence role.
    pub role: String,
    /// Relative or absolute path. Simple `*` globs are supported.
    pub path: String,
    /// Maximum bytes.
    #[serde(default = "default_evidence_max_bytes")]
    pub max_bytes: usize,
}

/// Promotion policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroJudgePromotionPolicy {
    /// Minimum deterministic host score.
    #[serde(default = "default_promotion_min_score")]
    pub min_score: f64,
    /// Replay canaries before promotion.
    #[serde(default = "default_true")]
    pub canary_replay: bool,
    /// Reject leaked fixture constants and hidden canaries.
    #[serde(default = "default_true")]
    pub anti_leak: bool,
}

impl Default for HeroJudgePromotionPolicy {
    fn default() -> Self {
        Self {
            min_score: default_promotion_min_score(),
            canary_replay: true,
            anti_leak: true,
        }
    }
}

/// Prompt lineage row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptVariant {
    /// Prompt id.
    pub id: String,
    /// `hero` or `judge`.
    pub role: String,
    /// Generation.
    pub generation: usize,
    /// Optional parent prompt id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// Storage-safe summary, never raw chain-of-thought.
    pub summary: String,
    /// Prompt hash.
    pub prompt_sha256: String,
    /// Deterministic host score.
    pub score: f64,
    /// Variant status.
    pub status: String,
}

/// One lane artifact row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroJudgeLaneArtifact {
    /// Artifact id.
    pub id: String,
    /// Generation.
    pub generation: usize,
    /// Lane kind.
    pub kind: String,
    /// Lane index.
    pub lane: usize,
    /// Model receipt id.
    pub model_receipt_id: String,
    /// Storage-safe summary.
    pub summary: String,
    /// Content hash.
    pub content_sha256: String,
    /// Deterministic score.
    pub score: f64,
    /// Plot-ready host-side lane metrics.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metrics: BTreeMap<String, f64>,
    /// Status.
    pub status: String,
}

/// Plot-ready lane row with fixed columns for CSV/JSONL aggregation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroJudgeLaneMetric {
    /// Run id.
    pub run_id: String,
    /// Generation.
    pub generation: usize,
    /// Coarse role bucket: `hero`, `judge`, `research`, or `knowledge`.
    pub role_group: String,
    /// Lane kind.
    pub kind: String,
    /// Artifact id.
    pub artifact_id: String,
    /// Lane index.
    pub lane: usize,
    /// Host score.
    pub score: f64,
    /// Claim quality.
    pub claim_quality: f64,
    /// Research/falsification-question quality.
    pub question_quality: f64,
    /// Rubric/judgment quality.
    pub rubric_quality: f64,
    /// Evidence grounding.
    pub evidence_grounding: f64,
    /// Structure/schema completeness.
    pub structural_completeness: f64,
    /// Storage safety.
    pub storage_safety: f64,
    /// Count-like host metrics.
    pub claim_count: f64,
    pub question_count: f64,
    pub rubric_item_count: f64,
    /// Receipt and artifact references.
    pub model_receipt_id: String,
    pub content_sha256: String,
    pub status: String,
}

/// Storage-safe card intended for independent reviewer packets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroJudgeReviewCard {
    /// Artifact id.
    pub artifact_id: String,
    /// Coarse role bucket.
    pub role_group: String,
    /// Lane kind.
    pub kind: String,
    /// Generation.
    pub generation: usize,
    /// Lane index.
    pub lane: usize,
    /// Host score.
    pub score: f64,
    /// Storage-safe summary only.
    pub summary: String,
    /// Content hash for audit.
    pub content_sha256: String,
    /// Plot-ready lane metrics.
    pub metrics: BTreeMap<String, f64>,
}

/// Reviewer packet for checking progress without raw chain-of-thought.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroJudgeReviewerPacket {
    /// Run id.
    pub run_id: String,
    /// Objective text used by the run.
    pub objective: String,
    /// Human-review guidance.
    pub reviewer_questions: Vec<String>,
    /// Per-generation quality metrics.
    pub quality_metrics: Vec<HeroJudgeQualityMetric>,
    /// Last promotion decision.
    pub promotion_decision: PromotionDecision,
    /// Storage-safe cards separated by role group.
    pub cards: Vec<HeroJudgeReviewCard>,
}

/// Per-generation plot-ready quality metrics for Hero/Judge evolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroJudgeQualityMetric {
    /// Run id.
    pub run_id: String,
    /// Generation.
    pub generation: usize,
    /// Host-scored quality of the proposed universal-physics ideas.
    pub theory_quality_index: f64,
    /// Host-scored quality of generated research/falsification questions.
    pub question_quality_index: f64,
    /// Host-scored quality of judge/rubric patches.
    pub rubric_quality_index: f64,
    /// Agreement between judge scores and verifier scores.
    pub judge_calibration_index: f64,
    /// Evidence/search grounding strength.
    pub evidence_grounding_index: f64,
    /// Mean verifier confidence.
    pub verifier_confidence: f64,
    /// Resistance to red-team pressure.
    pub red_team_resilience: f64,
    /// Final deterministic promotion score.
    pub promotion_score: f64,
    /// Weighted quality score intended for trend plots.
    pub overall_quality_index: f64,
    /// Change in weighted score from the prior generation.
    pub delta_overall_quality: f64,
    /// Best retained quality score through this generation.
    pub frontier_quality_index: f64,
    /// Change in retained frontier quality from the prior generation.
    pub delta_frontier_quality: f64,
    /// Whether this generation promoted a candidate.
    pub promoted: bool,
    /// Candidate counts used for the metric.
    pub hero_candidate_count: usize,
    pub judge_patch_count: usize,
    pub research_receipt_count: usize,
    pub knowledge_entry_count: usize,
}

/// Summary of quality change over a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroJudgeQualityTrend {
    /// Run id.
    pub run_id: String,
    /// Completed generations.
    pub generations: usize,
    /// First overall quality score.
    pub start_overall_quality: f64,
    /// Latest overall quality score.
    pub latest_overall_quality: f64,
    /// Latest minus first score.
    pub delta_overall_quality: f64,
    /// First retained frontier quality score.
    pub start_frontier_quality: f64,
    /// Latest retained frontier quality score.
    pub latest_frontier_quality: f64,
    /// Latest minus first retained frontier score.
    pub delta_frontier_quality: f64,
    /// Best generation by overall score.
    pub best_generation: usize,
    /// Best overall score.
    pub best_overall_quality: f64,
    /// Whether the latest score improved over the first score.
    pub improved: bool,
    /// Column names useful for plotting.
    pub metric_keys: Vec<String>,
}

/// One plot-ready row for a multi-run Hero/Judge series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroJudgeSeriesRow {
    /// Parent series id.
    pub series_id: String,
    /// 1-based trial index within the series.
    pub trial_index: usize,
    /// Child run id.
    pub run_id: String,
    /// Final generation represented by this row.
    pub generation: usize,
    /// Final quality metrics from the run.
    pub theory_quality_index: f64,
    pub question_quality_index: f64,
    pub rubric_quality_index: f64,
    pub judge_calibration_index: f64,
    pub evidence_grounding_index: f64,
    pub verifier_confidence: f64,
    pub red_team_resilience: f64,
    pub promotion_score: f64,
    pub overall_quality_index: f64,
    pub delta_overall_quality: f64,
    pub frontier_quality_index: f64,
    pub delta_frontier_quality: f64,
    /// Promotion result.
    pub promoted: bool,
    /// Winning candidate id when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontier_winner: Option<String>,
    /// Model/search accounting.
    pub model_calls_used: usize,
    pub model_call_budget: usize,
    pub search_receipt_count: usize,
    /// Final-generation lane means.
    pub hero_lane_mean: f64,
    pub judge_lane_mean: f64,
    /// Hashes for reviewer-traceable artifacts.
    pub quality_metrics_sha256: String,
    pub lane_metrics_sha256: String,
    pub reviewer_packet_sha256: String,
    pub promotion_decision_sha256: String,
    pub search_receipts_sha256: String,
}

/// Scoreboard entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrontierScore {
    /// Candidate id.
    pub candidate_id: String,
    /// Prompt id.
    pub prompt_id: String,
    /// Generation.
    pub generation: usize,
    /// Score.
    pub score: f64,
    /// Verifier score.
    pub verifier_score: f64,
    /// Red-team penalty.
    pub red_team_penalty: f64,
    /// Leak/canary status.
    pub leak_status: String,
    /// Promotion status.
    pub status: String,
}

/// Promotion decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionDecision {
    /// Run id.
    pub run_id: String,
    /// Generation.
    pub generation: usize,
    /// Winner candidate id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winner_candidate_id: Option<String>,
    /// Winner prompt id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winner_prompt_id: Option<String>,
    /// Winner score.
    pub score: f64,
    /// Whether the variant was promoted.
    pub promoted: bool,
    /// Decision reason.
    pub reason: String,
}

/// Knowledge-compound ledger row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    /// Entry id.
    pub id: String,
    /// Generation.
    pub generation: usize,
    /// `verified` or `rejected`.
    pub status: String,
    /// Claim summary.
    pub claim: String,
    /// Evidence references.
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    /// Source candidate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_candidate_id: Option<String>,
    /// Content hash.
    pub content_sha256: String,
}

/// Storage-safe deterministic search receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeroJudgeSearchReceipt {
    /// Receipt id.
    pub id: String,
    /// Provider id.
    pub provider: String,
    /// Query.
    pub query: String,
    /// Status.
    pub status: String,
    /// Optional reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Number of URLs/hits.
    pub url_count: usize,
    /// Content hash.
    pub content_sha256: String,
}

/// Runner summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroJudgeRunSummary {
    /// Run id.
    pub run_id: String,
    /// Output directory.
    pub output_dir: PathBuf,
    /// Generations completed.
    pub generation: usize,
    /// Hero lane count.
    pub hero_lane_count: usize,
    /// Judge lane count.
    pub judge_lane_count: usize,
    /// Frontier winner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontier_winner: Option<String>,
    /// Knowledge entries.
    pub knowledge_entry_count: usize,
    /// Search receipts.
    pub search_receipt_count: usize,
    /// Last promotion decision.
    pub last_promotion_decision: PromotionDecision,
    /// Model calls used.
    pub model_calls_used: usize,
    /// Model call budget.
    pub model_call_budget: usize,
    /// Last model task kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_model_kind: Option<String>,
    /// Artifact paths.
    pub prompt_lineage_json: PathBuf,
    pub frontier_scoreboard_json: PathBuf,
    pub promotion_decision_json: PathBuf,
    pub knowledge_compound_jsonl: PathBuf,
    pub search_receipts_json: PathBuf,
    pub quality_metrics_jsonl: PathBuf,
    pub quality_metrics_csv: PathBuf,
    pub quality_trend_json: PathBuf,
    pub lane_metrics_jsonl: PathBuf,
    pub lane_metrics_csv: PathBuf,
    pub hero_metrics_csv: PathBuf,
    pub judge_metrics_csv: PathBuf,
    pub reviewer_packet_json: PathBuf,
    pub complete_ok: PathBuf,
}

/// Summary for a multi-run Hero/Judge series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroJudgeSeriesSummary {
    /// Series id.
    pub series_id: String,
    /// Output directory for aggregate artifacts.
    pub output_dir: PathBuf,
    /// Number of completed runs.
    pub run_count: usize,
    /// Child run summaries.
    pub runs: Vec<HeroJudgeRunSummary>,
    /// Aggregate artifacts.
    pub run_summaries_jsonl: PathBuf,
    pub quality_metrics_jsonl: PathBuf,
    pub quality_metrics_csv: PathBuf,
    pub lane_metrics_jsonl: PathBuf,
    pub lane_metrics_csv: PathBuf,
    pub hero_metrics_csv: PathBuf,
    pub judge_metrics_csv: PathBuf,
    pub series_summary_csv: PathBuf,
    pub reviewer_index_json: PathBuf,
    pub complete_ok: PathBuf,
}

fn default_true() -> bool {
    true
}

fn default_generations() -> usize {
    2
}

fn default_hero_lanes() -> usize {
    6
}

fn default_judge_lanes() -> usize {
    4
}

fn default_verifier_lanes() -> usize {
    2
}

fn default_literature_lanes() -> usize {
    2
}

fn default_red_team_lanes() -> usize {
    2
}

fn default_max_parallel() -> usize {
    8
}

fn default_model_calls() -> usize {
    48
}

fn default_search_queries() -> usize {
    12
}

fn default_search_pages() -> usize {
    24
}

fn default_evidence_max_bytes() -> usize {
    64 * 1024
}

fn default_promotion_min_score() -> f64 {
    0.75
}
