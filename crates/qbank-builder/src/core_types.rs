use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cogcore_support::{fill_acceptance_metrics, fill_difficulty_score};

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

pub fn content_hash(sections: &[crate::PaperSection]) -> String {
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
    sections: &[crate::PaperSection],
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

pub fn license_is_redistributable(license: &crate::LicenseRecord) -> bool {
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

pub fn canonicalize_paper(mut paper: crate::PaperRecord) -> Result<crate::PaperRecord, String> {
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
    paper.schema_version = crate::PAPER_SCHEMA_VERSION.to_string();
    paper.content_hash = content_hash(&paper.sections);
    paper.publication_hash = publication_hash(&canonical_source_id, &paper.title, &paper.sections);
    paper.dedupe_keys.sort();
    paper.dedupe_keys.dedup();
    paper.source_ids.sort();
    paper.source_ids.dedup();
    Ok(paper)
}

pub fn finalize_challenge(mut challenge: crate::ChallengeRecord) -> crate::ChallengeRecord {
    let support_hashes = challenge
        .support
        .iter()
        .map(|support| support.section_hash.clone())
        .collect::<Vec<_>>();
    let production_schema = challenge.schema_version == crate::PRODUCTION_CHALLENGE_SCHEMA_VERSION
        || challenge.has_production_evidence();
    challenge.schema_version = if production_schema {
        crate::PRODUCTION_CHALLENGE_SCHEMA_VERSION.to_string()
    } else {
        crate::CHALLENGE_SCHEMA_VERSION.to_string()
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
