use super::live_candidate::evaluate_candidate;
use super::live_outcome::{
    candidate_failure, candidate_metrics, candidate_receipt, has_confident_correct_answer,
    rejected_candidate_outcome, rejected_candidate_outcome_with_distractors,
    rejected_prescreen_outcome, rejection_category_from_reasons, run_testing_phase,
    stem_leakage_score, CandidateOutcome, GenerationCandidate,
};
use super::live_write::write_final_artifact_from_outcome;
use super::*;

pub(crate) fn run_single_paper_jnoccio(
    config: &BuildPaperTournamentConfig,
    paper: &PaperRecord,
    all_papers: &[PaperRecord],
) -> Result<TournamentWriteResult, String> {
    let paper_started = Instant::now();
    validate_full_text_paper(paper, true)?;
    let runner = JnoccioHttpRunner::new(config)?;
    let quote_candidates =
        support_quote_candidates_with_min_score(paper, config.min_support_quote_score);
    if quote_candidates.is_empty() {
        return Err("paper has no eligible support quote candidates".to_string());
    }

    let mut failures = Vec::new();
    let mut ranked = collect_generation_pool(
        config,
        paper,
        &quote_candidates,
        &runner,
        paper_started,
        &mut failures,
    )?;
    ranked.sort_by(|left, right| {
        left.stem_leakage_score
            .partial_cmp(&right.stem_leakage_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.support_quote_score.cmp(&left.support_quote_score))
            .then_with(|| left.candidate_hash.cmp(&right.candidate_hash))
    });
    if ranked.is_empty() {
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
        return Err(if details.is_empty() {
            "no valid live generator output".to_string()
        } else {
            format!("no valid live generator output: {details}")
        });
    }

    let mut candidate_attempts = Vec::new();
    let mut best_rejected = None;
    for candidate in ranked
        .into_iter()
        .take(config.max_question_alternates_per_paper.max(1))
    {
        ensure_paper_time_remaining(config, paper, paper_started)?;
        let outcome = evaluate_candidate(
            config,
            paper,
            all_papers,
            candidate,
            failures.clone(),
            &runner,
            paper_started,
        )?;
        candidate_attempts.push(outcome.candidate_attempt.clone());
        if outcome.accepted {
            return write_final_artifact_from_outcome(config, paper, outcome, candidate_attempts);
        }
        if outcome.deterministic_paper_failure {
            best_rejected = Some(outcome);
            break;
        }
        best_rejected = Some(outcome);
    }
    let outcome = best_rejected.ok_or("no candidate was evaluated")?;
    write_final_artifact_from_outcome(config, paper, outcome, candidate_attempts)
}

fn collect_generation_pool(
    config: &BuildPaperTournamentConfig,
    paper: &PaperRecord,
    quote_candidates: &[SupportQuoteCandidate],
    runner: &JnoccioHttpRunner,
    paper_started: Instant,
    failures: &mut Vec<AgentFailure>,
) -> Result<Vec<GenerationCandidate>, String> {
    let mut candidates = Vec::new();
    for index in 0..config.generators.max(1) {
        ensure_paper_time_remaining(config, paper, paper_started)?;
        let prompt = generator_prompt(paper, index, quote_candidates);
        match runner.call_json::<GeneratorSelectionOutput>(
            "generator",
            index,
            &prompt,
            generator_selection_response_schema(),
        ) {
            Ok((selection, receipt)) => {
                let support_quote_id = selection.support_quote_id.clone();
                let output = generator_output_from_selection(
                    selection,
                    quote_candidates,
                    failures,
                    &receipt,
                );
                let trial = GeneratorTrial {
                    agent_name: format!("generator-{}", index + 1),
                    output,
                    receipt: receipt.clone(),
                };
                match validate_generator_output(&trial.output, paper) {
                    Ok(()) => {
                        let support = trial.output.support.first().expect("validated support");
                        let quote = support.quote.trim();
                        let matched_quote_score = quote_candidates
                            .iter()
                            .find(|candidate| candidate.id == support_quote_id)
                            .map(|candidate| candidate.score);
                        let support_quote_score = match matched_quote_score {
                            Some(score) => score,
                            None => support_quote_hardness_score("", quote),
                        };
                        candidates.push(GenerationCandidate {
                            candidate_index: index,
                            support_quote_id,
                            support_quote_score,
                            support_quote_hash: sha256_hex(quote.as_bytes()),
                            stem_leakage_score: stem_leakage_score(&trial.output.question, quote),
                            candidate_hash: sha256_hex(
                                format!("{}:{quote}", trial.output.question).as_bytes(),
                            ),
                            trial,
                        });
                    }
                    Err(err) => {
                        failures.push(failure("generation", &receipt.agent_name, err, &receipt))
                    }
                }
            }
            Err(err) => failures.push(live_call_failure("generation", index, err)),
        }
        if candidates.len()
            >= config
                .generator_pool_target
                .max(config.min_successful_generators)
        {
            break;
        }
    }
    Ok(candidates)
}
