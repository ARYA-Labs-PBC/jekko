//! Round-trip the public cogcore API to catch boundary regressions.

use cogcore::core::{
    ClaimModality, Core, FeedbackSignal, Intent, Outcome, PrivacyClass, RecallQuery, SourceRef,
    StoredEvent,
};

fn ev(id: &str, subject: &str, body: &str, tx: &str) -> StoredEvent {
    StoredEvent {
        id: id.to_string(),
        kind: "Claim".to_string(),
        subject: subject.to_string(),
        body: body.to_string(),
        tx_time: tx.to_string(),
        valid_from: Some("2020-01-01T00:00:00Z".to_string()),
        valid_to: None,
        privacy_class: PrivacyClass::Public,
        claim_modality: Some(ClaimModality::Observed),
        tags: Vec::new(),
        sources: vec![SourceRef {
            uri: "doi:example".to_string(),
            citation: "Example et al. 2024".to_string(),
            quality: 0.9,
        }],
        supersedes: Vec::new(),
        contradicts: Vec::new(),
    }
}

fn q(text: &str) -> RecallQuery {
    RecallQuery {
        text: text.to_string(),
        mentions: vec![text.to_string()],
        intent: Intent::Recall,
        token_budget: 4096,
    }
}

#[test]
fn observe_recall_feedback_forget_rebuild_round_trip() {
    let mut c = Core::default();

    let r1 = c.observe(ev(
        "e1",
        "neutrino",
        "neutrinos have mass",
        "2020-01-01T00:00:00Z",
    ));
    let r2 = c.observe(ev(
        "e2",
        "muon",
        "muons are heavier than electrons",
        "2020-01-01T00:00:00Z",
    ));
    assert_ne!(r1.hash, r2.hash, "receipt chain must advance");

    let recall = c.recall(&q("neutrino"));
    assert!(recall.used_ids.contains(&"e1".to_string()));
    assert!(!recall.context_pack_hash.is_empty());

    let fb = c.feedback(&FeedbackSignal {
        outcome: Outcome::TaskSuccess,
        used: vec!["e1".to_string()],
    });
    assert!(!fb.hash.is_empty());

    let tomb = c.forget("e2", "test cleanup");
    assert_eq!(tomb.memory_id, "e2");
    assert!(!tomb.deletion_proof.is_empty());

    let after = c.recall(&q("muon"));
    assert!(
        !after.used_ids.contains(&"e2".to_string()),
        "tombstoned event must not surface"
    );

    let h1 = c.export_state_hash();
    let rebuild = c.rebuild();
    let h2 = c.export_state_hash();
    assert_eq!(h1, h2, "rebuild must preserve state hash");
    assert!(!rebuild.hash.is_empty());
}

#[test]
fn historical_recall_does_not_mutate_state() {
    let mut c = Core::default();
    c.observe(ev("old", "topic", "v1", "2020-01-01T00:00:00Z"));
    c.observe(ev("new", "topic", "v2", "2025-01-01T00:00:00Z"));
    let h_before = c.export_state_hash();
    let _ = c.recall_as_of(&q("topic"), "2022-06-01T00:00:00Z");
    let _ = c.recall_at(&q("topic"), "2022-06-01T00:00:00Z");
    let h_after = c.export_state_hash();
    assert_eq!(h_before, h_after);
}

#[test]
fn canonical_id_is_stable_across_invocations() {
    let a = Core::canonical_event_id("Claim", "neutrino", "has mass", "2026-01-01T00:00:00Z");
    let b = Core::canonical_event_id("Claim", "neutrino", "has mass", "2026-01-01T00:00:00Z");
    assert_eq!(a, b);
    assert_eq!(a.len(), 16);
}
