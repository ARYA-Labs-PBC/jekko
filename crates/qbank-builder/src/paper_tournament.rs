use super::agent_json::{
    parse_agent_json, validate_generator_output, validate_grading_output, validate_testing_output,
    validate_verification_output,
};
use super::{
    canonical_paper_text, canonicalize_paper, collect_json_files, ensure_bank_layout,
    finalize_challenge, manifest_hash, pack_context, read_papers, sha256_hex, token_estimate,
    validate_full_text_paper, write_json_pretty, AcceptanceMetrics, AcceptanceRecord,
    AgentCallReceipt, AgentFailure, AnswerAttempt, AnswerKey, ArtifactProvenance, ChallengeRecord,
    ContextPackProvenance, FinalPaperChallengeArtifact, GeneratorAgentOutput, GeneratorTrial,
    GradingAgentOutput, GradingTrial, JudgeTrial, LicenseRecord, ModelDecision, ModelTrial,
    PaperRecord, PaperSection, RouteMetadata, SupportQuote, SupportRef, TestingAgentOutput,
    TestingTrial, TokenUsage, VerificationAgentOutput, VerificationTrial,
    FINAL_PAPER_CHALLENGE_SCHEMA_VERSION, HARD_MAX_TESTER_CORRECT_RATE, MIN_SUCCESSFUL_GRADERS,
    MIN_SUCCESSFUL_TESTERS, MIN_SUCCESSFUL_VERIFIERS, PAPER_SCHEMA_VERSION,
    PAPER_TOURNAMENT_SCHEMA_VERSION, PRODUCTION_CHALLENGE_SCHEMA_VERSION,
    PRODUCTION_MANIFEST_SCHEMA_VERSION, QBANK_REDUCER_VERSION,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum AgentRunnerMode {
    Mock,
    Jnoccio,
}

impl AgentRunnerMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mock => "mock_smoke",
            Self::Jnoccio => "live_jnoccio",
        }
    }

    pub fn is_mock(&self) -> bool {
        matches!(self, Self::Mock)
    }
}

#[derive(Debug, Clone)]
pub struct BuildPaperTournamentConfig {
    pub bank: PathBuf,
    pub run_root: PathBuf,
    pub target_accepted: usize,
    pub candidate_papers: usize,
    pub generators: usize,
    pub verifiers: usize,
    pub testers: usize,
    pub graders: usize,
    pub distractor_papers: usize,
    pub strict_production: bool,
    pub agent_runner: AgentRunnerMode,
    pub jnoccio_base_url: Option<String>,
    pub jnoccio_model: Option<String>,
    pub jnoccio_max_output_tokens: u64,
    pub jnoccio_request_timeout_seconds: u64,
    pub paper_timeout_seconds: u64,
    pub phase_retries: usize,
    pub progress_jsonl: Option<PathBuf>,
    pub candidate_manifest: Option<PathBuf>,
    pub resume: bool,
    pub allow_mock_smoke: bool,
    pub mock_agents: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct BuildPaperTournamentSummary {
    pub generated: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub failed: usize,
    pub run_root: PathBuf,
    pub sample_accepted_artifact: Option<PathBuf>,
    pub sample_rejected_artifact: Option<PathBuf>,
    pub reduce_report: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct SupportQuoteCandidate {
    pub(crate) id: String,
    pub(crate) section_id: String,
    pub(crate) section_hash: String,
    pub(crate) section_title: String,
    pub(crate) quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeneratorSelectionOutput {
    question: String,
    answer: String,
    difficulty_rationale: String,
    expected_failure_mode: String,
    support_quote_id: String,
    confidence: u8,
}

pub fn build_paper_tournament(
    config: &BuildPaperTournamentConfig,
) -> Result<BuildPaperTournamentSummary, String> {
    ensure_bank_layout(&config.bank)?;
    if config.strict_production && config.agent_runner.is_mock() {
        return Err(
            "strict production tournament requires --agent-runner jnoccio; mock smoke output is never production trusted"
                .to_string(),
        );
    }
    if config.agent_runner.is_mock() && !config.allow_mock_smoke {
        return Err("--agent-runner mock requires --allow-mock-smoke".to_string());
    }
    if let Some(path) = config.mock_agents.as_ref() {
        if !path.exists() {
            return Err(format!(
                "--mock-agents path does not exist: {}; use --agent-runner mock --allow-mock-smoke for built-in deterministic smoke data",
                path.display()
            ));
        }
    }
    if matches!(config.agent_runner, AgentRunnerMode::Jnoccio) {
        config
            .jnoccio_base_url
            .as_deref()
            .ok_or("--agent-runner jnoccio requires --jnoccio-base-url")?;
        if !config.strict_production {
            return Err(
                "--agent-runner jnoccio is only supported with --strict-production".to_string(),
            );
        }
        write_jnoccio_preflight_report(config)?;
    }
    let mut papers = read_papers(&config.bank)?;
    if config.agent_runner.is_mock() && papers.len() < config.target_accepted {
        for index in papers.len()..config.target_accepted {
            let paper = canonicalize_paper(smoke_paper(index))?;
            let path = config
                .bank
                .join("papers")
                .join(format!("{}.json", paper.publication_hash));
            if !path.exists() {
                write_json_pretty(&path, &paper)?;
            }
            papers.push(paper);
        }
    }
    if let Some(manifest_path) = config.candidate_manifest.as_ref() {
        papers = filter_papers_by_candidate_manifest(papers, manifest_path)?;
    }
    if papers.is_empty() {
        return Err(format!(
            "no paper JSON files found under {}",
            config.bank.join("papers").display()
        ));
    }
    papers.sort_by(|left, right| left.publication_hash.cmp(&right.publication_hash));
    if config.strict_production && matches!(config.agent_runner, AgentRunnerMode::Jnoccio) {
        papers.retain(|paper| {
            validate_full_text_paper(paper, true).is_ok()
                && paper_quality_allowed(paper)
                && !support_quote_candidates(paper).is_empty()
        });
        if papers.is_empty() {
            return Err(format!(
                "no strict-production full-text candidate papers found under {}",
                config.bank.join("papers").display()
            ));
        }
    }

    let mut generated = 0usize;
    let mut accepted = if config.resume {
        existing_accepted_count(&config.bank)?
    } else {
        0
    };
    let mut rejected = 0usize;
    let mut failed = 0usize;
    let mut outputs = Vec::new();
    let mut sample_accepted_artifact = None;
    let mut sample_rejected_artifact = None;
    let limit = config.candidate_papers.max(1).min(papers.len());

    for paper in papers.iter().take(limit) {
        if accepted >= config.target_accepted {
            break;
        }
        if config.resume && paper_already_attempted(&config.run_root, paper) {
            outputs.push(json!({
                "paper_hash": paper.publication_hash,
                "accepted": false,
                "resumed": true,
                "skipped": "existing_trial_artifact"
            }));
            continue;
        }
        generated += 1;
        let non_production = config.agent_runner.is_mock();
        let result = run_single_paper(config, paper, &papers, non_production);
        match result {
            Ok(TournamentWriteResult {
                challenge,
                artifact_path,
                challenge_path,
                accepted: challenge_accepted,
                errors,
            }) => {
                if challenge_accepted {
                    accepted += 1;
                    if sample_accepted_artifact.is_none() {
                        sample_accepted_artifact = Some(artifact_path.clone());
                    }
                } else {
                    rejected += 1;
                    if sample_rejected_artifact.is_none() {
                        sample_rejected_artifact = Some(artifact_path.clone());
                    }
                }
                outputs.push(json!({
                    "paper_hash": paper.publication_hash,
                    "challenge_hash": challenge.challenge_hash,
                    "accepted": challenge_accepted,
                    "artifact": artifact_path.display().to_string(),
                    "challenge": challenge_path.display().to_string(),
                    "route_summary": route_summary_for_challenge(&challenge),
                    "errors": errors,
                }));
            }
            Err(err) => {
                failed += 1;
                outputs.push(json!({
                    "paper_hash": paper.publication_hash,
                    "accepted": false,
                    "errors": [err],
                }));
            }
        }
    }

    let reduce_report = config.run_root.join("reports/qbank-reduce.json");
    write_json_pretty(
        &reduce_report,
        &json!({
            "schema_version": PAPER_TOURNAMENT_SCHEMA_VERSION,
            "bank": config.bank.display().to_string(),
            "run_root": config.run_root.display().to_string(),
            "target_accepted": config.target_accepted,
            "candidate_papers": config.candidate_papers,
            "agent_runner": config.agent_runner.as_str(),
            "mock_agents": config.mock_agents.as_ref().map(|path| path.display().to_string()),
            "allow_mock_smoke": config.allow_mock_smoke,
            "jnoccio_base_url": config.jnoccio_base_url.as_deref(),
            "jnoccio_model": configured_jnoccio_model(config),
            "jnoccio_max_output_tokens": config.jnoccio_max_output_tokens,
            "jnoccio_request_timeout_seconds": config.jnoccio_request_timeout_seconds,
            "paper_timeout_seconds": config.paper_timeout_seconds,
            "phase_retries": config.phase_retries,
            "progress_jsonl": progress_jsonl_path(config).display().to_string(),
            "candidate_manifest": config.candidate_manifest.as_ref().map(|path| path.display().to_string()),
            "resume": config.resume,
            "strict_production": config.strict_production,
            "generated": generated,
            "accepted": accepted,
            "rejected": rejected,
            "failed": failed,
            "outputs": outputs,
        }),
    )?;
    write_manifest(
        &config.bank,
        accepted,
        config.target_accepted,
        config.strict_production,
    )?;

    if accepted < config.target_accepted {
        return Err(format!(
            "paper tournament accepted {accepted} challenges; target is {}",
            config.target_accepted
        ));
    }

    Ok(BuildPaperTournamentSummary {
        generated,
        accepted,
        rejected,
        failed,
        run_root: config.run_root.clone(),
        sample_accepted_artifact,
        sample_rejected_artifact,
        reduce_report,
    })
}

fn filter_papers_by_candidate_manifest(
    papers: Vec<PaperRecord>,
    manifest_path: &Path,
) -> Result<Vec<PaperRecord>, String> {
    let text = std::fs::read_to_string(manifest_path)
        .map_err(|err| format!("read candidate manifest {}: {err}", manifest_path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|err| {
        format!(
            "parse candidate manifest {}: {err}",
            manifest_path.display()
        )
    })?;
    let hashes = value
        .get("papers")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            format!(
                "candidate manifest {} missing papers array",
                manifest_path.display()
            )
        })?
        .iter()
        .filter_map(|row| row.get("publication_hash").and_then(|value| value.as_str()))
        .map(str::to_string)
        .collect::<Vec<_>>();
    if hashes.is_empty() {
        return Err(format!(
            "candidate manifest {} contains no publication_hash entries",
            manifest_path.display()
        ));
    }
    let by_hash = papers
        .into_iter()
        .map(|paper| (paper.publication_hash.clone(), paper))
        .collect::<BTreeMap<_, _>>();
    let mut out = Vec::new();
    let mut missing = Vec::new();
    for hash in hashes {
        match by_hash.get(&hash) {
            Some(paper) => out.push(paper.clone()),
            None => missing.push(hash),
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "candidate manifest references {} papers missing from bank: {}",
            missing.len(),
            missing.into_iter().take(5).collect::<Vec<_>>().join(", ")
        ));
    }
    Ok(out)
}

fn existing_accepted_count(bank: &Path) -> Result<usize, String> {
    let mut files = Vec::new();
    collect_json_files(&bank.join("challenges"), &mut files)?;
    Ok(files
        .into_iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("manifest.json"))
        .count())
}

fn paper_already_attempted(run_root: &Path, paper: &PaperRecord) -> bool {
    run_root
        .join("trials")
        .join(&paper.publication_hash)
        .read_dir()
        .map(|mut entries| {
            entries.any(|entry| {
                entry
                    .map(|entry| entry.path().join("final.json").exists())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn write_manifest(
    bank: &Path,
    accepted: usize,
    target_accepted: usize,
    strict_production: bool,
) -> Result<(), String> {
    let mut files = Vec::new();
    collect_json_files(&bank.join("papers"), &mut files)?;
    collect_json_files(&bank.join("challenges"), &mut files)?;
    let hash = manifest_hash(&files)?;
    write_json_pretty(
        &bank.join("manifests").join("latest.json"),
        &json!({
            "schema_version": if strict_production { PRODUCTION_MANIFEST_SCHEMA_VERSION } else { "opencode-qbank-manifest-v1" },
            "strict_production": strict_production,
            "accepted_challenges": accepted,
            "min_required_accepted": target_accepted,
            "unique_publications": accepted,
            "manifest_hash": hash,
        }),
    )
}

fn write_jnoccio_preflight_report(config: &BuildPaperTournamentConfig) -> Result<(), String> {
    let base_url = config
        .jnoccio_base_url
        .as_deref()
        .ok_or("--agent-runner jnoccio requires --jnoccio-base-url")?
        .trim()
        .trim_end_matches('/');
    let requested_model = configured_jnoccio_model(config);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|err| format!("build jnoccio preflight http client: {err}"))?;
    let status_url = format!("{base_url}/v1/jnoccio/status");
    let status_text = fetch_required_text(&client, &status_url, "jnoccio status")?;
    let status_json: serde_json::Value = serde_json::from_str(&status_text)
        .map_err(|err| format!("jnoccio status response is not JSON: {err}"))?;
    let metrics_url = format!("{base_url}/v1/jnoccio/metrics");
    let metrics_result = fetch_optional_json(&client, &metrics_url);
    let reports_dir = config.run_root.join("reports");
    let status_path = reports_dir.join("jnoccio-status.json");
    let metrics_path = reports_dir.join("jnoccio-metrics.json");
    let preflight_path = reports_dir.join("jnoccio-preflight.json");
    write_json_pretty(&status_path, &status_json)?;

    let (metrics_status, metrics_hash) = match metrics_result {
        Ok(Some(metrics_json)) => {
            let metrics_text = serde_json::to_string(&metrics_json)
                .map_err(|err| format!("serialize jnoccio metrics: {err}"))?;
            write_json_pretty(&metrics_path, &metrics_json)?;
            (
                "captured".to_string(),
                Some(sha256_hex(metrics_text.as_bytes())),
            )
        }
        Ok(None) => ("not_available".to_string(), None),
        Err(err) => (format!("error: {err}"), None),
    };

    let model_catalog = summarize_jnoccio_models(&status_json);
    let gateway_visible_model = status_json
        .pointer("/health/visible_model")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    if let Some(visible_model) = gateway_visible_model.as_deref() {
        if !model_matches_gateway_visible_model(visible_model, &requested_model) {
            return Err(format!(
                "--jnoccio-model {requested_model:?} is not accepted by this Jnoccio gateway; current visible chat model is {visible_model:?}. Use the visible model or restart/reconfigure Jnoccio before the run."
            ));
        }
    }
    let requested_model_visible = model_catalog.iter().any(|entry| {
        ["id", "model_id", "visible_id", "name", "model"]
            .iter()
            .filter_map(|key| entry.get(*key).and_then(|value| value.as_str()))
            .any(|value| value == requested_model)
    }) || gateway_visible_model.as_deref()
        == Some(requested_model.as_str());
    let warnings = if requested_model_visible || model_catalog.is_empty() {
        Vec::<String>::new()
    } else {
        vec![format!(
            "requested model {requested_model:?} was not found verbatim in the status model catalog"
        )]
    };

    write_json_pretty(
        &preflight_path,
        &json!({
            "schema_version": "opencode-qbank-jnoccio-preflight-v1",
            "base_url": base_url,
            "status_url": status_url,
            "metrics_url": metrics_url,
            "requested_model": requested_model,
            "phase_max_output_tokens": config.jnoccio_max_output_tokens,
            "context_policy": {
                "safe_window_tokens": 128000,
                "target_fill_ratio": 0.82,
                "output_reserve_tokens": config.jnoccio_max_output_tokens
            },
            "status_hash": sha256_hex(status_text.as_bytes()),
            "gateway_visible_model": gateway_visible_model,
            "health": status_json.get("health").cloned(),
            "metrics_status": metrics_status,
            "metrics_hash": metrics_hash,
            "status_path": status_path.display().to_string(),
            "metrics_path": metrics_path.display().to_string(),
            "model_catalog_count": model_catalog.len(),
            "model_catalog": model_catalog,
            "warnings": warnings
        }),
    )
}

fn configured_jnoccio_model(config: &BuildPaperTournamentConfig) -> String {
    config
        .jnoccio_model
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(|| std::env::var("QBANK_JNOCCIO_MODEL").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "jnoccio/jnoccio-fusion".to_string())
}

fn model_matches_gateway_visible_model(visible_model: &str, requested_model: &str) -> bool {
    requested_model == visible_model
        || requested_model
            .strip_prefix("jnoccio/")
            .map(|suffix| suffix == visible_model)
            .unwrap_or(false)
        || visible_model
            .strip_prefix("jnoccio/")
            .map(|suffix| suffix == requested_model)
            .unwrap_or(false)
        || visible_model
            == requested_model
                .rsplit('/')
                .next()
                .unwrap_or(requested_model)
}

fn fetch_required_text(
    client: &reqwest::blocking::Client,
    url: &str,
    label: &str,
) -> Result<String, String> {
    let response = client
        .get(url)
        .send()
        .map_err(|err| format!("{label} request failed: {err}"))?;
    let status = response.status();
    let text = response
        .text()
        .map_err(|err| format!("{label} response read failed: {err}"))?;
    if !status.is_success() {
        return Err(format!("{label} returned HTTP {}: {text}", status.as_u16()));
    }
    Ok(text)
}

fn fetch_optional_json(
    client: &reqwest::blocking::Client,
    url: &str,
) -> Result<Option<serde_json::Value>, String> {
    let response = match client.get(url).send() {
        Ok(response) => response,
        Err(err) => return Err(format!("jnoccio metrics request failed: {err}")),
    };
    let status = response.status();
    let text = response
        .text()
        .map_err(|err| format!("jnoccio metrics response read failed: {err}"))?;
    if status.as_u16() == 404 {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(format!(
            "jnoccio metrics returned HTTP {}: {text}",
            status.as_u16()
        ));
    }
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|err| format!("jnoccio metrics response is not JSON: {err}"))
}

fn summarize_jnoccio_models(status: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut models = BTreeMap::<String, serde_json::Value>::new();
    if let Some(array) = status.get("models").and_then(|value| value.as_array()) {
        for value in array {
            insert_model_summary(&mut models, value);
        }
    }
    collect_model_summaries(status, &mut models);
    models.into_values().collect()
}

fn collect_model_summaries(
    value: &serde_json::Value,
    models: &mut BTreeMap<String, serde_json::Value>,
) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                collect_model_summaries(item, models);
            }
        }
        serde_json::Value::Object(map) => {
            let has_model_identity = ["id", "model_id", "visible_id", "model", "name"]
                .iter()
                .any(|key| map.contains_key(*key));
            let has_capacity = [
                "context_window",
                "context_window_tokens",
                "max_output_tokens",
            ]
            .iter()
            .any(|key| map.contains_key(*key));
            if has_model_identity && has_capacity {
                insert_model_summary(models, value);
            }
            for child in map.values() {
                collect_model_summaries(child, models);
            }
        }
        _ => {}
    }
}

fn insert_model_summary(
    models: &mut BTreeMap<String, serde_json::Value>,
    value: &serde_json::Value,
) {
    let summary = compact_model_summary(value);
    if summary.as_object().map(|object| object.is_empty()) == Some(true) {
        return;
    }
    let key = ["visible_id", "id", "model_id", "name", "model"]
        .iter()
        .filter_map(|field| summary.get(*field).and_then(|value| value.as_str()))
        .next()
        .map(str::to_string)
        .unwrap_or_else(|| {
            serde_json::to_string(&summary)
                .unwrap_or_else(|_| format!("model-{}", models.len() + 1))
        });
    models.entry(key).or_insert(summary);
}

fn compact_model_summary(value: &serde_json::Value) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    for key in [
        "id",
        "model_id",
        "visible_id",
        "name",
        "provider",
        "model",
        "display_name",
        "status",
        "enabled",
        "healthy",
        "keyed",
        "context_window",
        "context_window_tokens",
        "max_output_tokens",
        "max_tokens",
        "roles",
        "route_mode",
        "cooldown_until",
        "disabled_reason",
    ] {
        if let Some(field) = value.get(key).filter(|field| !field.is_null()) {
            object.insert(key.to_string(), field.clone());
        }
    }
    serde_json::Value::Object(object)
}

fn route_summary_for_challenge(challenge: &ChallengeRecord) -> serde_json::Value {
    let mut by_model = BTreeMap::<String, usize>::new();
    let mut by_primary = BTreeMap::<String, usize>::new();
    let mut by_winner = BTreeMap::<String, usize>::new();
    let mut by_mode = BTreeMap::<String, usize>::new();
    let mut selected_decisions = BTreeMap::<String, usize>::new();
    let mut decision_counts = BTreeMap::<String, usize>::new();
    let mut max_prompt_tokens = 0u64;
    let mut max_completion_tokens = 0u64;
    let mut max_total_tokens = 0u64;

    for route in &challenge.route_metadata {
        *by_model.entry(route.model.clone()).or_insert(0) += 1;
        if let Some(value) = route.primary_model_id.as_ref() {
            *by_primary.entry(value.clone()).or_insert(0) += 1;
        }
        if let Some(value) = route.winner_model_id.as_ref() {
            *by_winner.entry(value.clone()).or_insert(0) += 1;
        }
        if let Some(value) = route.route_mode.as_ref() {
            *by_mode.entry(value.clone()).or_insert(0) += 1;
        }
        if let Some(usage) = route.token_usage.as_ref() {
            max_prompt_tokens = max_prompt_tokens.max(usage.prompt_tokens);
            max_completion_tokens = max_completion_tokens.max(usage.completion_tokens);
            max_total_tokens = max_total_tokens.max(usage.total_tokens);
        }
        for decision in &route.model_decisions {
            *decision_counts
                .entry(decision.model_id.clone())
                .or_insert(0) += 1;
            if decision.selected {
                *selected_decisions
                    .entry(decision.model_id.clone())
                    .or_insert(0) += 1;
            }
        }
    }

    json!({
        "route_records": challenge.route_metadata.len(),
        "by_model": by_model,
        "by_primary_model": by_primary,
        "by_winner_model": by_winner,
        "by_route_mode": by_mode,
        "model_decisions": decision_counts,
        "selected_model_decisions": selected_decisions,
        "max_prompt_tokens": max_prompt_tokens,
        "max_completion_tokens": max_completion_tokens,
        "max_total_tokens": max_total_tokens
    })
}

struct TournamentWriteResult {
    challenge: ChallengeRecord,
    artifact_path: PathBuf,
    challenge_path: PathBuf,
    accepted: bool,
    errors: Vec<String>,
}

fn run_single_paper(
    config: &BuildPaperTournamentConfig,
    paper: &PaperRecord,
    all_papers: &[PaperRecord],
    non_production: bool,
) -> Result<TournamentWriteResult, String> {
    if matches!(config.agent_runner, AgentRunnerMode::Jnoccio) {
        return run_single_paper_jnoccio(config, paper, all_papers);
    }
    validate_full_text_paper(paper, config.strict_production && !non_production)?;
    let support_section = paper
        .sections
        .iter()
        .find(|section| section.section_id != "abstract" && section.section_id != "source")
        .or_else(|| paper.sections.first())
        .ok_or("paper has no sections")?;
    let quote = first_sentence(&support_section.text);
    let answer = answer_from_quote(&quote);
    let question = format!(
        "In the {} section of '{}', what exact hard recall statement anchors the reported result?",
        support_section.title, paper.title
    );
    let support = vec![super::SupportQuote {
        section_id: support_section.section_id.clone(),
        section_hash: support_section.section_hash.clone(),
        quote: quote.clone(),
        why_it_matters: "It is the minimal source span needed to answer the challenge.".to_string(),
    }];
    let distractor_hashes = select_distractors(paper, all_papers, config.distractor_papers);

    let generation_trials = (0..config.generators.max(1))
        .map(|index| {
            let output = GeneratorAgentOutput {
                question: question.clone(),
                answer: answer.clone(),
                difficulty_rationale: "The answer is a precise paper-local statement that is easy to miss in saturated context.".to_string(),
                expected_failure_mode: "Agents may answer from a distractor paper or paraphrase away the critical constant.".to_string(),
                support: support.clone(),
                confidence: 92,
            };
            let receipt = receipt("generator", index, &question, &paper.publication_hash);
            GeneratorTrial {
                agent_name: format!("generator-{}", index + 1),
                output,
                receipt,
            }
        })
        .collect::<Vec<_>>();
    let mut failures = Vec::new();
    for trial in &generation_trials {
        if let Err(err) = validate_generator_output(&trial.output, paper) {
            failures.push(failure(
                "generation",
                &trial.agent_name,
                err,
                &trial.receipt,
            ));
        }
    }

    let verification_trials = (0..config.verifiers.max(1))
        .map(|index| {
            let output = VerificationAgentOutput {
                accepted: true,
                answer: answer.clone(),
                confidence: 91,
                support_correct: true,
                reason: "The answer is directly supported by the quoted section.".to_string(),
                missing_or_wrong_support: Vec::new(),
            };
            let receipt = receipt("verification", index, &question, &paper.publication_hash);
            VerificationTrial {
                agent_name: format!("verifier-{}", index + 1),
                output,
                receipt,
            }
        })
        .collect::<Vec<_>>();
    for trial in &verification_trials {
        if let Err(err) = validate_verification_output(&trial.output) {
            failures.push(failure(
                "verification",
                &trial.agent_name,
                err,
                &trial.receipt,
            ));
        }
    }

    let testing_prompt = build_testing_prompt(paper, all_papers, &distractor_hashes, &question);
    if testing_prompt.contains("Answer key:")
        || testing_prompt.contains("hard_answer")
        || testing_prompt.contains("verification_trials")
    {
        failures.push(AgentFailure {
            phase: "testing".to_string(),
            agent_name: "prompt-builder".to_string(),
            error: "answer-key metadata leaked into testing prompt".to_string(),
            route_metadata: None,
            raw_output_hash: None,
        });
    }
    let testing_trials = (0..config.testers.max(1))
        .map(|index| {
            let output = TestingAgentOutput {
                answer: format!("The paper reports distractor result {}", index + 1),
                confidence: 37,
                reasoning_summary:
                    "The saturated context made the exact statement hard to isolate.".to_string(),
            };
            let receipt = receipt("testing", index, &testing_prompt, &paper.publication_hash);
            TestingTrial {
                agent_name: format!("tester-{}", index + 1),
                distractor_paper_hashes: distractor_hashes.clone(),
                output,
                receipt,
            }
        })
        .collect::<Vec<_>>();
    for trial in &testing_trials {
        if let Err(err) = validate_testing_output(&trial.output) {
            failures.push(failure("testing", &trial.agent_name, err, &trial.receipt));
        }
    }

    let mut grading_trials = Vec::new();
    for testing_trial in &testing_trials {
        for grader_index in 0..config.graders.max(1) {
            let correct = testing_trial
                .output
                .answer
                .to_ascii_lowercase()
                .contains(&answer.to_ascii_lowercase());
            let output = GradingAgentOutput {
                correct,
                score_0_100: if correct { 96 } else { 12 },
                matched_key_points: if correct {
                    vec![answer.clone()]
                } else {
                    Vec::new()
                },
                missed_key_points: if correct {
                    Vec::new()
                } else {
                    vec![answer.clone()]
                },
                reason: if correct {
                    "The tester answer matches the key.".to_string()
                } else {
                    "The tester answer misses the required paper-local statement.".to_string()
                },
            };
            let receipt = receipt(
                "grading",
                grader_index,
                &testing_trial.output.answer,
                &paper.publication_hash,
            );
            grading_trials.push(GradingTrial {
                agent_name: format!("grader-{}", grader_index + 1),
                testing_agent_name: testing_trial.agent_name.clone(),
                output,
                receipt,
            });
        }
    }
    for trial in &grading_trials {
        if let Err(err) = validate_grading_output(&trial.output) {
            failures.push(failure("grading", &trial.agent_name, err, &trial.receipt));
        }
    }

    let verifier_acceptance = verification_majority(&verification_trials);
    let tester_correct_rate = testing_correct_rate(&testing_trials, &grading_trials);
    let accepted = verifier_acceptance
        && testing_trials.len() >= MIN_SUCCESSFUL_TESTERS
        && tester_correct_rate <= HARD_MAX_TESTER_CORRECT_RATE
        && failures.is_empty();

    let canonical_text = canonical_paper_text(paper, non_production);
    let artifact_provenance = provenance(config, paper);
    let metrics = AcceptanceMetrics {
        focused_agreement: accepted_ratio(&verification_trials),
        focused_correct_rate: accepted_ratio(&verification_trials),
        answerability: accepted_ratio(&verification_trials),
        saturated_blind_correct_rate: tester_correct_rate,
        saturated_mean_confidence: mean_tester_confidence(&testing_trials),
        support_minimality: 1.0,
        distractor_pressure: if distractor_hashes.is_empty() {
            0.0
        } else {
            0.80
        },
    };
    let mut final_artifact = FinalPaperChallengeArtifact {
        schema_version: FINAL_PAPER_CHALLENGE_SCHEMA_VERSION.to_string(),
        paper_hash: paper.publication_hash.clone(),
        paper_content: canonical_text,
        artifact_provenance: Some(artifact_provenance.clone()),
        hard_question: question.clone(),
        hard_answer: answer.clone(),
        hard_agent_name: generation_trials
            .first()
            .map(|trial| trial.agent_name.clone())
            .unwrap_or_else(|| "generator-1".to_string()),
        generation_trials: generation_trials.clone(),
        verification_trials: verification_trials.clone(),
        testing_trials: testing_trials.clone(),
        grading_trials: grading_trials.clone(),
        failures,
        acceptance_metrics: metrics.clone(),
        artifact_hash: String::new(),
    };
    final_artifact.artifact_hash = final_paper_challenge_artifact_hash(&final_artifact)?;

    let challenge = challenge_from_artifact(
        paper,
        support_section,
        &quote,
        &question,
        &answer,
        &generation_trials,
        &verification_trials,
        &testing_trials,
        &grading_trials,
        &metrics,
        accepted,
        config,
    )?;
    let errors = if accepted {
        if non_production {
            Vec::new()
        } else {
            super::production_acceptance_errors(&challenge)
        }
    } else {
        vec!["tournament acceptance gates failed".to_string()]
    };
    let accepted = accepted && errors.is_empty();
    let challenge_dir = if accepted { "challenges" } else { "rejected" };
    let challenge_path = config
        .bank
        .join(challenge_dir)
        .join(format!("{}.json", challenge.challenge_hash));
    let artifact_path = config
        .run_root
        .join("trials")
        .join(&paper.publication_hash)
        .join(&challenge.challenge_hash)
        .join("final.json");
    write_json_pretty(&artifact_path, &final_artifact)?;
    write_json_pretty(&challenge_path, &challenge)?;

    Ok(TournamentWriteResult {
        challenge,
        artifact_path,
        challenge_path,
        accepted,
        errors,
    })
}

fn run_single_paper_jnoccio(
    config: &BuildPaperTournamentConfig,
    paper: &PaperRecord,
    all_papers: &[PaperRecord],
) -> Result<TournamentWriteResult, String> {
    let paper_started = Instant::now();
    validate_full_text_paper(paper, true)?;
    let runner = JnoccioHttpRunner::new(config)?;
    let quote_candidates = support_quote_candidates(paper);
    if quote_candidates.is_empty() {
        return Err("paper has no eligible support quote candidates".to_string());
    }

    let mut failures = Vec::new();
    let mut generation_trials = Vec::new();
    for index in 0..config.generators.max(1) {
        ensure_paper_time_remaining(config, paper, paper_started)?;
        let prompt = generator_prompt(paper, index, &quote_candidates);
        match runner.call_json::<GeneratorSelectionOutput>(
            "generator",
            index,
            &prompt,
            generator_selection_response_schema(),
        ) {
            Ok((selection, receipt)) => {
                let output = generator_output_from_selection(
                    selection,
                    &quote_candidates,
                    &mut failures,
                    &receipt,
                );
                if let Err(err) = validate_generator_output(&output, paper) {
                    failures.push(failure("generation", &receipt.agent_name, err, &receipt));
                }
                generation_trials.push(GeneratorTrial {
                    agent_name: format!("generator-{}", index + 1),
                    output,
                    receipt,
                });
            }
            Err(err) => failures.push(live_call_failure("generation", index, err)),
        }
    }

    let selected_generation = generation_trials
        .iter()
        .find(|trial| validate_generator_output(&trial.output, paper).is_ok())
        .ok_or_else(|| {
            let details = failures
                .iter()
                .map(|failure| {
                    format!(
                        "{}:{}: {}{}",
                        failure.phase,
                        failure.agent_name,
                        failure.error,
                        failure_route_label(failure.route_metadata.as_ref())
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            if details.is_empty() {
                "no valid live generator output".to_string()
            } else {
                format!("no valid live generator output: {details}")
            }
        })?;
    let support_quote = selected_generation
        .output
        .support
        .first()
        .ok_or("live generator output has no support")?;
    let support_section = paper
        .sections
        .iter()
        .find(|section| {
            section.section_id == support_quote.section_id
                && section.section_hash == support_quote.section_hash
        })
        .ok_or("live generator support section is unknown")?;
    let question = selected_generation.output.question.clone();
    let quote = support_quote.quote.trim().to_string();
    if !support_section.text.contains(&quote) {
        return Err("live generator support quote is absent from canonical full text".to_string());
    }
    let answer = quote.clone();
    let distractor_hashes = select_distractors(paper, all_papers, config.distractor_papers);

    let mut verification_trials = Vec::new();
    for index in 0..config.verifiers.max(1) {
        ensure_paper_time_remaining(config, paper, paper_started)?;
        let prompt = verifier_prompt(paper, &question, &answer, &quote);
        match runner.call_json::<VerificationAgentOutput>(
            "verification",
            index,
            &prompt,
            verification_response_schema(),
        ) {
            Ok((output, receipt)) => {
                if let Err(err) = validate_verification_output(&output) {
                    failures.push(failure("verification", &receipt.agent_name, err, &receipt));
                }
                verification_trials.push(VerificationTrial {
                    agent_name: format!("verifier-{}", index + 1),
                    output,
                    receipt,
                });
            }
            Err(err) => failures.push(live_call_failure("verification", index, err)),
        }
    }

    let testing_prompt = build_testing_prompt(paper, all_papers, &distractor_hashes, &question);
    if testing_prompt.contains("Answer key:")
        || testing_prompt.contains("hard_answer")
        || testing_prompt.contains("verification_trials")
    {
        failures.push(AgentFailure {
            phase: "testing".to_string(),
            agent_name: "prompt-builder".to_string(),
            error: "answer-key metadata leaked into testing prompt".to_string(),
            route_metadata: None,
            raw_output_hash: None,
        });
    }
    let mut testing_trials = Vec::new();
    for index in 0..config.testers.max(1) {
        ensure_paper_time_remaining(config, paper, paper_started)?;
        match runner.call_json::<TestingAgentOutput>(
            "testing",
            index,
            &testing_prompt,
            testing_response_schema(),
        ) {
            Ok((output, receipt)) => {
                if let Err(err) = validate_testing_output(&output) {
                    failures.push(failure("testing", &receipt.agent_name, err, &receipt));
                }
                testing_trials.push(TestingTrial {
                    agent_name: format!("tester-{}", index + 1),
                    distractor_paper_hashes: distractor_hashes.clone(),
                    output,
                    receipt,
                });
            }
            Err(err) => failures.push(live_call_failure("testing", index, err)),
        }
    }

    let mut grading_trials = Vec::new();
    for testing_trial in &testing_trials {
        for grader_index in 0..config.graders.max(1) {
            ensure_paper_time_remaining(config, paper, paper_started)?;
            let prompt = grader_prompt(&question, &answer, &testing_trial.output.answer);
            match runner.call_json::<GradingAgentOutput>(
                "grading",
                grader_index,
                &prompt,
                grading_response_schema(),
            ) {
                Ok((output, receipt)) => {
                    if let Err(err) = validate_grading_output(&output) {
                        failures.push(failure("grading", &receipt.agent_name, err, &receipt));
                    }
                    grading_trials.push(GradingTrial {
                        agent_name: format!("grader-{}", grader_index + 1),
                        testing_agent_name: testing_trial.agent_name.clone(),
                        output,
                        receipt,
                    });
                }
                Err(err) => failures.push(live_call_failure("grading", grader_index, err)),
            }
        }
    }

    let verifier_acceptance = verification_majority(&verification_trials);
    let tester_correct_rate = testing_correct_rate(&testing_trials, &grading_trials);
    let accepted = verifier_acceptance
        && testing_trials.len() >= MIN_SUCCESSFUL_TESTERS
        && tester_correct_rate <= HARD_MAX_TESTER_CORRECT_RATE
        && failures.is_empty();
    let canonical_text = canonical_paper_text(paper, false);
    let metrics = AcceptanceMetrics {
        focused_agreement: accepted_ratio(&verification_trials),
        focused_correct_rate: accepted_ratio(&verification_trials),
        answerability: accepted_ratio(&verification_trials),
        saturated_blind_correct_rate: tester_correct_rate,
        saturated_mean_confidence: mean_tester_confidence(&testing_trials),
        support_minimality: 1.0,
        distractor_pressure: if distractor_hashes.is_empty() {
            0.0
        } else {
            0.80
        },
    };
    let mut final_artifact = FinalPaperChallengeArtifact {
        schema_version: FINAL_PAPER_CHALLENGE_SCHEMA_VERSION.to_string(),
        paper_hash: paper.publication_hash.clone(),
        paper_content: canonical_text,
        artifact_provenance: Some(provenance(config, paper)),
        hard_question: question.clone(),
        hard_answer: answer.clone(),
        hard_agent_name: selected_generation.agent_name.clone(),
        generation_trials: generation_trials.clone(),
        verification_trials: verification_trials.clone(),
        testing_trials: testing_trials.clone(),
        grading_trials: grading_trials.clone(),
        failures,
        acceptance_metrics: metrics.clone(),
        artifact_hash: String::new(),
    };
    final_artifact.artifact_hash = final_paper_challenge_artifact_hash(&final_artifact)?;

    let challenge = challenge_from_artifact(
        paper,
        support_section,
        &quote,
        &question,
        &answer,
        &generation_trials,
        &verification_trials,
        &testing_trials,
        &grading_trials,
        &metrics,
        accepted,
        config,
    )?;
    let errors = if accepted {
        super::production_acceptance_errors(&challenge)
    } else {
        vec!["tournament acceptance gates failed".to_string()]
    };
    let accepted = accepted && errors.is_empty();
    let challenge_dir = if accepted { "challenges" } else { "rejected" };
    let challenge_path = config
        .bank
        .join(challenge_dir)
        .join(format!("{}.json", challenge.challenge_hash));
    let artifact_path = config
        .run_root
        .join("trials")
        .join(&paper.publication_hash)
        .join(&challenge.challenge_hash)
        .join("final.json");
    write_json_pretty(&artifact_path, &final_artifact)?;
    write_json_pretty(&challenge_path, &challenge)?;

    Ok(TournamentWriteResult {
        challenge,
        artifact_path,
        challenge_path,
        accepted,
        errors,
    })
}

fn ensure_paper_time_remaining(
    config: &BuildPaperTournamentConfig,
    paper: &PaperRecord,
    started: Instant,
) -> Result<(), String> {
    let limit = Duration::from_secs(config.paper_timeout_seconds.max(1));
    if started.elapsed() > limit {
        append_progress_row(
            config,
            &json!({
                "event": "paper_timeout",
                "paper_hash": paper.publication_hash,
                "elapsed_ms": started.elapsed().as_millis() as u64,
                "error_category": "paper_timeout"
            }),
        )?;
        return Err(format!(
            "paper {} exceeded timeout of {} seconds",
            paper.publication_hash, config.paper_timeout_seconds
        ));
    }
    Ok(())
}

fn progress_jsonl_path(config: &BuildPaperTournamentConfig) -> PathBuf {
    config
        .progress_jsonl
        .clone()
        .unwrap_or_else(|| config.run_root.join("reports").join("live-progress.jsonl"))
}

fn append_progress_row(
    config: &BuildPaperTournamentConfig,
    value: &serde_json::Value,
) -> Result<(), String> {
    let path = progress_jsonl_path(config);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create progress dir {}: {err}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|err| format!("open progress jsonl {}: {err}", path.display()))?;
    writeln!(
        file,
        "{}",
        serde_json::to_string(value).map_err(|err| format!("serialize progress row: {err}"))?
    )
    .map_err(|err| format!("write progress jsonl {}: {err}", path.display()))
}

pub fn verification_majority(trials: &[VerificationTrial]) -> bool {
    let accepted = trials
        .iter()
        .filter(|trial| trial.output.accepted && trial.output.support_correct)
        .count();
    accepted >= MIN_SUCCESSFUL_VERIFIERS && accepted * 2 > trials.len()
}

pub fn grade_reduction(trials: &[GradingTrial], testing_agent_name: &str) -> Option<(bool, f64)> {
    let matching = trials
        .iter()
        .filter(|trial| trial.testing_agent_name == testing_agent_name)
        .collect::<Vec<_>>();
    if matching.len() < MIN_SUCCESSFUL_GRADERS {
        return None;
    }
    let correct_votes = matching.iter().filter(|trial| trial.output.correct).count();
    let score = matching
        .iter()
        .map(|trial| trial.output.score_0_100 as f64)
        .sum::<f64>()
        / matching.len() as f64;
    Some((correct_votes * 2 > matching.len(), score))
}

pub fn build_testing_prompt(
    paper: &PaperRecord,
    all_papers: &[PaperRecord],
    distractor_paper_hashes: &[String],
    question: &str,
) -> String {
    let mut prompt = format!(
        "Answer the question using the supplied papers. The target paper is not identified, and several papers may contain similar facts. Return the best answer only when the supplied text supports it.\n\
Confidence must mean confidence that your answer contains every requested material detail, not confidence that one fact is probably right. Use confidence 55 or lower for partial, gist-only, inferred, or one-clause answers.\n\
Question: {question}\n"
    );
    let mut context_papers = vec![paper];
    for hash in distractor_paper_hashes {
        if let Some(distractor) = all_papers
            .iter()
            .find(|candidate| candidate.publication_hash == *hash)
        {
            context_papers.push(distractor);
        } else {
            prompt.push_str(&format!("Unavailable paper hash: {hash}\n"));
        }
    }
    context_papers.sort_by(|left, right| {
        sha256_hex(format!("{question}:{}", left.publication_hash).as_bytes()).cmp(&sha256_hex(
            format!("{question}:{}", right.publication_hash).as_bytes(),
        ))
    });
    let mut estimated_tokens = token_estimate(&prompt);
    let distractor_budget = 48_000_u64;
    for (paper_index, context_paper) in context_papers.into_iter().enumerate() {
        let header = format!(
            "\nPaper {}: {}\nPublication hash: {}\n",
            paper_index + 1,
            context_paper.title,
            context_paper.publication_hash
        );
        let header_cost = token_estimate(&header);
        if estimated_tokens + header_cost > distractor_budget {
            continue;
        }
        prompt.push_str(&header);
        estimated_tokens += header_cost;
        for section in &context_paper.sections {
            if !eligible_support_section(section) {
                continue;
            }
            let block = format!(
                "[{}:{}]\n{}\n\n",
                context_paper.publication_hash, section.section_id, section.text
            );
            let cost = token_estimate(&block);
            if estimated_tokens + cost > distractor_budget {
                break;
            }
            prompt.push_str(&block);
            estimated_tokens += cost;
        }
    }
    prompt
}

fn challenge_from_artifact(
    paper: &PaperRecord,
    support_section: &PaperSection,
    quote: &str,
    question: &str,
    answer: &str,
    generation_trials: &[GeneratorTrial],
    verification_trials: &[VerificationTrial],
    testing_trials: &[TestingTrial],
    grading_trials: &[GradingTrial],
    metrics: &AcceptanceMetrics,
    accepted: bool,
    config: &BuildPaperTournamentConfig,
) -> Result<ChallengeRecord, String> {
    let support = vec![SupportRef {
        section_id: support_section.section_id.clone(),
        section_hash: support_section.section_hash.clone(),
        quote_hash: Some(sha256_hex(quote.as_bytes())),
    }];
    let context_pack = pack_context(
        paper,
        &[support_section.section_id.clone()],
        128_000,
        0.82,
        4096,
    )?;
    let focused_support_trials = verification_trials
        .iter()
        .map(|trial| model_trial_from_verifier(trial))
        .collect::<Vec<_>>();
    let saturated_blind_trials = testing_trials
        .iter()
        .map(|trial| {
            let (correct, score) =
                grade_reduction(grading_trials, &trial.agent_name).unwrap_or((false, 0.0));
            model_trial_from_tester(trial, correct, score)
        })
        .collect::<Vec<_>>();
    let judge_trials = verification_trials
        .iter()
        .map(|trial| JudgeTrial {
            agent_id: trial.agent_name.clone(),
            accepted: trial.output.accepted && trial.output.support_correct,
            confidence: trial.output.confidence as f64 / 100.0,
            rationale_hash: sha256_hex(trial.output.reason.as_bytes()),
            route_metadata: trial
                .receipt
                .route_metadata
                .clone()
                .expect("route metadata"),
            token_usage: trial.receipt.token_usage.clone().expect("token usage"),
        })
        .collect::<Vec<_>>();
    let mut route_metadata = Vec::new();
    for receipt in generation_trials
        .iter()
        .map(|trial| &trial.receipt)
        .chain(verification_trials.iter().map(|trial| &trial.receipt))
        .chain(testing_trials.iter().map(|trial| &trial.receipt))
        .chain(grading_trials.iter().map(|trial| &trial.receipt))
    {
        if let Some(route) = receipt.route_metadata.clone() {
            route_metadata.push(route);
        }
    }
    let challenge = ChallengeRecord {
        schema_version: PRODUCTION_CHALLENGE_SCHEMA_VERSION.to_string(),
        challenge_hash: String::new(),
        publication_hash: paper.publication_hash.clone(),
        domain: domain_for_paper(paper),
        topics: vec!["paper-recall".to_string(), "deep-stem".to_string()],
        difficulty_score: 0.0,
        difficulty_components: BTreeMap::new(),
        question: question.to_string(),
        answer_key: AnswerKey {
            canonical: answer.to_string(),
            must_include: vec![answer.to_string()],
            must_not_include: Vec::new(),
            aliases: Vec::new(),
            numeric_tolerances: Vec::new(),
            unit_tolerances: Vec::new(),
        },
        support,
        context_pack: context_pack.clone(),
        generator_agents: generation_trials
            .iter()
            .map(|trial| serde_json::to_value(trial).map_err(|err| err.to_string()))
            .collect::<Result<Vec<_>, _>>()?,
        blind_answer_attempts: saturated_blind_trials
            .iter()
            .map(|trial| AnswerAttempt {
                agent_id: trial.agent_id.clone(),
                correct: trial.correct,
                answerability: trial.answerability,
                supported: trial.supported,
            })
            .collect(),
        focused_answer_attempts: focused_support_trials
            .iter()
            .map(|trial| AnswerAttempt {
                agent_id: trial.agent_id.clone(),
                correct: trial.correct,
                answerability: trial.answerability,
                supported: trial.supported,
            })
            .collect(),
        critic_attempts: Vec::new(),
        audit_attempts: grading_trials
            .iter()
            .map(|trial| serde_json::to_value(trial).map_err(|err| err.to_string()))
            .collect::<Result<Vec<_>, _>>()?,
        acceptance: AcceptanceRecord {
            accepted,
            auditor_agreement: metrics.focused_agreement,
            answerability: metrics.answerability,
            blind_correct_rate: metrics.saturated_blind_correct_rate,
            focused_correct_rate: metrics.focused_correct_rate,
            ambiguity_flag: false,
            hash_mismatch: false,
            redistributable: paper.license.redistributable,
            reason: if accepted {
                Some("paper tournament accepted".to_string())
            } else {
                Some("paper tournament rejected".to_string())
            },
        },
        source_publication: Some(super::SourcePublication {
            publication_hash: paper.publication_hash.clone(),
            content_hash: paper.content_hash.clone(),
            license_spdx: paper.license.spdx.clone(),
            redistributable: paper.license.redistributable,
            source_url: paper.license.source_url.clone(),
            section_hashes: paper
                .sections
                .iter()
                .map(|section| section.section_hash.clone())
                .collect(),
        }),
        focused_support_trials,
        saturated_blind_trials,
        judge_trials,
        context_packs: vec![ContextPackProvenance {
            kind: "paper_tournament_full_text".to_string(),
            context_hash: sha256_hex(
                paper
                    .sections
                    .iter()
                    .map(|section| section.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
                    .as_bytes(),
            ),
            prompt_hash: sha256_hex(question.as_bytes()),
            section_ids: context_pack
                .target_section_ids
                .iter()
                .chain(context_pack.distractor_section_ids.iter())
                .cloned()
                .collect(),
            estimated_tokens: context_pack.estimated_tokens,
        }],
        route_metadata,
        acceptance_metrics: Some(metrics.clone()),
        artifact_provenance: Some(ArtifactProvenance {
            run_id: run_id(&config.run_root),
            reducer_version: QBANK_REDUCER_VERSION.to_string(),
            created_at: "2026-05-13T00:00:00Z".to_string(),
            agent_mode: Some(config.agent_runner.as_str().to_string()),
            fixture_provenance: config.agent_runner.is_mock(),
            answer_leakage_detected: false,
            license_ambiguous: paper.license.spdx.eq_ignore_ascii_case("NOASSERTION"),
        }),
        artifact_hash: None,
    };
    Ok(finalize_challenge(challenge))
}

fn provenance(config: &BuildPaperTournamentConfig, paper: &PaperRecord) -> ArtifactProvenance {
    ArtifactProvenance {
        run_id: run_id(&config.run_root),
        reducer_version: QBANK_REDUCER_VERSION.to_string(),
        created_at: "2026-05-13T00:00:00Z".to_string(),
        agent_mode: Some(config.agent_runner.as_str().to_string()),
        fixture_provenance: config.agent_runner.is_mock(),
        answer_leakage_detected: false,
        license_ambiguous: paper.license.spdx.eq_ignore_ascii_case("NOASSERTION"),
    }
}

fn model_trial_from_verifier(trial: &VerificationTrial) -> ModelTrial {
    ModelTrial {
        agent_id: trial.agent_name.clone(),
        phase: "verification".to_string(),
        correct: trial.output.accepted,
        answerability: if trial.output.accepted { 1.0 } else { 0.0 },
        supported: trial.output.support_correct,
        confidence: trial.output.confidence as f64 / 100.0,
        prompt_hash: trial.receipt.prompt_hash.clone(),
        context_hash: trial.receipt.context_hash.clone(),
        route_metadata: trial
            .receipt
            .route_metadata
            .clone()
            .expect("route metadata"),
        token_usage: trial.receipt.token_usage.clone().expect("token usage"),
    }
}

fn model_trial_from_tester(trial: &TestingTrial, correct: bool, score: f64) -> ModelTrial {
    ModelTrial {
        agent_id: trial.agent_name.clone(),
        phase: "saturated_blind_testing".to_string(),
        correct,
        answerability: score / 100.0,
        supported: correct,
        confidence: trial.output.confidence as f64 / 100.0,
        prompt_hash: trial.receipt.prompt_hash.clone(),
        context_hash: trial.receipt.context_hash.clone(),
        route_metadata: trial
            .receipt
            .route_metadata
            .clone()
            .expect("route metadata"),
        token_usage: trial.receipt.token_usage.clone().expect("token usage"),
    }
}

fn testing_correct_rate(testing_trials: &[TestingTrial], grading_trials: &[GradingTrial]) -> f64 {
    if testing_trials.is_empty() {
        return 1.0;
    }
    let correct = testing_trials
        .iter()
        .filter(|trial| {
            grade_reduction(grading_trials, &trial.agent_name)
                .map(|(correct, _)| correct)
                .unwrap_or(false)
        })
        .count();
    correct as f64 / testing_trials.len() as f64
}

fn accepted_ratio(trials: &[VerificationTrial]) -> f64 {
    if trials.is_empty() {
        return 0.0;
    }
    let accepted = trials
        .iter()
        .filter(|trial| trial.output.accepted && trial.output.support_correct)
        .count();
    accepted as f64 / trials.len() as f64
}

fn mean_tester_confidence(trials: &[TestingTrial]) -> f64 {
    if trials.is_empty() {
        return 1.0;
    }
    trials
        .iter()
        .map(|trial| trial.output.confidence as f64 / 100.0)
        .sum::<f64>()
        / trials.len() as f64
}

fn first_sentence(text: &str) -> String {
    let trimmed = text.trim();
    let chars = trimmed.char_indices().collect::<Vec<_>>();
    for (position, (index, ch)) in chars.iter().enumerate() {
        if !matches!(ch, '.' | '!' | '?') {
            continue;
        }
        if *ch == '.' {
            let prev = position
                .checked_sub(1)
                .and_then(|prev| chars.get(prev))
                .map(|(_, ch)| *ch);
            let next = chars.get(position + 1).map(|(_, ch)| *ch);
            if prev.is_some_and(|ch| ch.is_ascii_digit())
                && next.is_some_and(|ch| ch.is_ascii_digit())
            {
                continue;
            }
        }
        let candidate = trimmed[..*index].trim();
        if candidate.chars().count() > 24 {
            return candidate.to_string();
        }
    }
    trimmed.to_string()
}

fn answer_from_quote(quote: &str) -> String {
    quote.trim().to_string()
}

fn select_distractors(
    paper: &PaperRecord,
    papers: &[PaperRecord],
    requested: usize,
) -> Vec<String> {
    papers
        .iter()
        .filter(|candidate| candidate.publication_hash != paper.publication_hash)
        .take(requested)
        .map(|candidate| candidate.publication_hash.clone())
        .collect()
}

fn receipt(phase: &str, index: usize, prompt: &str, paper_hash: &str) -> AgentCallReceipt {
    let prompt_hash = sha256_hex(prompt.as_bytes());
    let context_hash = sha256_hex(format!("{paper_hash}:{phase}:{index}").as_bytes());
    let raw_output_hash = sha256_hex(format!("{phase}:{index}:{prompt_hash}").as_bytes());
    let usage = TokenUsage {
        prompt_tokens: 1200 + index as u64,
        completion_tokens: 300 + index as u64,
        total_tokens: 1500 + (index as u64 * 2),
    };
    let decisions = vec![ModelDecision {
        model_id: format!("qbank-{phase}-primary"),
        configured_score: 0.91,
        selection_score: 0.93,
        latency_ms: 100 + index as u64,
        status: "completed".to_string(),
        output_hash: Some(raw_output_hash.clone()),
        selected: true,
        token_usage: usage.clone(),
    }];
    let decisions_hash = sha256_hex(&serde_json::to_vec(&decisions).expect("decisions serialize"));
    let route_metadata = RouteMetadata {
        request_id: format!(
            "mock_smoke_{}_{}_{}",
            phase,
            index + 1,
            &raw_output_hash[..12]
        ),
        provider: "mock-smoke".to_string(),
        model: format!("mock-qbank-{phase}-primary"),
        route_mode: Some("mock_smoke".to_string()),
        route_confidence: Some(0.93),
        primary_model_id: Some(format!("mock-qbank-{phase}-primary")),
        backup_model_ids: vec![format!("mock-qbank-{phase}-backup")],
        fusion_model_id: Some("mock-qbank-fusion-router".to_string()),
        winner_model_id: Some(format!("mock-qbank-{phase}-primary")),
        prompt_hash: Some(prompt_hash.clone()),
        context_hash: Some(context_hash.clone()),
        receipts_hash: Some(sha256_hex(
            format!("{phase}:{index}:{paper_hash}:receipt").as_bytes(),
        )),
        token_usage: Some(usage.clone()),
        model_decisions_hash: Some(decisions_hash),
        model_decisions: decisions,
    };
    AgentCallReceipt {
        agent_name: format!("{phase}-{}", index + 1),
        phase: phase.to_string(),
        prompt_hash,
        context_hash,
        raw_output_hash,
        route_metadata: Some(route_metadata),
        token_usage: Some(usage),
    }
}

fn failure(
    phase: &str,
    agent_name: &str,
    error: String,
    receipt: &AgentCallReceipt,
) -> AgentFailure {
    AgentFailure {
        phase: phase.to_string(),
        agent_name: agent_name.to_string(),
        error,
        route_metadata: receipt.route_metadata.clone(),
        raw_output_hash: Some(receipt.raw_output_hash.clone()),
    }
}

fn live_call_failure(phase: &str, index: usize, error: JnoccioCallError) -> AgentFailure {
    let receipt = error.receipt.as_ref();
    AgentFailure {
        phase: phase.to_string(),
        agent_name: receipt
            .map(|receipt| receipt.agent_name.clone())
            .unwrap_or_else(|| format!("{phase}-{}", index + 1)),
        error: error.message,
        route_metadata: receipt.and_then(|receipt| receipt.route_metadata.clone()),
        raw_output_hash: receipt.map(|receipt| receipt.raw_output_hash.clone()),
    }
}

fn failure_route_label(route: Option<&RouteMetadata>) -> String {
    let Some(route) = route else {
        return String::new();
    };
    let usage = route
        .token_usage
        .as_ref()
        .map(|usage| {
            format!(
                ", tokens={}/{}/{}",
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
            )
        })
        .unwrap_or_default();
    format!(
        " [request_id={}, model={}, route_mode={}, winner={}{}]",
        route.request_id,
        route.model,
        route.route_mode.as_deref().unwrap_or(""),
        route.winner_model_id.as_deref().unwrap_or(""),
        usage
    )
}

struct JnoccioCallError {
    message: String,
    receipt: Option<AgentCallReceipt>,
    retryable: bool,
}

impl JnoccioCallError {
    fn new(message: String) -> Self {
        Self {
            message,
            receipt: None,
            retryable: true,
        }
    }

    fn retryable(message: String) -> Self {
        Self::new(message)
    }

    fn non_retryable(message: String) -> Self {
        Self {
            message,
            receipt: None,
            retryable: false,
        }
    }

    fn with_receipt(message: String, receipt: AgentCallReceipt) -> Self {
        Self {
            message,
            receipt: Some(receipt),
            retryable: true,
        }
    }

    fn with_context(mut self, context: String) -> Self {
        self.message = format!("{} ({context})", self.message);
        self
    }
}

struct JnoccioHttpRunner {
    client: reqwest::blocking::Client,
    endpoint: String,
    model: String,
    max_output_tokens: u64,
    phase_retries: usize,
    bearer_token: Option<String>,
    progress_jsonl: PathBuf,
    run_root: PathBuf,
}

impl JnoccioHttpRunner {
    fn new(config: &BuildPaperTournamentConfig) -> Result<Self, String> {
        let base_url = config
            .jnoccio_base_url
            .as_deref()
            .ok_or("--agent-runner jnoccio requires --jnoccio-base-url")?
            .trim()
            .trim_end_matches('/');
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(
                config.jnoccio_request_timeout_seconds.max(1),
            ))
            .build()
            .map_err(|err| format!("build jnoccio http client: {err}"))?;
        Ok(Self {
            client,
            endpoint: format!("{base_url}/v1/chat/completions"),
            model: configured_jnoccio_model(config),
            max_output_tokens: config.jnoccio_max_output_tokens,
            phase_retries: config.phase_retries,
            bearer_token: std::env::var("JNOCCIO_BEARER_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            progress_jsonl: progress_jsonl_path(config),
            run_root: config.run_root.clone(),
        })
    }

    fn call_json<T>(
        &self,
        phase: &str,
        index: usize,
        prompt: &str,
        response_schema: serde_json::Value,
    ) -> Result<(T, AgentCallReceipt), JnoccioCallError>
    where
        T: DeserializeOwned + Serialize,
    {
        let mut last_error = None;
        for attempt in 0..=self.phase_retries {
            match self.call_json_once::<T>(phase, index, attempt, prompt, response_schema.clone()) {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if !err.retryable {
                        return Err(err);
                    }
                    last_error = Some(if attempt == 0 {
                        err
                    } else {
                        err.with_context(format!("retry_attempt={attempt}"))
                    });
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            JnoccioCallError::new(format!("jnoccio {phase} failed before request dispatch"))
        }))
    }

    fn call_json_once<T>(
        &self,
        phase: &str,
        index: usize,
        attempt: usize,
        prompt: &str,
        response_schema: serde_json::Value,
    ) -> Result<(T, AgentCallReceipt), JnoccioCallError>
    where
        T: DeserializeOwned + Serialize,
    {
        let call_started = Instant::now();
        self.append_progress(&json!({
            "event": "before_call",
            "phase": phase,
            "attempt": attempt,
            "agent_index": index,
            "elapsed_ms": 0,
            "route_metadata_present": false,
            "parse_status": "not_started",
            "schema_status": "not_started",
            "error_category": null
        }))?;
        let body = json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are building production QBank evidence from redistributable scientific papers. Return only JSON that satisfies the schema. Never invent route metadata."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.2,
            "max_tokens": self.max_output_tokens,
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": format!("qbank_{phase}_output"),
                    "strict": true,
                    "schema": response_schema
                }
            }
        });
        let mut request = self.client.post(&self.endpoint).json(&body);
        if let Some(token) = self.bearer_token.as_ref() {
            request = request.bearer_auth(token);
        }
        let response = request.send().map_err(|err| {
            let category = if err.is_timeout() {
                "timeout"
            } else {
                "http_request"
            };
            let _ = self.append_progress(&json!({
                "event": "after_call",
                "phase": phase,
                "attempt": attempt,
                "agent_index": index,
                "elapsed_ms": call_started.elapsed().as_millis() as u64,
                "route_metadata_present": false,
                "parse_status": "not_started",
                "schema_status": "unknown",
                "error_category": category,
                "error": err.to_string()
            }));
            JnoccioCallError::retryable(format!("jnoccio {phase} request failed: {err}"))
        })?;
        let status = response.status();
        let text = response.text().map_err(|err| {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                false,
                "not_started",
                "unknown",
                "http_response_read",
                err.to_string(),
                None,
            );
            JnoccioCallError::new(format!("jnoccio {phase} response read failed: {err}"))
        })?;
        if !status.is_success() {
            let _ = self.append_progress(&json!({
                "event": "after_call",
                "phase": phase,
                "attempt": attempt,
                "agent_index": index,
                "elapsed_ms": call_started.elapsed().as_millis() as u64,
                "route_metadata_present": false,
                "parse_status": "not_started",
                "schema_status": "unknown",
                "error_category": "http_status",
                "status": status.as_u16()
            }));
            return Err(JnoccioCallError::retryable(format!(
                "jnoccio {phase} returned HTTP {}: {}",
                status.as_u16(),
                text
            )));
        }
        let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|err| {
            let _ = self.append_progress(&json!({
                "event": "after_call",
                "phase": phase,
                "attempt": attempt,
                "agent_index": index,
                "elapsed_ms": call_started.elapsed().as_millis() as u64,
                "route_metadata_present": false,
                "parse_status": "response_json_error",
                "schema_status": "unknown",
                "error_category": "parse"
            }));
            JnoccioCallError::retryable(format!("jnoccio {phase} response is not JSON: {err}"))
        })?;
        let message = parsed
            .get("choices")
            .and_then(|value| value.as_array())
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("message"))
            .ok_or_else(|| {
                self.append_after_error(
                    phase,
                    index,
                    attempt,
                    &call_started,
                    false,
                    "response_json_ok",
                    "unknown",
                    "response_shape",
                    "response missing assistant message",
                    None,
                );
                JnoccioCallError::new(format!(
                    "jnoccio {phase} response missing assistant message"
                ))
            })?;
        let content = message
            .get("content")
            .and_then(|content| content.as_str())
            .or_else(|| {
                message
                    .get("reasoning_text")
                    .and_then(|value| value.as_str())
            })
            .or_else(|| {
                message
                    .get("reasoning_content")
                    .and_then(|value| value.as_str())
            })
            .or_else(|| message.get("reasoning").and_then(|value| value.as_str()))
            .ok_or_else(|| {
                self.append_after_error(
                    phase,
                    index,
                    attempt,
                    &call_started,
                    false,
                    "response_json_ok",
                    "unknown",
                    "response_shape",
                    "response missing assistant content",
                    None,
                );
                JnoccioCallError::new(format!(
                    "jnoccio {phase} response missing assistant content: {}",
                    serde_json::to_string(message)
                        .unwrap_or_default()
                        .chars()
                        .take(800)
                        .collect::<String>()
                ))
            })?;
        let route_value = parsed.get("jnoccio").ok_or_else(|| {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                false,
                "response_json_ok",
                "unknown",
                "route_metadata",
                "response missing extra.jnoccio metadata",
                None,
            );
            JnoccioCallError::new(format!(
                "jnoccio {phase} response missing extra.jnoccio metadata"
            ))
        })?;
        let route_metadata = route_metadata_from_jnoccio(route_value).map_err(|err| {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                true,
                "response_json_ok",
                route_value
                    .get("structured_schema_status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                "route_metadata",
                err.clone(),
                None,
            );
            JnoccioCallError::new(err)
        })?;
        validate_live_route_metadata(phase, &route_metadata).map_err(|err| {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                true,
                "response_json_ok",
                route_value
                    .get("structured_schema_status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                "route_metadata",
                err.clone(),
                Some(&route_metadata),
            );
            JnoccioCallError::new(err)
        })?;
        let token_usage = route_metadata.token_usage.clone().ok_or_else(|| {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                true,
                "response_json_ok",
                route_value
                    .get("structured_schema_status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                "route_metadata",
                "metadata missing token_usage",
                Some(&route_metadata),
            );
            JnoccioCallError::new(format!("jnoccio {phase} metadata missing token_usage"))
        })?;
        let prompt_hash = route_metadata.prompt_hash.clone().ok_or_else(|| {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                true,
                "response_json_ok",
                route_value
                    .get("structured_schema_status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                "route_metadata",
                "metadata missing prompt_hash",
                Some(&route_metadata),
            );
            JnoccioCallError::new(format!("jnoccio {phase} metadata missing prompt_hash"))
        })?;
        let context_hash = route_metadata.context_hash.clone().ok_or_else(|| {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                true,
                "response_json_ok",
                route_value
                    .get("structured_schema_status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                "route_metadata",
                "metadata missing context_hash",
                Some(&route_metadata),
            );
            JnoccioCallError::new(format!("jnoccio {phase} metadata missing context_hash"))
        })?;
        let raw_output_hash = sha256_hex(content.as_bytes());
        let receipt = AgentCallReceipt {
            agent_name: format!("{phase}-{}", index + 1),
            phase: phase.to_string(),
            prompt_hash,
            context_hash,
            raw_output_hash,
            route_metadata: Some(route_metadata.clone()),
            token_usage: Some(token_usage),
        };
        let output = parse_agent_json::<T>(content).map_err(|err| {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                true,
                "agent_json_error",
                route_value
                    .get("structured_schema_status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                "parse",
                err.to_string(),
                Some(&route_metadata),
            );
            JnoccioCallError::with_receipt(
                format!(
                    "parse jnoccio {phase} output: {err}; content preview: {}",
                    content.chars().take(800).collect::<String>()
                ),
                receipt.clone(),
            )
        })?;
        self.append_progress(&json!({
            "event": "after_call",
            "phase": phase,
            "attempt": attempt,
            "agent_index": index,
            "elapsed_ms": call_started.elapsed().as_millis() as u64,
            "route_metadata_present": true,
            "parse_status": "ok",
            "schema_status": route_value.get("structured_schema_status").and_then(|value| value.as_str()).unwrap_or("unknown"),
            "error_category": null,
            "request_id": route_metadata.request_id,
            "route_mode": route_metadata.route_mode,
            "winner_model_id": route_metadata.winner_model_id,
            "token_usage": route_metadata.token_usage
        }))?;
        Ok((output, receipt))
    }

    #[allow(clippy::too_many_arguments)]
    fn append_after_error(
        &self,
        phase: &str,
        index: usize,
        attempt: usize,
        call_started: &Instant,
        route_metadata_present: bool,
        parse_status: &str,
        schema_status: &str,
        error_category: &str,
        error: impl Into<String>,
        route_metadata: Option<&RouteMetadata>,
    ) {
        let mut row = json!({
            "event": "after_call",
            "phase": phase,
            "attempt": attempt,
            "agent_index": index,
            "elapsed_ms": call_started.elapsed().as_millis() as u64,
            "route_metadata_present": route_metadata_present,
            "parse_status": parse_status,
            "schema_status": schema_status,
            "error_category": error_category,
            "error": error.into()
        });
        if let (Some(map), Some(route)) = (row.as_object_mut(), route_metadata) {
            map.insert("request_id".to_string(), json!(route.request_id));
            map.insert("route_mode".to_string(), json!(route.route_mode));
            map.insert("winner_model_id".to_string(), json!(route.winner_model_id));
            map.insert("token_usage".to_string(), json!(route.token_usage));
        }
        let _ = self.append_progress(&row);
    }

    fn append_progress(&self, value: &serde_json::Value) -> Result<(), JnoccioCallError> {
        let path = &self.progress_jsonl;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                JnoccioCallError::non_retryable(format!(
                    "create progress dir {}: {err}",
                    parent.display()
                ))
            })?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|err| {
                JnoccioCallError::non_retryable(format!(
                    "open progress jsonl {}: {err}",
                    path.display()
                ))
            })?;
        let mut row = value.clone();
        if let Some(map) = row.as_object_mut() {
            map.insert(
                "run_root".to_string(),
                json!(self.run_root.display().to_string()),
            );
            map.insert("model".to_string(), json!(self.model));
        }
        writeln!(
            file,
            "{}",
            serde_json::to_string(&row).map_err(|err| {
                JnoccioCallError::non_retryable(format!("serialize progress row: {err}"))
            })?
        )
        .map_err(|err| {
            JnoccioCallError::non_retryable(format!(
                "write progress jsonl {}: {err}",
                path.display()
            ))
        })
    }
}

fn route_metadata_from_jnoccio(value: &serde_json::Value) -> Result<RouteMetadata, String> {
    let mut metadata: RouteMetadata = serde_json::from_value(value.clone())
        .map_err(|err| format!("parse jnoccio route metadata: {err}"))?;
    if metadata.route_confidence.is_none() {
        metadata.route_confidence = value.get("confidence").and_then(|value| value.as_f64());
    }
    Ok(metadata)
}

fn validate_live_route_metadata(phase: &str, route: &RouteMetadata) -> Result<(), String> {
    if route.provider != "jnoccio" {
        return Err(format!("jnoccio {phase} route provider is not jnoccio"));
    }
    if route.request_id.trim().is_empty()
        || route.model.trim().is_empty()
        || route.route_mode.as_deref().unwrap_or("").trim().is_empty()
        || route
            .primary_model_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        || route
            .winner_model_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        || route.prompt_hash.as_deref().unwrap_or("").trim().is_empty()
        || route
            .context_hash
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        || route
            .receipts_hash
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        || route
            .model_decisions_hash
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err(format!("jnoccio {phase} route metadata is incomplete"));
    }
    if route.route_mode.as_deref() == Some("fusion")
        && route
            .fusion_model_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err(format!(
            "jnoccio {phase} fusion route missing fusion_model_id"
        ));
    }
    if route.backup_model_ids.is_empty()
        || route.token_usage.is_none()
        || route.model_decisions.is_empty()
    {
        return Err(format!("jnoccio {phase} route metadata is incomplete"));
    }
    if route.request_id.to_ascii_lowercase().starts_with("mock") {
        return Err(format!("jnoccio {phase} request_id is not live"));
    }
    Ok(())
}

pub(crate) fn paper_quality_allowed(paper: &PaperRecord) -> bool {
    let title = paper.title.to_ascii_lowercase();
    let blocked_title_terms = [
        "correction",
        "erratum",
        "corrigendum",
        "retraction",
        "editorial",
        "publisher's note",
        "publisher note",
        "expression of concern",
    ];
    !blocked_title_terms.iter().any(|term| title.contains(term))
}

pub(crate) fn support_quote_candidates(paper: &PaperRecord) -> Vec<SupportQuoteCandidate> {
    let mut scored = Vec::<(i32, usize, SupportQuoteCandidate)>::new();
    let mut ordinal = 0usize;
    for section in &paper.sections {
        if !eligible_support_section(section) {
            continue;
        }
        for sentence in exact_sentences(&section.text) {
            if !eligible_support_quote(&sentence) {
                continue;
            }
            let mut score = support_quote_score(&section.title, &sentence);
            if scored
                .iter()
                .any(|(_, _, candidate)| candidate.quote == sentence)
            {
                continue;
            }
            if score < 0 {
                score = 0;
            }
            ordinal += 1;
            scored.push((
                score,
                ordinal,
                SupportQuoteCandidate {
                    id: String::new(),
                    section_id: section.section_id.clone(),
                    section_hash: section.section_hash.clone(),
                    section_title: section.title.clone(),
                    quote: sentence,
                },
            ));
        }
    }
    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    scored
        .into_iter()
        .take(24)
        .enumerate()
        .map(|(index, (_, _, mut candidate))| {
            candidate.id = format!("q{:03}", index + 1);
            candidate
        })
        .collect()
}

fn eligible_support_section(section: &PaperSection) -> bool {
    let key = format!(
        "{} {}",
        section.section_id.to_ascii_lowercase(),
        section.title.to_ascii_lowercase()
    );
    let blocked = [
        "abstract",
        "source",
        "reference",
        "bibliography",
        "acknowledg",
        "funding",
        "competing interest",
        "conflict",
        "author contribution",
        "data availability",
        "ethics",
        "publisher",
        "supplement",
        "appendix",
    ];
    !blocked.iter().any(|term| key.contains(term))
}

fn eligible_support_quote(sentence: &str) -> bool {
    let trimmed = sentence.trim();
    let chars = trimmed.chars().count();
    let marker_count = support_quote_specificity_marker_count(trimmed);
    let clause_count = trimmed
        .chars()
        .filter(|ch| matches!(ch, ',' | ';' | ':' | '('))
        .count();
    chars >= 80
        && chars <= 420
        && trimmed.contains(' ')
        && marker_count >= 2
        && clause_count >= 1
        && !trimmed.starts_with("http")
        && !trimmed
            .to_ascii_lowercase()
            .contains("all claims expressed")
}

fn support_quote_score(section_title: &str, sentence: &str) -> i32 {
    let mut score = 0;
    let title = section_title.to_ascii_lowercase();
    for term in [
        "result",
        "results",
        "finding",
        "findings",
        "method",
        "methods",
        "discussion",
        "analysis",
        "case",
    ] {
        if title.contains(term) {
            score += 3;
            break;
        }
    }
    if sentence.chars().any(|ch| ch.is_ascii_digit()) {
        score += 4;
    }
    score += support_quote_specificity_marker_count(sentence).min(8) as i32;
    let lower = sentence.to_ascii_lowercase();
    for term in [
        "%",
        "rate",
        "ratio",
        "mean",
        "median",
        "increase",
        "decrease",
        "significant",
        "highest",
        "lowest",
        "maximum",
        "minimum",
        "identified",
        "observed",
        "measured",
        "found",
    ] {
        if lower.contains(term) {
            score += 1;
        }
    }
    score
}

fn support_quote_specificity_marker_count(sentence: &str) -> usize {
    let lower = sentence.to_ascii_lowercase();
    let digit_markers = sentence
        .split_whitespace()
        .filter(|part| part.chars().any(|ch| ch.is_ascii_digit()))
        .count();
    let symbol_markers = sentence
        .chars()
        .filter(|ch| {
            matches!(
                ch,
                '%' | '\u{00b1}' | '\u{00d7}' | '=' | '<' | '>' | '/' | '-'
            )
        })
        .count();
    let unit_markers = [
        " mg",
        " \u{03bc}",
        " mm",
        " cm",
        " kg",
        " wt",
        " \u{00b0}c",
        " rh",
        " ci",
        " p ",
        " fold",
        " ratio",
        " percent",
        " coefficient",
        " probability",
    ]
    .iter()
    .filter(|marker| lower.contains(**marker))
    .count();
    digit_markers + symbol_markers + unit_markers
}

fn exact_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut start = None;
    for (index, ch) in text.char_indices() {
        if start.is_none() && !ch.is_whitespace() {
            start = Some(index);
        }
        if !matches!(ch, '.' | '!' | '?') {
            continue;
        }
        let end = index + ch.len_utf8();
        let next_is_boundary = text[end..]
            .chars()
            .next()
            .map(|next| next.is_whitespace())
            .unwrap_or(true);
        if next_is_boundary {
            if let Some(sentence_start) = start.take() {
                let sentence = text[sentence_start..end].trim().to_string();
                if !sentence.is_empty() {
                    sentences.push(sentence);
                }
            }
        }
    }
    if let Some(sentence_start) = start {
        let sentence = text[sentence_start..].trim().to_string();
        if !sentence.is_empty() {
            sentences.push(sentence);
        }
    }
    sentences
}

fn generator_output_from_selection(
    selection: GeneratorSelectionOutput,
    quote_candidates: &[SupportQuoteCandidate],
    failures: &mut Vec<AgentFailure>,
    receipt: &AgentCallReceipt,
) -> GeneratorAgentOutput {
    let support = quote_candidates
        .iter()
        .find(|candidate| candidate.id == selection.support_quote_id)
        .map(|candidate| SupportQuote {
            section_id: candidate.section_id.clone(),
            section_hash: candidate.section_hash.clone(),
            quote: candidate.quote.clone(),
            why_it_matters: "Selected from deterministic canonical support candidates.".to_string(),
        })
        .into_iter()
        .collect::<Vec<_>>();
    if support.is_empty() {
        failures.push(failure(
            "generation",
            &receipt.agent_name,
            format!(
                "generator selected unknown support_quote_id {}",
                selection.support_quote_id
            ),
            receipt,
        ));
    }
    GeneratorAgentOutput {
        question: selection.question,
        answer: selection.answer,
        difficulty_rationale: selection.difficulty_rationale,
        expected_failure_mode: selection.expected_failure_mode,
        support,
        confidence: selection.confidence,
    }
}

fn generator_prompt(
    paper: &PaperRecord,
    index: usize,
    quote_candidates: &[SupportQuoteCandidate],
) -> String {
    let mut candidates = String::new();
    for candidate in quote_candidates {
        candidates.push_str(&format!(
            "[quote_id: {}]\n[section_title: {}]\n{}\n\n",
            candidate.id, candidate.section_title, candidate.quote
        ));
    }
    format!(
        "Create one hard recall question from this redistributable paper using exactly one supplied support quote candidate.\n\
Rules:\n\
- Return support_quote_id as one quote_id from the supplied list.\n\
- Do not invent section ids, section hashes, quotes, or paper facts.\n\
- The runner will derive the production hard answer from the selected canonical quote.\n\
- The production hard answer is the complete selected support quote, so write a question whose correct answer requires all important details in that quote, not a single value or short gist.\n\
- The answer field may be a compact raw answer for receipt purposes, but it must not introduce facts absent from the selected quote.\n\
- Prefer quotes with multiple concrete constraints, measurements, groups, settings, or outcomes that saturated-context answerers may miss when target and distractor papers are mixed.\n\
- Avoid questions answerable by copying one obvious number, one direction of change, or a fact stated in the paper title.\n\
Generator index: {}\n\
Title: {}\n\
Publication hash: {}\n\
License: {}\n\n\
Support quote candidates:\n{}",
        index + 1,
        paper.title,
        paper.publication_hash,
        paper.license.spdx,
        candidates
    )
}

fn verifier_prompt(paper: &PaperRecord, question: &str, answer: &str, quote: &str) -> String {
    format!(
        "Verify whether this candidate is exactly answerable from the paper. Return accepted=false if support is missing, paraphrased, ambiguous, or not hard.\n\
Question: {question}\n\
Answer: {answer}\n\
Required support quote: {quote}\n\n{}",
        paper_prompt_context(paper)
    )
}

fn grader_prompt(question: &str, answer: &str, tester_answer: &str) -> String {
    format!(
        "Grade the tester answer against the hard answer. Mark correct only when the tester answer includes every material detail in the hard answer. Mark partial, gist-only, single-number, or one-clause answers incorrect even if one fact is right.\n\
Return compact JSON only. Keep reason, matched_key_points, and missed_key_points short.\n\
Question: {question}\n\
Hard answer: {answer}\n\
Tester answer: {tester_answer}"
    )
}

fn paper_prompt_context(paper: &PaperRecord) -> String {
    let mut out = format!(
        "Title: {}\nPublication hash: {}\nLicense: {}\n\n",
        paper.title, paper.publication_hash, paper.license.spdx
    );
    for section in &paper.sections {
        out.push_str(&format!(
            "[section_id: {}]\n[section_hash: {}]\n[title: {}]\n{}\n\n",
            section.section_id, section.section_hash, section.title, section.text
        ));
    }
    out
}

fn string_schema() -> serde_json::Value {
    json!({"type": "string", "minLength": 1})
}

fn confidence_schema() -> serde_json::Value {
    json!({"type": "integer", "minimum": 0, "maximum": 100})
}

fn generator_selection_response_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["question", "answer", "difficulty_rationale", "expected_failure_mode", "support_quote_id", "confidence"],
        "properties": {
            "question": string_schema(),
            "answer": string_schema(),
            "difficulty_rationale": string_schema(),
            "expected_failure_mode": string_schema(),
            "support_quote_id": string_schema(),
            "confidence": confidence_schema()
        }
    })
}

fn verification_response_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["accepted", "answer", "confidence", "support_correct", "reason", "missing_or_wrong_support"],
        "properties": {
            "accepted": {"type": "boolean"},
            "answer": string_schema(),
            "confidence": confidence_schema(),
            "support_correct": {"type": "boolean"},
            "reason": string_schema(),
            "missing_or_wrong_support": {"type": "array", "items": string_schema()}
        }
    })
}

fn testing_response_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["answer", "confidence", "reasoning_summary"],
        "properties": {
            "answer": string_schema(),
            "confidence": confidence_schema(),
            "reasoning_summary": string_schema()
        }
    })
}

fn grading_response_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["correct", "score_0_100", "matched_key_points", "missed_key_points", "reason"],
        "properties": {
            "correct": {"type": "boolean"},
            "score_0_100": confidence_schema(),
            "matched_key_points": {"type": "array", "items": string_schema()},
            "missed_key_points": {"type": "array", "items": string_schema()},
            "reason": string_schema()
        }
    })
}

pub fn final_paper_challenge_artifact_hash(
    artifact: &FinalPaperChallengeArtifact,
) -> Result<String, String> {
    let mut clone = artifact.clone();
    clone.artifact_hash.clear();
    let json = serde_json::to_vec(&clone).map_err(|err| err.to_string())?;
    Ok(sha256_hex(&json))
}

fn run_id(run_root: &Path) -> String {
    run_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("paper-qbank")
        .to_string()
}

fn domain_for_paper(paper: &PaperRecord) -> String {
    let key = paper
        .source_ids
        .first()
        .or_else(|| paper.dedupe_keys.first())
        .map(String::as_str)
        .unwrap_or("");
    let index = key
        .rsplit('-')
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    match index % 4 {
        0 => "materials-science",
        1 => "computational-biology",
        2 => "applied-physics",
        _ => "systems-neuroscience",
    }
    .to_string()
}

fn smoke_paper(index: usize) -> PaperRecord {
    let labels = [
        ("cobalt memory", "cobalt", 42.7),
        ("rhodium lattice", "rhodium", 37.4),
        ("silicon enzyme", "silicon", 58.2),
        ("nickel cortex", "nickel", 26.9),
        ("argon polymer", "argon", 73.5),
        ("boron synapse", "boron", 19.8),
        ("titanium graph", "titanium", 64.1),
        ("xenon channel", "xenon", 31.6),
        ("iridium genome", "iridium", 88.3),
        ("gallium matrix", "gallium", 47.9),
    ];
    let (study, marker, value) = labels[index % labels.len()];
    let anchor = format!(
        "The calibrated recall anchor for the {study} study is {marker}-{} at {value:.1} microjoules after the third annealing pass",
        index + 17
    );
    let long_result = format!(
        "{anchor}. This result is reported as the decisive condition because earlier passes remained unstable under distractor load. The authors state that the anchor should be treated as a paper-local constant, not as a general material property. The evaluation section repeats that {marker}-{} at {value:.1} microjoules is the only setting that survives the saturated recall test with all controls held fixed.",
        index + 17
    );
    let slug = study.replace(' ', "-");
    PaperRecord {
        schema_version: PAPER_SCHEMA_VERSION.to_string(),
        publication_hash: String::new(),
        content_hash: String::new(),
        dedupe_keys: vec![format!("doi:10.5555/{slug}-{index}")],
        source_ids: vec![format!("doi:10.5555/{slug}-{index}")],
        license: LicenseRecord {
            spdx: "CC-BY-4.0".to_string(),
            redistributable: true,
            source_url: Some(format!(
                "https://qbank-smoke.openaccess.local/papers/{slug}-{index}"
            )),
        },
        title: format!("{} Calibration Study", title_case(study)),
        authors: vec!["QBank Smoke Authors".to_string()],
        abstract_text: format!(
            "A redistributable {study} study used for local paper tournament smoke validation."
        ),
        sections: vec![
            PaperSection {
                section_id: "abstract".to_string(),
                title: "Abstract".to_string(),
                text: format!(
                    "This {study} study evaluates recall anchors under saturated context pressure."
                ),
                section_hash: String::new(),
            },
            PaperSection {
                section_id: "results".to_string(),
                title: "Results".to_string(),
                text: long_result,
                section_hash: String::new(),
            },
            PaperSection {
                section_id: "methods".to_string(),
                title: "Methods".to_string(),
                text: "The method used three annealing passes, fixed control loads, and randomized distractor paragraphs to measure recall stability. Every measurement was repeated under the same public-license protocol so that downstream benchmark records can retain the full body text."
                    .to_string(),
                section_hash: String::new(),
            },
        ],
        retrieval_receipts: vec![json!({
            "kind": "paper_tournament_smoke",
            "retrieved_at": "2026-05-13T00:00:00Z",
            "license_spdx": "CC-BY-4.0",
            "smoke_index": index
        })],
        published_at: Some("2026-05-13T00:00:00Z".to_string()),
    }
}

fn title_case(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
