use super::*;
use crate::adapters::baseline;
use crate::{Query, QueryIntent};
use crate::MemorySystem;
use std::path::Path;

#[test]
fn legacy_fixture_challenge_still_loads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/real-paper-bank");
    let challenges = load_challenges(&root).expect("load challenges");
    assert_eq!(challenges.len(), 1);
    assert_eq!(challenges[0].answer_key.canonical, "alpha equals one");
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
