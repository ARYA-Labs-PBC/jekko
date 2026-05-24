//! Unit tests for the advanced reasoning runner. Extracted from the original
//! single-file module to keep `reasoning_runner.rs` under the audit shape
//! threshold.

use std::fs;
use std::path::Path;

use async_trait::async_trait;
use tempfile::tempdir;

use anyhow::Result;
use jekko_store::db::Db;

use crate::bootstrap_check;
use crate::evidence::LoadedEvidence;
use crate::model_client::{FakeModelClient, ModelCallReceipt, ModelClient};
use crate::model_policy::ModelTaskKind;
use crate::port::{
    EvidenceInput, EvidenceInputKind, PortProofs, PortRuntimeOptions, PortTargetRequest,
};
use crate::reasoning::AdvancedReasoningConfig;
use crate::stage0_proof::build_stage0_master_plan;

use super::orchestrator::run_advanced_reasoning_tick_with_db;

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
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("src/lib.rs"),
        "pub fn ping() { helper(); }\nfn helper() {}\n",
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

fn target() -> PortTargetRequest {
    PortTargetRequest {
        target: "MiniKV".into(),
        replacement: "MiniKV Rust".into(),
        target_repo: None,
        replacement_repo: None,
        request: "port MiniKV".into(),
        worker_cap: 4,
    }
}

#[tokio::test]
async fn fake_advanced_tick_persists_artifacts_and_parity() {
    let dir = tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    bootstrap_repo(dir.path());
    let report = run_advanced_reasoning_tick_with_db(
        dir.path(),
        "run-advanced-1",
        target(),
        AdvancedReasoningConfig {
            enabled: true,
            worker_cap: 4,
            ..AdvancedReasoningConfig::default()
        },
        PortRuntimeOptions::default(),
        true,
        &FakeModelClient::success("not json but fake is allowed"),
        &db,
    )
    .await
    .unwrap();

    assert_eq!(report.advanced.state, "complete");
    assert_eq!(report.advanced.lane_count, 4);
    assert!(report.advanced.reasoning_graph_json.exists());
    assert!(report.advanced.parity_raw_jsonl.exists());
    assert!(
        jekko_store::daemon::list_reasoning_artifacts_for_run(db.connection(), "run-advanced-1")
            .unwrap()
            .len()
            >= 4
    );
}

struct InvalidLiveJsonClient;

#[async_trait]
impl ModelClient for InvalidLiveJsonClient {
    async fn complete(
        &self,
        kind: ModelTaskKind,
        _prompt: &str,
        _cwd: &Path,
    ) -> Result<ModelCallReceipt> {
        Ok(ModelCallReceipt {
            id: format!("invalid-{kind:?}"),
            kind: crate::model_client::kind_label(kind).to_string(),
            task_id: None,
            provider: "live-test".to_string(),
            model: "bad-json".to_string(),
            latency_ms: 1,
            success: true,
            cost_usd: Some(0.0),
            response: Some("not json".to_string()),
            error: None,
            budget_used: None,
            budget_remaining: None,
            route: Some(crate::model_client::kind_label(kind).to_string()),
            credential_policy: None,
            credential_user_id: None,
            retry_count: Some(0),
        })
    }
}

#[tokio::test]
async fn invalid_live_json_blocks_run_after_retries() {
    let dir = tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    bootstrap_repo(dir.path());
    let err = run_advanced_reasoning_tick_with_db(
        dir.path(),
        "run-advanced-bad-json",
        target(),
        AdvancedReasoningConfig {
            enabled: true,
            ..AdvancedReasoningConfig::default()
        },
        PortRuntimeOptions::default(),
        true,
        &InvalidLiveJsonClient,
        &db,
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(err.contains("model JSON parse failed"));
    let run = jekko_store::daemon::get_run(db.connection(), "run-advanced-bad-json")
        .unwrap()
        .unwrap();
    assert_eq!(run.status, "blocked");
}

#[test]
fn stage0_plan_is_derived_from_minikv_fixture_evidence() {
    let evidence = vec![LoadedEvidence {
        id: "fixture-plan".into(),
        kind: EvidenceInputKind::File,
        role: "target_plan".into(),
        source: "fixture.txt".into(),
        bytes_read: 64,
        clipped: false,
        sha256: "abc".into(),
        content: "MiniKV supports PUT GET DELETE TTL and compare-and-swap parity".into(),
        unavailable_reason: None,
    }];
    let plan = build_stage0_master_plan(target(), &evidence);
    let names = plan
        .stages
        .iter()
        .map(|stage| stage.name.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(names.contains("minikv") || names.contains("supports") || names.contains("parity"));
    assert!(!names.contains("cluster"));
    assert!(!names.contains("streams"));
}

#[tokio::test]
async fn requested_proofs_write_stage0_manifest_and_benchmark() {
    let dir = tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    bootstrap_repo(dir.path());
    fs::write(
        dir.path().join("fixture-plan.txt"),
        "MiniKV plan: PUT GET DELETE TTL parity with compact snapshots.",
    )
    .unwrap();
    let runtime = PortRuntimeOptions {
        evidence_inputs: vec![EvidenceInput {
            id: "fixture-plan".into(),
            kind: EvidenceInputKind::File,
            role: "target_plan".into(),
            path_or_url: "fixture-plan.txt".into(),
            max_bytes: 256,
        }],
        proofs: PortProofs {
            redis_jedis_stage0: true,
            reasoning_benchmark: true,
        },
        ..PortRuntimeOptions::default()
    };
    let report = run_advanced_reasoning_tick_with_db(
        dir.path(),
        "run-advanced-proofs",
        target(),
        AdvancedReasoningConfig {
            enabled: true,
            worker_cap: 2,
            ..AdvancedReasoningConfig::default()
        },
        runtime,
        true,
        &FakeModelClient::success("not json but fake is allowed"),
        &db,
    )
    .await
    .unwrap();
    assert!(report
        .advanced
        .stage0_master_plan_json
        .as_ref()
        .unwrap()
        .exists());
    assert!(report
        .advanced
        .reasoning_benchmark_json
        .as_ref()
        .unwrap()
        .exists());
    assert!(report.advanced.parity_generated_manifest_json.exists());
    assert!(report.advanced.parity_approved_ci_txt.exists());
    let benchmark = fs::read_to_string(report.advanced.reasoning_benchmark_json.unwrap()).unwrap();
    assert!(benchmark.contains("\"winner\": \"tournament\""));
}
