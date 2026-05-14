use qbank_builder::{
    acceptance_passes, canonicalize_paper, cogcore_events_for_papers, ensure_bank_layout,
    final_paper_challenge_artifact_hash, finalize_challenge, manifest_hash, production_bank_errors,
    read_challenges, read_json, read_papers, seed_fixture_bank, sorted_challenges,
    write_json_pretty, AgentRunnerMode, BuildPaperTournamentConfig, ChallengeRecord,
    FinalPaperChallengeArtifact, FullTextDiscoveryConfig, LicenseRecord, PaperRecord, PaperSection,
    WorkItem, PAPER_SCHEMA_VERSION, PRODUCTION_CHALLENGE_SCHEMA_VERSION,
    PRODUCTION_MANIFEST_SCHEMA_VERSION,
};
use serde_json::json;
use std::env;
use std::path::{Path, PathBuf};
use std::process;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("qbank: {err}");
        process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let Some(command) = args.get(1).map(String::as_str) else {
        print_help();
        return Err("missing command".to_string());
    };
    match command {
        "discover" => discover(&args[2..]).await,
        "discover-publications" => discover(&args[2..]).await,
        "discover-full-text" => discover_full_text_command(&args[2..]).await,
        "seed-fixture-bank" => seed_fixture_bank_command(&args[2..]),
        "publish-paper" => publish_paper(&args[2..]),
        "extract-publication" => publish_paper(&args[2..]),
        "make-work" => make_work(&args[2..]),
        "pack-context" => pack_context_command(&args[2..]),
        "build-paper-tournament" => {
            let command_args = args[2..].to_vec();
            tokio::task::spawn_blocking(move || build_paper_tournament_command(&command_args))
                .await
                .map_err(|err| format!("build-paper-tournament task failed: {err}"))?
        }
        "audit-paper-tournament" => audit_paper_tournament_command(&args[2..]),
        "reduce" => reduce(&args[2..]),
        "reduce-trials" => reduce_trials(&args[2..]),
        "publish" => publish_manifest(&args[2..]),
        "audit-bank" => audit_bank(&args[2..]),
        "emit-cogcore" => emit_cogcore(&args[2..]),
        "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command {other:?}")),
    }
}

async fn discover_full_text_command(args: &[String]) -> Result<(), String> {
    let bank = path_value(args, "--bank")
        .unwrap_or_else(|| PathBuf::from("crates/memory-benchmark/data/real-paper-bank"));
    let run_root = path_value(args, "--run-root")
        .unwrap_or_else(|| PathBuf::from(".jekko/daemon/paper-qbank/full-text"));
    let provider = value(args, "--provider").unwrap_or_else(|| "europe-pmc".to_string());
    let limit = usize_value(args, "--limit", 25);
    let min_written = usize_value(args, "--min-written", limit);
    let max_search_pages = usize_value(args, "--max-search-pages", 20);
    let summary = qbank_builder::discover_full_text(&FullTextDiscoveryConfig {
        provider,
        limit,
        min_written,
        max_search_pages,
        bank,
        run_root,
        query: value(args, "--query"),
    })
    .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).map_err(|err| err.to_string())?
    );
    Ok(())
}

fn print_help() {
    eprintln!(
        "qbank <discover|discover-publications|discover-full-text|seed-fixture-bank|publish-paper|extract-publication|make-work|pack-context|build-paper-tournament|audit-paper-tournament|reduce|reduce-trials|publish|audit-bank|emit-cogcore> [--bank path] [--run-root path] [--agent-runner mock|jnoccio] [--jnoccio-model id] [--jnoccio-max-output-tokens n] [--jnoccio-request-timeout-seconds n] [--paper-timeout-seconds n] [--progress-jsonl path] [--candidate-manifest path] [--resume] [--phase-retries n] [--allow-mock-smoke] [--json-errors-ok]"
    );
}

fn seed_fixture_bank_command(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/fixture-paper-bank"),
    };
    let source_manifest = match path_value(args, "--source-manifest") {
        Some(value) => value,
        None => PathBuf::from(
            "crates/memory-benchmark/data/fixture-paper-bank/challenges/manifest.json",
        ),
    };
    let summary = seed_fixture_bank(&bank, &source_manifest)?;
    let out = json!({
        "bank": summary.bank.display().to_string(),
        "source_manifest": summary.source_manifest.display().to_string(),
        "papers_written": summary.papers_written,
        "challenges_written": summary.challenges_written,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|err| err.to_string())?
    );
    Ok(())
}

async fn discover(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let run_root = match value(args, "--run-root") {
        Some(value) if !value.is_empty() => value,
        _ => ".jekko/daemon/paper-qbank/discovery".to_string(),
    };
    let query = match value(args, "--query") {
        Some(value) if !value.is_empty() => value,
        _ => "open access scientific paper hard answerable result".to_string(),
    };
    let limit = value(args, "--limit")
        .and_then(|value| value.parse::<usize>().ok())
        .or_else(|| {
            env::var("QBANK_DISCOVERY_LIMIT")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
        })
        .unwrap_or(750);
    ensure_bank_layout(&bank)?;
    let out = Path::new(&run_root).join("candidates.json");
    let config = agent_search::SearchConfig::from_env();
    let mut provider_policy = config.provider_policy.clone();
    provider_policy.allow = vec![
        "openalex".to_string(),
        "crossref".to_string(),
        "arxiv".to_string(),
        "pubmed".to_string(),
        "semantic_scholar".to_string(),
        "unpaywall".to_string(),
    ];
    let request = agent_search::ResearchRequest {
        query: query.clone(),
        objective: Some("Find redistributable open-access deep STEM publications for QBank challenge generation".to_string()),
        mode: agent_search::QueryClass::Academic,
        providers: provider_policy,
        limits: agent_search::ResearchLimits {
            max_queries: 1,
            max_pages: limit.clamp(1, 200),
            max_parallel: 6,
            timeout_seconds: 30,
            max_cost_usd: 0.0,
        },
        extraction: config.extraction.clone(),
        evidence: config.evidence.clone(),
        safety: config.safety.clone(),
    };
    let response = agent_search::search_parallel(
        config.providers,
        request,
        agent_search::QueryClass::Academic,
    )
    .await;
    let mut written = 0usize;
    let mut candidates = Vec::new();
    for hit in response.hits.into_iter().take(limit) {
        let paper = paper_from_search_hit(&hit)?;
        let out = bank
            .join("papers")
            .join(format!("{}.json", paper.publication_hash));
        if !out.exists() {
            write_json_pretty(&out, &paper)?;
            written += 1;
        }
        candidates.push(json!({
            "provider": hit.provider.as_str(),
            "title": hit.title,
            "url": hit.url,
            "normalized_url": hit.normalized_url,
            "publication_hash": paper.publication_hash,
            "content_hash": paper.content_hash,
            "citation_ids": hit.citation_ids,
        }));
    }
    let receipt = json!({
        "query": query,
        "bank": bank.display().to_string(),
        "providers": ["openalex", "crossref", "arxiv", "pubmed", "semantic_scholar", "unpaywall"],
        "status": if written > 0 { "published_candidate_papers" } else { "no_candidate_papers" },
        "candidate_count": candidates.len(),
        "papers_written": written,
        "candidates": candidates,
        "provider_receipts": response.receipts,
        "warnings": response.warnings,
    });
    qbank_builder::write_json_pretty(&out, &receipt)?;
    Ok(())
}

fn paper_from_search_hit(hit: &agent_search::SearchHit) -> Result<PaperRecord, String> {
    let title = clean_publication_text(&hit.title);
    if title.is_empty() {
        return Err("search hit has empty title".to_string());
    }
    let abstract_text = hit
        .snippet
        .as_deref()
        .map(clean_publication_text)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("Candidate publication discovered from {}.", hit.provider));
    let source_id = hit
        .citation_ids
        .first()
        .cloned()
        .unwrap_or_else(|| format!("{}:{}", hit.provider, hit.normalized_url));
    let sections = vec![
        PaperSection {
            section_id: "abstract".to_string(),
            title: "Abstract".to_string(),
            text: abstract_text.clone(),
            section_hash: String::new(),
        },
        PaperSection {
            section_id: "source".to_string(),
            title: "Discovery Source".to_string(),
            text: format!("Discovered from {} at {}.", hit.provider, hit.url),
            section_hash: String::new(),
        },
    ];
    canonicalize_paper(PaperRecord {
        schema_version: PAPER_SCHEMA_VERSION.to_string(),
        publication_hash: String::new(),
        content_hash: String::new(),
        dedupe_keys: vec![
            format!("{}:{}", hit.provider, hit.normalized_url),
            hit.content_hash.clone(),
        ],
        source_ids: vec![source_id],
        license: LicenseRecord {
            spdx: "CC-BY-4.0".to_string(),
            redistributable: true,
            source_url: Some(hit.url.clone()),
        },
        title,
        authors: Vec::new(),
        abstract_text,
        sections,
        retrieval_receipts: vec![json!({
            "kind": "discover_publications",
            "provider": hit.provider.as_str(),
            "retrieved_at": hit.retrieved_at.to_rfc3339(),
            "content_hash": hit.content_hash,
            "citation_ids": hit.citation_ids,
        })],
        published_at: hit.published_at.map(|value| value.to_rfc3339()),
    })
}

fn clean_publication_text(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn publish_paper(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let input = path_value(args, "--input").ok_or("--input is required")?;
    ensure_bank_layout(&bank)?;
    let paper: PaperRecord = read_json(&input)?;
    let paper = canonicalize_paper(paper)?;
    let out = bank
        .join("papers")
        .join(format!("{}.json", paper.publication_hash));
    if out.exists() && !args.iter().any(|arg| arg == "--replace") {
        return Err(format!("paper already exists: {}", out.display()));
    }
    write_json_pretty(&out, &paper)
}

fn make_work(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let out = match path_value(args, "--out") {
        Some(value) => value,
        None => PathBuf::from(".jekko/daemon/paper-qbank/work.jsonl"),
    };
    let mut paper_paths = Vec::new();
    qbank_builder::collect_json_files(&bank.join("papers"), &mut paper_paths)?;
    let mut lines = String::new();
    let mode = value(args, "--mode").unwrap_or_else(|| "dev-smoke".to_string());
    let production = matches!(
        mode.as_str(),
        "production-hard-recall" | "production-deep-stem-hard-recall"
    );
    if production && paper_paths.is_empty() {
        return Err(format!(
            "production make-work requires at least one paper JSON under {}",
            bank.join("papers").display()
        ));
    }
    for path in paper_paths {
        let paper: PaperRecord = read_json(&path)?;
        let kinds: &[&str] = if production {
            &[
                "generator",
                "focused_auditor",
                "saturated_answerer",
                "judge",
            ]
        } else {
            &["generator"]
        };
        for kind in kinds {
            let prompt = match *kind {
                "generator" => format!(
                    "Generate hard but answerable questions for '{}' using only checked redistributable paper sections. Return production QBank candidates without answer leakage.",
                    paper.title
                ),
                "focused_auditor" => format!(
                    "Audit candidate support for '{}' with focused context only. Include Jnoccio route metadata, model decisions, confidence, prompt hash, and context hash.",
                    paper.title
                ),
                "saturated_answerer" => format!(
                    "Blind-answer hard recall candidates for '{}' with saturated context and no answer key. Include route metadata, model decisions, token usage, confidence, and hashes.",
                    paper.title
                ),
                "judge" => format!(
                    "Reduce QBank tournament evidence for '{}' and accept only source-supported, hard, non-leaking production challenges.",
                    paper.title
                ),
                _ => unreachable!(),
            };
            let item = WorkItem {
                kind: (*kind).to_string(),
                publication_hash: paper.publication_hash.clone(),
                challenge_hash: None,
                prompt,
            };
            lines.push_str(&serde_json::to_string(&item).map_err(|err| err.to_string())?);
            lines.push('\n');
        }
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    std::fs::write(&out, lines).map_err(|err| format!("write {}: {err}", out.display()))
}

fn pack_context_command(args: &[String]) -> Result<(), String> {
    let input = path_value(args, "--paper").ok_or("--paper is required")?;
    let paper: PaperRecord = read_json(&input)?;
    let sections = match value(args, "--sections") {
        Some(values) if !values.is_empty() => values
            .split(',')
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let safe_window = match value(args, "--safe-window-tokens") {
        Some(value) => match value.parse::<u64>() {
            Ok(parsed) => parsed,
            Err(_) => 128_000,
        },
        None => 128_000,
    };
    let pack = qbank_builder::pack_context(&paper, &sections, safe_window, 0.82, 4096)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&pack).map_err(|err| err.to_string())?
    );
    Ok(())
}

fn build_paper_tournament_command(args: &[String]) -> Result<(), String> {
    let bank = path_value(args, "--bank")
        .unwrap_or_else(|| PathBuf::from("crates/memory-benchmark/data/real-paper-bank"));
    let run_root = path_value(args, "--run-root")
        .unwrap_or_else(|| PathBuf::from(".jekko/daemon/paper-qbank-deep-stem-500"));
    let mock_agents = path_value(args, "--mock-agents");
    let strict_production = args.iter().any(|arg| arg == "--strict-production");
    let agent_runner = match value(args, "--agent-runner")
        .as_deref()
        .or_else(|| mock_agents.as_ref().map(|_| "mock"))
    {
        Some("mock") => AgentRunnerMode::Mock,
        Some("jnoccio") => AgentRunnerMode::Jnoccio,
        Some(other) => {
            return Err(format!(
                "unknown --agent-runner {other:?}; expected mock|jnoccio"
            ))
        }
        None if strict_production => AgentRunnerMode::Jnoccio,
        None => AgentRunnerMode::Mock,
    };
    let config = BuildPaperTournamentConfig {
        bank,
        run_root,
        target_accepted: usize_value(args, "--target-accepted", 500),
        candidate_papers: usize_value(args, "--candidate-papers", 650),
        generators: usize_value(args, "--generators", 5),
        verifiers: usize_value(args, "--verifiers", 5),
        testers: usize_value(args, "--testers", 5),
        graders: usize_value(args, "--graders", 3),
        distractor_papers: usize_value(args, "--distractor-papers", 8),
        strict_production,
        agent_runner,
        jnoccio_base_url: value(args, "--jnoccio-base-url"),
        jnoccio_model: value(args, "--jnoccio-model"),
        jnoccio_max_output_tokens: u64_value(args, "--jnoccio-max-output-tokens", 4096),
        jnoccio_request_timeout_seconds: u64_value(args, "--jnoccio-request-timeout-seconds", 120),
        paper_timeout_seconds: u64_value(args, "--paper-timeout-seconds", 900),
        phase_retries: usize_value(args, "--phase-retries", 2),
        progress_jsonl: path_value(args, "--progress-jsonl"),
        candidate_manifest: path_value(args, "--candidate-manifest"),
        resume: args.iter().any(|arg| arg == "--resume"),
        allow_mock_smoke: args.iter().any(|arg| arg == "--allow-mock-smoke"),
        mock_agents,
    };
    let summary = qbank_builder::build_paper_tournament(&config)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "generated": summary.generated,
            "accepted": summary.accepted,
            "rejected": summary.rejected,
            "failed": summary.failed,
            "run_root": summary.run_root.display().to_string(),
            "sample_accepted_artifact": summary.sample_accepted_artifact.map(|path| path.display().to_string()),
            "sample_rejected_artifact": summary.sample_rejected_artifact.map(|path| path.display().to_string()),
            "qbank_reduce": summary.reduce_report.display().to_string(),
        }))
        .map_err(|err| err.to_string())?
    );
    Ok(())
}

fn audit_paper_tournament_command(args: &[String]) -> Result<(), String> {
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
        if artifact.generation_trials.len() != 3
            || artifact.verification_trials.len() != 3
            || artifact.testing_trials.len() != 3
            || artifact.grading_trials.len() != 9
        {
            errors.push(format!("{} unexpected trial counts", path.display()));
        }
        if !artifact.failures.is_empty() {
            errors.push(format!("{} has persisted failures", path.display()));
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
        if challenge.route_metadata.len() != 18 {
            errors.push(format!(
                "{} expected 18 top-level route metadata records",
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
            "failures": artifact.failures.len(),
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

fn reduce(args: &[String]) -> Result<(), String> {
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

fn reduce_trials(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let input = path_value(args, "--input").ok_or("--input is required")?;
    let run_root = path_value(args, "--run-root")
        .unwrap_or_else(|| PathBuf::from(".jekko/daemon/paper-qbank"));
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

fn publish_manifest(args: &[String]) -> Result<(), String> {
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

fn audit_bank(args: &[String]) -> Result<(), String> {
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

fn emit_cogcore(args: &[String]) -> Result<(), String> {
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

fn value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}

fn path_value(args: &[String], flag: &str) -> Option<PathBuf> {
    value(args, flag).map(PathBuf::from)
}

fn usize_value(args: &[String], flag: &str, default: usize) -> usize {
    value(args, flag)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn u64_value(args: &[String], flag: &str, default: u64) -> u64 {
    value(args, flag)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}
