use memory_benchmark::corpus::real_papers::load_challenges;
use memory_benchmark::runner::run_candidate_with_config;
use memory_benchmark::{Split, SuiteConfig};
use std::path::Path;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn loads_openqg_question_bank_challenges() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/real-paper-bank");
    let challenges = load_challenges(&root).expect("load challenges");
    assert_eq!(challenges.len(), 1);
    assert_eq!(challenges[0].answer_key.canonical, "alpha equals one");
}

#[test]
fn bench_real_papers_fixture_bank_is_dev_only() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    std::env::set_var("memory_benchmark_dev_qbank", "1");
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
    assert!(report.json.contains("\"dev_only\":true"));
    std::env::remove_var("memory_benchmark_dev_qbank");
}

#[test]
fn bench_real_papers_fixture_bank_fails_in_production() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    std::env::remove_var("memory_benchmark_dev_qbank");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/real-paper-bank");
    let config = SuiteConfig {
        split: Split::RealPapers,
        paper_bank_path: Some(root.display().to_string()),
        ..SuiteConfig::default()
    };
    let err = match run_candidate_with_config("reference_evidence_ledger", &config) {
        Ok(_) => panic!("fixture bank must not pass production"),
        Err(err) => err,
    };
    assert!(err.contains("missing paper JSON"), "{err}");
}
