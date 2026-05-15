use super::*;

pub(crate) fn tournament_rejection_reasons(
    valid_generators: usize,
    valid_verifiers: usize,
    counted_testers: usize,
    verifier_acceptance: bool,
    tester_correct_rate: f64,
    failures: &[AgentFailure],
    config: &BuildPaperTournamentConfig,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if valid_generators < config.min_successful_generators {
        reasons.push(format!(
            "generation quorum missing: {valid_generators}/{} valid generator outputs",
            config.min_successful_generators
        ));
    }
    if valid_verifiers < config.min_successful_verifiers {
        reasons.push(format!(
            "verification quorum missing: {valid_verifiers}/{} valid verifier outputs",
            config.min_successful_verifiers
        ));
    }
    if !verifier_acceptance {
        reasons.push("verifier majority did not accept support".to_string());
    }
    if counted_testers < config.min_successful_testers {
        reasons.push(format!(
            "testing quorum missing: {counted_testers}/{} tester outputs with grader quorum",
            config.min_successful_testers
        ));
    }
    if tester_correct_rate > HARD_MAX_TESTER_CORRECT_RATE {
        reasons.push(format!(
            "blind tester correct rate {tester_correct_rate:.3} exceeds {HARD_MAX_TESTER_CORRECT_RATE:.3}"
        ));
    }
    let fatal_failures = failures
        .iter()
        .filter(|failure| failure.fatal_for_acceptance)
        .map(|failure| {
            format!(
                "{}:{}:{}",
                failure.category, failure.phase, failure.agent_name
            )
        })
        .collect::<Vec<_>>();
    if !fatal_failures.is_empty() {
        reasons.push(format!(
            "fatal failure categories present: {}",
            fatal_failures.join(", ")
        ));
    }
    if reasons.is_empty() {
        reasons.push("tournament acceptance gates failed".to_string());
    }
    reasons
}

pub fn verification_majority(trials: &[VerificationTrial]) -> bool {
    verification_majority_with_min(trials, MIN_SUCCESSFUL_VERIFIERS)
}

pub(crate) fn verification_majority_with_min(
    trials: &[VerificationTrial],
    min_successful: usize,
) -> bool {
    let valid = valid_verification_trials(trials);
    let accepted = valid
        .iter()
        .filter(|trial| trial.output.accepted && trial.output.support_correct)
        .count();
    accepted >= min_successful && accepted * 2 > valid.len()
}

pub fn grade_reduction(trials: &[GradingTrial], testing_agent_name: &str) -> Option<(bool, f64)> {
    grade_reduction_with_min(trials, testing_agent_name, MIN_SUCCESSFUL_GRADERS)
}

pub(crate) fn grade_reduction_with_min(
    trials: &[GradingTrial],
    testing_agent_name: &str,
    min_successful: usize,
) -> Option<(bool, f64)> {
    let matching = trials
        .iter()
        .filter(|trial| {
            trial.testing_agent_name == testing_agent_name
                && validate_grading_output(&trial.output).is_ok()
        })
        .collect::<Vec<_>>();
    if matching.len() < min_successful {
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

pub(crate) fn valid_generation_trials<'a>(
    trials: &'a [GeneratorTrial],
    paper: &PaperRecord,
) -> Vec<&'a GeneratorTrial> {
    trials
        .iter()
        .filter(|trial| validate_generator_output(&trial.output, paper).is_ok())
        .collect()
}

pub(crate) fn valid_verification_trials(trials: &[VerificationTrial]) -> Vec<&VerificationTrial> {
    trials
        .iter()
        .filter(|trial| validate_verification_output(&trial.output).is_ok())
        .collect()
}

pub(crate) fn valid_testing_trials<'a>(
    testing_trials: &'a [TestingTrial],
    grading_trials: &[GradingTrial],
    min_successful_graders: usize,
) -> Vec<&'a TestingTrial> {
    testing_trials
        .iter()
        .filter(|trial| {
            validate_testing_output(&trial.output).is_ok()
                && grade_reduction_with_min(
                    grading_trials,
                    &trial.agent_name,
                    min_successful_graders,
                )
                .is_some()
        })
        .collect()
}
