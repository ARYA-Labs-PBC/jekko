use std::fs;

use tempfile::tempdir;

use super::*;

fn valid_superreasoning_body(extra: &str) -> String {
    format!(
        "<<<ZYAL v1:daemon id=case>>>\nsuperreasoning:\n  max_workers: 10\n  credential_policy: users-only\n  confidence_cap: 0.95\n  negative_memory: x\n  unsupported_claims: x\n  replay: required\n{extra}\nparity_lab:\n  fail_on: [failed_case]\ndone:\n  forbid: [model_only_claim]\n<<<END_ZYAL id=case>>>\n"
    )
}

#[test]
fn zyal_lint_super_rejects_worker_cap_over_10() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.zyal");
    fs::write(
        &path,
        "<<<ZYAL v1:daemon id=bad>>>\nsuperreasoning:\n  max_workers: 11\n  credential_policy: users-only\n  confidence_cap: 0.95\n  negative_memory: x\n  unsupported_claims: x\n  replay: required\nparity_lab:\n  fail_on: [failed_case]\ndone:\n  forbid: [model_only_claim]\n<<<END_ZYAL id=bad>>>\n",
    )
    .unwrap();
    let report = lint_file(&path, true).unwrap();
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.code == "SUPER002_WORKER_CAP"));
}

#[test]
fn zyal_lint_super_rejects_hardcoded_jnoccio_defaults() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.zyal");
    fs::write(
        &path,
        "<<<ZYAL v1:daemon id=bad>>>\nsuperreasoning:\n  max_workers: 10\n  credential_policy: users-only\n  confidence_cap: 0.95\n  negative_memory: x\n  unsupported_claims: x\n  replay: required\nmodel_policy:\n  routine: jnoccio/routine\nparity_lab:\n  fail_on: [failed_case]\ndone:\n  forbid: [model_only_claim]\n<<<END_ZYAL id=bad>>>\n",
    )
    .unwrap();
    let report = lint_file(&path, true).unwrap();
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.code == "SUPER004_JNOCCIO_DEFAULT"));
}

#[test]
fn zyal_lint_super_rejects_raw_reasoning_storage() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.zyal");
    fs::write(
        &path,
        valid_superreasoning_body("  store_raw_reasoning: true"),
    )
    .unwrap();
    let report = lint_file(&path, true).unwrap();
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.code == "SUPER001_RAW_REASONING"));
}

#[test]
fn zyal_lint_super_rejects_missing_users_only_policy() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.zyal");
    fs::write(
        &path,
        "<<<ZYAL v1:daemon id=bad>>>\nsuperreasoning:\n  max_workers: 10\n  confidence_cap: 0.95\n  negative_memory: x\n  unsupported_claims: x\n  replay: required\nparity_lab:\n  fail_on: [failed_case]\ndone:\n  forbid: [model_only_claim]\n<<<END_ZYAL id=bad>>>\n",
    )
    .unwrap();
    let report = lint_file(&path, true).unwrap();
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.code == "SUPER003_USERS_ONLY"));
}

#[test]
fn zyal_lint_super_rejects_missing_replay_parity_and_done_forbids() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.zyal");
    fs::write(
        &path,
        "<<<ZYAL v1:daemon id=bad>>>\nsuperreasoning:\n  max_workers: 10\n  credential_policy: users-only\n  confidence_cap: 0.95\n  negative_memory: x\n  unsupported_claims: x\ndone:\n  forbid: [raw_chain_of_thought_storage]\n<<<END_ZYAL id=bad>>>\n",
    )
    .unwrap();
    let report = lint_file(&path, true).unwrap();
    for code in [
        "SUPER007_REPLAY_GATE",
        "SUPER008_PARITY_FAIL_ON",
        "SUPER009_DONE_CRITERIA",
    ] {
        assert!(
            report.findings.iter().any(|finding| finding.code == code),
            "missing finding {code}: {:?}",
            report.findings
        );
    }
}

#[test]
fn zyal_lint_super_rejects_missing_confidence_cap() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.zyal");
    fs::write(
        &path,
        "<<<ZYAL v1:daemon id=bad>>>\nsuperreasoning:\n  max_workers: 10\n  credential_policy: users-only\n  negative_memory: x\n  unsupported_claims: x\n  replay: required\nparity_lab:\n  fail_on: [failed_case]\ndone:\n  forbid: [model_only_claim]\n<<<END_ZYAL id=bad>>>\n",
    )
    .unwrap();
    let report = lint_file(&path, true).unwrap();
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.code == "SUPER010_CONFIDENCE_CAP"));
}
