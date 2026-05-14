use super::model::{
    stable_challenge_hash, stable_section_hash, BankValidation, ContextPack, ModelDecision,
    ModelTrial, PaperChallenge, PaperRecord, RouteMetadata, PRODUCTION_CHALLENGE_SCHEMA_VERSION,
    PRODUCTION_MANIFEST_SCHEMA_VERSION,
};
use super::parse::{collect_json_files, read_challenges, read_paper};
use crate::qbank_hash::sha256_hex;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::Path;

const MIN_SUCCESSFUL_VERIFIERS: usize = 3;
const MIN_SUCCESSFUL_TESTERS: usize = 3;

pub fn validate_bank(
    root: &Path,
    allow_empty: bool,
    top_n: usize,
    min_required_accepted: usize,
) -> Result<BankValidation, String> {
    let mut result = BankValidation::default();
    result.min_required_accepted = min_required_accepted;
    let mut paper_paths = Vec::new();
    collect_json_files(&root.join("papers"), &mut paper_paths)?;
    let mut challenge_paths = Vec::new();
    collect_json_files(&root.join("challenges"), &mut challenge_paths)?;
    let mut rejected_paths = Vec::new();
    collect_json_files(&root.join("rejected"), &mut rejected_paths)?;
    let allow_fixture_qbank = env::var("memory_benchmark_dev_qbank").ok().as_deref() == Some("1");
    result.strict_production = !allow_fixture_qbank;
    result.manifest_schema = read_manifest_schema(root).unwrap_or_default();

    let mut seen_publications = BTreeSet::new();
    let mut papers_by_hash = BTreeMap::new();
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
                if !allow_fixture_qbank && paper_has_fixture_provenance(&paper) {
                    result
                        .errors
                        .push(format!("fixture provenance in paper {}", path.display()));
                }
                if !allow_fixture_qbank && paper.sections.is_empty() {
                    result.errors.push(format!(
                        "paper {} has no full-text sections",
                        path.display()
                    ));
                }
                for section in &paper.sections {
                    if !allow_fixture_qbank && section.text.trim().is_empty() {
                        result.errors.push(format!(
                            "{} section {} has empty full text",
                            path.display(),
                            section.section_id
                        ));
                    }
                    if !allow_fixture_qbank && section.section_hash.trim().is_empty() {
                        result.errors.push(format!(
                            "{} section {} missing section_hash",
                            path.display(),
                            section.section_id
                        ));
                    }
                    let expected = stable_section_hash(&section.text);
                    if !section.section_hash.is_empty() && section.section_hash != expected {
                        result.errors.push(format!(
                            "{} section {} hash mismatch",
                            path.display(),
                            section.section_id
                        ));
                    }
                }
                papers_by_hash.insert(paper.publication_hash.clone(), paper);
            }
            Err(err) => result.errors.push(err),
        }
    }

    let mut accepted = Vec::new();
    let mut seen_challenges = BTreeSet::new();
    let mut publication_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut domain_counts: BTreeMap<String, usize> = BTreeMap::new();
    for path in &challenge_paths {
        match read_challenges(path) {
            Ok(challenges) => {
                for challenge in challenges {
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
                    if let Err(err) =
                        validate_paper_presence(&challenge, &papers_by_hash, allow_fixture_qbank)
                    {
                        result.errors.push(format!("{}: {err}", path.display()));
                    }
                    if !allow_fixture_qbank {
                        for err in validate_production_challenge(&challenge) {
                            result.errors.push(format!("{}: {err}", path.display()));
                        }
                    }
                    *publication_counts
                        .entry(challenge.publication_hash.clone())
                        .or_default() += 1;
                    *domain_counts.entry(challenge.domain.clone()).or_default() += 1;
                    accepted.push(challenge);
                }
            }
            Err(err) => result.errors.push(err),
        }
    }
    result.accepted_challenges = accepted.len();
    result.rejected_challenges = rejected_paths.len();
    result.unique_publications = publication_counts.len();
    result.distinct_domains = domain_counts.len();
    result.top_selected = accepted.len().min(top_n);
    result.max_publication_share = publication_counts.values().copied().max().unwrap_or(0) as f32
        / accepted.len().max(1) as f32;
    result.max_domain_share =
        domain_counts.values().copied().max().unwrap_or(0) as f32 / accepted.len().max(1) as f32;
    result.source_diversity = if accepted.is_empty() {
        0.0
    } else {
        result.unique_publications as f32 / accepted.len() as f32
    };
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
    let bank_is_empty =
        paper_paths.is_empty() && challenge_paths.is_empty() && rejected_paths.is_empty();
    if allow_fixture_qbank {
        result
            .warnings
            .push("dev_only fixture qbank mode enabled".to_string());
    } else if !(allow_empty && bank_is_empty && result.accepted_challenges == 0) {
        if result.manifest_schema != PRODUCTION_MANIFEST_SCHEMA_VERSION {
            result.errors.push(format!(
                "manifest schema is not production v3: {}",
                if result.manifest_schema.is_empty() {
                    "missing"
                } else {
                    result.manifest_schema.as_str()
                }
            ));
        }
        if result.accepted_challenges < min_required_accepted {
            result.errors.push(format!(
                "production bank has {} accepted challenges; need at least {}",
                result.accepted_challenges, min_required_accepted
            ));
        }
        let required_unique_publications = ((min_required_accepted as f32) * 0.34).ceil() as usize;
        if result.unique_publications < required_unique_publications {
            result.errors.push(format!(
                "production bank has {} unique publications; need at least {}",
                result.unique_publications, required_unique_publications
            ));
        }
        for (publication, count) in &publication_counts {
            if *count > 3 {
                result.errors.push(format!(
                    "publication {} exceeds 3 accepted challenges ({})",
                    publication, count
                ));
            }
        }
        if min_required_accepted >= 10
            && result.accepted_challenges >= 10
            && result.max_domain_share > 0.35
        {
            let mut worst = String::new();
            let mut worst_count = 0usize;
            for (domain, count) in &domain_counts {
                if *count > worst_count {
                    worst = domain.clone();
                    worst_count = *count;
                }
            }
            result.errors.push(format!(
                "domain {} exceeds 35% share ({:.1}%)",
                worst,
                result.max_domain_share * 100.0
            ));
        }
    }
    result.qbank_trusted = !allow_fixture_qbank
        && result.accepted_challenges >= min_required_accepted
        && result.manifest_schema == PRODUCTION_MANIFEST_SCHEMA_VERSION
        && result.errors.is_empty();
    Ok(result)
}

fn validate_paper_presence(
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

fn validate_production_challenge(challenge: &PaperChallenge) -> Vec<String> {
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
    if challenge.focused_support_trials.len() < MIN_SUCCESSFUL_VERIFIERS {
        errors.push("missing focused support trials".to_string());
    }
    if challenge.saturated_blind_trials.len() < MIN_SUCCESSFUL_TESTERS {
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

fn validate_trial(field: &str, index: usize, trial: &ModelTrial, errors: &mut Vec<String>) {
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

fn validate_route(label: &str, route: &RouteMetadata, errors: &mut Vec<String>) {
    if route.request_id.trim().is_empty() {
        errors.push(format!("{label} missing request_id"));
    }
    if looks_fabricated_request_id(&route.request_id) {
        errors.push(format!("{label} request_id looks fabricated"));
    }
    if route.provider.trim().is_empty() || route.model.trim().is_empty() {
        errors.push(format!("{label} missing provider/model"));
    }
    if route.route_mode.as_deref().unwrap_or("").trim().is_empty() {
        errors.push(format!("{label} missing route_mode"));
    }
    if route
        .route_confidence
        .map(|value| !(0.0..=1.0).contains(&value))
        .unwrap_or(true)
    {
        errors.push(format!("{label} missing route_confidence"));
    }
    if route
        .primary_model_id
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
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
    if route
        .winner_model_id
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
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
    match route.token_usage.as_ref() {
        Some(usage) => validate_token_usage(
            &format!("{label}.token_usage"),
            usage.prompt_tokens,
            usage.completion_tokens,
            usage.total_tokens,
            errors,
        ),
        None => errors.push(format!("{label} missing token_usage")),
    }
}

fn validate_token_usage(
    label: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
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
        if !(0.0..=1.0).contains(&decision.configured_score) {
            errors.push(format!(
                "{label}.model_decisions[{index}] invalid configured_score"
            ));
        }
        if !(0.0..=1.0).contains(&decision.selection_score) {
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
        if decision.latency_ms == 0 {
            errors.push(format!("{label}.model_decisions[{index}] missing latency"));
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
        match serde_json::to_vec(decisions) {
            Ok(json) => {
                let computed = sha256_hex(&json);
                if computed != expected_hash {
                    errors.push(format!("{label} model_decisions_hash mismatch"));
                }
            }
            Err(err) => errors.push(format!(
                "{label} failed to serialize model_decisions: {err}"
            )),
        }
    }
}

fn looks_fabricated_request_id(request_id: &str) -> bool {
    let lower = request_id.trim().to_ascii_lowercase();
    lower.is_empty()
        || lower.starts_with("request-")
        || lower.starts_with("fixture-")
        || lower.starts_with("mock")
        || lower.starts_with("deterministic-")
        || lower.starts_with("seed-")
        || lower.starts_with("test-")
}

fn paper_has_fixture_provenance(paper: &PaperRecord) -> bool {
    paper.title.to_ascii_lowercase().contains("fixture")
        || paper.title.to_ascii_lowercase().contains("generated")
        || paper
            .dedupe_keys
            .iter()
            .chain(paper.source_ids.iter())
            .any(|value| {
                let lower = value.to_ascii_lowercase();
                lower.contains("fixture") || lower.contains("generated")
            })
        || paper
            .source_url
            .as_deref()
            .map(|url| {
                url.contains("example.invalid") || url.contains("qbank-smoke.openaccess.local")
            })
            .unwrap_or(false)
        || paper.retrieval_kinds.iter().any(|kind| {
            kind == "seed_fixture_bank" || kind == "generated" || kind.contains("smoke")
        })
}

fn read_manifest_schema(root: &Path) -> Result<String, String> {
    let path = root.join("manifests/latest.json");
    let text =
        std::fs::read_to_string(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let parsed =
        crate::json::parse(&text).map_err(|err| format!("parse {}: {err}", path.display()))?;
    let obj = match parsed {
        crate::json::Json::Object(obj) => obj,
        _ => return Err(format!("{}: manifest must be an object", path.display())),
    };
    Ok(match obj.get("schema_version") {
        Some(crate::json::Json::Str(value)) => value.clone(),
        _ => String::new(),
    })
}

fn validate_challenge_hash(challenge: &PaperChallenge) -> Result<(), String> {
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

fn challenge_order_plain(a: &PaperChallenge, b: &PaperChallenge) -> std::cmp::Ordering {
    b.difficulty_score
        .total_cmp(&a.difficulty_score)
        .then(b.focused_correct_rate.total_cmp(&a.focused_correct_rate))
        .then(a.blind_correct_rate.total_cmp(&b.blind_correct_rate))
        .then(a.publication_hash.cmp(&b.publication_hash))
        .then(a.challenge_hash.cmp(&b.challenge_hash))
}
