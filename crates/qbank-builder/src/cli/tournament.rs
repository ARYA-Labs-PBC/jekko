use super::*;

pub fn build_paper_tournament_command(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let run_root = match path_value(args, "--run-root") {
        Some(value) => value,
        None => PathBuf::from(".jekko/daemon/paper-qbank-deep-stem-500"),
    };
    let mock_agents = path_value(args, "--mock-agents");
    let strict_production = args.iter().any(|arg| arg == "--strict-production");
    let agent_runner = match value(args, "--agent-runner") {
        Some(value) if value == "mock" => AgentRunnerMode::Mock,
        Some(value) if value == "jnoccio" => AgentRunnerMode::Jnoccio,
        Some(value) => {
            return Err(format!(
                "unknown --agent-runner {value:?}; expected mock|jnoccio"
            ))
        }
        None if mock_agents.is_some() => AgentRunnerMode::Mock,
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
        graders: usize_value(args, "--graders", 5),
        min_successful_generators: usize_value(args, "--min-successful-generators", 1),
        min_successful_verifiers: usize_value(args, "--min-successful-verifiers", 3),
        min_successful_testers: usize_value(args, "--min-successful-testers", 3),
        min_successful_graders: usize_value(args, "--min-successful-graders", 3),
        distractor_papers: usize_value(args, "--distractor-papers", 8),
        strict_production,
        agent_runner,
        jnoccio_base_url: value(args, "--jnoccio-base-url"),
        jnoccio_model: value(args, "--jnoccio-model"),
        jnoccio_max_output_tokens: u64_value(args, "--jnoccio-max-output-tokens", 4096),
        jnoccio_request_timeout_seconds: u64_value(args, "--jnoccio-request-timeout-seconds", 900),
        paper_timeout_seconds: u64_value(args, "--paper-timeout-seconds", 3600),
        phase_retries: usize_value(args, "--phase-retries", 3),
        generator_pool_target: usize_value(args, "--generator-pool-target", 5),
        max_question_alternates_per_paper: usize_value(
            args,
            "--max-question-alternates-per-paper",
            5,
        ),
        blind_prescreen_testers: usize_value(args, "--blind-prescreen-testers", 3),
        blind_prescreen_max_correct_rate: f64_value(
            args,
            "--blind-prescreen-max-correct-rate",
            0.34,
        ),
        min_support_quote_score: i32_value(args, "--min-support-quote-score", 10),
        hard_distractors: strict_production || args.iter().any(|arg| arg == "--hard-distractors"),
        mask_blind_context_metadata: strict_production
            || args
                .iter()
                .any(|arg| arg == "--mask-blind-context-metadata"),
        route_model_deny: route_model_policies(args, "--route-model-deny"),
        route_model_allow: route_model_policies(args, "--route-model-allow"),
        write_rejection_analysis: !args
            .iter()
            .any(|arg| arg == "--no-write-rejection-analysis"),
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

fn route_model_policies(args: &[String], flag: &str) -> Vec<RouteModelPolicy> {
    let mut policies = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg != flag {
            continue;
        }
        let Some(value) = iter.next() else {
            continue;
        };
        if let Some((phase, pattern)) = value.split_once(':') {
            policies.push(RouteModelPolicy {
                phase: phase.to_string(),
                pattern: pattern.to_string(),
            });
        }
    }
    policies
}
