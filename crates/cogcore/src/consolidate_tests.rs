use super::*;
use crate::core::{ClaimModality, PrivacyClass, SourceRef};

fn ev(id: &str, q: f32, contradicts: Vec<String>) -> StoredEvent {
    StoredEvent {
        id: id.to_string(),
        kind: "Claim".to_string(),
        subject: "topic".to_string(),
        body: format!("body of {id}"),
        tx_time: "2026-01-01T00:00:00Z".to_string(),
        valid_from: None,
        valid_to: None,
        privacy_class: PrivacyClass::Public,
        claim_modality: Some(ClaimModality::Observed),
        tags: Vec::new(),
        sources: vec![SourceRef {
            uri: format!("doi:{id}"),
            citation: format!("Source {id}"),
            quality: q,
        }],
        supersedes: Vec::new(),
        contradicts,
    }
}

#[test]
fn rule_backend_summarize_picks_highest_quality_member() {
    let mut backend = RuleBackend;
    let mut budget = Budget::ZERO;
    let a = ev("a", 0.7, vec![]);
    let b = ev("b", 0.95, vec![]); // highest
    let c = ev("c", 0.5, vec![]);
    let members = vec![&a, &b, &c];
    let topic = crate::concept::Topic {
        id: 1,
        label: "test-topic".to_string(),
        concepts: Vec::new(),
        strength: 0.5,
        half_life_hours: 24.0,
        last_update_tx: "2026-01-01T00:00:00Z".to_string(),
        contradiction_pressure: 0.0,
        stats: crate::topic::empty_stats(),
    };
    let lesson = backend
        .summarize_topic(&topic, &members, &mut budget)
        .unwrap();
    assert!(lesson.summary_body.contains("body of b"));
    assert_eq!(lesson.source_cell_ids.len(), 3);
}

#[test]
fn rule_backend_verifies_known_si_units() {
    let mut backend = RuleBackend;
    let mut budget = Budget::ZERO;
    let eq = EqAtom {
        lhs: "E".to_string(),
        op: "=".to_string(),
        rhs: "mc^2".to_string(),
        units: Some("J".to_string()),
    };
    assert_eq!(
        backend.verify_equation_units(&eq, &[], &mut budget),
        Some(UnitVerdict::Consistent)
    );
}

#[test]
fn rule_backend_unverifiable_on_unknown_unit() {
    let mut backend = RuleBackend;
    let mut budget = Budget::ZERO;
    let eq = EqAtom {
        lhs: "X".to_string(),
        op: "=".to_string(),
        rhs: "Y".to_string(),
        units: Some("zorkbargs".to_string()),
    };
    assert_eq!(
        backend.verify_equation_units(&eq, &[], &mut budget),
        Some(UnitVerdict::Unverifiable)
    );
}

#[test]
fn rule_backend_unverifiable_when_no_units() {
    let mut backend = RuleBackend;
    let mut budget = Budget::ZERO;
    let eq = EqAtom {
        lhs: "X".to_string(),
        op: "=".to_string(),
        rhs: "Y".to_string(),
        units: None,
    };
    assert_eq!(
        backend.verify_equation_units(&eq, &[], &mut budget),
        Some(UnitVerdict::Unverifiable)
    );
}

#[test]
fn rule_backend_flags_low_quality_contradiction() {
    let mut backend = RuleBackend;
    let mut budget = Budget::ZERO;
    let bad = ev("bad", 0.2, vec!["good".to_string()]);
    let good = ev("good", 0.95, vec![]);
    let peers = vec![&good];
    let flag = backend.detect_adversarial_claim(&bad, &peers, &mut budget);
    assert!(flag.is_some());
    let f = flag.unwrap();
    assert_eq!(f.cell_id, "bad");
    assert_eq!(f.conflicting_peers, vec!["good".to_string()]);
}

#[test]
fn rule_backend_skips_high_quality_cell() {
    let mut backend = RuleBackend;
    let mut budget = Budget::ZERO;
    let good = ev("good", 0.95, vec!["other".to_string()]);
    let other = ev("other", 0.95, vec![]);
    let peers = vec![&other];
    assert!(backend
        .detect_adversarial_claim(&good, &peers, &mut budget)
        .is_none());
}

#[test]
fn default_backend_no_op_methods_return_none() {
    struct StubBackend;
    impl ConsolidationBackend for StubBackend {}
    let mut backend = StubBackend;
    let mut budget = Budget::ZERO;
    let cell = ev("x", 0.9, vec![]);
    assert!(backend
        .detect_adversarial_claim(&cell, &[], &mut budget)
        .is_none());
    let eq = EqAtom {
        lhs: "a".to_string(),
        op: "=".to_string(),
        rhs: "b".to_string(),
        units: None,
    };
    assert!(backend
        .verify_equation_units(&eq, &[], &mut budget)
        .is_none());
}
