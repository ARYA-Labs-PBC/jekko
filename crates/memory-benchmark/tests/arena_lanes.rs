use std::collections::BTreeSet;

use memory_benchmark::runner::run_candidate_with_config;
use memory_benchmark::{Split, SuiteConfig};

#[test]
fn arena_lanes_run_and_diverge() {
    let config = SuiteConfig {
        split: Split::PublicGenerated,
        fixture_count: 12,
        seed_label: "arena-lanes-smoke".to_string(),
        ..SuiteConfig::default()
    };

    let mut totals = BTreeSet::new();

    for idx in 0..20 {
        let lane = format!("arena_lane_{idx:02}");
        let report = run_candidate_with_config(&lane, &config).expect(&lane);
        assert_eq!(report.name, lane);
        assert!(report.json.contains(&format!("\"name\":\"{}\"", lane)));
        totals.insert(format!("{:.6}", report.total));
    }

    assert!(
        totals.len() > 1,
        "expected at least two distinct arena lane totals"
    );
}
