use super::*;

pub(crate) fn write_failure_summary(
    config: &BuildPaperTournamentConfig,
    outputs: &[serde_json::Value],
) -> Result<(), String> {
    let mut artifacts = Vec::new();
    collect_json_files(&config.run_root.join("trials"), &mut artifacts)?;
    artifacts.retain(|path| path.file_name().and_then(|name| name.to_str()) == Some("final.json"));

    let mut total = 0usize;
    let mut fatal = 0usize;
    let mut by_category = BTreeMap::<String, usize>::new();
    let mut rows = Vec::new();
    for path in artifacts {
        let artifact: FinalPaperChallengeArtifact = read_json_silent(&path)?;
        for failure in &artifact.failures {
            total += 1;
            if failure.fatal_for_acceptance {
                fatal += 1;
            }
            *by_category.entry(failure.category.clone()).or_default() += 1;
            rows.push(json!({
                "paper_hash": artifact.paper_hash,
                "artifact": path.display().to_string(),
                "phase": failure.phase,
                "agent_name": failure.agent_name,
                "category": failure.category,
                "fatal_for_acceptance": failure.fatal_for_acceptance,
                "error": failure.error,
                "request_id": failure.route_metadata.as_ref().map(|route| route.request_id.clone()),
            }));
        }
        for reason in &artifact.rejection_reasons {
            let category = rejection_category(reason);
            *by_category.entry(category.to_string()).or_default() += 1;
            total += 1;
            rows.push(json!({
                "paper_hash": artifact.paper_hash,
                "artifact": path.display().to_string(),
                "phase": "reduction",
                "agent_name": "paper-tournament",
                "category": category,
                "fatal_for_acceptance": false,
                "error": reason,
            }));
        }
    }
    for output in outputs {
        let Some(errors) = output.get("errors").and_then(|value| value.as_array()) else {
            continue;
        };
        if errors.is_empty() {
            continue;
        }
        let accepted = output
            .get("accepted")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        for error in errors.iter().filter_map(|value| value.as_str()) {
            let category = rejection_category(error);
            total += 1;
            let fatal_for_acceptance = !accepted;
            if fatal_for_acceptance {
                fatal += 1;
            }
            *by_category.entry(category.to_string()).or_default() += 1;
            rows.push(json!({
                "paper_hash": output.get("paper_hash").and_then(|value| value.as_str()),
                "challenge_hash": output.get("challenge_hash").and_then(|value| value.as_str()),
                "phase": "paper",
                "agent_name": "paper-tournament",
                "category": category,
                "fatal_for_acceptance": fatal_for_acceptance,
                "error": error,
            }));
        }
    }
    let summary = json!({
        "schema_version": "opencode-qbank-failure-summary-v1",
        "run_root": config.run_root.display().to_string(),
        "bank": config.bank.display().to_string(),
        "agent_runner": config.agent_runner.as_str(),
        "attempts": {
            "generators": config.generators,
            "verifiers": config.verifiers,
            "testers": config.testers,
            "graders": config.graders
        },
        "minimum_successful": {
            "generators": config.min_successful_generators,
            "verifiers": config.min_successful_verifiers,
            "testers": config.min_successful_testers,
            "graders_per_counted_tester": config.min_successful_graders
        },
        "total_failures": total,
        "fatal_failures": fatal,
        "nonfatal_failures": total.saturating_sub(fatal),
        "by_category": by_category,
        "failures": rows,
    });
    write_json_pretty(
        &config.run_root.join("reports/failure-summary.json"),
        &summary,
    )
}

pub(crate) fn read_json_silent<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text =
        std::fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("parse {}: {err}", path.display()))
}

pub(crate) fn rejection_category(error: &str) -> &'static str {
    let lower = error.to_ascii_lowercase();
    if lower.contains("license")
        || lower.contains("redistributable")
        || lower.contains("source url")
        || lower.contains("fixture")
        || lower.contains("full-text")
        || lower.contains("abstract-only")
        || lower.contains("correction")
        || lower.contains("erratum")
    {
        "source_quality"
    } else if lower.contains("quote") {
        "no_quote_candidates"
    } else if lower.contains("generation quorum") {
        "generator_schema"
    } else if lower.contains("verification") || lower.contains("verifier") {
        "verifier_reject"
    } else if lower.contains("testing quorum") {
        "tester_schema"
    } else if lower.contains("blind tester correct rate") {
        "blind_too_easy"
    } else if lower.contains("saturated mean confidence") {
        "blind_confidence"
    } else if lower.contains("route metadata") || lower.contains("model_decisions") {
        "route_metadata"
    } else if lower.contains("http") || lower.contains("request") {
        "route_http"
    } else if lower.contains("timeout") || lower.contains("exceeded timeout") {
        "paper_budget_exhausted"
    } else if lower.contains("parse") || lower.contains("schema") {
        "parse_schema"
    } else {
        "parse_schema"
    }
}

pub(crate) fn filter_papers_by_candidate_manifest(
    papers: Vec<PaperRecord>,
    manifest_path: &Path,
) -> Result<Vec<PaperRecord>, String> {
    let text = std::fs::read_to_string(manifest_path)
        .map_err(|err| format!("read candidate manifest {}: {err}", manifest_path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|err| {
        format!(
            "parse candidate manifest {}: {err}",
            manifest_path.display()
        )
    })?;
    let hashes = match value.get("papers").and_then(|value| value.as_array()) {
        Some(items) => items
            .iter()
            .filter_map(|row| row.get("publication_hash").and_then(|value| value.as_str()))
            .map(str::to_string)
            .collect::<Vec<_>>(),
        None => {
            return Err(format!(
                "candidate manifest {} missing papers array",
                manifest_path.display()
            ))
        }
    };
    if hashes.is_empty() {
        return Err(format!(
            "candidate manifest {} contains no publication_hash entries",
            manifest_path.display()
        ));
    }
    let by_hash = papers
        .into_iter()
        .map(|paper| (paper.publication_hash.clone(), paper))
        .collect::<BTreeMap<_, _>>();
    let mut out = Vec::new();
    let mut missing = Vec::new();
    for hash in hashes {
        match by_hash.get(&hash) {
            Some(paper) => out.push(paper.clone()),
            None => missing.push(hash),
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "candidate manifest references {} papers missing from bank: {}",
            missing.len(),
            missing.into_iter().take(5).collect::<Vec<_>>().join(", ")
        ));
    }
    Ok(out)
}

pub(crate) fn existing_accepted_count(bank: &Path) -> Result<usize, String> {
    let mut files = Vec::new();
    collect_json_files(&bank.join("challenges"), &mut files)?;
    Ok(files
        .into_iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("manifest.json"))
        .count())
}

pub(crate) fn paper_already_attempted(run_root: &Path, paper: &PaperRecord) -> bool {
    run_root
        .join("trials")
        .join(&paper.publication_hash)
        .read_dir()
        .map(|mut entries| {
            entries.any(|entry| {
                entry
                    .map(|entry| entry.path().join("final.json").exists())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

pub(crate) fn write_manifest(
    bank: &Path,
    accepted: usize,
    target_accepted: usize,
    strict_production: bool,
) -> Result<(), String> {
    let mut files = Vec::new();
    collect_json_files(&bank.join("papers"), &mut files)?;
    collect_json_files(&bank.join("challenges"), &mut files)?;
    let hash = manifest_hash(&files)?;
    write_json_pretty(
        &bank.join("manifests").join("latest.json"),
        &json!({
            "schema_version": if strict_production { PRODUCTION_MANIFEST_SCHEMA_VERSION } else { "opencode-qbank-manifest-v1" },
            "strict_production": strict_production,
            "accepted_challenges": accepted,
            "min_required_accepted": target_accepted,
            "unique_publications": accepted,
            "manifest_hash": hash,
        }),
    )
}
