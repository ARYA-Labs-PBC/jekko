use super::*;
use crate::adapters::baseline;
use crate::MemorySystem;
use crate::{Query, QueryIntent};
use std::path::Path;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn legacy_fixture_challenge_still_loads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/real-paper-bank");
    let challenges = load_challenges(&root).expect("load challenges");
    assert_eq!(challenges.len(), 1);
    assert_eq!(challenges[0].answer_key.canonical, "alpha equals one");
}

#[test]
fn manifest_array_loads_multiple_challenges() {
    let root = std::env::temp_dir().join(format!(
        "memory-benchmark-qbank-array-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("challenges")).expect("create temp qbank");
    std::fs::write(
        root.join("challenges/manifest.json"),
        r#"[
  {
    "challenge_hash": "qbank-a",
    "publication_hash": "paper-a",
    "question": "What is result A?",
    "answer_key": "result A",
    "support_sections": ["s1"],
    "acceptance": { "accepted": true, "reason": "fixture" }
  },
  {
    "challenge_hash": "qbank-b",
    "publication_hash": "paper-b",
    "question": "What is result B?",
    "answer_key": "result B",
    "support_sections": ["s1"],
    "acceptance": { "accepted": true, "reason": "fixture" }
  }
]"#,
    )
    .expect("write manifest");
    let challenges = load_challenges(&root).expect("load manifest array");
    assert_eq!(challenges.len(), 2);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn answer_key_is_not_observed_as_memory_event() {
    let loaded = LoadedChallenge {
        paper: Some(PaperRecord {
            publication_hash: "paper-a".to_string(),
            title: "Paper A".to_string(),
            license_spdx: "CC-BY-4.0".to_string(),
            redistributable: true,
            sections: vec![PaperSection {
                section_id: "s1".to_string(),
                title: "Result".to_string(),
                text: "The paper discusses alpha without revealing the hidden oracle phrase."
                    .to_string(),
                section_hash: "h1".to_string(),
            }],
        }),
        challenge: PaperChallenge {
            challenge_hash: "challenge-a".to_string(),
            publication_hash: "paper-a".to_string(),
            domain: "science".to_string(),
            topics: vec![],
            difficulty_score: 1.0,
            answerability: 1.0,
            focused_correct_rate: 1.0,
            blind_correct_rate: 0.0,
            question: "What is the hidden oracle phrase?".to_string(),
            answer_key: AnswerKey {
                canonical: "forbidden answer key phrase".to_string(),
                must_include: vec!["forbidden answer key phrase".to_string()],
                ..AnswerKey::default()
            },
            support: vec![SupportRef {
                section_id: "s1".to_string(),
                section_hash: "h1".to_string(),
            }],
            context_pack: ContextPack {
                target_section_ids: vec!["s1".to_string()],
                ..ContextPack::default()
            },
        },
    };
    let mut adapter = baseline::Adapter::default();
    super::run::observe_paper(&mut adapter, &loaded).expect("observe");
    let result = adapter.recall(&Query {
        text: "paper-a".to_string(),
        intent: QueryIntent::Fact,
        mentions: vec!["paper-a".to_string()],
        token_budget: 4096,
    });
    assert!(!result.answer.contains("forbidden answer key phrase"));
}

#[test]
fn production_missing_paper_fails_without_fixture_fallback() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    std::env::remove_var("memory_benchmark_dev_qbank");
    let loaded = LoadedChallenge {
        paper: None,
        challenge: fixture_challenge("missing-paper", 1.0, 0.0),
    };
    let mut adapter = baseline::Adapter::default();
    let err = super::run::observe_paper(&mut adapter, &loaded).expect_err("missing paper fails");
    assert!(err.contains("missing paper JSON for paper"));
}

#[test]
fn dev_fixture_qbank_mode_passes_and_marks_report_dev_only() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    std::env::set_var("memory_benchmark_dev_qbank", "1");
    let root = temp_qbank_dir("dev-fixture-qbank-mode");
    std::fs::create_dir_all(root.join("challenges")).expect("create challenges");
    std::fs::write(
        root.join("challenges/manifest.json"),
        r#"[
  {
    "challenge_hash": "qbank-dev",
    "publication_hash": "paper-dev",
    "question": "What is the fixture-only result?",
    "answer_key": { "canonical": "fixture-only result", "must_include": ["fixture-only"] },
    "support_sections": ["s1"],
    "acceptance": { "accepted": true, "reason": "fixture", "answerability": 1.0, "focused_correct_rate": 1.0, "blind_correct_rate": 0.0 }
  }
]"#,
    )
    .expect("write manifest");
    let mut adapter = baseline::Adapter::default();
    let config = crate::SuiteConfig {
        qbank_top_n: 1,
        ..crate::SuiteConfig::default()
    };
    let report = super::run::run_candidate("baseline", &mut adapter, &root, &config)
        .expect("dev fixture report");
    assert!(report.json.contains("\"dev_only\":true"));
    assert!(report.json.contains("\"qbank_trusted\":false"));
    std::env::remove_var("memory_benchmark_dev_qbank");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn validate_bank_requires_papers_unless_dev_mode_is_explicit() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    std::env::remove_var("memory_benchmark_dev_qbank");
    let root = temp_qbank_dir("validate-requires-paper");
    std::fs::create_dir_all(root.join("challenges")).expect("create challenges");
    std::fs::write(
        root.join("challenges/manifest.json"),
        r#"[
  {
    "challenge_hash": "qbank-prod",
    "publication_hash": "paper-prod",
    "question": "What is the result?",
    "answer_key": "result",
    "support_sections": ["s1"],
    "acceptance": { "accepted": true, "answerability": 1.0, "focused_correct_rate": 1.0, "blind_correct_rate": 0.0 }
  }
]"#,
    )
    .expect("write manifest");
    let prod = super::validation::validate_bank(&root, false, 50).expect("validate prod");
    assert!(
        prod.errors
            .iter()
            .any(|err| err.contains("missing redistributable paper JSON for paper-prod")),
        "errors: {:?}",
        prod.errors
    );

    std::env::set_var("memory_benchmark_dev_qbank", "1");
    let dev = super::validation::validate_bank(&root, false, 50).expect("validate dev");
    assert!(dev.errors.is_empty(), "dev errors: {:?}", dev.errors);
    assert!(dev
        .warnings
        .iter()
        .any(|warning| warning.contains("dev_only fixture qbank mode enabled")));
    std::env::remove_var("memory_benchmark_dev_qbank");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn top_n_sort_uses_hardness_then_rates_then_hashes() {
    let mut a = fixture_challenge("a", 0.9, 0.1);
    let b = fixture_challenge("b", 0.8, 0.0);
    let c = fixture_challenge("c", 0.9, 0.4);
    a.publication_hash = "paper-a".to_string();
    let mut loaded = vec![
        LoadedChallenge {
            challenge: b,
            paper: None,
        },
        LoadedChallenge {
            challenge: c,
            paper: None,
        },
        LoadedChallenge {
            challenge: a,
            paper: None,
        },
    ];
    loaded.sort_by(super::run::challenge_order);
    assert_eq!(
        loaded
            .iter()
            .map(|item| item.challenge.challenge_hash.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "c", "b"]
    );
}

fn fixture_challenge(hash: &str, difficulty: f32, blind: f32) -> PaperChallenge {
    PaperChallenge {
        challenge_hash: hash.to_string(),
        publication_hash: "paper".to_string(),
        domain: "science".to_string(),
        topics: vec![],
        difficulty_score: difficulty,
        answerability: 1.0,
        focused_correct_rate: 1.0,
        blind_correct_rate: blind,
        question: "q".to_string(),
        answer_key: AnswerKey {
            canonical: "alpha equals one".to_string(),
            must_include: vec!["alpha".to_string()],
            must_not_include: vec![],
            aliases: vec![],
            numeric_tolerances: vec![],
            unit_tolerances: vec![],
        },
        support: vec![SupportRef {
            section_id: "s1".to_string(),
            section_hash: "h1".to_string(),
        }],
        context_pack: ContextPack::default(),
    }
}

fn temp_qbank_dir(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("memory-benchmark-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    root
}
