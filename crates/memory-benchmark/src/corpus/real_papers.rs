//! Loader for OpenQG paper question-bank challenge records.

use crate::json::{self, Json};
use crate::memory_api::axes_to_json;
use crate::runner::CandidateReport;
use crate::runner_support::{accumulate, average, weighted_fraction};
use crate::{
    AxisScores, ClaimModality, Domain, Event, EventKind, MemorySystem, PrivacyClass, Query,
    QueryIntent, Source,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PaperChallenge {
    pub challenge_hash: String,
    pub publication_hash: String,
    pub question: String,
    pub answer_key: String,
    pub support_sections: Vec<String>,
}

pub fn load_challenges(root: &Path) -> Result<Vec<PaperChallenge>, String> {
    let challenge_root = if root.ends_with("challenges") {
        root.to_path_buf()
    } else {
        root.join("challenges")
    };
    let mut files = Vec::new();
    collect_json_files(&challenge_root, &mut files)?;
    let mut out = Vec::new();
    for file in files {
        let text =
            fs::read_to_string(&file).map_err(|err| format!("read {}: {}", file.display(), err))?;
        let parsed =
            json::parse(&text).map_err(|err| format!("parse {}: {}", file.display(), err))?;
        out.push(
            challenge_from_json(&parsed).map_err(|err| format!("{}: {}", file.display(), err))?,
        );
    }
    Ok(out)
}

pub fn run_candidate(
    candidate: &str,
    adapter: &mut dyn MemorySystem,
    bank: &Path,
) -> Result<CandidateReport, String> {
    let challenges = load_challenges(bank)?;
    if challenges.is_empty() {
        return Err(format!(
            "no accepted challenge JSON found under {}",
            bank.display()
        ));
    }

    let mut axis_totals = AxisScores::default();
    let mut axis_counts = AxisScores::default();
    let mut fixtures_passed = 0u32;
    let mut fixture_records = Vec::new();

    for (index, challenge) in challenges.iter().enumerate() {
        let event = Event {
            id: challenge.publication_hash.clone(),
            kind: EventKind::Claim,
            subject: challenge.publication_hash.clone(),
            body: format!(
                "Question: {}\nAnswer key: {}\nSupport sections: {}",
                challenge.question,
                challenge.answer_key,
                challenge.support_sections.join(", ")
            ),
            sources: vec![Source {
                uri: format!("openqg://paper/{}", challenge.publication_hash),
                citation: challenge.publication_hash.clone(),
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
            namespace: Some("openqg.real_papers".to_string()),
            privacy_class: PrivacyClass::Public,
            claim_modality: Some(ClaimModality::HumanApproved),
            tags: vec!["real-paper".to_string(), "openqg-question-bank".to_string()],
        };
        let _ = adapter.observe(&event);
        let query = Query {
            text: challenge.question.clone(),
            intent: QueryIntent::Fact,
            mentions: vec![challenge.publication_hash.clone()],
            token_budget: 4096,
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
        record.insert(
            "domain".to_string(),
            Json::Str(Domain::Science.name().to_string()),
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
    top.insert("total".to_string(), Json::Float(total as f64));
    top.insert("axes".to_string(), axes_to_json(&avg));
    top.insert(
        "fixtures_run".to_string(),
        Json::Int(challenges.len() as i64),
    );
    top.insert(
        "fixtures_passed".to_string(),
        Json::Int(fixtures_passed as i64),
    );
    top.insert("fixtures".to_string(), Json::Array(fixture_records));
    let json = Json::Object(top).to_string();

    Ok(CandidateReport {
        name: candidate.to_string(),
        total,
        fixtures_run: challenges.len() as u32,
        fixtures_passed,
        json,
    })
}

fn grade_answer(answer: &str, used_ids: &[String], challenge: &PaperChallenge) -> AxisScores {
    let answer_lower = answer.to_ascii_lowercase();
    let key_lower = challenge.answer_key.to_ascii_lowercase();
    let key_terms = key_lower
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|term| term.len() > 3)
        .collect::<Vec<_>>();
    let matched_terms = key_terms
        .iter()
        .filter(|term| answer_lower.contains(**term))
        .count();
    let correctness = if key_terms.is_empty() {
        0.5
    } else {
        matched_terms as f32 / key_terms.len() as f32
    };
    let provenance = if used_ids.iter().any(|id| id == &challenge.publication_hash) {
        1.0
    } else {
        0.25
    };
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

fn challenge_from_json(value: &Json) -> Result<PaperChallenge, String> {
    let obj = as_object(value)?;
    let accepted = as_object(required(obj, "acceptance")?)?
        .get("accepted")
        .and_then(as_bool)
        .unwrap_or(false);
    if !accepted {
        return Err("challenge is not accepted".to_string());
    }
    Ok(PaperChallenge {
        challenge_hash: required_string(obj, "challenge_hash")?,
        publication_hash: required_string(obj, "publication_hash")?,
        question: required_string(obj, "question")?,
        answer_key: required_string(obj, "answer_key")?,
        support_sections: required_array(obj, "support_sections")?
            .iter()
            .filter_map(as_str)
            .map(str::to_string)
            .collect(),
    })
}

fn collect_json_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
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

fn as_object(value: &Json) -> Result<&BTreeMap<String, Json>, String> {
    match value {
        Json::Object(obj) => Ok(obj),
        _ => Err("expected object".to_string()),
    }
}

fn required<'a>(obj: &'a BTreeMap<String, Json>, key: &str) -> Result<&'a Json, String> {
    obj.get(key).ok_or_else(|| format!("missing {key}"))
}

fn required_string(obj: &BTreeMap<String, Json>, key: &str) -> Result<String, String> {
    as_str(required(obj, key)?)
        .map(str::to_string)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn required_array<'a>(obj: &'a BTreeMap<String, Json>, key: &str) -> Result<&'a [Json], String> {
    match required(obj, key)? {
        Json::Array(items) => Ok(items),
        _ => Err(format!("{key} must be an array")),
    }
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
