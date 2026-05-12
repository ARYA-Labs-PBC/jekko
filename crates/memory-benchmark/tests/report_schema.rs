use memory_benchmark::runner::{run_candidate, run_candidate_with_config};
use memory_benchmark::{Split, SuiteConfig};

#[test]
fn generated_report_contains_gates_and_ci() {
    let config = SuiteConfig {
        split: Split::PublicGenerated,
        fixture_count: 10,
        ..SuiteConfig::default()
    };
    let report = run_candidate_with_config("baseline", &config).expect("report");
    assert!(report.json.contains("\"gate_findings\""));
    assert!(report.json.contains("\"bootstrap_ci\""));
}

#[test]
fn static_suite_still_has_100_fixtures() {
    let report = run_candidate("baseline").expect("report");
    assert_eq!(report.fixtures_run, 100);
}
