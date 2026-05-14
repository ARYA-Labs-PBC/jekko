use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const PAPER_SCHEMA_VERSION: &str = "opencode-paper-v1";
pub const CHALLENGE_SCHEMA_VERSION: &str = "opencode-qbank-challenge-v1";
pub const PRODUCTION_CHALLENGE_SCHEMA_VERSION: &str = "opencode-qbank-challenge-v3";
pub const PRODUCTION_MANIFEST_SCHEMA_VERSION: &str = "opencode-qbank-manifest-v3";
pub const PAPER_TOURNAMENT_SCHEMA_VERSION: &str = "opencode-paper-tournament-v1";
pub const FINAL_PAPER_CHALLENGE_SCHEMA_VERSION: &str = "opencode-paper-challenge-final-v1";
pub const QBANK_REDUCER_VERSION: &str = "qbank-paper-tournament-rust-v1";
pub const MIN_SUCCESSFUL_VERIFIERS: usize = 3;
pub const MIN_SUCCESSFUL_TESTERS: usize = 3;
pub const MIN_SUCCESSFUL_GRADERS: usize = 2;
pub const HARD_MAX_TESTER_CORRECT_RATE: f64 = 0.50;
mod agent_json;
mod bank;
mod fixture;
mod full_text;
mod full_text_import;
mod paper_tournament;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkItem {
    pub kind: String,
    pub publication_hash: String,
    pub challenge_hash: Option<String>,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CogcoreEventRecord {
    pub id: String,
    pub kind: String,
    pub subject: String,
    pub body: String,
    pub tx_time: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub privacy_class: String,
    pub claim_modality: Option<String>,
    pub tags: Vec<String>,
    pub sources: Vec<CogcoreSourceRef>,
    pub supersedes: Vec<String>,
    pub contradicts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CogcoreSourceRef {
    pub uri: String,
    pub citation: String,
    pub quality: f32,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn normalize_text(input: &str) -> String {
    let whitespace = Regex::new(r"\s+").expect("valid whitespace regex");
    whitespace
        .replace_all(&input.trim().to_lowercase(), " ")
        .to_string()
}

pub fn section_hash(text: &str) -> String {
    sha256_hex(normalize_text(text).as_bytes())
}

pub fn content_hash(sections: &[PaperSection]) -> String {
    let mut text = String::new();
    for section in sections {
        text.push_str(&normalize_text(&section.text));
        text.push('\n');
    }
    sha256_hex(text.as_bytes())
}

pub fn publication_hash(
    canonical_source_id: &str,
    title: &str,
    sections: &[PaperSection],
) -> String {
    let mut material = String::from("opencode-paper-v1\0");
    material.push_str(&normalize_text(canonical_source_id));
    material.push('\0');
    material.push_str(&normalize_text(title));
    material.push('\0');
    for section in sections {
        material.push_str(&normalize_text(&section.text));
        material.push('\n');
    }
    sha256_hex(material.as_bytes())
}

pub fn challenge_hash(
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

pub fn license_is_redistributable(license: &LicenseRecord) -> bool {
    if !license.redistributable {
        return false;
    }
    matches!(
        license.spdx.to_ascii_uppercase().as_str(),
        "CC-BY-4.0"
            | "CC-BY-3.0"
            | "CC-BY-SA-4.0"
            | "CC0-1.0"
            | "PDDL-1.0"
            | "PUBLIC-DOMAIN"
            | "MIT"
            | "BSD-2-CLAUSE"
            | "BSD-3-CLAUSE"
            | "APACHE-2.0"
    )
}

pub fn canonicalize_paper(mut paper: PaperRecord) -> Result<PaperRecord, String> {
    if !license_is_redistributable(&paper.license) {
        return Err(format!(
            "license {} is not redistributable",
            paper.license.spdx
        ));
    }
    if paper.sections.is_empty() {
        return Err("paper must contain at least one section".to_string());
    }
    for section in &mut paper.sections {
        section.section_hash = section_hash(&section.text);
    }
    let canonical_source_id = match paper.source_ids.first().cloned() {
        Some(source_id) => source_id,
        None => match paper.dedupe_keys.first().cloned() {
            Some(dedupe_key) => dedupe_key,
            None => paper.title.clone(),
        },
    };
    paper.schema_version = PAPER_SCHEMA_VERSION.to_string();
    paper.content_hash = content_hash(&paper.sections);
    paper.publication_hash = publication_hash(&canonical_source_id, &paper.title, &paper.sections);
    paper.dedupe_keys.sort();
    paper.dedupe_keys.dedup();
    paper.source_ids.sort();
    paper.source_ids.dedup();
    Ok(paper)
}

pub fn finalize_challenge(mut challenge: ChallengeRecord) -> ChallengeRecord {
    let support_hashes = challenge
        .support
        .iter()
        .map(|support| support.section_hash.clone())
        .collect::<Vec<_>>();
    let production_schema = challenge.schema_version == PRODUCTION_CHALLENGE_SCHEMA_VERSION
        || challenge.has_production_evidence();
    challenge.schema_version = if production_schema {
        PRODUCTION_CHALLENGE_SCHEMA_VERSION.to_string()
    } else {
        CHALLENGE_SCHEMA_VERSION.to_string()
    };
    if production_schema {
        fill_acceptance_metrics(&mut challenge);
        fill_difficulty_score(&mut challenge);
    }
    challenge.challenge_hash = challenge_hash(
        &challenge.publication_hash,
        &challenge.question,
        &challenge.answer_key.canonical,
        &support_hashes,
    );
    challenge.artifact_hash = None;
    let json = match serde_json::to_vec(&challenge) {
        Ok(json) => json,
        Err(err) => panic!("failed to serialize finalized challenge: {err}"),
    };
    challenge.artifact_hash = Some(sha256_hex(&json));
    challenge
}

fn fill_acceptance_metrics(challenge: &mut ChallengeRecord) {
    if challenge.acceptance_metrics.is_some() {
        return;
    }
    let saturated_mean_confidence = if challenge.saturated_blind_trials.is_empty() {
        1.0
    } else {
        challenge
            .saturated_blind_trials
            .iter()
            .map(|trial| trial.confidence)
            .sum::<f64>()
            / challenge.saturated_blind_trials.len() as f64
    };
    let support_minimality = if challenge.support.len() <= 2 {
        1.0
    } else {
        0.75
    };
    let distractor_pressure = if challenge.context_pack.distractor_section_ids.is_empty() {
        0.0
    } else {
        (challenge.context_pack.distractor_section_ids.len() as f64
            / (challenge.context_pack.target_section_ids.len()
                + challenge.context_pack.distractor_section_ids.len()) as f64)
            .min(1.0)
    };
    challenge.acceptance_metrics = Some(AcceptanceMetrics {
        focused_agreement: challenge.acceptance.auditor_agreement,
        focused_correct_rate: challenge.acceptance.focused_correct_rate,
        answerability: challenge.acceptance.answerability,
        saturated_blind_correct_rate: challenge.acceptance.blind_correct_rate,
        saturated_mean_confidence,
        support_minimality,
        distractor_pressure,
    });
}

fn fill_difficulty_score(challenge: &mut ChallengeRecord) {
    let Some(metrics) = challenge.acceptance_metrics.as_ref() else {
        return;
    };
    let blind_failure_rate = (1.0 - metrics.saturated_blind_correct_rate).clamp(0.0, 1.0);
    let low_confidence = (1.0 - metrics.saturated_mean_confidence).clamp(0.0, 1.0);
    let focused_agreement = metrics.focused_agreement.clamp(0.0, 1.0);
    let support_minimality = metrics.support_minimality.clamp(0.0, 1.0);
    let distractor_pressure = metrics.distractor_pressure.clamp(0.0, 1.0);
    let score = blind_failure_rate * 0.35
        + low_confidence * 0.20
        + focused_agreement * 0.20
        + support_minimality * 0.15
        + distractor_pressure * 0.10;
    challenge.difficulty_score = score.clamp(0.0, 1.0);
    challenge.difficulty_components = BTreeMap::from([
        ("blind_failure_rate".to_string(), blind_failure_rate),
        ("low_confidence".to_string(), low_confidence),
        ("focused_agreement".to_string(), focused_agreement),
        ("support_minimality".to_string(), support_minimality),
        ("distractor_pressure".to_string(), distractor_pressure),
    ]);
}

pub fn cogcore_events_for_papers(
    papers: &[PaperRecord],
    challenges: &[ChallengeRecord],
) -> Vec<CogcoreEventRecord> {
    let mut sorted_papers = papers.to_vec();
    sorted_papers.sort_by(|left, right| left.publication_hash.cmp(&right.publication_hash));
    let accepted_challenges = challenges
        .iter()
        .filter(|challenge| acceptance_passes(&challenge.acceptance))
        .collect::<Vec<_>>();
    let mut topics_by_section: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for challenge in accepted_challenges {
        for support in &challenge.support {
            let key = (
                challenge.publication_hash.clone(),
                support.section_id.clone(),
            );
            let topics = topics_by_section.entry(key).or_default();
            for topic in &challenge.topics {
                if !topics.contains(topic) {
                    topics.push(topic.clone());
                }
            }
            topics.sort();
        }
    }

    let mut out = Vec::new();
    for paper in &sorted_papers {
        for section in &paper.sections {
            let topics = topics_by_section
                .get(&(paper.publication_hash.clone(), section.section_id.clone()))
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            out.push(cogcore_section_event(paper, section, topics));
        }
    }
    out
}

fn cogcore_section_event(
    paper: &PaperRecord,
    section: &PaperSection,
    topics: &[String],
) -> CogcoreEventRecord {
    let tx_time = paper
        .published_at
        .clone()
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
    let mut tags = vec![
        "qbank".to_string(),
        "paper-section".to_string(),
        format!("publication:{}", paper.publication_hash),
        format!("section:{}", section.section_id),
        format!("section_hash:{}", section.section_hash),
    ];
    tags.extend(topics.iter().map(|topic| format!("topic:{topic}")));
    CogcoreEventRecord {
        id: String::new(),
        kind: "Claim".to_string(),
        subject: paper.title.clone(),
        body: section.text.clone(),
        tx_time: tx_time.clone(),
        valid_from: Some(tx_time),
        valid_to: None,
        privacy_class: "Public".to_string(),
        claim_modality: Some("AssertedBySource".to_string()),
        tags,
        sources: vec![paper_source_ref(paper, section)],
        supersedes: Vec::new(),
        contradicts: Vec::new(),
    }
}

fn paper_source_ref(paper: &PaperRecord, section: &PaperSection) -> CogcoreSourceRef {
    let uri = paper.license.source_url.clone().unwrap_or_else(|| {
        format!(
            "qbank://paper/{}/{}",
            paper.publication_hash, section.section_id
        )
    });
    CogcoreSourceRef {
        uri,
        citation: format!("{} :: {}", paper.title, section.title),
        quality: 0.95,
    }
}

pub use agent_json::{extract_agent_json, parse_agent_json};
pub use bank::{
    acceptance_passes, bank_subdir, challenge_sort_key, collect_json_files, ensure_bank_layout,
    manifest_hash, pack_context, production_acceptance_errors, production_acceptance_passes,
    production_bank_errors, read_challenges, read_json, read_papers, sorted_challenges,
    token_estimate, write_json_pretty,
};
pub use fixture::{seed_fixture_bank, SeedFixtureSummary};
pub use full_text::{canonical_paper_text, validate_full_text_paper};
pub use full_text_import::{
    discover_full_text, parse_europe_pmc_full_text_xml, FullTextDiscoveryConfig,
    FullTextDiscoverySummary,
};
pub use paper_tournament::{
    build_paper_tournament, build_testing_prompt, final_paper_challenge_artifact_hash,
    grade_reduction, verification_majority, AgentRunnerMode, BuildPaperTournamentConfig,
    BuildPaperTournamentSummary,
};
pub use trial_schema::{
    AgentCallReceipt, AgentFailure, CanonicalPaperText, FinalPaperChallengeArtifact,
    GeneratorAgentOutput, GeneratorTrial, GradingAgentOutput, GradingTrial, PaperTextSection,
    PaperTournamentArtifact, SupportQuote, TestingAgentOutput, TestingTrial,
    VerificationAgentOutput, VerificationTrial,
};

#[cfg(test)]
mod tests;
