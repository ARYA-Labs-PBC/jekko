use super::*;

pub fn reduce(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let input = path_value(args, "--input").ok_or("--input is required")?;
    let strict_production = args.iter().any(|arg| arg == "--strict-production");
    ensure_bank_layout(&bank)?;
    let mut challenge: ChallengeRecord = read_json(&input)?;
    challenge = finalize_challenge(challenge);
    let accepted = if strict_production {
        qbank_builder::production_acceptance_passes(&challenge)
    } else {
        acceptance_passes(&challenge.acceptance)
    };
    let dir = if accepted { "challenges" } else { "rejected" };
    let out = bank
        .join(dir)
        .join(format!("{}.json", challenge.challenge_hash));
    write_json_pretty(&out, &challenge)
}

pub fn reduce_trials(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let input = path_value(args, "--input").ok_or("--input is required")?;
    let run_root = match path_value(args, "--run-root") {
        Some(value) => value,
        None => PathBuf::from(".jekko/daemon/paper-qbank"),
    };
    let strict_production = args.iter().any(|arg| arg == "--strict-production");
    let min_accepted = value(args, "--min-accepted")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(500);
    ensure_bank_layout(&bank)?;
    let mut paths = Vec::new();
    qbank_builder::collect_json_files(&input, &mut paths)?;
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut outputs = Vec::new();
    for path in paths {
        let mut challenge: ChallengeRecord = read_json(&path)?;
        challenge = finalize_challenge(challenge);
        let errors = if strict_production {
            qbank_builder::production_acceptance_errors(&challenge)
        } else if acceptance_passes(&challenge.acceptance) {
            Vec::new()
        } else {
            vec!["base acceptance gates failed".to_string()]
        };
        let dir = if errors.is_empty() {
            accepted += 1;
            "challenges"
        } else {
            rejected += 1;
            "rejected"
        };
        let out = bank
            .join(dir)
            .join(format!("{}.json", challenge.challenge_hash));
        write_json_pretty(&out, &challenge)?;
        outputs.push(json!({
            "input": path.display().to_string(),
            "output": out.display().to_string(),
            "accepted": errors.is_empty(),
            "errors": errors,
        }));
    }
    let receipt = json!({
        "input": input.display().to_string(),
        "bank": bank.display().to_string(),
        "strict_production": strict_production,
        "min_required_accepted": min_accepted,
        "schema_version": PRODUCTION_CHALLENGE_SCHEMA_VERSION,
        "accepted": accepted,
        "rejected": rejected,
        "outputs": outputs,
    });
    let receipt_path = run_root.join("reports/qbank-reduce.json");
    write_json_pretty(&receipt_path, &receipt)?;
    if strict_production && accepted < min_accepted {
        return Err(format!(
            "strict production reduction accepted {accepted} challenges; need at least {min_accepted}"
        ));
    }
    Ok(())
}

pub fn publish_manifest(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    ensure_bank_layout(&bank)?;
    let challenges = sorted_challenges(read_challenges(&bank)?);
    let papers = read_papers(&bank)?;
    let strict_production = args.iter().any(|arg| arg == "--strict-production");
    let min_required_accepted = value(args, "--min-accepted")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(500);
    if strict_production {
        let mut errors = production_bank_errors(&challenges, min_required_accepted);
        let papers_by_hash = papers
            .iter()
            .map(|paper| (paper.publication_hash.as_str(), paper))
            .collect::<std::collections::BTreeMap<_, _>>();
        for challenge in &challenges {
            let Some(paper) = papers_by_hash.get(challenge.publication_hash.as_str()) else {
                errors.push(format!(
                    "{} missing redistributable paper JSON for {}",
                    challenge.challenge_hash, challenge.publication_hash
                ));
                continue;
            };
            if !paper.license.redistributable {
                errors.push(format!(
                    "{} paper {} is not redistributable",
                    challenge.challenge_hash, challenge.publication_hash
                ));
            }
            if paper.license.spdx.eq_ignore_ascii_case("NOASSERTION") {
                errors.push(format!(
                    "{} paper {} has ambiguous license",
                    challenge.challenge_hash, challenge.publication_hash
                ));
            }
            if paper.sections.is_empty() {
                errors.push(format!(
                    "{} paper {} has no sections",
                    challenge.challenge_hash, challenge.publication_hash
                ));
            }
            if paper
                .license
                .source_url
                .as_deref()
                .unwrap_or("")
                .contains("example.invalid")
            {
                errors.push(format!(
                    "{} paper {} uses fixture URL",
                    challenge.challenge_hash, challenge.publication_hash
                ));
            }
        }
        for challenge in &challenges {
            let challenge_errors = qbank_builder::production_acceptance_errors(challenge);
            if !challenge_errors.is_empty() {
                errors.push(format!(
                    "{} failed strict production gates: {}",
                    challenge.challenge_hash,
                    challenge_errors.join("; ")
                ));
            }
        }
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
    }
    let mut files = Vec::new();
    qbank_builder::collect_json_files(&bank.join("papers"), &mut files)?;
    qbank_builder::collect_json_files(&bank.join("challenges"), &mut files)?;
    let hash = manifest_hash(&files)?;
    let manifest = json!({
        "schema_version": if strict_production { PRODUCTION_MANIFEST_SCHEMA_VERSION } else { "opencode-qbank-manifest-v1" },
        "strict_production": strict_production,
        "accepted_challenges": challenges.len(),
        "min_required_accepted": if strict_production { min_required_accepted } else { 0 },
        "unique_publications": challenges.iter().map(|challenge| &challenge.publication_hash).collect::<std::collections::BTreeSet<_>>().len(),
        "manifest_hash": hash,
        "top_challenge_hashes": challenges.iter().map(|challenge| challenge.challenge_hash.clone()).collect::<Vec<_>>()
    });
    write_json_pretty(&bank.join("manifests").join("latest.json"), &manifest)
}

pub fn audit_bank(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let strict_production = args.iter().any(|arg| arg == "--strict-production");
    let json_errors_ok = args.iter().any(|arg| arg == "--json-errors-ok");
    let min_required_accepted = value(args, "--min-accepted")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(500);
    ensure_bank_layout(&bank)?;
    let challenges = sorted_challenges(read_challenges(&bank)?);
    let papers = read_papers(&bank)?;
    let mut errors = Vec::new();
    if strict_production {
        errors.extend(production_bank_errors(&challenges, min_required_accepted));
        let papers_by_hash = papers
            .iter()
            .map(|paper| (paper.publication_hash.as_str(), paper))
            .collect::<std::collections::BTreeMap<_, _>>();
        for challenge in &challenges {
            if !papers_by_hash.contains_key(challenge.publication_hash.as_str()) {
                errors.push(format!(
                    "{} missing redistributable paper JSON for {}",
                    challenge.challenge_hash, challenge.publication_hash
                ));
            }
        }
        for challenge in &challenges {
            errors.extend(qbank_builder::production_acceptance_errors(challenge));
        }
    }
    let report = json!({
        "bank": bank.display().to_string(),
        "strict_production": strict_production,
        "min_required_accepted": min_required_accepted,
        "accepted_challenges": challenges.len(),
        "unique_publications": challenges.iter().map(|challenge| &challenge.publication_hash).collect::<std::collections::BTreeSet<_>>().len(),
        "schema_version": if strict_production { PRODUCTION_MANIFEST_SCHEMA_VERSION } else { "opencode-qbank-manifest-v1" },
        "errors": errors,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?
    );
    if strict_production && !json_errors_ok && !errors.is_empty() {
        return Err(format!(
            "strict production audit found {} error(s)",
            errors.len()
        ));
    }
    Ok(())
}

pub fn emit_cogcore(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let out = match path_value(args, "--out") {
        Some(value) => value,
        None => bank.join("cogcore-events.jsonl"),
    };
    ensure_bank_layout(&bank)?;
    let papers = read_papers(&bank)?;
    let challenges = read_challenges(&bank)?;
    let events = cogcore_events_for_papers(&papers, &challenges);
    let mut lines = String::new();
    for event in events {
        lines.push_str(&serde_json::to_string(&event).map_err(|err| err.to_string())?);
        lines.push('\n');
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    std::fs::write(&out, lines).map_err(|err| format!("write {}: {err}", out.display()))
}
