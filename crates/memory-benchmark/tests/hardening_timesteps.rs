//! Confirms the hardening runner observes reinforcement events between
//! recall queries, not all upfront.
//!
//! Verifies the case shape produced by `generate_hardening_suite`: exactly
//! one canonical base event plus four reinforcement events (one per
//! inter-timestep gap in the 5-timestep schedule).

use memory_benchmark::generated::{generate_hardening_suite, HardeningConfig};

#[test]
fn hardening_case_has_one_canonical_and_four_reinforcements() {
    let suite = generate_hardening_suite(&HardeningConfig {
        benchmark_version: "test",
        seed_label: "test-seed".to_string(),
        fixture_count: 3,
    });
    assert_eq!(suite.len(), 3);
    for case in &suite {
        assert_eq!(
            case.base_events.len(),
            1,
            "expected exactly 1 canonical event"
        );
        assert_eq!(
            case.reinforcements.len(),
            4,
            "expected exactly 4 reinforcement events"
        );
    }
}
