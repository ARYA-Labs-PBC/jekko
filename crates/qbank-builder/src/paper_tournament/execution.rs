use super::*;

pub(crate) struct TournamentWriteResult {
    pub(crate) challenge: ChallengeRecord,
    pub(crate) artifact_path: PathBuf,
    pub(crate) challenge_path: PathBuf,
    pub(crate) accepted: bool,
    pub(crate) errors: Vec<String>,
}

pub(crate) fn run_single_paper(
    config: &BuildPaperTournamentConfig,
    paper: &PaperRecord,
    all_papers: &[PaperRecord],
    non_production: bool,
) -> Result<TournamentWriteResult, String> {
    if matches!(config.agent_runner, AgentRunnerMode::Jnoccio) {
        return run_single_paper_jnoccio(config, paper, all_papers);
    }
    validate_full_text_paper(paper, config.strict_production && !non_production)?;
    let support_section = match paper
        .sections
        .iter()
        .find(|section| section.section_id != "abstract" && section.section_id != "source")
    {
        Some(section) => section,
        None => match paper.sections.first() {
            Some(section) => section,
            None => return Err("paper has no sections".to_string()),
        },
    };
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
                required_key_points: vec![quote.clone(), quote.clone(), quote.clone()],
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
            category: "generator_support".to_string(),
            phase: "testing".to_string(),
            agent_name: "prompt-builder".to_string(),
            error: "answer-key metadata leaked into testing prompt".to_string(),
            fatal_for_acceptance: true,
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
    let accepted = verifier_acceptance
        && valid_generators >= config.min_successful_generators
        && valid_verifiers >= config.min_successful_verifiers
        && counted_testers >= config.min_successful_testers
        && tester_correct_rate <= HARD_MAX_TESTER_CORRECT_RATE
        && !failures.iter().any(|failure| failure.fatal_for_acceptance);

    let canonical_text = canonical_paper_text(paper, non_production);
    let artifact_provenance = provenance(config, paper);
    let metrics = AcceptanceMetrics {
        focused_agreement: accepted_ratio(&verification_trials),
        focused_correct_rate: accepted_ratio(&verification_trials),
        answerability: accepted_ratio(&verification_trials),
        saturated_blind_correct_rate: tester_correct_rate,
        saturated_mean_confidence: mean_tester_confidence(
            &testing_trials,
            &grading_trials,
            config.min_successful_graders,
        ),
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
            .unwrap_or("generator-1".to_string()),
        generation_trials: generation_trials.clone(),
        verification_trials: verification_trials.clone(),
        testing_trials: testing_trials.clone(),
        grading_trials: grading_trials.clone(),
        failures,
        candidate_attempts: Vec::new(),
        selected_candidate_index: Some(0),
        paper_rejection_category: None,
        rejection_reasons: Vec::new(),
        production_errors: Vec::new(),
        acceptance_metrics: metrics.clone(),
        artifact_hash: String::new(),
    };

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
        tournament_rejection_reasons(
            valid_generators,
            valid_verifiers,
            counted_testers,
            verifier_acceptance,
            tester_correct_rate,
            &final_artifact.failures,
            config,
        )
    };
    let accepted = accepted && errors.is_empty();
    final_artifact.rejection_reasons = if accepted { Vec::new() } else { errors.clone() };
    final_artifact.production_errors = if accepted && !non_production {
        Vec::new()
    } else if non_production {
        Vec::new()
    } else {
        super::production_acceptance_errors(&challenge)
    };
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

    Ok(TournamentWriteResult {
        challenge,
        artifact_path,
        challenge_path,
        accepted,
        errors,
    })
}
