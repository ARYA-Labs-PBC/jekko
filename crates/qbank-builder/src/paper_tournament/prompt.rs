use super::prompt_support::{model_trial_from_tester, model_trial_from_verifier};
use super::*;

pub fn build_testing_prompt(
    paper: &PaperRecord,
    all_papers: &[PaperRecord],
    distractor_paper_hashes: &[String],
    question: &str,
) -> String {
    build_testing_prompt_with_options(paper, all_papers, distractor_paper_hashes, question, false)
}

pub(crate) fn build_testing_prompt_with_options(
    paper: &PaperRecord,
    all_papers: &[PaperRecord],
    distractor_paper_hashes: &[String],
    question: &str,
    mask_metadata: bool,
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
        let paper_label = paper_label(paper_index);
        let header = if mask_metadata {
            format!("\nPaper {paper_label}\n")
        } else {
            format!(
                "\nPaper {}: {}\nPublication hash: {}\n",
                paper_index + 1,
                context_paper.title,
                context_paper.publication_hash
            )
        };
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
            let block = if mask_metadata {
                format!("[Paper {paper_label} section]\n{}\n\n", section.text)
            } else {
                format!(
                    "[{}:{}]\n{}\n\n",
                    context_paper.publication_hash, section.section_id, section.text
                )
            };
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

fn paper_label(index: usize) -> String {
    let letter = ((index % 26) as u8 + b'A') as char;
    if index < 26 {
        letter.to_string()
    } else {
        format!("{}{}", letter, index / 26)
    }
}

pub(crate) fn challenge_from_artifact(
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
        .filter(|trial| {
            validate_testing_output(&trial.output).is_ok()
                && grade_reduction_with_min(
                    grading_trials,
                    &trial.agent_name,
                    config.min_successful_graders,
                )
                .is_some()
        })
        .map(|trial| {
            let (correct, score) = grade_reduction_with_min(
                grading_trials,
                &trial.agent_name,
                config.min_successful_graders,
            )
            .unwrap_or((false, 0.0));
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
            must_include: match generation_trials
                .first()
                .map(|trial| trial.output.required_key_points.clone())
                .filter(|points| !points.is_empty())
            {
                Some(points) => points,
                None => vec![answer.to_string()],
            },
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
