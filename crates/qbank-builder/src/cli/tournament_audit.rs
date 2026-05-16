use super::*;

pub fn audit_paper_tournament_command(args: &[String]) -> Result<(), String> {
    let bank = path_value(args, "--bank").ok_or("--bank is required")?;
    let run_root = path_value(args, "--run-root").ok_or("--run-root is required")?;
    let allow_mock_smoke = args.iter().any(|arg| arg == "--allow-mock-smoke");
    let mut artifact_paths = Vec::new();
    qbank_builder::collect_json_files(&run_root.join("trials"), &mut artifact_paths)?;
    artifact_paths
        .retain(|path| path.file_name().and_then(|name| name.to_str()) == Some("final.json"));
    let mut errors = Vec::new();
    let mut rows = Vec::new();
    for path in &artifact_paths {
        let artifact: FinalPaperChallengeArtifact = read_json(path)?;
        let challenge_hash = path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();
        if challenge_hash.trim().is_empty() {
            errors.push(format!(
                "{} missing challenge hash path component",
                path.display()
            ));
        }
        if artifact.paper_content.non_production && !allow_mock_smoke {
            errors.push(format!(
                "{} is mock/non-production and requires --allow-mock-smoke",
                path.display()
            ));
        }
        if artifact
            .artifact_provenance
            .as_ref()
            .map(|provenance| provenance.fixture_provenance)
            .unwrap_or(false)
            && !allow_mock_smoke
        {
            errors.push(format!(
                "{} has fixture provenance and requires --allow-mock-smoke",
                path.display()
            ));
        }
        if final_paper_challenge_artifact_hash(&artifact)? != artifact.artifact_hash {
            errors.push(format!("{} artifact_hash mismatch", path.display()));
        }
        if artifact.generation_trials.len() < MIN_SUCCESSFUL_GENERATORS
            || artifact.verification_trials.len() < MIN_SUCCESSFUL_VERIFIERS
            || artifact.testing_trials.len() < MIN_SUCCESSFUL_TESTERS
        {
            errors.push(format!(
                "{} missing minimum successful trial quorum",
                path.display()
            ));
        }
        let tester_grader_quorum = artifact
            .testing_trials
            .iter()
            .filter(|trial| {
                qbank_builder::grade_reduction(&artifact.grading_trials, &trial.agent_name)
                    .is_some()
            })
            .count();
        if tester_grader_quorum < MIN_SUCCESSFUL_TESTERS {
            errors.push(format!(
                "{} has {tester_grader_quorum} tester outputs with grader quorum; need {MIN_SUCCESSFUL_TESTERS}",
                path.display()
            ));
            errors.push(format!(
                "{} grader quorum requires {MIN_SUCCESSFUL_GRADERS} valid graders per counted tester",
                path.display()
            ));
        }
        if artifact
            .failures
            .iter()
            .any(|failure| failure.fatal_for_acceptance)
        {
            errors.push(format!("{} has fatal persisted failures", path.display()));
        }
        let calibrated_confidence = calibrated_artifact_confidence(&artifact);
        if (artifact.acceptance_metrics.saturated_mean_confidence - calibrated_confidence).abs()
            > 0.000_001
        {
            errors.push(format!(
                "{} saturated confidence does not recompute from raw tester/grader trials",
                path.display()
            ));
        }
        if !artifact
            .paper_content
            .full_text
            .contains(&artifact.hard_answer)
        {
            errors.push(format!("{} answer absent from full text", path.display()));
        }
        if artifact.hard_question.trim().is_empty()
            || artifact.hard_answer.trim().is_empty()
            || artifact.hard_agent_name.trim().is_empty()
        {
            errors.push(format!(
                "{} missing final hard challenge fields",
                path.display()
            ));
        }
        let hard_generation = artifact
            .generation_trials
            .iter()
            .find(|trial| trial.agent_name == artifact.hard_agent_name);
        match hard_generation.and_then(|trial| trial.output.support.first()) {
            Some(support) if support.quote == artifact.hard_answer => {}
            Some(_) => errors.push(format!(
                "{} hard generator support quote mismatch",
                path.display()
            )),
            None => errors.push(format!(
                "{} missing hard generator support quote",
                path.display()
            )),
        }
        for trial in &artifact.generation_trials {
            for support in &trial.output.support {
                let known = artifact.paper_content.sections.iter().any(|section| {
                    section.section_id == support.section_id
                        && section.section_hash == support.section_hash
                        && section.text.contains(&support.quote)
                });
                if !known {
                    errors.push(format!(
                        "{} generator support does not match canonical full text",
                        path.display()
                    ));
                }
            }
        }
        let challenge_path = bank
            .join("challenges")
            .join(format!("{challenge_hash}.json"));
        if !challenge_path.exists() {
            errors.push(format!(
                "{} missing matching accepted challenge",
                challenge_path.display()
            ));
            continue;
        }
        let challenge: ChallengeRecord = read_json(&challenge_path)?;
        if challenge.challenge_hash != challenge_hash {
            errors.push(format!(
                "{} challenge_hash does not match artifact path",
                challenge_path.display()
            ));
        }
        if challenge.answer_key.canonical != artifact.hard_answer {
            errors.push(format!(
                "{} challenge answer mismatch",
                challenge_path.display()
            ));
        }
        if challenge.question != artifact.hard_question {
            errors.push(format!(
                "{} challenge question mismatch",
                challenge_path.display()
            ));
        }
        if challenge.route_metadata.len()
            < MIN_SUCCESSFUL_GENERATORS + MIN_SUCCESSFUL_VERIFIERS + MIN_SUCCESSFUL_TESTERS
        {
            errors.push(format!(
                "{} missing minimum top-level route metadata records",
                challenge_path.display()
            ));
        }
        if challenge
            .artifact_provenance
            .as_ref()
            .map(|provenance| provenance.fixture_provenance)
            .unwrap_or(false)
            && !allow_mock_smoke
        {
            errors.push(format!(
                "{} has mock provenance and requires --allow-mock-smoke",
                challenge_path.display()
            ));
        }
        for (index, route) in challenge.route_metadata.iter().enumerate() {
            audit_route_metadata(
                &format!("{} route_metadata[{index}]", challenge_path.display()),
                route,
                &mut errors,
            );
        }
        rows.push(json!({
            "artifact": path.display().to_string(),
            "challenge_hash": challenge_hash,
            "paper_hash": artifact.paper_hash,
            "title": artifact.paper_content.title,
            "hard_answer": artifact.hard_answer,
            "generation_trials": artifact.generation_trials.len(),
            "verification_trials": artifact.verification_trials.len(),
            "testing_trials": artifact.testing_trials.len(),
            "grading_trials": artifact.grading_trials.len(),
            "tester_grader_quorum": tester_grader_quorum,
            "failures": artifact.failures.len(),
            "fatal_failures": artifact.failures.iter().filter(|failure| failure.fatal_for_acceptance).count(),
            "rejection_reasons": artifact.rejection_reasons,
            "production_errors": artifact.production_errors,
            "route_metadata": challenge.route_metadata.len(),
            "domain": challenge.domain,
        }));
    }
    let report = json!({
        "run_root": run_root.display().to_string(),
        "bank": bank.display().to_string(),
        "allow_mock_smoke": allow_mock_smoke,
        "artifacts": artifact_paths.len(),
        "errors": errors,
        "rows": rows,
    });
    write_json_pretty(
        &run_root.join("reports/paper-tournament-audit.json"),
        &report,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?
    );
    if report
        .get("errors")
        .and_then(|value| value.as_array())
        .map(|items| !items.is_empty())
        .unwrap_or(false)
    {
        return Err("paper tournament audit failed".to_string());
    }
    Ok(())
}

fn calibrated_artifact_confidence(artifact: &FinalPaperChallengeArtifact) -> f64 {
    let counted = artifact
        .testing_trials
        .iter()
        .filter_map(|trial| {
            qbank_builder::grade_reduction(&artifact.grading_trials, &trial.agent_name).map(
                |(correct, _)| {
                    if correct {
                        trial.output.confidence as f64 / 100.0
                    } else {
                        0.0
                    }
                },
            )
        })
        .collect::<Vec<_>>();
    if counted.is_empty() {
        return 1.0;
    }
    counted.iter().sum::<f64>() / counted.len() as f64
}

fn audit_route_metadata(
    label: &str,
    route: &qbank_builder::RouteMetadata,
    errors: &mut Vec<String>,
) {
    if route.request_id.trim().is_empty() {
        errors.push(format!("{label} missing request_id"));
    }
    if route.provider.trim().is_empty() || route.model.trim().is_empty() {
        errors.push(format!("{label} missing provider/model"));
    }
    if route.prompt_hash.as_deref().unwrap_or("").trim().is_empty()
        || route
            .context_hash
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        || route
            .receipts_hash
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        || route
            .model_decisions_hash
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        errors.push(format!("{label} missing route hashes"));
    }
    if route.token_usage.is_none() {
        errors.push(format!("{label} missing token_usage"));
    }
    if route
        .winner_model_id
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        errors.push(format!("{label} missing winner_model_id"));
    }
    if route.model_decisions.is_empty() {
        errors.push(format!("{label} missing model_decisions"));
    }
}
