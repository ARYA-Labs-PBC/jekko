//! Loader and scorer for native real-paper QBank challenge records.
//!
//! Candidate systems observe only publication sections and context distractors.
//! Answer keys are parsed solely for post-recall grading.

use crate::json::{self, Json};
use crate::memory_api::axes_to_json;
use crate::qbank_hash::sha256_hex;
use crate::runner::CandidateReport;
use crate::runner_support::{accumulate, average, weighted_fraction};
use crate::{
    AxisScores, ClaimModality, Domain, Event, EventKind, MemorySystem, PrivacyClass, Query,
    QueryIntent, Source, SuiteConfig,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_BANK: &str = "crates/memory-benchmark/data/real-paper-bank";

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

pub fn default_bank_path() -> &'static Path {
    Path::new(DEFAULT_BANK)
}

pub fn load_challenges(root: &Path) -> Result<Vec<PaperChallenge>, String> {
    Ok(load_bank(root, &SuiteConfig::default())?
        .into_iter()
        .map(|loaded| loaded.challenge)
        .collect())
}

pub fn load_bank(root: &Path, config: &SuiteConfig) -> Result<Vec<LoadedChallenge>, String> {
    let mut loaded = Vec::new();
    for challenge in load_all_challenges(root)? {
        if let Some(topic) = config.qbank_topic_focus.as_deref() {
            if !challenge
                .topics
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(topic))
            {
                continue;
            }
        }
        let paper = load_paper(root, &challenge.publication_hash).ok();
        loaded.push(LoadedChallenge { challenge, paper });
    }
    loaded.sort_by(challenge_order);

    if let Some(path) = config.qbank_selection_path.as_deref() {
        let selected = load_selection(Path::new(path))?;
        loaded.retain(|item| selected.contains(&item.challenge.challenge_hash));
    }
    if config.qbank_top_n > 0 && loaded.len() > config.qbank_top_n {
        loaded.truncate(config.qbank_top_n);
    }
    Ok(loaded)
}

pub fn run_candidate(
    candidate: &str,
    adapter: &mut dyn MemorySystem,
    bank: &Path,
    config: &SuiteConfig,
) -> Result<CandidateReport, String> {
    let loaded = load_bank(bank, config)?;
    if loaded.is_empty() {
        return Err(format!(
            "no accepted challenge JSON found under {}",
            bank.display()
        ));
    }

    let mut axis_totals = AxisScores::default();
    let mut axis_counts = AxisScores::default();
    let mut fixtures_passed = 0u32;
    let mut fixture_records = Vec::new();

    for (index, loaded_challenge) in loaded.iter().enumerate() {
        observe_paper(adapter, loaded_challenge)?;
        let challenge = &loaded_challenge.challenge;
        let query = Query {
            text: challenge.question.clone(),
            intent: QueryIntent::Fact,
            mentions: vec![challenge.publication_hash.clone()],
            token_budget: config.context_budget,
        };
        let result = adapter.recall(&query);
        let axes = grade_answer(&result.answer, &result.used_ids, challenge);
        let weighted = weighted_fraction(&axes);
        if weighted >= 0.50 {
            fixtures_passed += 1;
        }
        accumulate(&mut axis_totals, &mut axis_counts, &axes);

        let mut record = BTreeMap::new();
        record.insert("id".to_string(), Json::Int((index + 1) as i64));
        record.insert(
            "challenge_hash".to_string(),
            Json::Str(challenge.challenge_hash.clone()),
        );
        record.insert(
            "publication_hash".to_string(),
            Json::Str(challenge.publication_hash.clone()),
        );
        record.insert("domain".to_string(), Json::Str(challenge.domain.clone()));
        record.insert(
            "difficulty_score".to_string(),
            Json::Float(challenge.difficulty_score as f64),
        );
        record.insert("axes".to_string(), axes_to_json(&axes));
        record.insert("weighted".to_string(), Json::Float(weighted as f64));
        fixture_records.push(Json::Object(record));
    }

    let avg = average(&axis_totals, &axis_counts);
    let total = weighted_average_total(&avg, &axis_counts);
    let mut top = BTreeMap::new();
    top.insert("name".to_string(), Json::Str(candidate.to_string()));
    top.insert("suite".to_string(), Json::Str("real-papers".to_string()));
    top.insert(
        "paper_bank".to_string(),
        Json::Str(bank.display().to_string()),
    );
    top.insert(
        "qbank_top_n".to_string(),
        Json::Int(config.qbank_top_n as i64),
    );
    top.insert("total".to_string(), Json::Float(total as f64));
    top.insert("axes".to_string(), axes_to_json(&avg));
    top.insert("fixtures_run".to_string(), Json::Int(loaded.len() as i64));
    top.insert(
        "fixtures_passed".to_string(),
        Json::Int(fixtures_passed as i64),
    );
    top.insert("fixtures".to_string(), Json::Array(fixture_records));
    let json = Json::Object(top).to_string();

    Ok(CandidateReport {
        name: candidate.to_string(),
        total,
        fixtures_run: loaded.len() as u32,
        fixtures_passed,
        json,
    })
}

fn observe_paper(adapter: &mut dyn MemorySystem, loaded: &LoadedChallenge) -> Result<(), String> {
    let challenge = &loaded.challenge;
    let paper = match &loaded.paper {
        Some(paper) => paper.clone(),
        None => fixture_paper_from_challenge(challenge),
    };
    if !paper.redistributable {
        return Err(format!(
            "publication {} is not redistributable",
            paper.publication_hash
        ));
    }
    let wanted = wanted_section_ids(challenge);
    for section in paper.sections {
        if !wanted.is_empty() && !wanted.contains(&section.section_id) {
            continue;
        }
        let event = Event {
            id: format!("{}#{}", paper.publication_hash, section.section_id),
            kind: EventKind::Claim,
            subject: paper.publication_hash.clone(),
            body: format!(
                "Paper: {}\nSection {} ({})\n{}",
                paper.title, section.section_id, section.title, section.text
            ),
            sources: vec![Source {
                uri: format!(
                    "qbank://paper/{}/{}",
                    paper.publication_hash, section.section_id
                ),
                citation: format!("{}#{}", paper.publication_hash, section.section_id),
                quality: 1.0,
            }],
            valid_from: None,
            valid_to: None,
            tx_time: "2026-05-12T00:00:00Z".to_string(),
            event_time: None,
            observation_time: None,
            review_time: None,
            policy_time: None,
            dependencies: Vec::new(),
            supersedes: Vec::new(),
            contradicts: Vec::new(),
            derived_from: Vec::new(),
            namespace: Some("opencode.real_papers.qbank".to_string()),
            privacy_class: PrivacyClass::Public,
            claim_modality: Some(ClaimModality::AssertedBySource),
            tags: vec![
                "real-paper".to_string(),
                "qbank".to_string(),
                format!("license:{}", paper.license_spdx),
            ],
        };
        let _ = adapter.observe(&event);
    }
    Ok(())
}

fn wanted_section_ids(challenge: &PaperChallenge) -> BTreeSet<String> {
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

fn fixture_paper_from_challenge(challenge: &PaperChallenge) -> PaperRecord {
    PaperRecord {
        publication_hash: challenge.publication_hash.clone(),
        title: "fixture paper".to_string(),
        license_spdx: "CC-BY-4.0".to_string(),
        redistributable: true,
        sections: challenge
            .support
            .iter()
            .map(|support| PaperSection {
                section_id: support.section_id.clone(),
                title: support.section_id.clone(),
                text: challenge.answer_key.canonical.clone(),
                section_hash: support.section_hash.clone(),
            })
            .collect(),
    }
}

fn grade_answer(answer: &str, used_ids: &[String], challenge: &PaperChallenge) -> AxisScores {
    let answer_lower = answer.to_ascii_lowercase();
    let mut required = challenge.answer_key.must_include.clone();
    if required.is_empty() {
        required = challenge
            .answer_key
            .canonical
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .filter(|term| term.len() > 3)
            .map(str::to_string)
            .collect();
    }
    let required_hits = required
        .iter()
        .filter(|term| answer_lower.contains(&term.to_ascii_lowercase()))
        .count();
    let alias_hit = challenge
        .answer_key
        .aliases
        .iter()
        .any(|alias| answer_lower.contains(&alias.to_ascii_lowercase()));
    let numeric_hits = challenge
        .answer_key
        .numeric_tolerances
        .iter()
        .filter(|tolerance| numeric_match(&answer_lower, tolerance))
        .count();
    let required_score = if required.is_empty() {
        if alias_hit || numeric_hits > 0 {
            1.0
        } else {
            0.5
        }
    } else {
        required_hits as f32 / required.len() as f32
    };
    let forbidden_penalty = challenge
        .answer_key
        .must_not_include
        .iter()
        .any(|term| answer_lower.contains(&term.to_ascii_lowercase()));
    let mut correctness = required_score.max(if alias_hit { 0.9 } else { 0.0 });
    if !challenge.answer_key.numeric_tolerances.is_empty() {
        correctness = correctness
            .max(numeric_hits as f32 / challenge.answer_key.numeric_tolerances.len() as f32);
    }
    if forbidden_penalty {
        correctness *= 0.25;
    }

    let required_support = challenge
        .support
        .iter()
        .map(|support| format!("{}#{}", challenge.publication_hash, support.section_id))
        .collect::<Vec<_>>();
    let support_hits = required_support
        .iter()
        .filter(|id| {
            used_ids
                .iter()
                .any(|used| used == *id || used == &challenge.publication_hash)
        })
        .count();
    let provenance = if required_support.is_empty() {
        if used_ids.iter().any(|id| id == &challenge.publication_hash) {
            1.0
        } else {
            0.5
        }
    } else {
        support_hits as f32 / required_support.len() as f32
    };
    let citation_minimality = if used_ids.len() <= required_support.len().saturating_add(2).max(1) {
        1.0
    } else {
        0.75
    };
    let provenance = provenance.min(citation_minimality);

    AxisScores {
        correctness,
        provenance,
        math_science: correctness.min(provenance),
        bitemporal_recall: f32::NAN,
        contradiction: f32::NAN,
        english_discourse_coreference: f32::NAN,
        privacy_redaction: f32::NAN,
        procedural_skill: f32::NAN,
        feedback_adaptation: f32::NAN,
        determinism_rebuild: f32::NAN,
    }
}

fn numeric_match(answer_lower: &str, tolerance: &NumericTolerance) -> bool {
    for token in answer_lower.split(|ch: char| !(ch.is_ascii_digit() || ch == '.' || ch == '-')) {
        let Ok(value) = token.parse::<f64>() else {
            continue;
        };
        if (value - tolerance.value).abs() <= tolerance.tolerance {
            if let Some(unit) = tolerance.unit.as_deref() {
                return answer_lower.contains(&unit.to_ascii_lowercase());
            }
            return true;
        }
    }
    false
}

pub fn validate_bank(
    root: &Path,
    allow_empty: bool,
    top_n: usize,
) -> Result<BankValidation, String> {
    let mut result = BankValidation::default();
    let mut paper_paths = Vec::new();
    collect_json_files(&root.join("papers"), &mut paper_paths)?;
    let mut challenge_paths = Vec::new();
    collect_json_files(&root.join("challenges"), &mut challenge_paths)?;
    let mut rejected_paths = Vec::new();
    collect_json_files(&root.join("rejected"), &mut rejected_paths)?;

    let mut seen_publications = BTreeSet::new();
    for path in &paper_paths {
        match read_paper(path) {
            Ok(paper) => {
                if !seen_publications.insert(paper.publication_hash.clone()) {
                    result.duplicate_publications += 1;
                    result
                        .errors
                        .push(format!("duplicate publication {}", paper.publication_hash));
                }
                if !paper.redistributable {
                    result
                        .errors
                        .push(format!("non-redistributable paper {}", path.display()));
                }
                for section in &paper.sections {
                    let expected = stable_section_hash(&section.text);
                    if !section.section_hash.is_empty() && section.section_hash != expected {
                        result.errors.push(format!(
                            "{} section {} hash mismatch",
                            path.display(),
                            section.section_id
                        ));
                    }
                }
            }
            Err(err) => result.errors.push(err),
        }
    }

    let mut accepted = Vec::new();
    let mut seen_challenges = BTreeSet::new();
    for path in &challenge_paths {
        match read_challenge(path) {
            Ok(challenge) => {
                if !seen_challenges.insert(challenge.challenge_hash.clone()) {
                    result
                        .errors
                        .push(format!("duplicate challenge {}", challenge.challenge_hash));
                }
                if let Err(err) = validate_challenge_hash(&challenge) {
                    result.errors.push(format!("{}: {err}", path.display()));
                }
                if let Err(err) = validate_acceptance(&challenge) {
                    result.errors.push(format!("{}: {err}", path.display()));
                }
                if challenge.context_pack.estimated_tokens
                    > context_token_budget(&challenge.context_pack)
                {
                    result.errors.push(format!(
                        "{}: context pack exceeds token limit",
                        path.display()
                    ));
                }
                accepted.push(challenge);
            }
            Err(err) => result.errors.push(err),
        }
    }
    result.accepted_challenges = accepted.len();
    result.rejected_challenges = rejected_paths.len();
    result.top_selected = accepted.len().min(top_n);
    accepted.sort_by(challenge_order_plain);
    let mut manifest_material = String::new();
    for challenge in accepted.iter().take(result.top_selected) {
        manifest_material.push_str(&challenge.challenge_hash);
        manifest_material.push('\n');
    }
    result.manifest_hash = sha256_hex(manifest_material.as_bytes());

    if !allow_empty && result.accepted_challenges == 0 {
        result
            .errors
            .push("bank has no accepted challenges".to_string());
    }
    Ok(result)
}

fn validate_challenge_hash(challenge: &PaperChallenge) -> Result<(), String> {
    if challenge.challenge_hash.len() != 64 {
        return Ok(()); // Back-compat for older fixture hashes is handled by content checks.
    }
    let support_hashes = challenge
        .support
        .iter()
        .map(|support| support.section_hash.clone())
        .collect::<Vec<_>>();
    let expected = stable_challenge_hash(
        &challenge.publication_hash,
        &challenge.question,
        &challenge.answer_key.canonical,
        &support_hashes,
    );
    if expected != challenge.challenge_hash {
        return Err("challenge_hash mismatch".to_string());
    }
    Ok(())
}

fn validate_acceptance(challenge: &PaperChallenge) -> Result<(), String> {
    if challenge.answerability < 0.90 {
        return Err("answerability below 0.90".to_string());
    }
    if challenge.blind_correct_rate > 0.50 {
        return Err("blind_correct_rate above 0.50".to_string());
    }
    if challenge.focused_correct_rate < 0.90 {
        return Err("focused_correct_rate below 0.90".to_string());
    }
    Ok(())
}

fn context_token_budget(context: &ContextPack) -> u32 {
    ((context.safe_window_tokens as f32 * context.target_fill_ratio).floor() as i64
        - context.output_reserve_tokens as i64)
        .max(0) as u32
}

fn load_all_challenges(root: &Path) -> Result<Vec<PaperChallenge>, String> {
    let challenge_root = if root.ends_with("challenges") {
        root.to_path_buf()
    } else {
        root.join("challenges")
    };
    let mut files = Vec::new();
    collect_json_files(&challenge_root, &mut files)?;
    let mut out = Vec::new();
    for file in files {
        out.push(read_challenge(&file)?);
    }
    Ok(out)
}

fn load_paper(root: &Path, publication_hash: &str) -> Result<PaperRecord, String> {
    let paper_path = root.join("papers").join(format!("{publication_hash}.json"));
    read_paper(&paper_path)
}

fn read_paper(file: &Path) -> Result<PaperRecord, String> {
    let text =
        fs::read_to_string(file).map_err(|err| format!("read {}: {}", file.display(), err))?;
    let parsed = json::parse(&text).map_err(|err| format!("parse {}: {}", file.display(), err))?;
    paper_from_json(&parsed).map_err(|err| format!("{}: {}", file.display(), err))
}

fn read_challenge(file: &Path) -> Result<PaperChallenge, String> {
    let text =
        fs::read_to_string(file).map_err(|err| format!("read {}: {}", file.display(), err))?;
    let parsed = json::parse(&text).map_err(|err| format!("parse {}: {}", file.display(), err))?;
    challenge_from_json(&parsed).map_err(|err| format!("{}: {}", file.display(), err))
}

fn paper_from_json(value: &Json) -> Result<PaperRecord, String> {
    let obj = as_object(value)?;
    let license_obj = obj.get("license").and_then(as_object_ok);
    let license_spdx = license_obj
        .and_then(|license| license.get("spdx").and_then(as_str))
        .unwrap_or("NOASSERTION")
        .to_string();
    let redistributable = license_obj
        .and_then(|license| license.get("redistributable").and_then(as_bool))
        .unwrap_or(false);
    let sections = required_array(obj, "sections")?
        .iter()
        .map(section_from_json)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PaperRecord {
        publication_hash: required_string(obj, "publication_hash")?,
        title: optional_string(obj, "title").unwrap_or_else(|| "untitled".to_string()),
        license_spdx,
        redistributable,
        sections,
    })
}

fn section_from_json(value: &Json) -> Result<PaperSection, String> {
    let obj = as_object(value)?;
    Ok(PaperSection {
        section_id: required_string(obj, "section_id")?,
        title: optional_string(obj, "title").unwrap_or_default(),
        text: required_string(obj, "text")?,
        section_hash: optional_string(obj, "section_hash").unwrap_or_default(),
    })
}

fn challenge_from_json(value: &Json) -> Result<PaperChallenge, String> {
    let obj = as_object(value)?;
    let acceptance = as_object(required(obj, "acceptance")?)?;
    let accepted = acceptance
        .get("accepted")
        .and_then(as_bool)
        .unwrap_or(false);
    if !accepted {
        return Err("challenge is not accepted".to_string());
    }
    let answer_key = match required(obj, "answer_key")? {
        Json::Str(value) => AnswerKey {
            canonical: value.clone(),
            must_include: Vec::new(),
            must_not_include: Vec::new(),
            aliases: Vec::new(),
            numeric_tolerances: Vec::new(),
            unit_tolerances: Vec::new(),
        },
        other => answer_key_from_json(other)?,
    };
    let support = if let Some(value) = obj.get("support") {
        required_array_value(value, "support")?
            .iter()
            .map(support_from_json)
            .collect::<Result<Vec<_>, _>>()?
    } else {
        required_array(obj, "support_sections")?
            .iter()
            .filter_map(as_str)
            .map(|section_id| SupportRef {
                section_id: section_id.to_string(),
                section_hash: String::new(),
            })
            .collect()
    };
    Ok(PaperChallenge {
        challenge_hash: required_string(obj, "challenge_hash")?,
        publication_hash: required_string(obj, "publication_hash")?,
        domain: optional_string(obj, "domain")
            .unwrap_or_else(|| Domain::Science.name().to_string()),
        topics: optional_string_array(obj, "topics"),
        difficulty_score: optional_f32(obj, "difficulty_score").unwrap_or(0.0),
        answerability: optional_f32(acceptance, "answerability").unwrap_or(1.0),
        focused_correct_rate: optional_f32(acceptance, "focused_correct_rate").unwrap_or(1.0),
        blind_correct_rate: optional_f32(acceptance, "blind_correct_rate").unwrap_or(0.0),
        question: required_string(obj, "question")?,
        answer_key,
        support,
        context_pack: obj
            .get("context_pack")
            .map(context_pack_from_json)
            .transpose()?
            .unwrap_or_default(),
    })
}

fn answer_key_from_json(value: &Json) -> Result<AnswerKey, String> {
    let obj = as_object(value)?;
    Ok(AnswerKey {
        canonical: required_string(obj, "canonical")?,
        must_include: optional_string_array(obj, "must_include"),
        must_not_include: optional_string_array(obj, "must_not_include"),
        aliases: optional_string_array(obj, "aliases"),
        numeric_tolerances: obj
            .get("numeric_tolerances")
            .map(|value| {
                required_array_value(value, "numeric_tolerances")?
                    .iter()
                    .map(numeric_tolerance_from_json)
                    .collect::<Result<Vec<_>, String>>()
            })
            .transpose()?
            .unwrap_or_default(),
        unit_tolerances: optional_string_array(obj, "unit_tolerances"),
    })
}

fn numeric_tolerance_from_json(value: &Json) -> Result<NumericTolerance, String> {
    let obj = as_object(value)?;
    Ok(NumericTolerance {
        value: required_f64(obj, "value")?,
        tolerance: required_f64(obj, "tolerance")?,
        unit: optional_string(obj, "unit"),
    })
}

fn support_from_json(value: &Json) -> Result<SupportRef, String> {
    let obj = as_object(value)?;
    Ok(SupportRef {
        section_id: required_string(obj, "section_id")?,
        section_hash: optional_string(obj, "section_hash").unwrap_or_default(),
    })
}

fn context_pack_from_json(value: &Json) -> Result<ContextPack, String> {
    let obj = as_object(value)?;
    Ok(ContextPack {
        safe_window_tokens: optional_i64(obj, "safe_window_tokens").unwrap_or(128000) as u32,
        target_fill_ratio: optional_f32(obj, "target_fill_ratio").unwrap_or(0.82),
        output_reserve_tokens: optional_i64(obj, "output_reserve_tokens").unwrap_or(4096) as u32,
        estimated_tokens: optional_i64(obj, "estimated_tokens").unwrap_or(0) as u32,
        target_section_ids: optional_string_array(obj, "target_section_ids"),
        distractor_section_ids: optional_string_array(obj, "distractor_section_ids"),
    })
}

fn collect_json_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    let entries =
        fs::read_dir(root).map_err(|err| format!("read_dir {}: {}", root.display(), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

fn load_selection(path: &Path) -> Result<BTreeSet<String>, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect())
}

fn challenge_order(a: &LoadedChallenge, b: &LoadedChallenge) -> std::cmp::Ordering {
    challenge_order_plain(&a.challenge, &b.challenge)
}

fn challenge_order_plain(a: &PaperChallenge, b: &PaperChallenge) -> std::cmp::Ordering {
    b.difficulty_score
        .total_cmp(&a.difficulty_score)
        .then(b.focused_correct_rate.total_cmp(&a.focused_correct_rate))
        .then(a.blind_correct_rate.total_cmp(&b.blind_correct_rate))
        .then(a.publication_hash.cmp(&b.publication_hash))
        .then(a.challenge_hash.cmp(&b.challenge_hash))
}

fn weighted_average_total(avg: &AxisScores, counts: &AxisScores) -> f32 {
    let w = AxisScores::WEIGHTS;
    let pairs = [
        (avg.correctness, w.correctness, counts.correctness),
        (avg.provenance, w.provenance, counts.provenance),
        (
            avg.bitemporal_recall,
            w.bitemporal_recall,
            counts.bitemporal_recall,
        ),
        (avg.contradiction, w.contradiction, counts.contradiction),
        (avg.math_science, w.math_science, counts.math_science),
        (
            avg.english_discourse_coreference,
            w.english_discourse_coreference,
            counts.english_discourse_coreference,
        ),
        (
            avg.privacy_redaction,
            w.privacy_redaction,
            counts.privacy_redaction,
        ),
        (
            avg.procedural_skill,
            w.procedural_skill,
            counts.procedural_skill,
        ),
        (
            avg.feedback_adaptation,
            w.feedback_adaptation,
            counts.feedback_adaptation,
        ),
        (
            avg.determinism_rebuild,
            w.determinism_rebuild,
            counts.determinism_rebuild,
        ),
    ];
    let mut sum = 0.0_f32;
    let mut wsum = 0.0_f32;
    for (value, weight, count) in pairs {
        if count > 0.0 {
            sum += value * weight;
            wsum += weight;
        }
    }
    if wsum > 0.0 {
        sum / wsum * 100.0
    } else {
        0.0
    }
}

fn stable_section_hash(text: &str) -> String {
    sha256_hex(normalize_text(text).as_bytes())
}

fn stable_challenge_hash(
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

fn normalize_text(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn as_object(value: &Json) -> Result<&BTreeMap<String, Json>, String> {
    match value {
        Json::Object(obj) => Ok(obj),
        _ => Err("expected object".to_string()),
    }
}

fn as_object_ok(value: &Json) -> Option<&BTreeMap<String, Json>> {
    match value {
        Json::Object(obj) => Some(obj),
        _ => None,
    }
}

fn required<'a>(obj: &'a BTreeMap<String, Json>, key: &str) -> Result<&'a Json, String> {
    match obj.get(key) {
        Some(value) => Ok(value),
        None => Err(format!("missing {key}")),
    }
}

fn required_string(obj: &BTreeMap<String, Json>, key: &str) -> Result<String, String> {
    let value = required(obj, key)?;
    match as_str(value) {
        Some(s) => Ok(s.to_string()),
        None => Err(format!("{key} must be a string")),
    }
}

fn optional_string(obj: &BTreeMap<String, Json>, key: &str) -> Option<String> {
    obj.get(key).and_then(as_str).map(str::to_string)
}

fn required_f64(obj: &BTreeMap<String, Json>, key: &str) -> Result<f64, String> {
    match required(obj, key)? {
        Json::Float(value) => Ok(*value),
        Json::Int(value) => Ok(*value as f64),
        _ => Err(format!("{key} must be a number")),
    }
}

fn optional_i64(obj: &BTreeMap<String, Json>, key: &str) -> Option<i64> {
    match obj.get(key) {
        Some(Json::Int(value)) => Some(*value),
        Some(Json::Float(value)) => Some(*value as i64),
        _ => None,
    }
}

fn optional_f32(obj: &BTreeMap<String, Json>, key: &str) -> Option<f32> {
    match obj.get(key) {
        Some(Json::Int(value)) => Some(*value as f32),
        Some(Json::Float(value)) => Some(*value as f32),
        _ => None,
    }
}

fn required_array<'a>(obj: &'a BTreeMap<String, Json>, key: &str) -> Result<&'a [Json], String> {
    required_array_value(required(obj, key)?, key)
}

fn required_array_value<'a>(value: &'a Json, key: &str) -> Result<&'a [Json], String> {
    match value {
        Json::Array(items) => Ok(items),
        _ => Err(format!("{key} must be an array")),
    }
}

fn optional_string_array(obj: &BTreeMap<String, Json>, key: &str) -> Vec<String> {
    obj.get(key)
        .and_then(|value| required_array_value(value, key).ok())
        .map(|items| {
            items
                .iter()
                .filter_map(as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn as_str(value: &Json) -> Option<&str> {
    match value {
        Json::Str(value) => Some(value.as_str()),
        _ => None,
    }
}

fn as_bool(value: &Json) -> Option<bool> {
    match value {
        Json::Bool(value) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::baseline;

    #[test]
    fn legacy_fixture_challenge_still_loads() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/real-paper-bank");
        let challenges = load_challenges(&root).expect("load challenges");
        assert_eq!(challenges.len(), 1);
        assert_eq!(challenges[0].answer_key.canonical, "alpha equals one");
    }

    #[test]
    fn answer_key_is_not_observed_as_memory_event() {
        let loaded = LoadedChallenge {
            paper: Some(PaperRecord {
                publication_hash: "paper-a".to_string(),
                title: "Paper A".to_string(),
                license_spdx: "CC-BY-4.0".to_string(),
                redistributable: true,
                sections: vec![PaperSection {
                    section_id: "s1".to_string(),
                    title: "Result".to_string(),
                    text: "The paper discusses alpha without revealing the hidden oracle phrase."
                        .to_string(),
                    section_hash: "h1".to_string(),
                }],
            }),
            challenge: PaperChallenge {
                challenge_hash: "challenge-a".to_string(),
                publication_hash: "paper-a".to_string(),
                domain: "science".to_string(),
                topics: vec![],
                difficulty_score: 1.0,
                answerability: 1.0,
                focused_correct_rate: 1.0,
                blind_correct_rate: 0.0,
                question: "What is the hidden oracle phrase?".to_string(),
                answer_key: AnswerKey {
                    canonical: "forbidden answer key phrase".to_string(),
                    must_include: vec!["forbidden answer key phrase".to_string()],
                    ..AnswerKey::default()
                },
                support: vec![SupportRef {
                    section_id: "s1".to_string(),
                    section_hash: "h1".to_string(),
                }],
                context_pack: ContextPack {
                    target_section_ids: vec!["s1".to_string()],
                    ..ContextPack::default()
                },
            },
        };
        let mut adapter = baseline::Adapter::default();
        observe_paper(&mut adapter, &loaded).expect("observe");
        let result = adapter.recall(&Query {
            text: "paper-a".to_string(),
            intent: QueryIntent::Fact,
            mentions: vec!["paper-a".to_string()],
            token_budget: 4096,
        });
        assert!(!result.answer.contains("forbidden answer key phrase"));
    }

    #[test]
    fn top_n_sort_uses_hardness_then_rates_then_hashes() {
        let mut config = SuiteConfig {
            qbank_top_n: 2,
            ..SuiteConfig::default()
        };
        config.paper_bank_path = None;
        let mut a = fixture_challenge("a", 0.9, 0.1);
        let b = fixture_challenge("b", 0.8, 0.0);
        let c = fixture_challenge("c", 0.9, 0.4);
        a.publication_hash = "paper-a".to_string();
        let mut loaded = vec![
            LoadedChallenge {
                challenge: b,
                paper: None,
            },
            LoadedChallenge {
                challenge: c,
                paper: None,
            },
            LoadedChallenge {
                challenge: a,
                paper: None,
            },
        ];
        loaded.sort_by(challenge_order);
        assert_eq!(
            loaded
                .iter()
                .map(|item| item.challenge.challenge_hash.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "c", "b"]
        );
    }

    fn fixture_challenge(hash: &str, difficulty: f32, blind: f32) -> PaperChallenge {
        PaperChallenge {
            challenge_hash: hash.to_string(),
            publication_hash: "paper".to_string(),
            domain: "science".to_string(),
            topics: vec![],
            difficulty_score: difficulty,
            answerability: 1.0,
            focused_correct_rate: 1.0,
            blind_correct_rate: blind,
            question: "q".to_string(),
            answer_key: AnswerKey {
                canonical: "alpha equals one".to_string(),
                must_include: vec!["alpha".to_string()],
                must_not_include: vec![],
                aliases: vec![],
                numeric_tolerances: vec![],
                unit_tolerances: vec![],
            },
            support: vec![SupportRef {
                section_id: "s1".to_string(),
                section_hash: "h1".to_string(),
            }],
            context_pack: ContextPack::default(),
        }
    }
}
