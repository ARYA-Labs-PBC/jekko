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
    pub phase: String,
    pub agent_name: String,
    pub error: String,
    pub route_metadata: Option<RouteMetadata>,
    pub raw_output_hash: Option<String>,
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
    pub acceptance_metrics: AcceptanceMetrics,
    pub artifact_hash: String,
}
