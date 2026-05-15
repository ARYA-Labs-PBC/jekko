use super::{AcceptanceMetrics, RouteMetadata, TokenUsage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaperTextSection {
    pub section_id: String,
    pub title: String,
    pub text: String,
    pub section_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalPaperText {
    pub title: String,
    pub abstract_text: String,
    pub full_text: String,
    pub sections: Vec<PaperTextSection>,
    pub source_urls: Vec<String>,
    pub license_spdx: String,
    pub redistributable: bool,
    pub content_hash: String,
    pub non_production: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentCallReceipt {
    pub agent_name: String,
    pub phase: String,
    pub prompt_hash: String,
    pub context_hash: String,
    pub raw_output_hash: String,
    pub route_metadata: Option<RouteMetadata>,
    pub token_usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentFailure {
    #[serde(default = "default_failure_category")]
    pub category: String,
    pub phase: String,
    pub agent_name: String,
    pub error: String,
    #[serde(default)]
    pub fatal_for_acceptance: bool,
    pub route_metadata: Option<RouteMetadata>,
    pub raw_output_hash: Option<String>,
}

fn default_failure_category() -> String {
    "parse_schema".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SupportQuote {
    pub section_id: String,
    pub section_hash: String,
    pub quote: String,
    pub why_it_matters: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneratorAgentOutput {
    pub question: String,
    pub answer: String,
    pub difficulty_rationale: String,
    pub expected_failure_mode: String,
    #[serde(default)]
    pub required_key_points: Vec<String>,
    pub support: Vec<SupportQuote>,
    pub confidence: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationAgentOutput {
    pub accepted: bool,
    pub answer: String,
    pub confidence: u8,
    pub support_correct: bool,
    pub reason: String,
    pub missing_or_wrong_support: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestingAgentOutput {
    pub answer: String,
    pub confidence: u8,
    pub reasoning_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GradingAgentOutput {
    pub correct: bool,
    pub score_0_100: u8,
    pub matched_key_points: Vec<String>,
    pub missed_key_points: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneratorTrial {
    pub agent_name: String,
    pub output: GeneratorAgentOutput,
    pub receipt: AgentCallReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationTrial {
    pub agent_name: String,
    pub output: VerificationAgentOutput,
    pub receipt: AgentCallReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestingTrial {
    pub agent_name: String,
    pub distractor_paper_hashes: Vec<String>,
    pub output: TestingAgentOutput,
    pub receipt: AgentCallReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GradingTrial {
    pub agent_name: String,
    pub testing_agent_name: String,
    pub output: GradingAgentOutput,
    pub receipt: AgentCallReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateAttemptReceipt {
    pub candidate_index: usize,
    pub support_quote_id: String,
    pub question: String,
    pub support_quote_hash: String,
    pub support_quote_score: i32,
    pub stem_leakage_score: f64,
    pub distractor_hashes: Vec<String>,
    pub prescreen_trials: Vec<TestingTrial>,
    pub prescreen_grading_trials: Vec<GradingTrial>,
    pub rejection_reasons: Vec<String>,
    pub rejection_category: Option<String>,
    pub accepted_for_full_tournament: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaperTournamentArtifact {
    pub schema_version: String,
    pub paper_hash: String,
    pub paper_content: CanonicalPaperText,
    pub generation_trials: Vec<GeneratorTrial>,
    pub verification_trials: Vec<VerificationTrial>,
    pub testing_trials: Vec<TestingTrial>,
    pub grading_trials: Vec<GradingTrial>,
    pub failures: Vec<AgentFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FinalPaperChallengeArtifact {
    pub schema_version: String,
    pub paper_hash: String,
    pub paper_content: CanonicalPaperText,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_provenance: Option<super::ArtifactProvenance>,
    pub hard_question: String,
    pub hard_answer: String,
    pub hard_agent_name: String,
    pub generation_trials: Vec<GeneratorTrial>,
    pub verification_trials: Vec<VerificationTrial>,
    pub testing_trials: Vec<TestingTrial>,
    pub grading_trials: Vec<GradingTrial>,
    pub failures: Vec<AgentFailure>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidate_attempts: Vec<CandidateAttemptReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paper_rejection_category: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rejection_reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub production_errors: Vec<String>,
    pub acceptance_metrics: AcceptanceMetrics,
    pub artifact_hash: String,
}
