use super::validation_support::{validate_route, validate_token_usage};
use crate::corpus::real_papers::model::{
    stable_challenge_hash, stable_section_hash, ModelTrial, PaperChallenge, PaperRecord,
    PRODUCTION_CHALLENGE_SCHEMA_VERSION,
};
use std::collections::BTreeMap;
pub(super) fn validate_paper_presence(
    challenge: &PaperChallenge,
    papers_by_hash: &BTreeMap<String, PaperRecord>,
    allow_fixture_qbank: bool,
) -> Result<(), String> {
    let Some(paper) = papers_by_hash.get(&challenge.publication_hash) else {
        if allow_fixture_qbank {
            return Ok(());
        }
        return Err(format!(
            "missing redistributable paper JSON for {}",
            challenge.publication_hash
        ));
    };
    if !paper.redistributable {
        return Err(format!(
            "paper {} is not redistributable",
            challenge.publication_hash
        ));
    }
    for support in &challenge.support {
        if support.section_hash.is_empty() {
            if allow_fixture_qbank {
                continue;
            }
            return Err(format!(
                "support section {} for {} lacks section_hash",
                support.section_id, challenge.publication_hash
            ));
        }
        let Some(section) = paper
            .sections
            .iter()
            .find(|section| section.section_id == support.section_id)
        else {
            return Err(format!(
                "support section {} missing from paper {}",
                support.section_id, challenge.publication_hash
            ));
        };
        let expected = stable_section_hash(&section.text);
        if support.section_hash != expected {
            return Err(format!(
                "support section {} hash mismatch for {}",
                support.section_id, challenge.publication_hash
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_production_challenge(challenge: &PaperChallenge) -> Vec<String> {
    let mut errors = Vec::new();
    if challenge.schema_version != PRODUCTION_CHALLENGE_SCHEMA_VERSION {
        errors.push("challenge schema is not production v3".to_string());
    }
    match challenge.source_publication.as_ref() {
        Some(source) => {
            if source.publication_hash != challenge.publication_hash {
                errors.push("source_publication hash does not match challenge".to_string());
            }
            if source.content_hash.trim().is_empty() {
                errors.push("source_publication missing content_hash".to_string());
            }
            if !source.redistributable {
                errors.push("source_publication is not redistributable".to_string());
            }
            if source.license_spdx.eq_ignore_ascii_case("NOASSERTION") {
                errors.push("source_publication license is ambiguous".to_string());
            }
            if source
                .source_url
                .as_deref()
                .unwrap_or("")
                .contains("example.invalid")
                || source
                    .source_url
                    .as_deref()
                    .unwrap_or("")
                    .contains("qbank-smoke.openaccess.local")
            {
                errors.push("source_publication uses fixture URL".to_string());
            }
            if source.section_hashes.is_empty() {
                errors.push("source_publication missing section hashes".to_string());
            }
        }
        None => errors.push("missing source_publication".to_string()),
    }
    if challenge.focused_support_trials.len() < 3 {
        errors.push("missing focused support trials".to_string());
    }
    if challenge.saturated_blind_trials.len() < 3 {
        errors.push("missing saturated blind trials".to_string());
    }
    if challenge.judge_trials.is_empty() {
        errors.push("missing judge trials".to_string());
    }
    if challenge.context_packs.is_empty() {
        errors.push("missing context pack provenance".to_string());
    }
    if challenge.route_metadata.is_empty() {
        errors.push("missing top-level route metadata".to_string());
    }
    for (index, trial) in challenge.focused_support_trials.iter().enumerate() {
        validate_trial("focused_support_trials", index, trial, &mut errors);
    }
    for (index, trial) in challenge.saturated_blind_trials.iter().enumerate() {
        validate_trial("saturated_blind_trials", index, trial, &mut errors);
    }
    for (index, judge) in challenge.judge_trials.iter().enumerate() {
        if judge.agent_id.trim().is_empty() {
            errors.push(format!("judge_trials[{index}] missing agent_id"));
        }
        if !judge.accepted {
            errors.push(format!("judge_trials[{index}] did not accept challenge"));
        }
        if !(0.0..=1.0).contains(&judge.confidence) {
            errors.push(format!("judge_trials[{index}] confidence outside [0,1]"));
        }
        if judge.rationale_hash.trim().is_empty() {
            errors.push(format!("judge_trials[{index}] missing rationale_hash"));
        }
        validate_route(
            &format!("judge_trials[{index}].route_metadata"),
            &judge.route_metadata,
            &mut errors,
        );
        validate_token_usage(
            &format!("judge_trials[{index}].token_usage"),
            judge.token_usage.prompt_tokens,
            judge.token_usage.completion_tokens,
            judge.token_usage.total_tokens,
            &mut errors,
        );
    }
    for (index, route) in challenge.route_metadata.iter().enumerate() {
        validate_route(&format!("route_metadata[{index}]"), route, &mut errors);
    }
    for (index, pack) in challenge.context_packs.iter().enumerate() {
        if pack.kind.trim().is_empty() {
            errors.push(format!("context_packs[{index}] missing kind"));
        }
        if pack.context_hash.trim().is_empty() || pack.prompt_hash.trim().is_empty() {
            errors.push(format!("context_packs[{index}] missing hashes"));
        }
        if pack.section_ids.is_empty() || pack.estimated_tokens == 0 {
            errors.push(format!(
                "context_packs[{index}] missing section/token provenance"
            ));
        }
    }
    match challenge.acceptance_metrics.as_ref() {
        Some(metrics) => {
            if metrics.focused_agreement < 0.75 {
                errors.push("focused agreement below 0.75".to_string());
            }
            if metrics.focused_correct_rate < 0.90 || challenge.focused_correct_rate < 0.90 {
                errors.push("focused correct rate below 0.90".to_string());
            }
            if metrics.answerability < 0.90 || challenge.answerability < 0.90 {
                errors.push("answerability below 0.90".to_string());
            }
            if metrics.saturated_blind_correct_rate > 0.50 || challenge.blind_correct_rate > 0.50 {
                errors.push("saturated blind correct rate above 0.50".to_string());
            }
            if metrics.saturated_mean_confidence > 0.55 {
                errors.push("saturated mean confidence above 0.55".to_string());
            }
        }
        None => errors.push("missing acceptance_metrics".to_string()),
    }
    match challenge.artifact_provenance.as_ref() {
        Some(provenance) => {
            if provenance.run_id.trim().is_empty() || provenance.reducer_version.trim().is_empty() {
                errors.push("artifact provenance missing run or reducer".to_string());
            }
            if provenance.fixture_provenance {
                errors.push("fixture provenance is not allowed".to_string());
            }
            if provenance.agent_mode.as_deref() != Some("live_jnoccio") {
                errors.push("artifact provenance is not live_jnoccio".to_string());
            }
            if provenance.answer_leakage_detected {
                errors.push("answer leakage detected".to_string());
            }
            if provenance.license_ambiguous {
                errors.push("license ambiguity detected".to_string());
            }
        }
        None => errors.push("missing artifact_provenance".to_string()),
    }
    if challenge.question.to_ascii_lowercase().contains("fixture")
        || challenge
            .question
            .to_ascii_lowercase()
            .contains("generated")
        || challenge.topics.iter().any(|topic| {
            topic.eq_ignore_ascii_case("fixture") || topic.eq_ignore_ascii_case("generated")
        })
    {
        errors.push("fixture marker in challenge".to_string());
    }
    errors
}

pub(super) fn validate_trial(
    field: &str,
    index: usize,
    trial: &ModelTrial,
    errors: &mut Vec<String>,
) {
    if trial.agent_id.trim().is_empty() || trial.phase.trim().is_empty() {
        errors.push(format!("{field}[{index}] missing identity"));
    }
    if trial.prompt_hash.trim().is_empty() || trial.context_hash.trim().is_empty() {
        errors.push(format!("{field}[{index}] missing hashes"));
    }
    if !(0.0..=1.0).contains(&trial.confidence) {
        errors.push(format!("{field}[{index}] confidence outside [0,1]"));
    }
    if !(0.0..=1.0).contains(&trial.answerability) {
        errors.push(format!("{field}[{index}] answerability outside [0,1]"));
    }
    if field == "focused_support_trials" && (!trial.correct || !trial.supported) {
        errors.push(format!("{field}[{index}] failed support/correctness"));
    }
    validate_route(
        &format!("{field}[{index}].route_metadata"),
        &trial.route_metadata,
        errors,
    );
    validate_token_usage(
        &format!("{field}[{index}].token_usage"),
        trial.token_usage.prompt_tokens,
        trial.token_usage.completion_tokens,
        trial.token_usage.total_tokens,
        errors,
    );
}

pub(super) fn validate_challenge_hash(challenge: &PaperChallenge) -> Result<(), String> {
    if challenge.challenge_hash.len() != 64 {
        return Ok(()); // Older fixture hashes are accepted by content checks.
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

pub(super) fn validate_acceptance(challenge: &PaperChallenge) -> Result<(), String> {
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
