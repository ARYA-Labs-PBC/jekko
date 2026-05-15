use super::*;

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
                && support_quote_candidates_with_min_score(paper, config.min_support_quote_score)
                    .len()
                    >= 3
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
            "generator_pool_target": config.generator_pool_target,
            "max_question_alternates_per_paper": config.max_question_alternates_per_paper,
            "blind_prescreen_testers": config.blind_prescreen_testers,
            "blind_prescreen_max_correct_rate": config.blind_prescreen_max_correct_rate,
            "min_support_quote_score": config.min_support_quote_score,
            "hard_distractors": config.hard_distractors,
            "mask_blind_context_metadata": config.mask_blind_context_metadata,
            "generators": config.generators,
            "verifiers": config.verifiers,
            "testers": config.testers,
            "graders": config.graders,
            "min_successful_generators": config.min_successful_generators,
            "min_successful_verifiers": config.min_successful_verifiers,
            "min_successful_testers": config.min_successful_testers,
            "min_successful_graders": config.min_successful_graders,
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
    write_failure_summary(config, &outputs)?;
    if config.write_rejection_analysis {
        write_rejection_analysis(config)?;
    }
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
