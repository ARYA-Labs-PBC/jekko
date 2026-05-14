//! Scale validation — ingest 10K cells, measure recall p50/p99, assert
//! memory ceiling. Uses `std::time::Instant` for measurement only (not on
//! the hot path; this is test-side timing).
//!
//! Determinism: synthetic events are generated from FNV-1a hash of (topic,
//! idx) so the content is reproducible; recall queries are issued in a
//! fixed loop so order is stable.

use cogcore::core::{
    ClaimModality, Core, Intent, PrivacyClass, RecallQuery, SourceRef, StoredEvent,
};
use std::time::Instant;

fn synthetic_event(topic: &str, idx: usize) -> StoredEvent {
    let subject = format!("{} subject {}", topic, idx);
    let body = format!(
        "Synthetic body for {} cell {} — describes properties P{}, Q{}, R{}.",
        topic,
        idx,
        idx % 7,
        idx % 11,
        idx % 13
    );
    StoredEvent {
        id: String::new(),
        kind: "Claim".to_string(),
        subject,
        body,
        tx_time: format!("2026-{:02}-{:02}T00:00:00Z", (idx % 12) + 1, (idx % 28) + 1),
        valid_from: Some("2026-01-01T00:00:00Z".to_string()),
        valid_to: None,
        privacy_class: PrivacyClass::Public,
        claim_modality: Some(ClaimModality::Observed),
        tags: Vec::new(),
        sources: vec![SourceRef {
            uri: format!("doi:scale-{idx:05}"),
            citation: format!("Scale test source {idx}"),
            quality: 0.9,
        }],
        supersedes: Vec::new(),
        contradicts: Vec::new(),
    }
}

fn percentile(mut nanos: Vec<u128>, p: f64) -> u128 {
    nanos.sort();
    let idx = ((nanos.len() as f64 - 1.0) * p).round() as usize;
    nanos[idx]
}

// Performance assertion: skipped in dev profile (cargo test) where
// recall p99 is dominated by debug-overhead. Always runs in release
// (cargo test --release). Run explicitly with:
//   cargo test --manifest-path crates/cogcore/Cargo.toml --release scale_10k
#[cfg_attr(debug_assertions, ignore)]
#[test]
fn scale_10k_ingest_then_recall_under_budget() {
    let mut core = Core::default();
    let n = 10_000usize;
    let topics = ["neutrino", "muon", "photon", "tau", "boson"];

    // Ingest phase — 10K cells across 5 topics. Time it (informational).
    let ingest_start = Instant::now();
    for idx in 0..n {
        let topic = topics[idx % topics.len()];
        let _ = core.observe(synthetic_event(topic, idx));
    }
    let ingest_elapsed = ingest_start.elapsed();
    let ingest_per_cell_us = ingest_elapsed.as_micros() as f64 / n as f64;
    eprintln!(
        "scale_10k: ingested {} cells in {:?} ({:.2} µs/cell mean)",
        n, ingest_elapsed, ingest_per_cell_us
    );

    // Memory ceiling: state_bytes < 256 MiB
    let state_bytes = core.state_bytes();
    eprintln!(
        "scale_10k: state_bytes={} ({:.2} MiB)",
        state_bytes,
        state_bytes as f64 / (1024.0 * 1024.0)
    );
    assert!(
        state_bytes < 256 * 1024 * 1024,
        "state_bytes {} exceeded 256 MiB",
        state_bytes
    );

    // Recall phase — 200 warm queries (after a 50-query warmup to populate
    // any lazy caches inside cogcore).
    let warm_q = |topic: &str, idx: usize| RecallQuery {
        text: format!("{} subject {}", topic, idx),
        mentions: vec![topic.to_string()],
        intent: Intent::Recall,
        token_budget: 4096,
    };

    // Warmup (results discarded)
    for w in 0..50usize {
        let topic = topics[w % topics.len()];
        let _ = core.recall(&warm_q(topic, w * 37));
    }

    // Measured loop
    let mut times: Vec<u128> = Vec::with_capacity(200);
    for q in 0..200usize {
        let topic = topics[q % topics.len()];
        let query = warm_q(topic, q * 17);
        let t0 = Instant::now();
        let result = core.recall(&query);
        let dt = t0.elapsed().as_nanos();
        times.push(dt);
        // Sanity: every query should hit at least one cell.
        assert!(!result.used_ids.is_empty(), "query {} returned no hits", q);
    }

    let p50 = percentile(times.clone(), 0.50);
    let p95 = percentile(times.clone(), 0.95);
    let p99 = percentile(times.clone(), 0.99);
    eprintln!(
        "scale_10k: recall latency p50={:.2}µs p95={:.2}µs p99={:.2}µs",
        p50 as f64 / 1000.0,
        p95 as f64 / 1000.0,
        p99 as f64 / 1000.0
    );

    // Phase 2 expectation: p99 < 5ms on 10K cells (warm cache).
    // CI machines vary; allow 20ms ceiling to absorb noise but log a
    // warning if p99 > 5ms.
    let p99_ms = p99 as f64 / 1_000_000.0;
    assert!(
        p99_ms < 20.0,
        "recall p99 {:.2}ms exceeded 20ms ceiling (target: < 5ms)",
        p99_ms
    );
    if p99_ms >= 5.0 {
        eprintln!(
            "scale_10k: WARNING — p99 recall {:.2}ms exceeded 5ms target (still under 20ms ceiling)",
            p99_ms
        );
    }
}
