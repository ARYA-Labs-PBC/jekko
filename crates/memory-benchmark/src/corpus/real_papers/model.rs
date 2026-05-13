use crate::qbank_hash::sha256_hex;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct PaperRecord {
    pub publication_hash: String,
    pub title: String,
    pub license_spdx: String,
    pub redistributable: bool,
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
    pub manifest_hash: String,
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

pub(crate) fn wanted_section_ids(challenge: &PaperChallenge) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for support in &challenge.support {
        ids.insert(support.section_id.clone());
    }
    for id in &challenge.context_pack.target_section_ids {
        ids.insert(id.clone());
    }
    for id in &challenge.context_pack.distractor_section_ids {
        ids.insert(id.clone());
    }
    ids
}

pub(crate) fn normalize_text(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
