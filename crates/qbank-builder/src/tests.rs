use super::*;

fn sample_paper() -> PaperRecord {
    PaperRecord {
        schema_version: String::new(),
        publication_hash: String::new(),
        content_hash: String::new(),
        dedupe_keys: vec!["doi:10.1/example".to_string()],
        source_ids: vec!["doi:10.1/example".to_string()],
        license: LicenseRecord {
            spdx: "CC-BY-4.0".to_string(),
            redistributable: true,
            source_url: None,
        },
        title: "Alpha Paper".to_string(),
        authors: vec!["Ada".to_string()],
        abstract_text: "abstract".to_string(),
        sections: vec![
            PaperSection {
                section_id: "s1".to_string(),
                title: "Result".to_string(),
                text: "Alpha equals one in the calibrated fixture.".to_string(),
                section_hash: String::new(),
            },
            PaperSection {
                section_id: "s2".to_string(),
                title: "Distractor".to_string(),
                text: "Beta equals two.".to_string(),
                section_hash: String::new(),
            },
        ],
        retrieval_receipts: Vec::new(),
        published_at: Some("2026-01-01".to_string()),
    }
}

fn accepted_challenge(paper: &PaperRecord, canonical: &str, accepted: bool) -> ChallengeRecord {
    finalize_challenge(ChallengeRecord {
        schema_version: CHALLENGE_SCHEMA_VERSION.to_string(),
        challenge_hash: String::new(),
        publication_hash: paper.publication_hash.clone(),
        domain: "science".to_string(),
        topics: vec!["alpha".to_string()],
        difficulty_score: 0.8,
        difficulty_components: BTreeMap::new(),
        question: "Which calibrated fixture value does the result section state?".to_string(),
        answer_key: AnswerKey {
            canonical: canonical.to_string(),
            must_include: vec![canonical.to_string()],
            must_not_include: vec![],
            aliases: vec![],
            numeric_tolerances: vec![],
            unit_tolerances: vec![],
        },
        support: vec![SupportRef {
            section_id: paper.sections[0].section_id.clone(),
            section_hash: paper.sections[0].section_hash.clone(),
            quote_hash: None,
        }],
        context_pack: ContextPack {
            safe_window_tokens: 128_000,
            target_fill_ratio: 0.82,
            output_reserve_tokens: 4096,
            estimated_tokens: 10,
            target_section_ids: vec![paper.sections[0].section_id.clone()],
            distractor_section_ids: vec![paper.sections[1].section_id.clone()],
        },
        generator_agents: vec![],
        blind_answer_attempts: vec![],
        focused_answer_attempts: vec![],
        critic_attempts: vec![],
        audit_attempts: vec![],
        acceptance: AcceptanceRecord {
            accepted,
            auditor_agreement: 1.0,
            answerability: 1.0,
            blind_correct_rate: 0.0,
            focused_correct_rate: 1.0,
            ambiguity_flag: false,
            hash_mismatch: false,
            redistributable: true,
            reason: None,
        },
        source_publication: None,
        focused_support_trials: vec![],
        saturated_blind_trials: vec![],
        judge_trials: vec![],
        context_packs: vec![],
        route_metadata: vec![],
        acceptance_metrics: None,
        artifact_provenance: None,
        artifact_hash: None,
    })
}

#[test]
fn hashes_are_stable_and_prefixed() {
    let paper = canonicalize_paper(sample_paper()).expect("paper");
    let again = canonicalize_paper(sample_paper()).expect("paper");
    assert_eq!(paper.publication_hash, again.publication_hash);
    assert_eq!(paper.publication_hash.len(), 64);
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn ambiguous_license_is_rejected() {
    let mut paper = sample_paper();
    paper.license.spdx = "NOASSERTION".to_string();
    assert!(canonicalize_paper(paper).is_err());
}

#[test]
fn context_pack_enforces_budget_and_target_presence() {
    let paper = canonicalize_paper(sample_paper()).expect("paper");
    let pack = pack_context(&paper, &["s1".to_string()], 128_000, 0.82, 4096).expect("pack");
    assert!(pack.estimated_tokens <= ((128_000_f64 * 0.82).floor() as u64 - 4096));
    assert_eq!(pack.target_section_ids, vec!["s1"]);
    assert!(pack_context(&paper, &["missing".to_string()], 128_000, 0.82, 4096).is_err());
}

#[test]
fn acceptance_thresholds_are_hard_gates() {
    let mut acceptance = AcceptanceRecord {
        accepted: true,
        auditor_agreement: 0.75,
        answerability: 0.90,
        blind_correct_rate: 0.50,
        focused_correct_rate: 0.90,
        ambiguity_flag: false,
        hash_mismatch: false,
        redistributable: true,
        reason: None,
    };
    assert!(acceptance_passes(&acceptance));
    acceptance.blind_correct_rate = 0.51;
    assert!(!acceptance_passes(&acceptance));
}

#[test]
fn challenge_sort_is_deterministic() {
    let base_acceptance = AcceptanceRecord {
        accepted: true,
        auditor_agreement: 1.0,
        answerability: 1.0,
        blind_correct_rate: 0.2,
        focused_correct_rate: 0.95,
        ambiguity_flag: false,
        hash_mismatch: false,
        redistributable: true,
        reason: None,
    };
    let mk = |hash: &str, difficulty: f64, blind: f64| ChallengeRecord {
        schema_version: CHALLENGE_SCHEMA_VERSION.to_string(),
        challenge_hash: hash.to_string(),
        publication_hash: "paper".to_string(),
        domain: "science".to_string(),
        topics: vec![],
        difficulty_score: difficulty,
        difficulty_components: BTreeMap::new(),
        question: "q".to_string(),
        answer_key: AnswerKey {
            canonical: "a".to_string(),
            must_include: vec![],
            must_not_include: vec![],
            aliases: vec![],
            numeric_tolerances: vec![],
            unit_tolerances: vec![],
        },
        support: vec![],
        context_pack: ContextPack {
            safe_window_tokens: 1,
            target_fill_ratio: 1.0,
            output_reserve_tokens: 0,
            estimated_tokens: 1,
            target_section_ids: vec![],
            distractor_section_ids: vec![],
        },
        generator_agents: vec![],
        blind_answer_attempts: vec![],
        focused_answer_attempts: vec![],
        critic_attempts: vec![],
        audit_attempts: vec![],
        acceptance: AcceptanceRecord {
            blind_correct_rate: blind,
            ..base_acceptance.clone()
        },
        source_publication: None,
        focused_support_trials: vec![],
        saturated_blind_trials: vec![],
        judge_trials: vec![],
        context_packs: vec![],
        route_metadata: vec![],
        acceptance_metrics: None,
        artifact_provenance: None,
        artifact_hash: None,
    };
    let sorted = sorted_challenges(vec![
        mk("b", 0.7, 0.1),
        mk("a", 0.9, 0.4),
        mk("c", 0.9, 0.2),
    ]);
    assert_eq!(
        sorted
            .iter()
            .map(|c| c.challenge_hash.as_str())
            .collect::<Vec<_>>(),
        vec!["c", "a", "b"]
    );
}

#[test]
fn cogcore_events_are_deterministic_and_omit_answer_keys() {
    let paper = canonicalize_paper(sample_paper()).expect("paper");
    let accepted = accepted_challenge(&paper, "secret answer key should not leak", true);
    let rejected = accepted_challenge(&paper, "rejected answer should not leak", false);

    let events = cogcore_events_for_papers(
        std::slice::from_ref(&paper),
        &[rejected.clone(), accepted.clone()],
    );
    let again = cogcore_events_for_papers(&[paper], &[accepted, rejected]);

    assert_eq!(events, again);
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|event| event.id.is_empty()));
    assert_eq!(events[0].kind, "Claim");
    assert_eq!(events[0].subject, "Alpha Paper");
    assert!(events[0].tags.contains(&"topic:alpha".to_string()));
    assert!(events[1].tags.contains(&"section:s2".to_string()));

    let jsonl = events
        .iter()
        .map(|event| serde_json::to_string(event).expect("json"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(jsonl.contains("Alpha equals one in the calibrated fixture."));
    assert!(!jsonl.contains("Which calibrated fixture value"));
    assert!(!jsonl.contains("secret answer key should not leak"));
    assert!(!jsonl.contains("rejected answer should not leak"));
}

#[test]
fn read_challenges_skips_directory_manifest() {
    let root = tempfile::tempdir().expect("tempdir");
    let challenge_dir = root.path().join("challenges");
    std::fs::create_dir_all(&challenge_dir).expect("challenge dir");

    let paper = canonicalize_paper(sample_paper()).expect("paper");
    let challenge = accepted_challenge(&paper, "alpha equals one", true);
    write_json_pretty(
        &challenge_dir.join(format!("{}.json", challenge.challenge_hash)),
        &challenge,
    )
    .expect("challenge write");
    std::fs::write(
        challenge_dir.join("manifest.json"),
        "[{\"challenge_hash\":\"fixture-manifest-entry\"}]\n",
    )
    .expect("manifest write");

    let loaded = read_challenges(root.path()).expect("read challenges");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].challenge_hash, challenge.challenge_hash);
}

#[test]
fn seed_fixture_bank_writes_challenges_and_papers() {
    let root = tempfile::tempdir().expect("tempdir");
    let source_manifest = root.path().join("seed-manifest.json");
    std::fs::write(
        &source_manifest,
        r#"[
  {
    "challenge_hash": "fixture-qbank-001",
    "publication_hash": "paper-fixture-001",
    "question": "According to the fixture paper, what is result 001?",
    "answer_key": "result 001",
    "support_sections": ["s1"],
    "context_pack": { "target_fill_ratio": 0.82, "output_reserve_tokens": 4096 },
    "acceptance": { "accepted": true, "reason": "fixture" }
  }
]"#,
    )
    .expect("write seed manifest");

    let bank = root.path().join("bank");
    let summary = seed_fixture_bank(&bank, &source_manifest).expect("seed bank");
    assert_eq!(summary.papers_written, 1);
    assert_eq!(summary.challenges_written, 1);

    let papers = read_papers(&bank).expect("read papers");
    let challenges = read_challenges(&bank).expect("read challenges");
    assert_eq!(papers.len(), 1);
    assert_eq!(challenges.len(), 1);
    assert_eq!(papers[0].publication_hash, "paper-fixture-001");
    assert_eq!(challenges[0].publication_hash, "paper-fixture-001");
    assert_eq!(challenges[0].support.len(), 1);
    assert_eq!(
        challenges[0].support[0].section_hash,
        papers[0].sections[0].section_hash
    );
}

#[test]
fn agent_json_extracts_direct_fenced_and_wrapped_objects() {
    let direct: GeneratorAgentOutput = parse_agent_json(
        r#"{"question":"q","answer":"a","difficulty_rationale":"r","expected_failure_mode":"f","support":[],"confidence":80}"#,
    )
    .expect("direct json");
    assert_eq!(direct.confidence, 80);

    let fenced: TestingAgentOutput = parse_agent_json(
        "```json\n{\"answer\":\"a\",\"confidence\":40,\"reasoning_summary\":\"r\"}\n```",
    )
    .expect("fenced json");
    assert_eq!(fenced.answer, "a");

    let wrapped: VerificationAgentOutput = parse_agent_json(
        "Here is the object: {\"accepted\":true,\"answer\":\"a\",\"confidence\":90,\"support_correct\":true,\"reason\":\"r\",\"missing_or_wrong_support\":[]} done.",
    )
    .expect("wrapped json");
    assert!(wrapped.accepted);
    assert!(parse_agent_json::<TestingAgentOutput>(
        "{\"answer\":\"a\",\"confidence\":40,\"reasoning_summary\":\"r\"} and {\"answer\":\"b\"}"
    )
    .is_err());
}

#[test]
fn generator_validation_rejects_bad_confidence_and_missing_quote() {
    let paper = canonicalize_paper(sample_paper()).expect("paper");
    let valid_support = vec![SupportQuote {
        section_id: "s1".to_string(),
        section_hash: paper.sections[0].section_hash.clone(),
        quote: "Alpha equals one in the calibrated fixture.".to_string(),
        why_it_matters: "support".to_string(),
    }];
    let mut output = GeneratorAgentOutput {
        question: "What value?".to_string(),
        answer: "one".to_string(),
        difficulty_rationale: "hard".to_string(),
        expected_failure_mode: "miss".to_string(),
        required_key_points: vec![
            "Alpha equals one".to_string(),
            "calibrated".to_string(),
            "fixture".to_string(),
        ],
        support: valid_support,
        confidence: 101,
    };
    assert!(agent_json::validate_generator_output(&output, &paper).is_err());
    output.confidence = 80;
    output.support[0].quote = "absent quote".to_string();
    assert!(agent_json::validate_generator_output(&output, &paper).is_err());
}

#[test]
fn generator_validation_preserves_raw_answer_but_trusts_exact_support_quote() {
    let paper = canonicalize_paper(sample_paper()).expect("paper");
    let output = GeneratorAgentOutput {
        question: "What value anchors the calibrated fixture?".to_string(),
        answer: "The calibrated value is one.".to_string(),
        difficulty_rationale: "hard".to_string(),
        expected_failure_mode: "paraphrase".to_string(),
        required_key_points: vec![
            "Alpha equals one".to_string(),
            "calibrated".to_string(),
            "fixture".to_string(),
        ],
        support: vec![SupportQuote {
            section_id: "s1".to_string(),
            section_hash: paper.sections[0].section_hash.clone(),
            quote: "Alpha equals one in the calibrated fixture.".to_string(),
            why_it_matters: "support".to_string(),
        }],
        confidence: 80,
    };

    agent_json::validate_generator_output(&output, &paper).expect("valid support quote");
    assert_ne!(output.answer, output.support[0].quote);
}

#[test]
fn full_text_validation_rejects_abstract_only_production_paper() {
    let mut paper = sample_paper();
    paper.sections = vec![PaperSection {
        section_id: "abstract".to_string(),
        title: "Abstract".to_string(),
        text: "Only an abstract is present.".to_string(),
        section_hash: String::new(),
    }];
    paper.license.source_url = Some("https://openaccess.example/paper".to_string());
    paper.retrieval_receipts = vec![serde_json::json!({"kind":"test_import"})];
    let paper = canonicalize_paper(paper).expect("paper");
    assert!(validate_full_text_paper(&paper, true).is_err());
}

#[test]
fn live_support_quote_candidates_are_exact_body_quotes() {
    let mut paper = sample_paper();
    paper.title = "Real Results Paper".to_string();
    paper.sections = vec![
        PaperSection {
            section_id: "s1".to_string(),
            title: "Results".to_string(),
            text: "The measured fracture strain increased to 87 percent after the third annealing pass, while the control group remained below 40 percent in the same assay. This follow-up sentence is shorter.".to_string(),
            section_hash: String::new(),
        },
        PaperSection {
            section_id: "refs".to_string(),
            title: "References".to_string(),
            text: "The reference list contains 100 entries and should never be used as a hard answer support quote.".to_string(),
            section_hash: String::new(),
        },
    ];
    let paper = canonicalize_paper(paper).expect("paper");

    let candidates = crate::paper_tournament::support_quote_candidates(&paper);
    assert!(!candidates.is_empty());
    assert_eq!(candidates[0].id, "q001");
    assert_eq!(candidates[0].section_id, "s1");
    assert!(paper.sections[0].text.contains(&candidates[0].quote));
    assert_eq!(candidates[0].section_hash, paper.sections[0].section_hash);
}

#[test]
fn live_paper_quality_rejects_corrections_and_errata() {
    let mut paper = sample_paper();
    paper.title = "Correction: Alpha Paper".to_string();
    assert!(!crate::paper_tournament::paper_quality_allowed(&paper));
    paper.title = "Alpha Paper With Full Results".to_string();
    paper.sections = vec![PaperSection {
        section_id: "results".to_string(),
        title: "Results".to_string(),
        text: "The measured fracture strain increased to 87 percent after the third annealing pass, while the control group remained below 40 percent in the same assay. ".repeat(12),
        section_hash: String::new(),
    }];
    assert!(crate::paper_tournament::paper_quality_allowed(&paper));
    paper.sections[0].title = "Table captions".to_string();
    paper.sections[0].text =
        "Table 1 caption reports a value. Figure 2 caption repeats the value. ".repeat(30);
    assert!(!crate::paper_tournament::paper_quality_allowed(&paper));
}

#[test]
fn testing_prompt_blinds_target_and_includes_distractor_content_without_answer_key() {
    let primary = canonicalize_paper(sample_paper()).expect("primary");
    let mut distractor = sample_paper();
    distractor.title = "Distractor Paper".to_string();
    distractor.dedupe_keys = vec!["doi:10.1/distractor".to_string()];
    distractor.source_ids = vec!["doi:10.1/distractor".to_string()];
    distractor.sections[0].text =
        "The distractor result reports a calibrated beta value of 19 percent after the control pass."
            .to_string();
    let distractor = canonicalize_paper(distractor).expect("distractor");
    let prompt = build_testing_prompt(
        &primary,
        &[primary.clone(), distractor.clone()],
        &[distractor.publication_hash.clone()],
        "Which calibrated fixture value does the result section state?",
    );

    assert!(prompt.contains("Paper "));
    assert!(prompt.contains("Distractor Paper"));
    assert!(prompt.contains("calibrated beta value"));
    assert!(prompt.contains(
        "Confidence must mean confidence that your answer contains every requested material detail"
    ));
    assert!(!prompt.contains("Primary paper"));
    assert!(!prompt.contains("Distractor paper:"));
    assert!(!prompt.contains("Answer key:"));
    assert!(!prompt.contains("hard_answer"));
}

#[test]
fn tournament_majority_and_grader_reduction_follow_plan_thresholds() {
    let verifier = |accepted: bool, index: usize| VerificationTrial {
        agent_name: format!("v{index}"),
        output: VerificationAgentOutput {
            accepted,
            answer: "answer".to_string(),
            confidence: 80,
            support_correct: accepted,
            reason: "reason".to_string(),
            missing_or_wrong_support: Vec::new(),
        },
        receipt: AgentCallReceipt {
            agent_name: format!("v{index}"),
            phase: "verification".to_string(),
            prompt_hash: "p".to_string(),
            context_hash: "c".to_string(),
            raw_output_hash: "r".to_string(),
            route_metadata: None,
            token_usage: None,
        },
    };
    assert!(verification_majority(&[
        verifier(true, 1),
        verifier(true, 2),
        verifier(true, 3),
        verifier(false, 4),
        verifier(false, 5),
    ]));
    assert!(!verification_majority(&[
        verifier(true, 1),
        verifier(true, 2),
        verifier(false, 3),
        verifier(false, 4),
        verifier(false, 5),
    ]));

    let grading = |correct: bool, index: usize| GradingTrial {
        agent_name: format!("g{index}"),
        testing_agent_name: "tester".to_string(),
        output: GradingAgentOutput {
            correct,
            score_0_100: if correct { 90 } else { 10 },
            matched_key_points: Vec::new(),
            missed_key_points: Vec::new(),
            reason: "reason".to_string(),
        },
        receipt: AgentCallReceipt {
            agent_name: format!("g{index}"),
            phase: "grading".to_string(),
            prompt_hash: "p".to_string(),
            context_hash: "c".to_string(),
            raw_output_hash: "r".to_string(),
            route_metadata: None,
            token_usage: None,
        },
    };
    let reduced = grade_reduction(
        &[grading(true, 1), grading(true, 2), grading(false, 3)],
        "tester",
    )
    .expect("reduction");
    assert!(reduced.0);
    assert!(reduced.1 > 60.0);
    assert!(grade_reduction(&[grading(true, 1), grading(true, 2)], "tester").is_none());
}

#[test]
fn mock_tournament_preserves_decimal_answer_text() {
    let root = tempfile::tempdir().expect("tempdir");
    let bank = root.path().join("bank");
    let run_root = root.path().join("run");
    let config = BuildPaperTournamentConfig {
        bank,
        run_root,
        target_accepted: 1,
        candidate_papers: 1,
        generators: 3,
        verifiers: 3,
        testers: 3,
        graders: 3,
        min_successful_generators: 1,
        min_successful_verifiers: 3,
        min_successful_testers: 3,
        min_successful_graders: 3,
        distractor_papers: 2,
        strict_production: false,
        agent_runner: AgentRunnerMode::Mock,
        jnoccio_base_url: None,
        jnoccio_model: None,
        jnoccio_max_output_tokens: 4096,
        jnoccio_request_timeout_seconds: 120,
        paper_timeout_seconds: 900,
        phase_retries: 2,
        generator_pool_target: 5,
        max_question_alternates_per_paper: 5,
        blind_prescreen_testers: 3,
        blind_prescreen_max_correct_rate: 0.34,
        min_support_quote_score: 10,
        hard_distractors: false,
        mask_blind_context_metadata: false,
        route_model_deny: Vec::new(),
        route_model_allow: Vec::new(),
        write_rejection_analysis: true,
        progress_jsonl: None,
        candidate_manifest: None,
        resume: false,
        allow_mock_smoke: true,
        mock_agents: None,
    };
    let summary = build_paper_tournament(&config).expect("build tournament");
    let artifact_path = summary
        .sample_accepted_artifact
        .expect("sample accepted artifact");
    let artifact: FinalPaperChallengeArtifact = read_json(&artifact_path).expect("artifact");
    assert_eq!(
        final_paper_challenge_artifact_hash(&artifact).expect("artifact hash"),
        artifact.artifact_hash
    );
    assert!(artifact.hard_answer.contains("42.7 microjoules"));
    assert!(artifact.hard_answer.contains("annealing pass"));
    assert!(artifact.generation_trials[0].output.support[0]
        .quote
        .contains("42.7 microjoules"));
    assert_eq!(
        artifact.hard_answer,
        artifact.generation_trials[0].output.support[0].quote
    );
}

#[test]
fn mock_tournament_writes_accepted_artifact_and_challenge() {
    let root = tempfile::tempdir().expect("tempdir");
    let bank = root.path().join("bank");
    let run_root = root.path().join("run");
    let config = BuildPaperTournamentConfig {
        bank: bank.clone(),
        run_root: run_root.clone(),
        target_accepted: 1,
        candidate_papers: 1,
        generators: 3,
        verifiers: 3,
        testers: 3,
        graders: 3,
        min_successful_generators: 1,
        min_successful_verifiers: 3,
        min_successful_testers: 3,
        min_successful_graders: 3,
        distractor_papers: 2,
        strict_production: false,
        agent_runner: AgentRunnerMode::Mock,
        jnoccio_base_url: None,
        jnoccio_model: None,
        jnoccio_max_output_tokens: 4096,
        jnoccio_request_timeout_seconds: 120,
        paper_timeout_seconds: 900,
        phase_retries: 2,
        generator_pool_target: 5,
        max_question_alternates_per_paper: 5,
        blind_prescreen_testers: 3,
        blind_prescreen_max_correct_rate: 0.34,
        min_support_quote_score: 10,
        hard_distractors: false,
        mask_blind_context_metadata: false,
        route_model_deny: Vec::new(),
        route_model_allow: Vec::new(),
        write_rejection_analysis: true,
        progress_jsonl: None,
        candidate_manifest: None,
        resume: false,
        allow_mock_smoke: true,
        mock_agents: None,
    };
    let summary = build_paper_tournament(&config).expect("build tournament");
    assert_eq!(summary.accepted, 1);
    assert!(summary.reduce_report.exists());
    let artifact_path = summary
        .sample_accepted_artifact
        .expect("sample accepted artifact");
    let artifact: FinalPaperChallengeArtifact = read_json(&artifact_path).expect("artifact");
    assert!(!artifact.paper_content.full_text.is_empty());
    assert!(!artifact.hard_question.is_empty());
    assert!(!artifact.hard_answer.is_empty());
    assert!(!artifact.artifact_hash.is_empty());
    assert_eq!(artifact.acceptance_metrics.saturated_mean_confidence, 0.0);
    assert!(run_root.join("reports/failure-summary.json").exists());

    let challenges = read_challenges(&bank).expect("challenges");
    assert_eq!(challenges.len(), 1);
    assert_eq!(
        challenges[0]
            .acceptance_metrics
            .as_ref()
            .expect("metrics")
            .saturated_mean_confidence,
        0.0
    );
    assert!(!challenges[0].challenge_hash.is_empty());
    assert!(challenges[0]
        .artifact_provenance
        .as_ref()
        .is_some_and(|provenance| provenance.fixture_provenance));
    assert!(!production_acceptance_errors(&challenges[0]).is_empty());
}
