use crate::hero_judge::{HeroJudgeLaneArtifact, HeroJudgeSearchReceipt};
use crate::model_policy::ModelTaskKind;

pub(super) fn storage_safe_summary(value: &str) -> String {
    value
        .replace("chain-of-thought", "private reasoning")
        .replace("chain_of_thought", "private_reasoning")
        .chars()
        .take(320)
        .collect()
}

pub(super) fn arrayish_count(value: &serde_json::Value, keys: &[&str]) -> usize {
    keys.iter()
        .filter_map(|key| value.get(*key))
        .map(value_count)
        .sum::<usize>()
}

fn value_count(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Array(items) => items.len(),
        serde_json::Value::Object(items) => items.len(),
        serde_json::Value::String(text) if text.trim().is_empty() => 0,
        serde_json::Value::String(_) => 1,
        _ => 0,
    }
}

pub(super) fn evidence_grounding_score(value: &serde_json::Value, evidence_refs: usize) -> f64 {
    let text = value.to_string().to_ascii_lowercase();
    let marker_bonus = ["sha256", "evidence", "doi", "arxiv", "citation", "source"]
        .iter()
        .filter(|marker| text.contains(**marker))
        .count();
    (normalized(evidence_refs, 4) * 0.70 + normalized(marker_bonus, 3) * 0.30).clamp(0.0, 1.0)
}

pub(super) fn storage_safety_score(value: &serde_json::Value, summary: &str) -> f64 {
    let text = format!("{} {}", value, summary).to_ascii_lowercase();
    if text.contains("hidden_canary")
        || text.contains("fixture_leak")
        || text.contains("raw_chain_of_thought")
        || text.contains("chain_of_thought")
        || text.contains("chain-of-thought")
    {
        0.0
    } else {
        1.0
    }
}

pub(super) fn structural_score(
    kind: ModelTaskKind,
    value: &serde_json::Value,
    claims: usize,
    questions: usize,
    rubric: usize,
    evidence_refs: usize,
) -> f64 {
    let mut passed = 0.0;
    let mut total = 3.0;
    if value.get("summary").is_some() {
        passed += 1.0;
    }
    if value.get("score").is_some() {
        passed += 1.0;
    }
    if claims > 0 {
        passed += 1.0;
    }
    if kind == ModelTaskKind::HeroGenerate {
        total += 1.0;
        if questions > 0 {
            passed += 1.0;
        }
    }
    if matches!(
        kind,
        ModelTaskKind::JudgePatch
            | ModelTaskKind::Verifier
            | ModelTaskKind::MetaJudge
            | ModelTaskKind::RedTeam
    ) {
        total += 1.0;
        if rubric > 0 {
            passed += 1.0;
        }
    }
    total += 1.0;
    if evidence_refs > 0 {
        passed += 1.0;
    }
    passed / total
}

pub(super) fn normalized(count: usize, target: usize) -> f64 {
    if target == 0 {
        return 1.0;
    }
    (count as f64 / target as f64).clamp(0.0, 1.0)
}

pub(super) fn mean_metric(
    artifacts: &[HeroJudgeLaneArtifact],
    key: &str,
    default_score: f64,
) -> f64 {
    if artifacts.is_empty() {
        return default_score;
    }
    artifacts
        .iter()
        .map(|artifact| metric_value(artifact, key, default_score))
        .sum::<f64>()
        / artifacts.len() as f64
}

pub(super) fn mean_metric_all(
    groups: &[&[HeroJudgeLaneArtifact]],
    key: &str,
    default_score: f64,
) -> f64 {
    let mut total = 0.0;
    let mut count = 0_usize;
    for group in groups {
        for artifact in *group {
            total += metric_value(artifact, key, default_score);
            count += 1;
        }
    }
    if count == 0 {
        default_score
    } else {
        total / count as f64
    }
}

pub(super) fn max_metric(
    artifacts: &[HeroJudgeLaneArtifact],
    key: &str,
    default_score: f64,
) -> f64 {
    artifacts
        .iter()
        .map(|artifact| metric_value(artifact, key, default_score))
        .max_by(f64::total_cmp)
        .unwrap_or(default_score)
}

pub(super) fn metric_value(artifact: &HeroJudgeLaneArtifact, key: &str, default_score: f64) -> f64 {
    artifact
        .metrics
        .get(key)
        .copied()
        .unwrap_or(default_score)
        .clamp(0.0, 1.0)
}

pub(super) fn search_receipt_score(receipts: &[HeroJudgeSearchReceipt]) -> f64 {
    if receipts.is_empty() {
        return 0.0;
    }
    let ok = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .count() as f64;
    let url_density = receipts
        .iter()
        .map(|receipt| receipt.url_count)
        .sum::<usize>() as f64
        / receipts.len() as f64;
    ((ok / receipts.len() as f64) * 0.70 + (url_density / 4.0).clamp(0.0, 1.0) * 0.30)
        .clamp(0.0, 1.0)
}

pub(super) fn red_team_penalty(artifacts: &[HeroJudgeLaneArtifact]) -> f64 {
    if artifacts.is_empty() {
        return 0.0;
    }
    let pressure = artifacts
        .iter()
        .map(|artifact| metric_value(artifact, "red_team_pressure", artifact.score * 0.50))
        .sum::<f64>()
        / artifacts.len() as f64;
    (pressure * 0.08).clamp(0.0, 0.08)
}

pub(super) fn leak_status(artifact: &HeroJudgeLaneArtifact) -> String {
    let text = artifact.summary.to_ascii_lowercase();
    if text.contains("hidden_canary")
        || text.contains("fixture_leak")
        || text.contains("hidden constant")
        || text.contains("raw_chain_of_thought")
        || text.contains("chain_of_thought")
    {
        "leak_detected".to_string()
    } else {
        "clean".to_string()
    }
}
