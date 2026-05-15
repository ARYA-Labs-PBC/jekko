use crate::json::Json;
use crate::qbank_hash::sha256_hex;
use serde::{Deserialize, Serialize};

pub const PRODUCTION_CHALLENGE_SCHEMA_VERSION: &str = "opencode-qbank-challenge-v3";
pub const PRODUCTION_MANIFEST_SCHEMA_VERSION: &str = "opencode-qbank-manifest-v3";

#[derive(Debug, Clone)]
pub struct PaperRecord {
    pub publication_hash: String,
    pub title: String,
    pub license_spdx: String,
    pub redistributable: bool,
    pub dedupe_keys: Vec<String>,
    pub source_ids: Vec<String>,
    pub source_url: Option<String>,
    pub retrieval_receipts: Vec<Json>,
    pub review_receipts: Vec<Json>,
    pub retrieval_kinds: Vec<String>,
    pub sections: Vec<PaperSection>,
}

#[derive(Debug, Clone)]
pub struct PaperSection {
    pub section_id: String,
    pub title: String,
    pub text: String,
    pub section_hash: String,
}

#[derive(Debug, Clone)]
pub struct PaperChallenge {
    pub schema_version: String,
    pub challenge_hash: String,
    pub publication_hash: String,
    pub domain: String,
    pub topics: Vec<String>,
    pub difficulty_score: f32,
    pub answerability: f32,
    pub focused_correct_rate: f32,
    pub blind_correct_rate: f32,
    pub question: String,
    pub answer_key: AnswerKey,
    pub support: Vec<SupportRef>,
    pub context_pack: ContextPack,
    pub source_publication: Option<SourcePublication>,
    pub focused_support_trials: Vec<ModelTrial>,
    pub saturated_blind_trials: Vec<ModelTrial>,
    pub judge_trials: Vec<JudgeTrial>,
    pub context_packs: Vec<ContextPackProvenance>,
    pub route_metadata: Vec<RouteMetadata>,
    pub acceptance_metrics: Option<AcceptanceMetrics>,
    pub artifact_provenance: Option<ArtifactProvenance>,
}

#[derive(Debug, Clone)]
pub struct SourcePublication {
    pub publication_hash: String,
    pub content_hash: String,
    pub license_spdx: String,
    pub redistributable: bool,
    pub source_url: Option<String>,
    pub section_hashes: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AnswerKey {
    pub canonical: String,
    pub must_include: Vec<String>,
    pub must_not_include: Vec<String>,
    pub aliases: Vec<String>,
    pub numeric_tolerances: Vec<NumericTolerance>,
    pub unit_tolerances: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NumericTolerance {
    pub value: f64,
    pub tolerance: f64,
    pub unit: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SupportRef {
    pub section_id: String,
    pub section_hash: String,
}

#[derive(Debug, Clone, Default)]
pub struct ContextPack {
    pub safe_window_tokens: u32,
    pub target_fill_ratio: f32,
    pub output_reserve_tokens: u32,
    pub estimated_tokens: u32,
    pub target_section_ids: Vec<String>,
    pub distractor_section_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ContextPackProvenance {
    pub kind: String,
    pub context_hash: String,
    pub prompt_hash: String,
    pub section_ids: Vec<String>,
    pub estimated_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct ModelTrial {
    pub agent_id: String,
    pub phase: String,
    pub correct: bool,
    pub answerability: f32,
    pub supported: bool,
    pub confidence: f32,
    pub prompt_hash: String,
    pub context_hash: String,
    pub route_metadata: RouteMetadata,
    pub token_usage: TokenUsage,
}

#[derive(Debug, Clone)]
pub struct JudgeTrial {
    pub agent_id: String,
    pub accepted: bool,
    pub confidence: f32,
    pub rationale_hash: String,
    pub route_metadata: RouteMetadata,
    pub token_usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMetadata {
    pub request_id: String,
    pub provider: String,
    pub model: String,
    pub route_mode: Option<String>,
    pub route_confidence: Option<f32>,
    pub primary_model_id: Option<String>,
    pub backup_model_ids: Vec<String>,
    pub fusion_model_id: Option<String>,
    pub winner_model_id: Option<String>,
    pub prompt_hash: Option<String>,
    pub context_hash: Option<String>,
    pub receipts_hash: Option<String>,
    pub token_usage: Option<TokenUsage>,
    pub model_decisions_hash: Option<String>,
    pub model_decisions: Vec<ModelDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDecision {
    pub model_id: String,
    pub configured_score: f32,
    pub selection_score: f32,
    pub latency_ms: u64,
    pub status: String,
    pub output_hash: Option<String>,
    pub selected: bool,
    pub token_usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct AcceptanceMetrics {
    pub focused_agreement: f32,
    pub focused_correct_rate: f32,
    pub answerability: f32,
    pub saturated_blind_correct_rate: f32,
    pub saturated_mean_confidence: f32,
}

#[derive(Debug, Clone)]
pub struct ArtifactProvenance {
    pub run_id: String,
    pub reducer_version: String,
    pub agent_mode: Option<String>,
    pub fixture_provenance: bool,
    pub answer_leakage_detected: bool,
    pub license_ambiguous: bool,
}

#[derive(Debug, Clone)]
pub struct LoadedChallenge {
    pub challenge: PaperChallenge,
    pub paper: Option<PaperRecord>,
}

#[derive(Debug, Clone, Default)]
pub struct BankValidation {
    pub accepted_challenges: usize,
    pub rejected_challenges: usize,
    pub duplicate_publications: usize,
    pub top_selected: usize,
    pub unique_publications: usize,
    pub distinct_domains: usize,
    pub max_publication_share: f32,
    pub max_domain_share: f32,
    pub source_diversity: f32,
    pub min_required_accepted: usize,
    pub manifest_hash: String,
    pub manifest_schema: String,
    pub strict_production: bool,
    pub qbank_trusted: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn stable_section_hash(text: &str) -> String {
    sha256_hex(normalize_text(text).as_bytes())
}

pub fn stable_challenge_hash(
    publication_hash: &str,
    question: &str,
    answer: &str,
    support_section_hashes: &[String],
) -> String {
    let mut sorted = support_section_hashes.to_vec();
    sorted.sort();
    let mut material = String::from("opencode-qbank-challenge-v1\0");
    material.push_str(publication_hash);
    material.push('\0');
    material.push_str(&normalize_text(question));
    material.push('\0');
    material.push_str(&normalize_text(answer));
    material.push('\0');
    material.push_str(&sorted.join("\0"));
    sha256_hex(material.as_bytes())
}

pub(crate) fn normalize_text(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
