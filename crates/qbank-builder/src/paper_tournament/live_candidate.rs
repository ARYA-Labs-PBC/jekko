use super::gating::{
    tournament_rejection_reasons, valid_generation_trials, valid_testing_trials,
    valid_verification_trials, verification_majority_with_min,
};
use super::live_outcome::{
    candidate_failure, candidate_metrics, candidate_receipt, has_confident_correct_answer,
    rejected_candidate_outcome, rejected_candidate_outcome_with_distractors,
    rejected_prescreen_outcome, rejection_category_from_reasons, run_testing_phase,
    CandidateOutcome, GenerationCandidate,
};
use super::*;

pub(crate) fn evaluate_candidate(
    config: &BuildPaperTournamentConfig,
    paper: &PaperRecord,
    all_papers: &[PaperRecord],
    candidate: GenerationCandidate,
    mut failures: Vec<AgentFailure>,
    runner: &JnoccioHttpRunner,
    paper_started: Instant,
) -> Result<CandidateOutcome, String> {
    let support_quote = candidate
        .trial
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
        .cloned()
        .ok_or("live generator support section is unknown")?;
    let question = candidate.trial.output.question.clone();
    let quote = support_quote.quote.trim().to_string();
    if !support_section.text.contains(&quote) {
        failures.push(candidate_failure(
            "generator_support",
            "generation",
            &candidate.trial.agent_name,
            "live generator support quote is absent from canonical full text",
            true,
        ));
        return Ok(rejected_candidate_outcome(
            config,
            candidate,
            support_section,
            quote,
            question,
            failures,
            "generator_support",
            true,
        ));
    }
    let answer = quote.clone();
    let distractor_hashes = if config.hard_distractors {
        select_hard_distractors(
            paper,
            all_papers,
            &quote,
            &question,
            config.distractor_papers,
        )
    } else {
        select_distractors(paper, all_papers, config.distractor_papers)
    };
    if config.strict_production && candidate.stem_leakage_score > 0.22 {
        failures.push(candidate_failure(
            "stem_leakage",
            "generation",
            &candidate.trial.agent_name,
            format!(
                "stem leakage score {:.3} exceeds 0.220",
                candidate.stem_leakage_score
            ),
            true,
        ));
        return Ok(rejected_candidate_outcome_with_distractors(
            config,
            candidate,
            support_section,
            quote,
            question,
            failures,
            distractor_hashes,
            "stem_leakage",
            false,
        ));
    }

    let (prescreen_trials, prescreen_grading_trials, mut prescreen_failures) = run_testing_phase(
        config,
        paper,
        all_papers,
        runner,
        paper_started,
        &question,
        &answer,
        &distractor_hashes,
        config.blind_prescreen_testers.max(1),
        3,
        "prescreen-tester",
    )?;
    failures.append(&mut prescreen_failures);
    let prescreen_counted = valid_testing_trials(
        &prescreen_trials,
        &prescreen_grading_trials,
        config.min_successful_graders.min(3),
    )
    .len();
    let prescreen_correct_rate = testing_correct_rate_with_min(
        &prescreen_trials,
        &prescreen_grading_trials,
        config.min_successful_graders.min(3),
    );
    if prescreen_counted < 2 {
        failures.push(candidate_failure(
            "tester_schema",
            "testing",
            "prescreen",
            "prescreen counted fewer than 2 tester outputs",
            true,
        ));
        let receipt = candidate_receipt(
            &candidate,
            &question,
            &distractor_hashes,
            prescreen_trials,
            prescreen_grading_trials,
            vec!["prescreen counted fewer than 2 tester outputs".to_string()],
            Some("tester_schema".to_string()),
            false,
        );
        return Ok(rejected_prescreen_outcome(
            config,
            candidate,
            support_section,
            quote,
            question,
            failures,
            distractor_hashes,
            receipt,
            "tester_schema",
        ));
    }
    if prescreen_correct_rate > config.blind_prescreen_max_correct_rate
        || has_confident_correct_answer(
            &prescreen_trials,
            &prescreen_grading_trials,
            config.min_successful_graders.min(3),
        )
    {
        failures.push(candidate_failure(
            "blind_too_easy_prescreen",
            "testing",
            "prescreen",
            format!(
                "blind prescreen correct rate {prescreen_correct_rate:.3} exceeds {:.3}",
                config.blind_prescreen_max_correct_rate
            ),
            true,
        ));
        let receipt = candidate_receipt(
            &candidate,
            &question,
            &distractor_hashes,
            prescreen_trials,
            prescreen_grading_trials,
            vec![format!(
                "blind prescreen correct rate {prescreen_correct_rate:.3} exceeds {:.3}",
                config.blind_prescreen_max_correct_rate
            )],
            Some("blind_too_easy_prescreen".to_string()),
            false,
        );
        return Ok(rejected_prescreen_outcome(
            config,
            candidate,
            support_section,
            quote,
            question,
            failures,
            distractor_hashes,
            receipt,
            "blind_too_easy_prescreen",
        ));
    }

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
        if verification_majority_with_min(&verification_trials, config.min_successful_verifiers) {
            break;
        }
    }

    let (testing_trials, grading_trials, mut testing_failures) = run_testing_phase(
        config,
        paper,
        all_papers,
        runner,
        paper_started,
        &question,
        &answer,
        &distractor_hashes,
        config.testers.max(1),
        config.graders.max(1),
        "tester",
    )?;
    failures.append(&mut testing_failures);

    let generation_trials = vec![candidate.trial.clone()];
    let valid_generators = valid_generation_trials(&generation_trials, paper).len();
    let valid_verifiers = valid_verification_trials(&verification_trials).len();
    let counted_testers = valid_testing_trials(
        &testing_trials,
        &grading_trials,
        config.min_successful_graders,
    )
    .len();
    let verifier_acceptance =
        verification_majority_with_min(&verification_trials, config.min_successful_verifiers);
    let tester_correct_rate = testing_correct_rate_with_min(
        &testing_trials,
        &grading_trials,
        config.min_successful_graders,
    );
    let mut accepted = verifier_acceptance
        && valid_generators >= config.min_successful_generators
        && valid_verifiers >= config.min_successful_verifiers
        && counted_testers >= config.min_successful_testers
        && tester_correct_rate <= HARD_MAX_TESTER_CORRECT_RATE
        && !failures.iter().any(|failure| failure.fatal_for_acceptance);
    let metrics = candidate_metrics(
        &verification_trials,
        &testing_trials,
        &grading_trials,
        &distractor_hashes,
        config,
    );
    let errors = if accepted {
        Vec::new()
    } else {
        tournament_rejection_reasons(
            valid_generators,
            valid_verifiers,
            counted_testers,
            verifier_acceptance,
            tester_correct_rate,
            &failures,
            config,
        )
    };
    if !errors.is_empty() {
        accepted = false;
    }
    let category = rejection_category_from_reasons(&errors, &failures);
    let candidate_attempt = candidate_receipt(
        &candidate,
        &question,
        &distractor_hashes,
        prescreen_trials,
        prescreen_grading_trials,
        errors.clone(),
        if accepted {
            None
        } else {
            Some(category.clone())
        },
        true,
    );
    Ok(CandidateOutcome {
        candidate,
        support_section,
        quote,
        question,
        answer,
        distractor_hashes,
        verification_trials,
        testing_trials,
        grading_trials,
        failures,
        candidate_attempt,
        metrics,
        accepted,
        errors,
        deterministic_paper_failure: false,
    })
}
