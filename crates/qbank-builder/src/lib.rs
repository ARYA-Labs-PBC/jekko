use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const PAPER_SCHEMA_VERSION: &str = "opencode-paper-v1";
pub const CHALLENGE_SCHEMA_VERSION: &str = "opencode-qbank-challenge-v1";

mod bank;

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
    pub artifact_hash: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkItem {
    pub kind: String,
    pub publication_hash: String,
    pub challenge_hash: Option<String>,
    pub prompt: String,
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
    challenge.schema_version = CHALLENGE_SCHEMA_VERSION.to_string();
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

pub use bank::{
    acceptance_passes, bank_subdir, challenge_sort_key, collect_json_files, ensure_bank_layout,
    manifest_hash, pack_context, read_challenges, read_json, sorted_challenges, token_estimate,
    write_json_pretty,
};

#[cfg(test)]
mod tests;
