use memory_benchmark::oracle::temporal::visible_at;
use memory_benchmark::{Event, EventKind, PrivacyClass};

fn event(id: &str, tx: &str, valid_from: &str, valid_to: Option<&str>) -> Event {
    Event {
        id: id.to_string(),
        kind: EventKind::Claim,
        subject: id.to_string(),
        body: id.to_string(),
        sources: vec![],
        valid_from: Some(valid_from.to_string()),
        valid_to: valid_to.map(str::to_string),
        tx_time: tx.to_string(),
        event_time: None,
        observation_time: None,
        review_time: None,
        policy_time: None,
        dependencies: vec![],
        supersedes: vec![],
        contradicts: vec![],
        derived_from: vec![],
        namespace: None,
        privacy_class: PrivacyClass::Public,
        claim_modality: None,
        tags: vec![],
    }
}

#[test]
fn temporal_oracle_separates_valid_and_tx_time() {
    let events = vec![
        event(
            "visible",
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00Z",
            None,
        ),
        event(
            "future_tx",
            "2026-03-01T00:00:00Z",
            "2026-01-01T00:00:00Z",
            None,
        ),
        event(
            "expired",
            "2026-01-01T00:00:00Z",
            "2025-01-01T00:00:00Z",
            Some("2026-01-15T00:00:00Z"),
        ),
    ];
    let got = visible_at(
        &events,
        Some("2026-02-01T00:00:00Z"),
        Some("2026-02-01T00:00:00Z"),
        None,
        None,
    );
    assert_eq!(
        got.iter()
            .map(|event| event.id.as_str())
            .collect::<Vec<_>>(),
        vec!["visible"]
    );
}
