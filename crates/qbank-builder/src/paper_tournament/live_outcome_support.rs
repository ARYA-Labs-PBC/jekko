use super::live_outcome::{CandidateOutcome, GenerationCandidate};
use super::*;
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

pub(super) fn stem_leakage_score(question: &str, quote: &str) -> f64 {
    let question_tokens = content_tokens(question);
    let answer_tokens = content_tokens(quote);
    if answer_tokens.is_empty() {
        return 1.0;
    }
    let overlap = answer_tokens
        .iter()
        .filter(|token| question_tokens.contains(*token))
        .count();
    overlap as f64 / answer_tokens.len() as f64
}

pub(super) fn has_confident_correct_answer(
    testing_trials: &[TestingTrial],
    grading_trials: &[GradingTrial],
    min_successful_graders: usize,
) -> bool {
    testing_trials.iter().any(|trial| {
        let Some((correct, score)) =
            grade_reduction_with_min(grading_trials, &trial.agent_name, min_successful_graders)
        else {
            return false;
        };
        correct && trial.output.confidence >= 85 && score >= 90.0
    })
}

pub(super) fn candidate_metrics(
    verification_trials: &[VerificationTrial],
    testing_trials: &[TestingTrial],
    grading_trials: &[GradingTrial],
    distractor_hashes: &[String],
    config: &BuildPaperTournamentConfig,
) -> AcceptanceMetrics {
    AcceptanceMetrics {
        focused_agreement: accepted_ratio(verification_trials),
        focused_correct_rate: accepted_ratio(verification_trials),
        answerability: accepted_ratio(verification_trials),
        saturated_blind_correct_rate: testing_correct_rate_with_min(
            testing_trials,
            grading_trials,
            config.min_successful_graders,
        ),
        saturated_mean_confidence: mean_tester_confidence(
            testing_trials,
            grading_trials,
            config.min_successful_graders,
        ),
        support_minimality: 1.0,
        distractor_pressure: if distractor_hashes.is_empty() {
            0.0
        } else {
            0.80
        },
    }
}

pub(super) fn candidate_receipt(
    candidate: &GenerationCandidate,
    question: &str,
    distractor_hashes: &[String],
    prescreen_trials: Vec<TestingTrial>,
    prescreen_grading_trials: Vec<GradingTrial>,
    rejection_reasons: Vec<String>,
    rejection_category: Option<String>,
    accepted_for_full_tournament: bool,
) -> CandidateAttemptReceipt {
    CandidateAttemptReceipt {
        candidate_index: candidate.candidate_index,
        support_quote_id: candidate.support_quote_id.clone(),
        question: question.to_string(),
        support_quote_hash: candidate.support_quote_hash.clone(),
        support_quote_score: candidate.support_quote_score,
        stem_leakage_score: candidate.stem_leakage_score,
        distractor_hashes: distractor_hashes.to_vec(),
        prescreen_trials,
        prescreen_grading_trials,
        rejection_reasons,
        rejection_category,
        accepted_for_full_tournament,
    }
}

pub(super) fn candidate_failure(
    category: &str,
    phase: &str,
    agent_name: &str,
    error: impl Into<String>,
    fatal_for_acceptance: bool,
) -> AgentFailure {
    AgentFailure {
        category: category.to_string(),
        phase: phase.to_string(),
        agent_name: agent_name.to_string(),
        error: error.into(),
        fatal_for_acceptance,
        route_metadata: None,
        raw_output_hash: None,
    }
}

pub(super) fn rejection_category_from_reasons(
    reasons: &[String],
    failures: &[AgentFailure],
) -> String {
    if let Some(category) = failures
        .iter()
        .find(|failure| failure.fatal_for_acceptance)
        .map(|failure| failure.category.clone())
    {
        return category;
    }
    let joined = reasons.join(" ").to_ascii_lowercase();
    if joined.contains("prescreen") {
        "blind_too_easy_prescreen".to_string()
    } else if joined.contains("blind tester correct rate") {
        "blind_too_easy".to_string()
    } else if joined.contains("verifier") {
        "verifier_reject".to_string()
    } else if joined.contains("testing quorum") {
        "tester_schema".to_string()
    } else if joined.contains("generation quorum") {
        "generator_schema".to_string()
    } else {
        "paper_budget_exhausted".to_string()
    }
}

pub(super) fn rejected_candidate_outcome(
    config: &BuildPaperTournamentConfig,
    candidate: GenerationCandidate,
    support_section: PaperSection,
    quote: String,
    question: String,
    failures: Vec<AgentFailure>,
    category: &str,
    deterministic_paper_failure: bool,
) -> CandidateOutcome {
    rejected_candidate_outcome_with_distractors(
        config,
        candidate,
        support_section,
        quote,
        question,
        failures,
        Vec::new(),
        category,
        deterministic_paper_failure,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn rejected_candidate_outcome_with_distractors(
    config: &BuildPaperTournamentConfig,
    candidate: GenerationCandidate,
    support_section: PaperSection,
    quote: String,
    question: String,
    failures: Vec<AgentFailure>,
    distractor_hashes: Vec<String>,
    category: &str,
    deterministic_paper_failure: bool,
) -> CandidateOutcome {
    let errors = vec![category.to_string()];
    let receipt = candidate_receipt(
        &candidate,
        &question,
        &distractor_hashes,
        Vec::new(),
        Vec::new(),
        errors.clone(),
        Some(category.to_string()),
        false,
    );
    CandidateOutcome {
        candidate,
        support_section,
        quote: quote.clone(),
        question,
        answer: quote,
        distractor_hashes: distractor_hashes.clone(),
        verification_trials: Vec::new(),
        testing_trials: Vec::new(),
        grading_trials: Vec::new(),
        failures,
        candidate_attempt: receipt,
        metrics: candidate_metrics(&[], &[], &[], &distractor_hashes, config),
        accepted: false,
        errors,
        deterministic_paper_failure,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn rejected_prescreen_outcome(
    config: &BuildPaperTournamentConfig,
    candidate: GenerationCandidate,
    support_section: PaperSection,
    quote: String,
    question: String,
    failures: Vec<AgentFailure>,
    distractor_hashes: Vec<String>,
    receipt: CandidateAttemptReceipt,
    category: &str,
) -> CandidateOutcome {
    CandidateOutcome {
        candidate,
        support_section,
        quote: quote.clone(),
        question,
        answer: quote,
        distractor_hashes: distractor_hashes.clone(),
        verification_trials: Vec::new(),
        testing_trials: Vec::new(),
        grading_trials: Vec::new(),
        failures,
        candidate_attempt: receipt,
        metrics: candidate_metrics(&[], &[], &[], &distractor_hashes, config),
        accepted: false,
        errors: vec![category.to_string()],
        deterministic_paper_failure: false,
    }
}

pub(crate) fn ensure_paper_time_remaining(
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
                "error_category": "paper_budget_exhausted"
            }),
        )?;
        return Err(format!(
            "paper {} exceeded timeout of {} seconds",
            paper.publication_hash, config.paper_timeout_seconds
        ));
    }
    Ok(())
}

pub(crate) fn progress_jsonl_path(config: &BuildPaperTournamentConfig) -> PathBuf {
    match config.progress_jsonl.clone() {
        Some(value) => value,
        None => config.run_root.join("reports").join("live-progress.jsonl"),
    }
}

pub(crate) fn append_progress_row(
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
