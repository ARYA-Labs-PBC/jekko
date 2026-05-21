#![allow(
    dead_code,
    unused_imports,
    clippy::cloned_ref_to_slice_refs,
    clippy::if_same_then_else,
    clippy::manual_div_ceil,
    clippy::manual_inspect,
    clippy::manual_range_contains,
    clippy::manual_unwrap_or,
    clippy::manual_unwrap_or_default,
    clippy::redundant_closure,
    clippy::redundant_comparisons,
    clippy::result_large_err,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod agent_json;
mod bank;
#[path = "lib_cogcore.rs"]
mod cogcore_support;
mod core_types;
mod fixture;
mod full_text;
mod full_text_import;
mod paper_tournament;
mod schema;
mod trial_schema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaperRecord {
    pub schema_version: String,
    pub publication_hash: String,
    pub content_hash: String,
    pub dedupe_keys: Vec<String>,
    pub source_ids: Vec<String>,
    pub license: LicenseRecord,
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub sections: Vec<PaperSection>,
    pub retrieval_receipts: Vec<serde_json::Value>,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LicenseRecord {
    pub spdx: String,
    pub redistributable: bool,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaperSection {
    pub section_id: String,
    pub title: String,
    pub text: String,
    pub section_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChallengeRecord {
    pub schema_version: String,
    pub challenge_hash: String,
    pub publication_hash: String,
    pub domain: String,
    pub topics: Vec<String>,
    pub difficulty_score: f64,
    pub difficulty_components: BTreeMap<String, f64>,
    pub question: String,
    pub answer_key: AnswerKey,
    pub support: Vec<SupportRef>,
    pub context_pack: ContextPack,
    pub generator_agents: Vec<serde_json::Value>,
    pub blind_answer_attempts: Vec<AnswerAttempt>,
    pub focused_answer_attempts: Vec<AnswerAttempt>,
    pub critic_attempts: Vec<serde_json::Value>,
    pub audit_attempts: Vec<serde_json::Value>,
    pub acceptance: AcceptanceRecord,
    #[serde(default)]
    pub source_publication: Option<SourcePublication>,
    #[serde(default)]
    pub focused_support_trials: Vec<ModelTrial>,
    #[serde(default)]
    pub saturated_blind_trials: Vec<ModelTrial>,
    #[serde(default)]
    pub judge_trials: Vec<JudgeTrial>,
    #[serde(default)]
    pub context_packs: Vec<ContextPackProvenance>,
    #[serde(default)]
    pub route_metadata: Vec<RouteMetadata>,
    #[serde(default)]
    pub acceptance_metrics: Option<AcceptanceMetrics>,
    #[serde(default)]
    pub artifact_provenance: Option<ArtifactProvenance>,
    #[serde(default)]
    pub artifact_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourcePublication {
    pub publication_hash: String,
    pub content_hash: String,
    pub license_spdx: String,
    pub redistributable: bool,
    pub source_url: Option<String>,
    #[serde(default)]
    pub section_hashes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnswerKey {
    pub canonical: String,
    pub must_include: Vec<String>,
    pub must_not_include: Vec<String>,
    pub aliases: Vec<String>,
    pub numeric_tolerances: Vec<NumericTolerance>,
    pub unit_tolerances: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NumericTolerance {
    pub value: f64,
    pub tolerance: f64,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SupportRef {
    pub section_id: String,
    pub section_hash: String,
    pub quote_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextPack {
    pub safe_window_tokens: u64,
    pub target_fill_ratio: f64,
    pub output_reserve_tokens: u64,
    pub estimated_tokens: u64,
    pub target_section_ids: Vec<String>,
    pub distractor_section_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextPackProvenance {
    pub kind: String,
    pub context_hash: String,
    pub prompt_hash: String,
    pub section_ids: Vec<String>,
    pub estimated_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelTrial {
    pub agent_id: String,
    pub phase: String,
    pub correct: bool,
    pub answerability: f64,
    pub supported: bool,
    pub confidence: f64,
    pub prompt_hash: String,
    pub context_hash: String,
    pub route_metadata: RouteMetadata,
    pub token_usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JudgeTrial {
    pub agent_id: String,
    pub accepted: bool,
    pub confidence: f64,
    pub rationale_hash: String,
    pub route_metadata: RouteMetadata,
    pub token_usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteMetadata {
    pub request_id: String,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub route_mode: Option<String>,
    #[serde(default)]
    pub route_confidence: Option<f64>,
    #[serde(default)]
    pub primary_model_id: Option<String>,
    #[serde(default)]
    pub backup_model_ids: Vec<String>,
    #[serde(default)]
    pub fusion_model_id: Option<String>,
    #[serde(default)]
    pub winner_model_id: Option<String>,
    #[serde(default)]
    pub prompt_hash: Option<String>,
    #[serde(default)]
    pub context_hash: Option<String>,
    #[serde(default)]
    pub receipts_hash: Option<String>,
    #[serde(default)]
    pub token_usage: Option<TokenUsage>,
    #[serde(default)]
    pub model_decisions_hash: Option<String>,
    #[serde(default)]
    pub model_decisions: Vec<ModelDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelDecision {
    pub model_id: String,
    pub configured_score: f64,
    pub selection_score: f64,
    pub latency_ms: u64,
    pub status: String,
    #[serde(default)]
    pub output_hash: Option<String>,
    pub selected: bool,
    pub token_usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AcceptanceMetrics {
    pub focused_agreement: f64,
    pub focused_correct_rate: f64,
    pub answerability: f64,
    pub saturated_blind_correct_rate: f64,
    pub saturated_mean_confidence: f64,
    pub support_minimality: f64,
    pub distractor_pressure: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArtifactProvenance {
    pub run_id: String,
    pub reducer_version: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_mode: Option<String>,
    pub fixture_provenance: bool,
    pub answer_leakage_detected: bool,
    pub license_ambiguous: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnswerAttempt {
    pub agent_id: String,
    pub correct: bool,
    pub answerability: f64,
    pub supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AcceptanceRecord {
    pub accepted: bool,
    pub auditor_agreement: f64,
    pub answerability: f64,
    pub blind_correct_rate: f64,
    pub focused_correct_rate: f64,
    pub ambiguity_flag: bool,
    pub hash_mismatch: bool,
    pub redistributable: bool,
    pub reason: Option<String>,
}

impl ChallengeRecord {
    pub fn has_production_evidence(&self) -> bool {
        self.source_publication.is_some()
            || !self.focused_support_trials.is_empty()
            || !self.saturated_blind_trials.is_empty()
            || !self.judge_trials.is_empty()
            || self.acceptance_metrics.is_some()
            || self.artifact_provenance.is_some()
    }
}

pub use core_types::{
    canonicalize_paper, challenge_hash, content_hash, finalize_challenge,
    license_is_redistributable, normalize_text, publication_hash, section_hash, sha256_hex,
    CogcoreEventRecord, CogcoreSourceRef, WorkItem,
};
pub use schema::*;

pub use agent_json::{extract_agent_json, parse_agent_json};
pub use bank::{
    acceptance_passes, bank_subdir, challenge_sort_key, collect_json_files, ensure_bank_layout,
    manifest_hash, pack_context, production_acceptance_errors, production_acceptance_passes,
    production_bank_errors, read_challenges, read_json, read_papers, sorted_challenges,
    token_estimate, write_json_pretty,
};
pub use cogcore_support::cogcore_events_for_papers;
pub use fixture::{seed_fixture_bank, SeedFixtureSummary};
pub use full_text::{canonical_paper_text, validate_full_text_paper};
pub use full_text_import::{
    discover_full_text, parse_europe_pmc_full_text_xml, FullTextDiscoveryConfig,
    FullTextDiscoverySummary,
};
pub use paper_tournament::{
    build_paper_tournament, build_testing_prompt, final_paper_challenge_artifact_hash,
    grade_reduction, verification_majority, AgentRunnerMode, BuildPaperTournamentConfig,
    BuildPaperTournamentSummary, RouteModelPolicy,
};
pub use trial_schema::{
    AgentCallReceipt, AgentFailure, CandidateAttemptReceipt, CanonicalPaperText,
    FinalPaperChallengeArtifact, GeneratorAgentOutput, GeneratorTrial, GradingAgentOutput,
    GradingTrial, PaperTextSection, PaperTournamentArtifact, SupportQuote, TestingAgentOutput,
    TestingTrial, VerificationAgentOutput, VerificationTrial,
};

#[cfg(test)]
mod tests;
