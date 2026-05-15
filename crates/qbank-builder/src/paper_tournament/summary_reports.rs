use super::*;

pub(crate) fn write_rejection_analysis(config: &BuildPaperTournamentConfig) -> Result<(), String> {
    let mut artifacts = Vec::new();
    collect_json_files(&config.run_root.join("trials"), &mut artifacts)?;
    artifacts.retain(|path| path.file_name().and_then(|name| name.to_str()) == Some("final.json"));

    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut failed = 0usize;
    let mut rejection_counts = BTreeMap::<String, usize>::new();
    let mut phase_execution_failures = BTreeMap::<String, usize>::new();
    let mut route_model_distribution = BTreeMap::<String, BTreeMap<String, usize>>::new();
    let mut blind_histogram = BTreeMap::<String, usize>::new();
    let mut verifier_histogram = BTreeMap::<String, usize>::new();
    let mut top_rejected = Vec::new();
    let mut candidate_attempt_counts = Vec::new();

    for path in artifacts {
        let artifact: FinalPaperChallengeArtifact = read_json_silent(&path)?;
        let artifact_accepted =
            artifact.rejection_reasons.is_empty() && artifact.production_errors.is_empty();
        if artifact_accepted {
            accepted += 1;
        } else {
            rejected += 1;
        }
        if !artifact.rejection_reasons.is_empty() && artifact.generation_trials.is_empty() {
            failed += 1;
        }
        let category = match artifact.paper_rejection_category.clone() {
            Some(category) => category,
            None => match artifact.rejection_reasons.first() {
                Some(reason) => rejection_category(reason).to_string(),
                None => "accepted".to_string(),
            },
        };
        if !artifact_accepted {
            *rejection_counts.entry(category.clone()).or_default() += 1;
        }
        for failure in &artifact.failures {
            *phase_execution_failures
                .entry(format!("{}:{}", failure.phase, failure.category))
                .or_default() += 1;
        }
        for receipt in artifact
            .generation_trials
            .iter()
            .map(|trial| &trial.receipt)
            .chain(
                artifact
                    .verification_trials
                    .iter()
                    .map(|trial| &trial.receipt),
            )
            .chain(artifact.testing_trials.iter().map(|trial| &trial.receipt))
            .chain(artifact.grading_trials.iter().map(|trial| &trial.receipt))
        {
            if let Some(route) = receipt.route_metadata.as_ref() {
                let phase = receipt.phase.clone();
                let model = match route.winner_model_id.clone() {
                    Some(model) => model,
                    None => route.model.clone(),
                };
                *route_model_distribution
                    .entry(phase)
                    .or_default()
                    .entry(model)
                    .or_default() += 1;
            }
        }
        let blind_bucket = rate_bucket(artifact.acceptance_metrics.saturated_blind_correct_rate);
        *blind_histogram.entry(blind_bucket).or_default() += 1;
        let verifier_bucket = rate_bucket(artifact.acceptance_metrics.focused_agreement);
        *verifier_histogram.entry(verifier_bucket).or_default() += 1;
        candidate_attempt_counts.push(artifact.candidate_attempts.len());
        if !artifact_accepted && top_rejected.len() < 20 {
            top_rejected.push(json!({
                "paper_hash": artifact.paper_hash,
                "question": artifact.hard_question,
                "category": category,
                "reasons": artifact.rejection_reasons,
                "candidate_attempts": artifact.candidate_attempts.len(),
                "next_action_hint": next_action_hint(&artifact.paper_rejection_category),
            }));
        }
    }
    let dominant_bottleneck = match rejection_counts.iter().max_by_key(|(_, count)| *count) {
        Some((category, _)) => category.clone(),
        None => "none".to_string(),
    };
    write_json_pretty(
        &config.run_root.join("reports/rejection-analysis.json"),
        &json!({
            "schema_version": "opencode-qbank-rejection-analysis-v1",
            "run_root": config.run_root.display().to_string(),
            "bank": config.bank.display().to_string(),
            "accepted": accepted,
            "rejected": rejected,
            "failed": failed,
            "rejection_counts_by_category": rejection_counts,
            "phase_execution_failure_counts": phase_execution_failures,
            "route_model_distribution_by_phase": route_model_distribution,
            "blind_correct_rate_histogram": blind_histogram,
            "verifier_acceptance_histogram": verifier_histogram,
            "top_rejected_candidate_stems": top_rejected,
            "paper_level_candidate_attempt_counts": candidate_attempt_counts,
            "dominant_bottleneck": dominant_bottleneck,
        }),
    )
}

pub(crate) fn write_jnoccio_preflight_report(
    config: &BuildPaperTournamentConfig,
) -> Result<(), String> {
    let base_url = config
        .jnoccio_base_url
        .as_deref()
        .ok_or("--agent-runner jnoccio requires --jnoccio-base-url")?
        .trim()
        .trim_end_matches('/');
    let requested_model = configured_jnoccio_model(config);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|err| format!("build jnoccio preflight http client: {err}"))?;
    let status_url = format!("{base_url}/v1/jnoccio/status");
    let status_text = fetch_required_text(&client, &status_url, "jnoccio status")?;
    let status_json: serde_json::Value = serde_json::from_str(&status_text)
        .map_err(|err| format!("jnoccio status response is not JSON: {err}"))?;
    let metrics_url = format!("{base_url}/v1/jnoccio/metrics");
    let metrics_result = fetch_optional_json(&client, &metrics_url);
    let reports_dir = config.run_root.join("reports");
    let status_path = reports_dir.join("jnoccio-status.json");
    let metrics_path = reports_dir.join("jnoccio-metrics.json");
    let preflight_path = reports_dir.join("jnoccio-preflight.json");
    write_json_pretty(&status_path, &status_json)?;

    let (metrics_status, metrics_hash) = match metrics_result {
        Ok(Some(metrics_json)) => {
            let metrics_text = serde_json::to_string(&metrics_json)
                .map_err(|err| format!("serialize jnoccio metrics: {err}"))?;
            write_json_pretty(&metrics_path, &metrics_json)?;
            (
                "captured".to_string(),
                Some(sha256_hex(metrics_text.as_bytes())),
            )
        }
        Ok(None) => ("not_available".to_string(), None),
        Err(err) => (format!("error: {err}"), None),
    };

    let model_catalog = summarize_jnoccio_models(&status_json);
    let gateway_visible_model = status_json
        .pointer("/health/visible_model")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    if let Some(visible_model) = gateway_visible_model.as_deref() {
        if !model_matches_gateway_visible_model(visible_model, &requested_model) {
            return Err(format!(
                "--jnoccio-model {requested_model:?} is not accepted by this Jnoccio gateway; current visible chat model is {visible_model:?}. Use the visible model or restart/reconfigure Jnoccio before the run."
            ));
        }
    }
    let requested_model_visible = model_catalog.iter().any(|entry| {
        ["id", "model_id", "visible_id", "name", "model"]
            .iter()
            .filter_map(|key| entry.get(*key).and_then(|value| value.as_str()))
            .any(|value| value == requested_model)
    }) || gateway_visible_model.as_deref()
        == Some(requested_model.as_str());
    let warnings = if requested_model_visible || model_catalog.is_empty() {
        Vec::<String>::new()
    } else {
        vec![format!(
            "requested model {requested_model:?} was not found verbatim in the status model catalog"
        )]
    };

    write_json_pretty(
        &preflight_path,
        &json!({
            "schema_version": "opencode-qbank-jnoccio-preflight-v1",
            "base_url": base_url,
            "status_url": status_url,
            "metrics_url": metrics_url,
            "requested_model": requested_model,
            "phase_max_output_tokens": config.jnoccio_max_output_tokens,
            "context_policy": {
                "safe_window_tokens": 128000,
                "target_fill_ratio": 0.82,
                "output_reserve_tokens": config.jnoccio_max_output_tokens
            },
            "status_hash": sha256_hex(status_text.as_bytes()),
            "gateway_visible_model": gateway_visible_model,
            "health": status_json.get("health").cloned(),
            "metrics_status": metrics_status,
            "metrics_hash": metrics_hash,
            "status_path": status_path.display().to_string(),
            "metrics_path": metrics_path.display().to_string(),
            "model_catalog_count": model_catalog.len(),
            "model_catalog": model_catalog,
            "warnings": warnings
        }),
    )
}

fn rate_bucket(rate: f64) -> String {
    let lower = (rate * 10.0).floor() / 10.0;
    let upper = (lower + 0.1).min(1.0);
    format!("{lower:.1}-{upper:.1}")
}

fn next_action_hint(category: &Option<String>) -> &'static str {
    match category.as_deref() {
        Some("blind_too_easy") | Some("blind_too_easy_prescreen") | Some("stem_leakage") => {
            "generate lower-leakage stems and harder distractors"
        }
        Some("verifier_reject") | Some("generator_support") | Some("no_quote_candidates") => {
            "tighten support quote selection and key point coverage"
        }
        Some("route_http") | Some("route_metadata") | Some("route_model_policy") => {
            "inspect route stability and model policy"
        }
        Some("tester_schema") | Some("grader_schema") | Some("parse_schema") => {
            "inspect schema failures and retry policy"
        }
        _ => "inspect final artifact failures",
    }
}
