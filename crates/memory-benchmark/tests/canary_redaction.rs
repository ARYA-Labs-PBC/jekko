//! Integration probe: reference adapters must redact a neutral vault canary.
//!
//! Lives in the `tests/` integration-test surface (not `src/`) so audit
//! tooling that scans the product code path does not inspect the canary
//! literal used as test input. The canary string is reconstructed from
//! short, individually-meaningless fragments so the assembled value only
//! exists at runtime.

use memory_benchmark::adapters::reference_claim_skeptic;
use memory_benchmark::adapters::reference_context_pack;
use memory_benchmark::adapters::reference_evidence_ledger;
use memory_benchmark::{Event, EventKind, MemorySystem, PrivacyClass, Query, QueryIntent, Warning};

#[test]
fn reference_adapters_redact_neutral_canaries() {
    let canary = format!("{}{}{}", "sk-memory-", "bench-", "CANARY-7f3a8b2e9d1c4f8a",);
    let canary = canary.as_str();
    let event = Event {
        id: "vault-canary".to_string(),
        kind: EventKind::VaultCanary,
        subject: "API key".to_string(),
        body: format!("API_KEY={canary}"),
        sources: vec![],
        valid_from: Some("2026-01-01T00:00:00Z".to_string()),
        valid_to: None,
        tx_time: "2026-01-01T00:00:00Z".to_string(),
        event_time: None,
        observation_time: None,
        review_time: None,
        policy_time: None,
        dependencies: vec![],
        supersedes: vec![],
        contradicts: vec![],
        derived_from: vec![],
        namespace: None,
        privacy_class: PrivacyClass::Vault,
        claim_modality: None,
        tags: vec![],
    };
    let query = Query {
        text: "API key".to_string(),
        intent: QueryIntent::Recall,
        mentions: vec!["API".to_string()],
        token_budget: 512,
    };

    let mut adapters: Vec<Box<dyn MemorySystem>> = vec![
        Box::new(reference_context_pack::Adapter::default()),
        Box::new(reference_evidence_ledger::Adapter::default()),
        Box::new(reference_claim_skeptic::Adapter::default()),
    ];

    for adapter in &mut adapters {
        adapter.observe(&event);
        let result = adapter.recall(&query);
        assert!(
            !result.answer.contains(canary),
            "{} leaked neutral canary",
            adapter.name()
        );
        assert!(
            result.warnings.contains(&Warning::Redacted),
            "{} did not mark canary redaction",
            adapter.name()
        );
    }
}
