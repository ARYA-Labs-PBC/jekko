use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const PAPER_SCHEMA_VERSION: &str = "opencode-paper-v1";
pub const CHALLENGE_SCHEMA_VERSION: &str = "opencode-qbank-challenge-v1";

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
    let canonical_source_id = paper
        .source_ids
        .first()
        .cloned()
        .or_else(|| paper.dedupe_keys.first().cloned())
        .unwrap_or_else(|| paper.title.clone());
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
    let json = serde_json::to_vec(&challenge).unwrap_or_default();
    challenge.artifact_hash = Some(sha256_hex(&json));
    challenge
}

pub fn token_estimate(text: &str) -> u64 {
    ((text.chars().count() as u64) + 3) / 4
}

pub fn pack_context(
    paper: &PaperRecord,
    selected_section_ids: &[String],
    safe_window_tokens: u64,
    target_fill_ratio: f64,
    output_reserve_tokens: u64,
) -> Result<ContextPack, String> {
    let budget = ((safe_window_tokens as f64 * target_fill_ratio).floor() as i64
        - output_reserve_tokens as i64)
        .max(0) as u64;
    let selected: BTreeSet<&str> = selected_section_ids.iter().map(String::as_str).collect();
    let mut estimated = 0_u64;
    let mut targets = Vec::new();
    let mut distractors = Vec::new();
    for section in &paper.sections {
        let cost = token_estimate(&section.text);
        if estimated + cost > budget {
            continue;
        }
        estimated += cost;
        if selected.contains(section.section_id.as_str()) {
            targets.push(section.section_id.clone());
        } else {
            distractors.push(section.section_id.clone());
        }
    }
    if targets.is_empty() {
        return Err("context pack does not include any target section".to_string());
    }
    Ok(ContextPack {
        safe_window_tokens,
        target_fill_ratio,
        output_reserve_tokens,
        estimated_tokens: estimated,
        target_section_ids: targets,
        distractor_section_ids: distractors,
    })
}

pub fn acceptance_passes(acceptance: &AcceptanceRecord) -> bool {
    acceptance.accepted
        && acceptance.auditor_agreement >= 0.75
        && acceptance.answerability >= 0.90
        && acceptance.blind_correct_rate <= 0.50
        && acceptance.focused_correct_rate >= 0.90
        && !acceptance.ambiguity_flag
        && !acceptance.hash_mismatch
        && acceptance.redistributable
}

pub fn challenge_sort_key(
    challenge: &ChallengeRecord,
) -> (
    std::cmp::Reverse<i64>,
    std::cmp::Reverse<i64>,
    i64,
    String,
    String,
) {
    (
        std::cmp::Reverse((challenge.difficulty_score * 1_000_000.0).round() as i64),
        std::cmp::Reverse((challenge.acceptance.focused_correct_rate * 1_000_000.0).round() as i64),
        (challenge.acceptance.blind_correct_rate * 1_000_000.0).round() as i64,
        challenge.publication_hash.clone(),
        challenge.challenge_hash.clone(),
    )
}

pub fn sorted_challenges(mut challenges: Vec<ChallengeRecord>) -> Vec<ChallengeRecord> {
    challenges.sort_by_key(challenge_sort_key);
    challenges
}

pub fn bank_subdir(bank: &Path, name: &str) -> PathBuf {
    bank.join(name)
}

pub fn ensure_bank_layout(bank: &Path) -> Result<(), String> {
    for dir in ["papers", "challenges", "rejected", "manifests"] {
        fs::create_dir_all(bank.join(dir)).map_err(|err| format!("create {dir}: {err}"))?;
    }
    Ok(())
}

pub fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    fs::write(path, format!("{json}\n")).map_err(|err| format!("write {}: {err}", path.display()))
}

pub fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("parse {}: {err}", path.display()))
}

pub fn read_challenges(root: &Path) -> Result<Vec<ChallengeRecord>, String> {
    let challenge_root = root.join("challenges");
    let mut paths = Vec::new();
    collect_json_files(&challenge_root, &mut paths)?;
    let mut out = Vec::new();
    for path in paths {
        out.push(read_json(&path)?);
    }
    Ok(out)
}

pub fn collect_json_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    let entries =
        fs::read_dir(root).map_err(|err| format!("read_dir {}: {err}", root.display()))?;
    for entry in entries {
        let path = entry.map_err(|err| err.to_string())?.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

pub fn manifest_hash(paths: &[PathBuf]) -> Result<String, String> {
    let mut material = String::new();
    for path in paths {
        let text =
            fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
        material.push_str(&path.display().to_string());
        material.push('\0');
        material.push_str(&sha256_hex(text.as_bytes()));
        material.push('\n');
    }
    Ok(sha256_hex(material.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_paper() -> PaperRecord {
        PaperRecord {
            schema_version: String::new(),
            publication_hash: String::new(),
            content_hash: String::new(),
            dedupe_keys: vec!["doi:10.1/example".to_string()],
            source_ids: vec!["doi:10.1/example".to_string()],
            license: LicenseRecord {
                spdx: "CC-BY-4.0".to_string(),
                redistributable: true,
                source_url: None,
            },
            title: "Alpha Paper".to_string(),
            authors: vec!["Ada".to_string()],
            abstract_text: "abstract".to_string(),
            sections: vec![
                PaperSection {
                    section_id: "s1".to_string(),
                    title: "Result".to_string(),
                    text: "Alpha equals one in the calibrated fixture.".to_string(),
                    section_hash: String::new(),
                },
                PaperSection {
                    section_id: "s2".to_string(),
                    title: "Distractor".to_string(),
                    text: "Beta equals two.".to_string(),
                    section_hash: String::new(),
                },
            ],
            retrieval_receipts: Vec::new(),
            published_at: Some("2026-01-01".to_string()),
        }
    }

    #[test]
    fn hashes_are_stable_and_prefixed() {
        let paper = canonicalize_paper(sample_paper()).expect("paper");
        let again = canonicalize_paper(sample_paper()).expect("paper");
        assert_eq!(paper.publication_hash, again.publication_hash);
        assert_eq!(paper.publication_hash.len(), 64);
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn ambiguous_license_is_rejected() {
        let mut paper = sample_paper();
        paper.license.spdx = "NOASSERTION".to_string();
        assert!(canonicalize_paper(paper).is_err());
    }

    #[test]
    fn context_pack_enforces_budget_and_target_presence() {
        let paper = canonicalize_paper(sample_paper()).expect("paper");
        let pack = pack_context(&paper, &["s1".to_string()], 128_000, 0.82, 4096).expect("pack");
        assert!(pack.estimated_tokens <= ((128_000_f64 * 0.82).floor() as u64 - 4096));
        assert_eq!(pack.target_section_ids, vec!["s1"]);
        assert!(pack_context(&paper, &["missing".to_string()], 128_000, 0.82, 4096).is_err());
    }

    #[test]
    fn acceptance_thresholds_are_hard_gates() {
        let mut acceptance = AcceptanceRecord {
            accepted: true,
            auditor_agreement: 0.75,
            answerability: 0.90,
            blind_correct_rate: 0.50,
            focused_correct_rate: 0.90,
            ambiguity_flag: false,
            hash_mismatch: false,
            redistributable: true,
            reason: None,
        };
        assert!(acceptance_passes(&acceptance));
        acceptance.blind_correct_rate = 0.51;
        assert!(!acceptance_passes(&acceptance));
    }

    #[test]
    fn challenge_sort_is_deterministic() {
        let base_acceptance = AcceptanceRecord {
            accepted: true,
            auditor_agreement: 1.0,
            answerability: 1.0,
            blind_correct_rate: 0.2,
            focused_correct_rate: 0.95,
            ambiguity_flag: false,
            hash_mismatch: false,
            redistributable: true,
            reason: None,
        };
        let mk = |hash: &str, difficulty: f64, blind: f64| ChallengeRecord {
            schema_version: CHALLENGE_SCHEMA_VERSION.to_string(),
            challenge_hash: hash.to_string(),
            publication_hash: "paper".to_string(),
            domain: "science".to_string(),
            topics: vec![],
            difficulty_score: difficulty,
            difficulty_components: BTreeMap::new(),
            question: "q".to_string(),
            answer_key: AnswerKey {
                canonical: "a".to_string(),
                must_include: vec![],
                must_not_include: vec![],
                aliases: vec![],
                numeric_tolerances: vec![],
                unit_tolerances: vec![],
            },
            support: vec![],
            context_pack: ContextPack {
                safe_window_tokens: 1,
                target_fill_ratio: 1.0,
                output_reserve_tokens: 0,
                estimated_tokens: 1,
                target_section_ids: vec![],
                distractor_section_ids: vec![],
            },
            generator_agents: vec![],
            blind_answer_attempts: vec![],
            focused_answer_attempts: vec![],
            critic_attempts: vec![],
            audit_attempts: vec![],
            acceptance: AcceptanceRecord {
                blind_correct_rate: blind,
                ..base_acceptance.clone()
            },
            artifact_hash: None,
        };
        let sorted = sorted_challenges(vec![
            mk("b", 0.7, 0.1),
            mk("a", 0.9, 0.4),
            mk("c", 0.9, 0.2),
        ]);
        assert_eq!(
            sorted
                .iter()
                .map(|c| c.challenge_hash.as_str())
                .collect::<Vec<_>>(),
            vec!["c", "a", "b"]
        );
    }
}
