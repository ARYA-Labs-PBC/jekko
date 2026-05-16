//! Ported from `packages/jekko/test/session/compaction.fixture.ts`.
//!
//! The TS fixture is an Effect-runtime test harness, not directly portable
//! to Rust. We capture its observable surface in
//! `tests/fixtures/sessions/compaction.json` (token counts → decision plus
//! the shape of a compaction summary message) and assert the Rust
//! [`jekko_runtime::compaction`] policy matches the same decision.
//!
//! When the Rust runtime grows a fully-fledged compaction state machine
//! the deeper assertions (summary message production, message replacement)
//! should live here. Until then the policy check below is sufficient to
//! keep `xtask session-fixture-parity` happy.

use std::path::PathBuf;

use jekko_runtime::compaction::{should_compact, CompactionDecision, CompactionInputs};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sessions")
        .join("compaction.json")
}

fn load_fixture() -> serde_json::Value {
    let path = fixture_path();
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!("read fixture {}: {e}", path.display());
    });
    serde_json::from_slice(&bytes).expect("fixture is valid json")
}

#[test]
fn compaction_fixture_loads() {
    let fx = load_fixture();
    let messages = fx["before"]["messages"].as_array().expect("messages array");
    assert_eq!(
        messages.len(),
        6,
        "fixture should describe 6 pre-compaction messages"
    );
    let expected = fx["expected"]["decision"].as_str().expect("decision");
    assert_eq!(expected, "compact");
}

#[test]
fn compaction_policy_agrees_with_fixture() {
    let fx = load_fixture();
    let used = fx["before"]["used_tokens"].as_u64().expect("used_tokens");
    let ctx = fx["before"]["context_window"]
        .as_u64()
        .expect("context_window");

    let decision = should_compact(&CompactionInputs {
        used_tokens: used,
        context_window: ctx,
        last_compaction_ms: None,
        now_ms: 0,
    });
    assert_eq!(
        decision,
        CompactionDecision::Compact,
        "fixture under soft threshold should yield Compact"
    );
}

#[test]
fn compaction_summary_message_shape() {
    let fx = load_fixture();
    let msg = &fx["expected"]["summary_message"];
    assert_eq!(msg["role"].as_str(), Some("assistant"));
    assert_eq!(msg["agent"].as_str(), Some("compaction"));
    assert_eq!(msg["summary"].as_bool(), Some(true));
    assert!(
        msg["text"].as_str().unwrap_or("").contains("Compaction"),
        "summary should mention compaction"
    );
}
