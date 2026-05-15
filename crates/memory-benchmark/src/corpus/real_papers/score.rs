use super::super::model::{
    LoadedChallenge, NumericTolerance, PaperChallenge, PaperRecord, PaperSection,
};
use super::allow_fixture_qbank;
use crate::{AxisScores, ClaimModality, Event, EventKind, MemorySystem, PrivacyClass, Source};

pub(crate) fn observe_paper(
    adapter: &mut dyn MemorySystem,
    loaded: &LoadedChallenge,
) -> Result<(), String> {
    let challenge = &loaded.challenge;
    let paper = match &loaded.paper {
        Some(paper) => paper.clone(),
        None if allow_fixture_qbank() => fixture_paper_from_challenge(challenge),
        None => {
            return Err(format!(
                "missing paper JSON for {}",
                challenge.publication_hash
            ));
        }
    };
    if !paper.redistributable {
        return Err(format!(
            "publication {} is not redistributable",
            paper.publication_hash
        ));
    }
    for section in paper.sections {
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

fn fixture_paper_from_challenge(challenge: &PaperChallenge) -> PaperRecord {
    PaperRecord {
        publication_hash: challenge.publication_hash.clone(),
        title: "fixture paper".to_string(),
        license_spdx: "CC-BY-4.0".to_string(),
        redistributable: true,
        dedupe_keys: Vec::new(),
        source_ids: Vec::new(),
        source_url: None,
        retrieval_receipts: Vec::new(),
        review_receipts: Vec::new(),
        retrieval_kinds: Vec::new(),
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

pub(crate) fn grade_answer(
    answer: &str,
    used_ids: &[String],
    challenge: &PaperChallenge,
) -> AxisScores {
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
        compounding: f32::NAN,
        topic_hardening: f32::NAN,
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

pub(crate) fn weighted_average_total(avg: &AxisScores, counts: &AxisScores) -> f32 {
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
        (avg.compounding, w.compounding, counts.compounding),
        (
            avg.topic_hardening,
            w.topic_hardening,
            counts.topic_hardening,
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

pub(crate) fn challenge_order(a: &LoadedChallenge, b: &LoadedChallenge) -> std::cmp::Ordering {
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
