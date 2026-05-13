use memory_benchmark::corpus::real_papers::load_challenges;
use memory_benchmark::runner::run_candidate_with_config;
use memory_benchmark::{Split, SuiteConfig};
use std::path::Path;

#[test]
fn loads_openqg_question_bank_challenges() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/real-paper-bank");
    let challenges = load_challenges(&root).expect("load challenges");
    assert_eq!(challenges.len(), 1);
    assert_eq!(challenges[0].answer_key.canonical, "alpha equals one");
}

#[test]
fn bench_real_papers_suite_requires_explicit_bank() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/real-paper-bank");
    let config = SuiteConfig {
        split: Split::RealPapers,
        paper_bank_path: Some(root.display().to_string()),
        ..SuiteConfig::default()
    };
    let report =
        run_candidate_with_config("reference_evidence_ledger", &config).expect("real paper report");
    assert_eq!(report.fixtures_run, 50);
    assert!(report.json.contains("\"suite\":\"real-papers\""));
}
