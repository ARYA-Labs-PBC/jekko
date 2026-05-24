use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::*;

fn write(path: &Path, value: &serde_json::Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_string_pretty(value).unwrap()).unwrap();
}

fn good_artifact(dir: &Path, name: &str, bytes: &[u8]) -> serde_json::Value {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    serde_json::json!({
        "path": path.display().to_string(),
        "sha256": sha256_hex(bytes),
    })
}

fn fixtures(dir: &Path) -> (serde_json::Value, serde_json::Value) {
    let a = good_artifact(dir, "claim_ledger.jsonl", b"{}\n");
    let b = good_artifact(dir, "negative_memory.jsonl", b"{}\n");
    let packet = serde_json::json!({
        "schema_version": PACKET_SCHEMA,
        "run_id": "run-1",
        "objective": "test",
        "source_runbook_sha256": "src",
        "effective_generations": 1,
        "budget_contract": {"effective_generations": 1, "model_call_budget": 8, "search_query_budget": 1, "search_page_budget": 2, "max_parallel": 2, "max_workers": 2},
        "model_route_contract": {},
        "lane_plan": [{"id": "literature", "role": "research", "route": "routine", "max_workers": 2, "required_artifacts": []}],
        "artifact_contract": {"required_artifacts": [], "forbidden_content": ["raw_chain_of_thought"], "claim_ledger": "claim_ledger.jsonl", "unsupported_claims_ledger": "uns.jsonl", "negative_memory": "negative_memory.jsonl"},
        "privacy_contract": {"store_raw_reasoning": false, "users_only_credentials": true, "model_visible_target_values": false, "storage_safe_summaries_only": true},
        "promotion_gates": {"proof_gate": true, "replay_gate": true, "parity_gate": true, "leak_gate": true, "jankurai_gate": true},
        "credential_policy": "users-only",
        "policy_hash": "policy-hash",
        "replay_receipt": {},
        "stable_hash": "stable-hash",
    });
    let receipt = serde_json::json!({
        "schema_version": RECEIPT_SCHEMA,
        "run_id": "run-1",
        "packet_hash": "stable-hash",
        "policy_hash": "policy-hash",
        "source_runbook_sha256": "src",
        "artifact_hashes": [a, b],
        "gate_results": {
            "proof_gate": {"status": "passed", "required": true, "evidence": ["e"]},
            "replay_gate": {"status": "passed", "required": true, "evidence": ["e"]},
            "parity_gate": {"status": "not_applicable", "required": false, "evidence": [], "message": "no parity target"},
            "leak_gate": {"status": "passed", "required": true, "evidence": ["e"]},
            "jankurai_gate": {"status": "passed", "required": true, "evidence": ["e"]},
        },
        "replay_gate_passed": true,
        "parity_gate_passed": true,
        "leak_gate_passed": true,
        "jankurai_gate_passed": true,
        "proof_gate_passed": true,
        "status": "passed",
    });
    (packet, receipt)
}

#[test]
fn verify_passes_on_consistent_fixtures() {
    let dir = tempdir().unwrap();
    let (packet, receipt) = fixtures(dir.path());
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let report = verify(dir.path()).unwrap();
    assert_eq!(report.status, "passed", "failures: {:?}", report.failures);
    assert_eq!(report.exit_code(), 0);
    assert_eq!(report.gates.len(), 5);
}

#[test]
fn verify_detects_packet_receipt_hash_drift() {
    let dir = tempdir().unwrap();
    let (mut packet, receipt) = fixtures(dir.path());
    packet["stable_hash"] = serde_json::Value::String("DRIFTED".into());
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let report = verify(dir.path()).unwrap();
    assert_eq!(report.status, "failed");
    assert!(report
        .failures
        .iter()
        .any(|f| f.contains("packet stable_hash") && f.contains("receipt.packet_hash")));
}

#[test]
fn verify_detects_artifact_tamper() {
    let dir = tempdir().unwrap();
    let (packet, receipt) = fixtures(dir.path());
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let target = receipt["artifact_hashes"][0]["path"].as_str().unwrap();
    fs::write(target, b"tampered").unwrap();
    let report = verify(dir.path()).unwrap();
    assert_eq!(report.status, "failed");
    assert_eq!(report.artifact_hash_mismatches.len(), 1);
    assert!(report.failures.iter().any(|f| f.contains("hash mismatch")));
}

#[test]
fn verify_detects_missing_artifact() {
    let dir = tempdir().unwrap();
    let (packet, receipt) = fixtures(dir.path());
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let target = receipt["artifact_hashes"][0]["path"].as_str().unwrap();
    fs::remove_file(target).unwrap();
    let report = verify(dir.path()).unwrap();
    assert_eq!(report.status, "failed");
    assert_eq!(report.missing_artifacts.len(), 1);
}

#[test]
fn verify_rejects_raw_reasoning_privacy_contract() {
    let dir = tempdir().unwrap();
    let (mut packet, receipt) = fixtures(dir.path());
    packet["privacy_contract"]["store_raw_reasoning"] = serde_json::Value::Bool(true);
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let report = verify(dir.path()).unwrap();
    assert_eq!(report.status, "failed");
    assert!(report
        .failures
        .iter()
        .any(|f| f.contains("store_raw_reasoning")));
}

#[test]
fn verify_rejects_oversized_lane_workers() {
    let dir = tempdir().unwrap();
    let (mut packet, receipt) = fixtures(dir.path());
    packet["lane_plan"][0]["max_workers"] = serde_json::Value::Number(11.into());
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let report = verify(dir.path()).unwrap();
    assert_eq!(report.status, "failed");
    assert!(report.failures.iter().any(|f| f.contains("max_workers=11")));
}

#[test]
fn verify_rejects_gate_failed_status() {
    let dir = tempdir().unwrap();
    let (packet, mut receipt) = fixtures(dir.path());
    receipt["gate_results"]["parity_gate"]["status"] = serde_json::Value::String("failed".into());
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let report = verify(dir.path()).unwrap();
    assert_eq!(report.status, "failed");
    assert!(report
        .failures
        .iter()
        .any(|f| f.contains("gate parity_gate status is failed")));
}

#[test]
fn verify_rejects_pending_gate() {
    let dir = tempdir().unwrap();
    let (packet, mut receipt) = fixtures(dir.path());
    receipt["gate_results"]["leak_gate"]["status"] = serde_json::Value::String("pending".into());
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let report = verify(dir.path()).unwrap();
    assert_eq!(report.status, "failed");
    assert!(report
        .failures
        .iter()
        .any(|f| f.contains("gate leak_gate status is pending")));
}

#[test]
fn verify_errors_on_missing_packet_file() {
    let dir = tempdir().unwrap();
    let (_, receipt) = fixtures(dir.path());
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let err = verify(dir.path()).unwrap_err().to_string();
    assert!(err.contains("superreasoning_packet.json"));
}

#[test]
fn verify_detects_missing_schema_version() {
    let dir = tempdir().unwrap();
    let (mut packet, receipt) = fixtures(dir.path());
    packet.as_object_mut().unwrap().remove("schema_version");
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let report = verify(dir.path()).unwrap();
    assert_eq!(report.status, "failed");
    assert!(report
        .failures
        .iter()
        .any(|f| f.contains("missing schema_version")));
}

#[test]
fn verify_detects_wrong_receipt_schema() {
    let dir = tempdir().unwrap();
    let (packet, mut receipt) = fixtures(dir.path());
    receipt["schema_version"] = serde_json::Value::String("bogus.schema.v999".into());
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let report = verify(dir.path()).unwrap();
    assert_eq!(report.status, "failed");
    assert!(report
        .failures
        .iter()
        .any(|f| f.contains("bogus.schema.v999") && f.contains(RECEIPT_SCHEMA)));
}

#[test]
fn verify_detects_missing_gate_results() {
    let dir = tempdir().unwrap();
    let (packet, mut receipt) = fixtures(dir.path());
    receipt.as_object_mut().unwrap().remove("gate_results");
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let report = verify(dir.path()).unwrap();
    assert_eq!(report.status, "failed");
    assert!(report
        .failures
        .iter()
        .any(|f| f.contains("missing gate_results")));
}

#[test]
fn verify_tolerates_unknown_extra_fields() {
    let dir = tempdir().unwrap();
    let (mut packet, mut receipt) = fixtures(dir.path());
    packet["future_extension_field"] = serde_json::json!({"foo": [1, 2, 3]});
    receipt["future_extension_field"] = serde_json::Value::String("hello".into());
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let report = verify(dir.path()).unwrap();
    assert_eq!(report.status, "passed", "failures: {:?}", report.failures);
}

#[test]
fn verify_deterministic_between_runs() {
    let dir = tempdir().unwrap();
    let (packet, receipt) = fixtures(dir.path());
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let first = verify(dir.path()).unwrap();
    let second = verify(dir.path()).unwrap();
    assert_eq!(first.status, second.status);
    assert_eq!(first.packet_hash, second.packet_hash);
    assert_eq!(first.artifact_count, second.artifact_count);
    assert_eq!(first.gates.len(), second.gates.len());
    assert_eq!(first.failures.len(), second.failures.len());
}

#[test]
fn verify_strict_returns_err_on_failure() {
    let dir = tempdir().unwrap();
    let (packet, mut receipt) = fixtures(dir.path());
    receipt["status"] = serde_json::Value::String("failed".into());
    write(&dir.path().join(PACKET_FILE), &packet);
    write(&dir.path().join(RECEIPT_FILE), &receipt);
    let err = verify_strict(dir.path()).unwrap_err().to_string();
    assert!(err.contains("replay verification failed"));
}
