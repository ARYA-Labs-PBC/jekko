use super::{
    sha256_hex, AcceptanceRecord, ChallengeRecord, ContextPack, ModelDecision, ModelTrial,
    PaperRecord, RouteMetadata, MIN_SUCCESSFUL_TESTERS, MIN_SUCCESSFUL_VERIFIERS,
    PRODUCTION_CHALLENGE_SCHEMA_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

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
        let worst_domain = domain_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(domain, _)| (*domain).to_string())
            .unwrap_or_default();
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

fn validate_model_trial(field: &str, index: usize, trial: &ModelTrial, errors: &mut Vec<String>) {
    if trial.agent_id.trim().is_empty() {
        errors.push(format!("{field}[{index}] missing agent_id"));
    }
    if trial.prompt_hash.trim().is_empty() {
        errors.push(format!("{field}[{index}] missing prompt hash"));
    }
    if trial.context_hash.trim().is_empty() {
        errors.push(format!("{field}[{index}] missing context hash"));
    }
    if !(0.0..=1.0).contains(&trial.confidence) {
        errors.push(format!("{field}[{index}] confidence outside [0,1]"));
    }
    validate_route_metadata(
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

fn validate_route_metadata(label: &str, route: &RouteMetadata, errors: &mut Vec<String>) {
    if route.request_id.trim().is_empty() {
        errors.push(format!("{label} missing request_id"));
    }
    if looks_fabricated_request_id(&route.request_id) {
        errors.push(format!("{label} request_id looks fabricated"));
    }
    if route.provider.trim().is_empty() {
        errors.push(format!("{label} missing provider"));
    }
    if route.model.trim().is_empty() {
        errors.push(format!("{label} missing model"));
    }
    if route
        .route_mode
        .as_deref()
        .map(str::trim)
        .map(str::is_empty)
        .unwrap_or(false)
    {
        errors.push(format!("{label} has empty route_mode"));
    }
    if let Some(confidence) = route.route_confidence {
        if !(0.0..=1.0).contains(&confidence) {
            errors.push(format!("{label} route_confidence outside [0,1]"));
        }
    } else {
        errors.push(format!("{label} missing route_confidence"));
    }
    if route.primary_model_id.is_none() {
        errors.push(format!("{label} missing primary_model_id"));
    }
    if route.backup_model_ids.is_empty() {
        errors.push(format!("{label} missing backup_model_ids"));
    }
    if route.route_mode.as_deref() == Some("fusion")
        && route
            .fusion_model_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        errors.push(format!("{label} fusion route missing fusion_model_id"));
    }
    if route.winner_model_id.is_none() {
        errors.push(format!("{label} missing winner_model_id"));
    }
    if route.prompt_hash.as_deref().unwrap_or("").trim().is_empty() {
        errors.push(format!("{label} missing prompt_hash"));
    }
    if route
        .context_hash
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        errors.push(format!("{label} missing context_hash"));
    }
    if route
        .receipts_hash
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        errors.push(format!("{label} missing receipts_hash"));
    }
    if route
        .model_decisions_hash
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        errors.push(format!("{label} missing model_decisions_hash"));
    }
    if route.model_decisions.is_empty() {
        errors.push(format!("{label} missing model_decisions"));
    } else {
        validate_model_decisions(
            label,
            &route.model_decisions,
            route.model_decisions_hash.as_deref(),
            errors,
        );
    }
    if route.token_usage.is_none() {
        errors.push(format!("{label} missing token_usage"));
    } else if let Some(usage) = route.token_usage.as_ref() {
        validate_token_usage(
            &format!("{label}.token_usage"),
            usage.prompt_tokens,
            usage.completion_tokens,
            usage.total_tokens,
            errors,
        );
    }
}

fn validate_token_usage(
    label: &str,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    errors: &mut Vec<String>,
) {
    if prompt_tokens == 0 || completion_tokens == 0 || total_tokens == 0 {
        errors.push(format!("{label} missing token usage"));
    }
    if total_tokens != prompt_tokens + completion_tokens {
        errors.push(format!(
            "{label} total_tokens does not match prompt + completion"
        ));
    }
}

fn validate_model_decisions(
    label: &str,
    decisions: &[ModelDecision],
    expected_hash: Option<&str>,
    errors: &mut Vec<String>,
) {
    let mut selected = 0usize;
    for (index, decision) in decisions.iter().enumerate() {
        if decision.model_id.trim().is_empty() {
            errors.push(format!("{label}.model_decisions[{index}] missing model_id"));
        }
        if !decision.configured_score.is_finite() || decision.configured_score < 0.0 {
            errors.push(format!(
                "{label}.model_decisions[{index}] invalid configured_score"
            ));
        }
        if !decision.selection_score.is_finite() || decision.selection_score < 0.0 {
            errors.push(format!(
                "{label}.model_decisions[{index}] invalid selection_score"
            ));
        }
        if decision.status.trim().is_empty() {
            errors.push(format!("{label}.model_decisions[{index}] missing status"));
        }
        if decision
            .output_hash
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            errors.push(format!(
                "{label}.model_decisions[{index}] missing output_hash"
            ));
        }
        if decision.token_usage.prompt_tokens == 0
            || decision.token_usage.completion_tokens == 0
            || decision.token_usage.total_tokens == 0
        {
            errors.push(format!(
                "{label}.model_decisions[{index}] missing token usage"
            ));
        }
        if decision.token_usage.total_tokens
            != decision.token_usage.prompt_tokens + decision.token_usage.completion_tokens
        {
            errors.push(format!(
                "{label}.model_decisions[{index}] token usage total mismatch"
            ));
        }
        if decision.selected {
            selected += 1;
        }
    }
    if selected == 0 {
        errors.push(format!("{label} has no selected model decision"));
    }
    if selected > 1 {
        errors.push(format!("{label} has multiple selected model decisions"));
    }
    if let Some(expected_hash) = expected_hash {
        let computed = match serde_json::to_vec(decisions) {
            Ok(json) => sha256_hex(&json),
            Err(err) => {
                errors.push(format!(
                    "{label} failed to serialize model_decisions: {err}"
                ));
                return;
            }
        };
        if computed != expected_hash {
            errors.push(format!("{label} model_decisions_hash mismatch"));
        }
    }
}

fn looks_fabricated_request_id(request_id: &str) -> bool {
    let request_id = request_id.trim();
    if request_id.is_empty() {
        return true;
    }
    let lower = request_id.to_ascii_lowercase();
    lower.starts_with("request-")
        || lower.starts_with("fixture-")
        || lower.starts_with("mock")
        || lower.starts_with("deterministic-")
        || lower.starts_with("seed-")
        || lower.starts_with("test-")
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

pub fn write_json_pretty<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    fs::write(path, format!("{json}\n")).map_err(|err| format!("write {}: {err}", path.display()))
}

pub fn read_json<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("parse {}: {err}", path.display()))
}

pub fn read_challenges(root: &Path) -> Result<Vec<ChallengeRecord>, String> {
    let challenge_root = root.join("challenges");
    let mut paths = Vec::new();
    collect_json_files(&challenge_root, &mut paths)?;
    let mut out = Vec::new();
    for path in paths {
        if path.file_name().and_then(|name| name.to_str()) == Some("manifest.json") {
            continue;
        }
        out.push(read_json(&path)?);
    }
    Ok(out)
}

pub fn read_papers(root: &Path) -> Result<Vec<PaperRecord>, String> {
    let paper_root = root.join("papers");
    let mut paths = Vec::new();
    collect_json_files(&paper_root, &mut paths)?;
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
