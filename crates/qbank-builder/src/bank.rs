use super::{
    AcceptanceRecord, ChallengeRecord, ContextPack, PaperRecord, MIN_SUCCESSFUL_TESTERS,
    MIN_SUCCESSFUL_VERIFIERS, PRODUCTION_CHALLENGE_SCHEMA_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "bank_io.rs"]
mod bank_io;

#[path = "bank_validation.rs"]
mod bank_validation;

pub use bank_io::{
    bank_subdir, challenge_sort_key, collect_json_files, ensure_bank_layout, manifest_hash,
    read_challenges, read_json, read_papers, sorted_challenges, write_json_pretty,
};
use bank_validation::{validate_model_trial, validate_route_metadata, validate_token_usage};

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

pub fn production_acceptance_passes(challenge: &ChallengeRecord) -> bool {
    production_acceptance_errors(challenge).is_empty()
}

pub fn production_bank_errors(
    challenges: &[ChallengeRecord],
    min_required_accepted: usize,
) -> Vec<String> {
    let mut errors = Vec::new();
    if challenges.is_empty() {
        errors.push("bank has no accepted challenges".to_string());
        return errors;
    }

    let mut publication_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut domain_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for challenge in challenges {
        *publication_counts
            .entry(challenge.publication_hash.as_str())
            .or_default() += 1;
        *domain_counts.entry(challenge.domain.as_str()).or_default() += 1;
        if challenge.route_metadata.is_empty() {
            errors.push(format!(
                "{} missing top-level route metadata",
                challenge.challenge_hash
            ));
        }
    }

    for (publication, count) in &publication_counts {
        if *count > 3 {
            errors.push(format!(
                "publication {publication} has {count} accepted challenges; max 3 allowed"
            ));
        }
    }

    let unique_publications = publication_counts.len();
    let required_unique_publications = ((min_required_accepted as f64) * 0.34).ceil() as usize;
    if unique_publications < required_unique_publications {
        errors.push(format!(
            "bank has {unique_publications} unique publications; need at least {required_unique_publications} for {min_required_accepted} accepted challenges"
        ));
    }

    let accepted = challenges.len().max(1);
    let max_domain_share =
        domain_counts.values().copied().max().unwrap_or(0) as f64 / accepted as f64;
    if min_required_accepted >= 10 && accepted >= 10 && max_domain_share > 0.35 {
        let worst_domain =
            if let Some((domain, _)) = domain_counts.iter().max_by_key(|(_, count)| *count) {
                (*domain).to_string()
            } else {
                String::new()
            };
        errors.push(format!(
            "domain {worst_domain} exceeds 35% share ({:.1}%)",
            max_domain_share * 100.0
        ));
    }

    for challenge in challenges {
        if challenge
            .route_metadata
            .iter()
            .any(|route| route.model_decisions.is_empty())
        {
            errors.push(format!(
                "{} is missing model_decisions",
                challenge.challenge_hash
            ));
        }
        if challenge
            .artifact_provenance
            .as_ref()
            .map(|provenance| {
                provenance.fixture_provenance
                    || provenance.answer_leakage_detected
                    || provenance.agent_mode.as_deref() != Some("live_jnoccio")
            })
            .unwrap_or(false)
        {
            errors.push(format!(
                "{} has invalid artifact provenance",
                challenge.challenge_hash
            ));
        }
    }

    errors
}

pub fn production_acceptance_errors(challenge: &ChallengeRecord) -> Vec<String> {
    let mut errors = Vec::new();
    if challenge.schema_version != PRODUCTION_CHALLENGE_SCHEMA_VERSION {
        errors.push("schema_version is not production v3".to_string());
    }
    if !acceptance_passes(&challenge.acceptance) {
        errors.push("base acceptance gates failed".to_string());
    }
    if challenge.acceptance.auditor_agreement < 0.75 {
        errors.push("focused agreement below 0.75".to_string());
    }
    if challenge.acceptance.focused_correct_rate < 0.90 {
        errors.push("focused correct rate below 0.90".to_string());
    }
    if challenge.acceptance.answerability < 0.90 {
        errors.push("answerability below 0.90".to_string());
    }
    if challenge.acceptance.blind_correct_rate > 0.50 {
        errors.push("saturated blind correct rate above 0.50".to_string());
    }
    if challenge.focused_support_trials.len() < MIN_SUCCESSFUL_VERIFIERS {
        errors.push(format!(
            "fewer than {MIN_SUCCESSFUL_VERIFIERS} focused support trials"
        ));
    }
    if challenge.saturated_blind_trials.len() < MIN_SUCCESSFUL_TESTERS {
        errors.push(format!(
            "fewer than {MIN_SUCCESSFUL_TESTERS} saturated blind trials"
        ));
    }
    if challenge.judge_trials.is_empty() {
        errors.push("missing judge trial".to_string());
    }
    match challenge.acceptance_metrics.as_ref() {
        Some(metrics) => {
            if metrics.saturated_mean_confidence > 0.55 {
                errors.push("saturated mean confidence above 0.55".to_string());
            }
            let recomputed = if challenge.saturated_blind_trials.is_empty() {
                1.0
            } else {
                challenge
                    .saturated_blind_trials
                    .iter()
                    .map(|trial| trial.confidence)
                    .sum::<f64>()
                    / challenge.saturated_blind_trials.len() as f64
            };
            if (metrics.saturated_mean_confidence - recomputed).abs() > 0.000_001 {
                errors.push(
                    "saturated mean confidence does not match calibrated blind trials".to_string(),
                );
            }
        }
        None => errors.push("missing acceptance metrics".to_string()),
    }
    match challenge.source_publication.as_ref() {
        Some(source) => {
            if !source.redistributable {
                errors.push("source publication is not redistributable".to_string());
            }
            if source.license_spdx.eq_ignore_ascii_case("NOASSERTION") {
                errors.push("source publication license is ambiguous".to_string());
            }
            if source.section_hashes.is_empty() {
                errors.push("source publication has no section hashes".to_string());
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
                errors.push("source publication uses fixture URL".to_string());
            }
        }
        None => errors.push("missing source publication".to_string()),
    }
    match challenge.artifact_provenance.as_ref() {
        Some(provenance) => {
            if provenance.fixture_provenance {
                errors.push("fixture provenance is not allowed in production".to_string());
            }
            if provenance
                .agent_mode
                .as_deref()
                .map(|mode| mode != "live_jnoccio")
                .unwrap_or(true)
            {
                errors.push("artifact provenance is not live_jnoccio".to_string());
            }
            if provenance.answer_leakage_detected {
                errors.push("answer leakage detected".to_string());
            }
            if provenance.license_ambiguous {
                errors.push("license ambiguity detected".to_string());
            }
        }
        None => errors.push("missing artifact provenance".to_string()),
    }
    if challenge
        .support
        .iter()
        .any(|support| support.section_hash.trim().is_empty())
    {
        errors.push("support is missing section hashes".to_string());
    }
    for (index, trial) in challenge.focused_support_trials.iter().enumerate() {
        validate_model_trial("focused_support_trials", index, trial, &mut errors);
    }
    for (index, trial) in challenge.saturated_blind_trials.iter().enumerate() {
        validate_model_trial("saturated_blind_trials", index, trial, &mut errors);
    }
    for (index, trial) in challenge.judge_trials.iter().enumerate() {
        validate_route_metadata(
            &format!("judge_trials[{index}].route_metadata"),
            &trial.route_metadata,
            &mut errors,
        );
        if trial.confidence <= 0.0 {
            errors.push(format!("judge_trials[{index}] missing confidence"));
        }
        if trial.rationale_hash.trim().is_empty() {
            errors.push(format!("judge_trials[{index}] missing rationale hash"));
        }
        validate_token_usage(
            &format!("judge_trials[{index}].token_usage"),
            trial.token_usage.prompt_tokens,
            trial.token_usage.completion_tokens,
            trial.token_usage.total_tokens,
            &mut errors,
        );
    }
    for (index, metadata) in challenge.route_metadata.iter().enumerate() {
        validate_route_metadata(&format!("route_metadata[{index}]"), metadata, &mut errors);
    }
    if challenge.route_metadata.is_empty() {
        errors.push("missing top-level route metadata".to_string());
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
