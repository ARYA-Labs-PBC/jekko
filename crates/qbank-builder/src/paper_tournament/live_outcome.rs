pub(super) use super::live_outcome_support::{
    append_progress_row, candidate_failure, candidate_metrics, candidate_receipt,
    ensure_paper_time_remaining, has_confident_correct_answer, progress_jsonl_path,
    rejected_candidate_outcome, rejected_candidate_outcome_with_distractors,
    rejected_prescreen_outcome, rejection_category_from_reasons, stem_leakage_score,
};
use super::*;

#[derive(Debug, Clone)]
pub(super) struct GenerationCandidate {
    pub(super) candidate_index: usize,
    pub(super) support_quote_id: String,
    pub(super) support_quote_score: i32,
    pub(super) support_quote_hash: String,
    pub(super) stem_leakage_score: f64,
    pub(super) candidate_hash: String,
    pub(super) trial: GeneratorTrial,
}

#[derive(Debug, Clone)]
pub(super) struct CandidateOutcome {
    pub(super) candidate: GenerationCandidate,
    pub(super) support_section: PaperSection,
    pub(super) quote: String,
    pub(super) question: String,
    pub(super) answer: String,
    pub(super) distractor_hashes: Vec<String>,
    pub(super) verification_trials: Vec<VerificationTrial>,
    pub(super) testing_trials: Vec<TestingTrial>,
    pub(super) grading_trials: Vec<GradingTrial>,
    pub(super) failures: Vec<AgentFailure>,
    pub(super) candidate_attempt: CandidateAttemptReceipt,
    pub(super) metrics: AcceptanceMetrics,
    pub(super) accepted: bool,
    pub(super) errors: Vec<String>,
    pub(super) deterministic_paper_failure: bool,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_testing_phase(
    config: &BuildPaperTournamentConfig,
    paper: &PaperRecord,
    all_papers: &[PaperRecord],
    runner: &JnoccioHttpRunner,
    paper_started: Instant,
    question: &str,
    answer: &str,
    distractor_hashes: &[String],
    tester_limit: usize,
    grader_limit: usize,
    agent_prefix: &str,
) -> Result<(Vec<TestingTrial>, Vec<GradingTrial>, Vec<AgentFailure>), String> {
    let mut failures = Vec::new();
    let testing_prompt = build_testing_prompt_with_options(
        paper,
        all_papers,
        distractor_hashes,
        question,
        config.mask_blind_context_metadata,
    );
    if testing_prompt.contains("Answer key:")
        || testing_prompt.contains("hard_answer")
        || testing_prompt.contains("verification_trials")
    {
        failures.push(AgentFailure {
            category: "generator_support".to_string(),
            phase: "testing".to_string(),
            agent_name: "prompt-builder".to_string(),
            error: "answer-key metadata leaked into testing prompt".to_string(),
            fatal_for_acceptance: true,
            route_metadata: None,
            raw_output_hash: None,
        });
    }
    let mut testing_trials = Vec::new();
    for index in 0..tester_limit.max(1) {
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
                    agent_name: format!("{agent_prefix}-{}", index + 1),
                    distractor_paper_hashes: distractor_hashes.to_vec(),
                    output,
                    receipt,
                });
            }
            Err(err) => failures.push(live_call_failure("testing", index, err)),
        }
    }

    let mut grading_trials = Vec::new();
    for testing_trial in &testing_trials {
        for grader_index in 0..grader_limit.max(1) {
            ensure_paper_time_remaining(config, paper, paper_started)?;
            let prompt = grader_prompt(question, answer, &testing_trial.output.answer);
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
            if grade_reduction_with_min(
                &grading_trials,
                &testing_trial.agent_name,
                config.min_successful_graders.min(grader_limit.max(1)),
            )
            .is_some()
            {
                break;
            }
        }
    }
    Ok((testing_trials, grading_trials, failures))
}
