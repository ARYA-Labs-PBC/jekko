use std::fs;
use std::path::Path;

use jankurai_runner::bootstrap_check;
use jankurai_runner::hero_judge::{HeroJudgeMissingProviderPolicy, HeroJudgeRunbook};
use jankurai_runner::hero_judge_runner::run_hero_judge_run_with_db;
use jankurai_runner::model_client::FakeModelClient;
use jekko_store::db::Db;
use tempfile::tempdir;

fn bootstrap_repo(dir: &Path) {
    std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir)
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .status()
        .unwrap();
    for file in bootstrap_check::CANONICAL_FILES {
        let abs = dir.join(file.rel);
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(abs, "").unwrap();
    }
    fs::create_dir_all(dir.join("docs")).unwrap();
    fs::write(
        dir.join("docs/zyal-research-loops.md"),
        "OpenQG research loops require verified evidence and receipts.",
    )
    .unwrap();
    fs::create_dir_all(dir.join("tips/rolling")).unwrap();
    fs::write(
        dir.join("tips/rolling/tip1.txt"),
        "admit falsifiable theories",
    )
    .unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-q", "-m", "seed"])
        .current_dir(dir)
        .status()
        .unwrap();
}

fn runbook() -> HeroJudgeRunbook {
    serde_yaml::from_str(
        r#"
job:
  name: openqg-hero-judge
  objective: Evolve OpenQG prompts.
hero_judge:
  generations: 1
  population:
    hero_lanes: 2
    judge_lanes: 1
    verifier_lanes: 1
    literature_lanes: 1
    red_team_lanes: 1
    max_parallel: 2
  budgets:
    model_calls: 12
    search_queries: 1
    search_pages: 2
"#,
    )
    .unwrap()
}

#[tokio::test]
async fn deterministic_run_writes_required_artifacts() {
    let dir = tempdir().unwrap();
    bootstrap_repo(dir.path());
    let db = Db::open_in_memory().unwrap();
    let report = run_hero_judge_run_with_db(
        dir.path(),
        "hero-judge-smoke",
        &dir.path().join("agent/zyal/openqg-hero-judge-evolve.zyal"),
        runbook(),
        Some(1),
        false,
        &FakeModelClient::success("not json but fake is allowed"),
        &db,
    )
    .await
    .unwrap();
    assert!(report.prompt_lineage_json.exists());
    assert!(report.frontier_scoreboard_json.exists());
    assert!(report.promotion_decision_json.exists());
    assert!(report.knowledge_compound_jsonl.exists());
    assert!(report.search_receipts_json.exists());
    assert!(report.quality_metrics_jsonl.exists());
    assert!(report.quality_metrics_csv.exists());
    assert!(report.quality_trend_json.exists());
    assert!(report.complete_ok.exists());
    assert_eq!(report.knowledge_entry_count, 1);
    assert!(report.last_promotion_decision.promoted);
}

#[tokio::test]
async fn deterministic_metrics_show_quality_trend() {
    let dir = tempdir().unwrap();
    bootstrap_repo(dir.path());
    let db = Db::open_in_memory().unwrap();
    let mut rb = runbook();
    rb.hero_judge.generations = 2;
    let report = run_hero_judge_run_with_db(
        dir.path(),
        "hero-judge-trend",
        &dir.path().join("agent/zyal/openqg-hero-judge-evolve.zyal"),
        rb,
        None,
        false,
        &FakeModelClient::success("not json but fake is allowed"),
        &db,
    )
    .await
    .unwrap();
    let metrics = fs::read_to_string(&report.quality_metrics_jsonl).unwrap();
    assert_eq!(metrics.lines().count(), 2);
    let trend: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report.quality_trend_json).unwrap()).unwrap();
    assert_eq!(
        trend.get("generations").and_then(serde_json::Value::as_u64),
        Some(2)
    );
    assert!(
        trend
            .get("delta_overall_quality")
            .and_then(serde_json::Value::as_f64)
            .unwrap()
            >= 0.0
    );
}

#[tokio::test]
async fn fail_missing_live_search_when_policy_requires_it() {
    let dir = tempdir().unwrap();
    bootstrap_repo(dir.path());
    let db = Db::open_in_memory().unwrap();
    let mut rb = runbook();
    rb.hero_judge.research.live_when_available = true;
    rb.hero_judge.research.missing_provider = HeroJudgeMissingProviderPolicy::Fail;
    let err = run_hero_judge_run_with_db(
        dir.path(),
        "hero-judge-no-search",
        &dir.path().join("agent/zyal/openqg-hero-judge-evolve.zyal"),
        rb,
        Some(1),
        true,
        &FakeModelClient::success("{}"),
        &db,
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(err.contains("AGENT_SEARCH_LIVE"));
}
