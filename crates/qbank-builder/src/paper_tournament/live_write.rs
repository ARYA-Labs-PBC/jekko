use super::live_outcome::{rejection_category_from_reasons, CandidateOutcome};
use super::*;

pub(super) fn write_final_artifact_from_outcome(
    config: &BuildPaperTournamentConfig,
    paper: &PaperRecord,
    outcome: CandidateOutcome,
    candidate_attempts: Vec<CandidateAttemptReceipt>,
) -> Result<TournamentWriteResult, String> {
    let generation_trials = vec![outcome.candidate.trial.clone()];
    let canonical_text = canonical_paper_text(paper, false);
    let mut final_artifact = FinalPaperChallengeArtifact {
        schema_version: FINAL_PAPER_CHALLENGE_SCHEMA_VERSION.to_string(),
        paper_hash: paper.publication_hash.clone(),
        paper_content: canonical_text,
        artifact_provenance: Some(provenance(config, paper)),
        hard_question: outcome.question.clone(),
        hard_answer: outcome.answer.clone(),
        hard_agent_name: outcome.candidate.trial.agent_name.clone(),
        generation_trials: generation_trials.clone(),
        verification_trials: outcome.verification_trials.clone(),
        testing_trials: outcome.testing_trials.clone(),
        grading_trials: outcome.grading_trials.clone(),
        failures: outcome.failures,
        candidate_attempts,
        selected_candidate_index: Some(outcome.candidate.candidate_index),
        paper_rejection_category: if outcome.accepted {
            None
        } else {
            Some(rejection_category_from_reasons(&outcome.errors, &[]))
        },
        rejection_reasons: Vec::new(),
        production_errors: Vec::new(),
        acceptance_metrics: outcome.metrics.clone(),
        artifact_hash: String::new(),
    };

    let challenge = challenge_from_artifact(
        paper,
        &outcome.support_section,
        &outcome.quote,
        &outcome.question,
        &outcome.answer,
        &generation_trials,
        &outcome.verification_trials,
        &outcome.testing_trials,
        &outcome.grading_trials,
        &outcome.metrics,
        outcome.accepted,
        config,
    )?;
    let errors = if outcome.accepted {
        super::production_acceptance_errors(&challenge)
    } else {
        outcome.errors.clone()
    };
    let accepted = outcome.accepted && errors.is_empty();
    final_artifact.rejection_reasons = if accepted { Vec::new() } else { errors.clone() };
    final_artifact.production_errors = super::production_acceptance_errors(&challenge);
    if accepted {
        final_artifact.production_errors.clear();
    }
    final_artifact.artifact_hash = final_paper_challenge_artifact_hash(&final_artifact)?;
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
    if !accepted {
        let report_path = config
            .bank
            .join("rejected")
            .join(format!("{}.report.json", challenge.challenge_hash));
        write_json_pretty(
            &report_path,
            &json!({
                "schema_version": "opencode-qbank-rejected-challenge-report-v1",
                "challenge_hash": challenge.challenge_hash,
                "paper_hash": paper.publication_hash,
                "rejection_reasons": final_artifact.rejection_reasons,
                "production_errors": final_artifact.production_errors,
                "paper_rejection_category": final_artifact.paper_rejection_category,
                "candidate_attempts": final_artifact.candidate_attempts,
                "failures": final_artifact.failures,
            }),
        )?;
    }

    Ok(TournamentWriteResult {
        challenge,
        artifact_path,
        challenge_path,
        accepted,
        errors,
    })
}
