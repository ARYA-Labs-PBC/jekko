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
