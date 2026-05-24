use std::path::Path;

use serde_yaml::Value;

use super::query::{
    recursive_bool_key, recursive_key_exists, recursive_numeric_keys, recursive_string_contains,
    recursive_values_for_key, yaml_f64, yaml_path, yaml_sequence_at,
};
use super::types::{finding, LintFinding};

pub(super) fn lint_strict(path: &Path, yaml: &Value, findings: &mut Vec<LintFinding>) {
    if recursive_bool_key(yaml, &["store_raw_reasoning", "raw_reasoning"], true) {
        finding(
            findings,
            path,
            "SUPER001_RAW_REASONING",
            "strict superreasoning runbooks must not store raw reasoning",
        );
    }
    for (key, value) in recursive_numeric_keys(yaml) {
        if matches!(key.as_str(), "worker_cap" | "max_workers" | "max_parallel") && value > 10 {
            finding(
                findings,
                path,
                "SUPER002_WORKER_CAP",
                "superreasoning worker caps must be <= 10",
            );
        }
    }
    lint_required_policy(path, yaml, findings);
    lint_required_gates(path, yaml, findings);
    lint_confidence_cap(path, yaml, findings);
}

fn lint_required_policy(path: &Path, yaml: &Value, findings: &mut Vec<LintFinding>) {
    if !credential_policy_is_users_only(yaml) {
        finding(
            findings,
            path,
            "SUPER003_USERS_ONLY",
            "live superreasoning runbooks must declare users-only credential policy",
        );
    }
    if recursive_string_contains(yaml, "jnoccio") {
        finding(
            findings,
            path,
            "SUPER004_JNOCCIO_DEFAULT",
            "strict superreasoning runbooks must not hardcode generic jnoccio defaults",
        );
    }
}

fn lint_required_gates(path: &Path, yaml: &Value, findings: &mut Vec<LintFinding>) {
    if !recursive_key_exists(yaml, "negative_memory")
        && !recursive_key_exists(yaml, "require_negative_memory")
    {
        finding(
            findings,
            path,
            "SUPER005_NEGATIVE_MEMORY",
            "strict superreasoning runbooks must declare negative memory",
        );
    }
    if !recursive_key_exists(yaml, "unsupported_claims")
        && !recursive_key_exists(yaml, "require_unsupported_claims_ledger")
    {
        finding(
            findings,
            path,
            "SUPER006_UNSUPPORTED_CLAIMS",
            "strict superreasoning runbooks must declare an unsupported-claims ledger",
        );
    }
    if !recursive_key_exists(yaml, "replay") && !recursive_key_exists(yaml, "require_replay_gate") {
        finding(
            findings,
            path,
            "SUPER007_REPLAY_GATE",
            "strict superreasoning runbooks must require a replay gate",
        );
    }
    lint_parity_and_done(path, yaml, findings);
}

fn lint_parity_and_done(path: &Path, yaml: &Value, findings: &mut Vec<LintFinding>) {
    if !yaml_sequence_at(yaml, &["parity_lab", "fail_on"])
        .map(|items| !items.is_empty())
        .unwrap_or(false)
    {
        finding(
            findings,
            path,
            "SUPER008_PARITY_FAIL_ON",
            "strict superreasoning runbooks must declare parity fail-on policy",
        );
    }
    let forbids_model_only_claim = yaml_sequence_at(yaml, &["done", "forbid"])
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .any(|item| item == "model_only_claim")
        })
        .unwrap_or(false);
    if !forbids_model_only_claim {
        finding(
            findings,
            path,
            "SUPER009_DONE_CRITERIA",
            "strict superreasoning runbooks must declare safe done criteria",
        );
    }
}

fn lint_confidence_cap(path: &Path, yaml: &Value, findings: &mut Vec<LintFinding>) {
    match recursive_values_for_key(yaml, "confidence_cap")
        .into_iter()
        .find_map(yaml_f64)
    {
        Some(value) if value > 0.0 && value <= 1.0 => {}
        _ => finding(
            findings,
            path,
            "SUPER010_CONFIDENCE_CAP",
            "strict superreasoning runbooks must declare a bounded confidence cap",
        ),
    }
}

fn credential_policy_is_users_only(yaml: &Value) -> bool {
    [
        &["superreasoning", "credential_policy"][..],
        &["hero_judge", "super_reasoning", "credential_policy"][..],
        &["superreasoning", "live_local", "credential_policy"][..],
    ]
    .iter()
    .any(|path| {
        yaml_path(yaml, path)
            .and_then(Value::as_str)
            .map(|value| value == "users-only")
            .unwrap_or(false)
    })
}
