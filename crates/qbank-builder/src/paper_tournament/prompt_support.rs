use super::*;

pub(crate) fn provenance(
    config: &BuildPaperTournamentConfig,
    paper: &PaperRecord,
) -> ArtifactProvenance {
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

pub(crate) fn model_trial_from_verifier(trial: &VerificationTrial) -> ModelTrial {
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

pub(crate) fn model_trial_from_tester(
    trial: &TestingTrial,
    correct: bool,
    score: f64,
) -> ModelTrial {
    ModelTrial {
        agent_id: trial.agent_name.clone(),
        phase: "saturated_blind_testing".to_string(),
        correct,
        answerability: score / 100.0,
        supported: correct,
        confidence: if correct {
            trial.output.confidence as f64 / 100.0
        } else {
            0.0
        },
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

pub(crate) fn testing_correct_rate_with_min(
    testing_trials: &[TestingTrial],
    grading_trials: &[GradingTrial],
    min_successful_graders: usize,
) -> f64 {
    if testing_trials.is_empty() {
        return 1.0;
    }
    let counted = valid_testing_trials(testing_trials, grading_trials, min_successful_graders);
    if counted.is_empty() {
        return 1.0;
    }
    let correct = counted
        .iter()
        .filter(|trial| {
            grade_reduction_with_min(grading_trials, &trial.agent_name, min_successful_graders)
                .map(|(correct, _)| correct)
                .unwrap_or(false)
        })
        .count();
    correct as f64 / counted.len() as f64
}

pub(crate) fn accepted_ratio(trials: &[VerificationTrial]) -> f64 {
    let valid = valid_verification_trials(trials);
    if valid.is_empty() {
        return 0.0;
    }
    let accepted = valid
        .iter()
        .filter(|trial| trial.output.accepted && trial.output.support_correct)
        .count();
    accepted as f64 / valid.len() as f64
}

pub(crate) fn mean_tester_confidence(
    trials: &[TestingTrial],
    grading_trials: &[GradingTrial],
    min_successful_graders: usize,
) -> f64 {
    if trials.is_empty() {
        return 1.0;
    }
    let counted = valid_testing_trials(trials, grading_trials, min_successful_graders);
    if counted.is_empty() {
        return 1.0;
    }
    counted
        .iter()
        .map(|trial| {
            let (correct, _) =
                grade_reduction_with_min(grading_trials, &trial.agent_name, min_successful_graders)
                    .unwrap_or((false, 0.0));
            if correct {
                trial.output.confidence as f64 / 100.0
            } else {
                0.0
            }
        })
        .sum::<f64>()
        / counted.len() as f64
}
