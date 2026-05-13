//! Run cogcore as a registered candidate against the public smoke suite
//! and the seeded generator. Phase 1 target: total in the reference band
//! [70, 90]; Phase 2+ should push past 85.

use memory_benchmark::runner::run_candidate;

#[test]
fn cogcore_t0_smoke_clears_phase_2_target() {
    let report = run_candidate("cogcore").expect("cogcore candidate must run");
    // Phase 2 target: ≥ 85 on T0. Phase 1 stub hit 79.96 (matched
    // reference_context_pack). Phase 2 adds BM25 + Hebbian + FSRS +
    // concept attach + RecallTouch and is expected to clear the
    // 85-point milestone.
    assert!(
        report.total >= 85.0,
        "cogcore total {} below Phase 2 target 85 (fixtures {}/{})",
        report.total,
        report.fixtures_passed,
        report.fixtures_run
    );
    assert!(
        report.fixtures_run >= 80,
        "expected at least 80 fixtures executed, got {}",
        report.fixtures_run
    );
    assert!(!report.json.is_empty(), "report JSON should not be empty");
}
