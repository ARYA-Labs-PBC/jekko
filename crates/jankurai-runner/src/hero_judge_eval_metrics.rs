use std::collections::BTreeMap;

use crate::hero_judge::{
    FrontierScore, HeroJudgeLaneArtifact, HeroJudgeLaneMetric, HeroJudgeQualityMetric,
    HeroJudgeQualityTrend, HeroJudgeReviewCard, HeroJudgeSearchReceipt, PromotionDecision,
};
use crate::model_policy::ModelTaskKind;

pub struct GenerationMetricInputs<'a> {
    pub run_id: &'a str,
    pub generation: usize,
    pub literature: &'a [HeroJudgeLaneArtifact],
    pub heroes: &'a [HeroJudgeLaneArtifact],
    pub judges: &'a [HeroJudgeLaneArtifact],
    pub verifiers: &'a [HeroJudgeLaneArtifact],
    pub red_team: &'a [HeroJudgeLaneArtifact],
    pub meta: &'a [HeroJudgeLaneArtifact],
    pub decision: &'a PromotionDecision,
    pub search_receipts: &'a [HeroJudgeSearchReceipt],
    pub previous_overall: Option<f64>,
    pub previous_frontier: Option<f64>,
    pub knowledge_entry_count: usize,
}

pub fn average_score(artifacts: &[HeroJudgeLaneArtifact], default_score: f64) -> f64 {
    if artifacts.is_empty() {
        return default_score;
    }
    artifacts.iter().map(|artifact| artifact.score).sum::<f64>() / artifacts.len() as f64
}

pub fn summary_from_value(
    kind: ModelTaskKind,
    generation: usize,
    lane: usize,
    value: &serde_json::Value,
) -> String {
    match value
        .get("summary")
        .and_then(serde_json::Value::as_str)
        .map(storage_safe_summary)
    {
        Some(summary) => summary,
        None => format!(
            "{} generation {generation} lane {lane} completed with storage-safe summary.",
            crate::model_client::kind_label(kind)
        ),
    }
}

pub fn synthetic_lane_value(kind: ModelTaskKind, generation: usize) -> serde_json::Value {
    serde_json::json!({
        "summary": format!("deterministic {} summary", crate::model_client::kind_label(kind)),
        "claims": ["bounded evidence", "canary checked", "promotion-gated"],
        "questions": ["What falsifiable signal would move this theory up or down?"],
        "rubric": ["evidence grounding", "falsifiability", "calibration"],
        "evidence_refs": ["deterministic-local-evidence"],
        "score": lane_default_score(kind, generation),
    })
}

pub fn parse_substitute_lane_value(kind: ModelTaskKind, generation: usize) -> serde_json::Value {
    serde_json::json!({
        "summary": format!("live {} response completed but required storage-safe JSON substitute", crate::model_client::kind_label(kind)),
        "claims": [
            "live model call completed",
            "strict JSON parse failed",
            "raw provider text was not copied into the artifact"
        ],
        "questions": ["Which prompt constraint would make this lane return stricter structured JSON?"],
        "rubric": ["live receipt present", "storage-safe substitute", "requires reviewer caution"],
        "evidence_refs": ["live-model-receipt"],
        "score": (lane_default_score(kind, generation) * 0.75).clamp(0.0, 1.0),
        "parse_substitute": true,
    })
}

pub fn score_from_value(kind: ModelTaskKind, generation: usize, value: &serde_json::Value) -> f64 {
    match value.get("score").and_then(serde_json::Value::as_f64) {
        Some(score) => score.clamp(0.0, 1.0),
        None => lane_default_score(kind, generation),
    }
}

pub fn lane_default_score(kind: ModelTaskKind, generation: usize) -> f64 {
    let base = match kind {
        ModelTaskKind::HeroGenerate => 0.88,
        ModelTaskKind::JudgePatch => 0.82,
        ModelTaskKind::Verifier => 0.86,
        ModelTaskKind::LiteratureSynthesis => 0.80,
        ModelTaskKind::RedTeam => 0.20,
        ModelTaskKind::MetaJudge => 0.87,
        ModelTaskKind::KnowledgeCurate => 0.84,
        _ => 0.75,
    };
    (base + generation as f64 * 0.005).min(0.95)
}

pub fn rounded(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

pub fn lane_quality_metrics(
    kind: ModelTaskKind,
    value: &serde_json::Value,
    summary: &str,
    score: f64,
) -> BTreeMap<String, f64> {
    let claims = arrayish_count(value, &["claims", "hypotheses", "theories"]);
    let questions = arrayish_count(
        value,
        &["questions", "research_questions", "hard_questions"],
    );
    let rubric = arrayish_count(value, &["rubric", "criteria", "scoring_rubric"]);
    let evidence_refs = arrayish_count(value, &["evidence_refs", "citations", "sources"]);
    let evidence_grounding = evidence_grounding_score(value, evidence_refs);
    let storage_safety = storage_safety_score(value, summary);
    let structural = structural_score(kind, value, claims, questions, rubric, evidence_refs);
    let claim_quality =
        (score * 0.55 + normalized(claims, 4) * 0.25 + evidence_grounding * 0.20).clamp(0.0, 1.0);
    let question_quality = if kind == ModelTaskKind::HeroGenerate {
        (score * 0.45
            + normalized(questions, 5) * 0.30
            + normalized(claims, 4) * 0.10
            + evidence_grounding * 0.15)
            .clamp(0.0, 1.0)
    } else {
        (score * 0.35 + normalized(questions, 3) * 0.25 + evidence_grounding * 0.15 + 0.25)
            .clamp(0.0, 1.0)
    };
    let rubric_quality = if matches!(
        kind,
        ModelTaskKind::JudgePatch
            | ModelTaskKind::Verifier
            | ModelTaskKind::MetaJudge
            | ModelTaskKind::RedTeam
    ) {
        (score * 0.40 + normalized(rubric, 5) * 0.35 + evidence_grounding * 0.15 + 0.10)
            .clamp(0.0, 1.0)
    } else {
        (score * 0.35 + normalized(rubric, 3) * 0.25 + evidence_grounding * 0.15 + 0.25)
            .clamp(0.0, 1.0)
    };

    let mut metrics = BTreeMap::from([
        ("claim_count".to_string(), claims as f64),
        ("claim_quality".to_string(), rounded(claim_quality)),
        (
            "evidence_grounding".to_string(),
            rounded(evidence_grounding),
        ),
        ("question_count".to_string(), questions as f64),
        ("question_quality".to_string(), rounded(question_quality)),
        ("rubric_item_count".to_string(), rubric as f64),
        ("rubric_quality".to_string(), rounded(rubric_quality)),
        ("storage_safety".to_string(), rounded(storage_safety)),
        ("structural_completeness".to_string(), rounded(structural)),
    ]);
    if kind == ModelTaskKind::RedTeam {
        let red_team_pressure = (score * 0.35
            + normalized(questions, 4) * 0.25
            + normalized(rubric, 4) * 0.20
            + (1.0 - storage_safety) * 0.20)
            .clamp(0.0, 1.0);
        metrics.insert("red_team_pressure".to_string(), rounded(red_team_pressure));
    }
    metrics
}

pub fn generation_quality_metric(input: GenerationMetricInputs<'_>) -> HeroJudgeQualityMetric {
    let literature_support = average_score(input.literature, 0.80);
    let verifier_confidence = average_score(input.verifiers, 0.84);
    let judge_score = average_score(input.judges, verifier_confidence);
    let meta_score = average_score(input.meta, verifier_confidence);
    let red_team_resilience = (1.0 - (red_team_penalty(input.red_team) / 0.08)).clamp(0.0, 1.0);
    let theory_quality = (input.decision.score * 0.65
        + verifier_confidence * 0.15
        + literature_support * 0.10
        + red_team_resilience * 0.10)
        .clamp(0.0, 1.0);
    let question_quality = (mean_metric(input.heroes, "question_quality", 0.50) * 0.70
        + max_metric(input.heroes, "question_quality", 0.50) * 0.30)
        .clamp(0.0, 1.0);
    let rubric_quality = (mean_metric(input.judges, "rubric_quality", 0.50) * 0.65
        + mean_metric(input.verifiers, "rubric_quality", 0.50) * 0.20
        + mean_metric(input.meta, "rubric_quality", 0.50) * 0.15)
        .clamp(0.0, 1.0);
    let judge_calibration = (1.0 - (judge_score - verifier_confidence).abs() * 2.0)
        .min((1.0 - (meta_score - verifier_confidence).abs() * 2.0).clamp(0.0, 1.0))
        .clamp(0.0, 1.0);
    let lane_grounding = mean_metric_all(
        &[
            input.literature,
            input.heroes,
            input.judges,
            input.verifiers,
            input.red_team,
            input.meta,
        ],
        "evidence_grounding",
        0.50,
    );
    let search_grounding = search_receipt_score(input.search_receipts);
    let evidence_grounding = (lane_grounding * 0.70 + search_grounding * 0.30).clamp(0.0, 1.0);
    let overall_quality = (theory_quality * 0.35
        + question_quality * 0.15
        + rubric_quality * 0.20
        + judge_calibration * 0.10
        + evidence_grounding * 0.10
        + red_team_resilience * 0.10)
        .clamp(0.0, 1.0);
    let delta = input
        .previous_overall
        .map(|previous| overall_quality - previous)
        .unwrap_or(0.0);
    let previous_frontier = input.previous_frontier.unwrap_or(overall_quality);
    let frontier_quality = previous_frontier.max(overall_quality);
    let delta_frontier = frontier_quality - previous_frontier;

    HeroJudgeQualityMetric {
        run_id: input.run_id.to_string(),
        generation: input.generation,
        theory_quality_index: rounded(theory_quality),
        question_quality_index: rounded(question_quality),
        rubric_quality_index: rounded(rubric_quality),
        judge_calibration_index: rounded(judge_calibration),
        evidence_grounding_index: rounded(evidence_grounding),
        verifier_confidence: rounded(verifier_confidence),
        red_team_resilience: rounded(red_team_resilience),
        promotion_score: rounded(input.decision.score),
        overall_quality_index: rounded(overall_quality),
        delta_overall_quality: rounded(delta),
        frontier_quality_index: rounded(frontier_quality),
        delta_frontier_quality: rounded(delta_frontier),
        promoted: input.decision.promoted,
        hero_candidate_count: input.heroes.len(),
        judge_patch_count: input.judges.len(),
        research_receipt_count: input.search_receipts.len(),
        knowledge_entry_count: input.knowledge_entry_count,
    }
}

pub fn quality_trend(run_id: &str, metrics: &[HeroJudgeQualityMetric]) -> HeroJudgeQualityTrend {
    let first = metrics.first();
    let latest = metrics.last();
    let best = metrics
        .iter()
        .max_by(|a, b| a.overall_quality_index.total_cmp(&b.overall_quality_index));
    let start = first
        .map(|metric| metric.overall_quality_index)
        .unwrap_or(0.0);
    let latest_value = latest
        .map(|metric| metric.overall_quality_index)
        .unwrap_or(0.0);
    let start_frontier = first
        .map(|metric| metric.frontier_quality_index)
        .unwrap_or(0.0);
    let latest_frontier = latest
        .map(|metric| metric.frontier_quality_index)
        .unwrap_or(0.0);
    HeroJudgeQualityTrend {
        run_id: run_id.to_string(),
        generations: metrics.len(),
        start_overall_quality: start,
        latest_overall_quality: latest_value,
        delta_overall_quality: rounded(latest_value - start),
        start_frontier_quality: start_frontier,
        latest_frontier_quality: latest_frontier,
        delta_frontier_quality: rounded(latest_frontier - start_frontier),
        best_generation: best.map(|metric| metric.generation).unwrap_or(0),
        best_overall_quality: best
            .map(|metric| metric.overall_quality_index)
            .unwrap_or(0.0),
        improved: latest_frontier > start_frontier,
        metric_keys: vec![
            "theory_quality_index".to_string(),
            "question_quality_index".to_string(),
            "rubric_quality_index".to_string(),
            "judge_calibration_index".to_string(),
            "evidence_grounding_index".to_string(),
            "red_team_resilience".to_string(),
            "overall_quality_index".to_string(),
            "frontier_quality_index".to_string(),
        ],
    }
}

pub fn scoreboard_for_generation(
    generation: usize,
    heroes: &[HeroJudgeLaneArtifact],
    verifier_score: f64,
    red_team: &[HeroJudgeLaneArtifact],
    decision: &PromotionDecision,
) -> Vec<FrontierScore> {
    let penalty = red_team_penalty(red_team);
    heroes
        .iter()
        .map(|hero| {
            let leak_status = leak_status(hero);
            let score = if leak_status == "clean" {
                (hero.score * 0.70 + verifier_score * 0.25 - penalty).clamp(0.0, 1.0)
            } else {
                0.0
            };
            FrontierScore {
                candidate_id: hero.id.clone(),
                prompt_id: format!("prompt-{}", hero.id),
                generation,
                score,
                verifier_score,
                red_team_penalty: penalty,
                leak_status,
                status: if decision.winner_candidate_id.as_deref() == Some(hero.id.as_str())
                    && decision.promoted
                {
                    "promoted".to_string()
                } else {
                    "scored".to_string()
                },
            }
        })
        .collect()
}

pub fn review_cards(groups: &[&[HeroJudgeLaneArtifact]]) -> Vec<HeroJudgeReviewCard> {
    groups
        .iter()
        .flat_map(|group| group.iter())
        .map(|artifact| HeroJudgeReviewCard {
            artifact_id: artifact.id.clone(),
            role_group: role_group(&artifact.kind).to_string(),
            kind: artifact.kind.clone(),
            generation: artifact.generation,
            lane: artifact.lane,
            score: rounded(artifact.score),
            summary: storage_safe_summary(&artifact.summary),
            content_sha256: artifact.content_sha256.clone(),
            metrics: artifact.metrics.clone(),
        })
        .collect()
}

pub fn reviewer_questions() -> Vec<String> {
    vec![
        "Are hero artifacts becoming more derivable, falsifiable, and evidence-grounded across generations?".to_string(),
        "Are judge artifacts becoming better calibrated, less gameable, and more explicit about leakage, hidden parameters, and extraction maps?".to_string(),
        "Does the retained frontier improve without storing private reasoning or importing fixture constants?".to_string(),
    ]
}

pub fn lane_metric_records(
    run_id: &str,
    groups: &[&[HeroJudgeLaneArtifact]],
) -> Vec<HeroJudgeLaneMetric> {
    groups
        .iter()
        .flat_map(|group| group.iter())
        .map(|artifact| HeroJudgeLaneMetric {
            run_id: run_id.to_string(),
            generation: artifact.generation,
            role_group: role_group(&artifact.kind).to_string(),
            kind: artifact.kind.clone(),
            artifact_id: artifact.id.clone(),
            lane: artifact.lane,
            score: rounded(artifact.score),
            claim_quality: rounded(metric_value(artifact, "claim_quality", artifact.score)),
            question_quality: rounded(metric_value(artifact, "question_quality", artifact.score)),
            rubric_quality: rounded(metric_value(artifact, "rubric_quality", artifact.score)),
            evidence_grounding: rounded(metric_value(artifact, "evidence_grounding", 0.0)),
            structural_completeness: rounded(metric_value(
                artifact,
                "structural_completeness",
                0.0,
            )),
            storage_safety: rounded(metric_value(artifact, "storage_safety", 1.0)),
            claim_count: metric_value(artifact, "claim_count", 0.0),
            question_count: metric_value(artifact, "question_count", 0.0),
            rubric_item_count: metric_value(artifact, "rubric_item_count", 0.0),
            model_receipt_id: artifact.model_receipt_id.clone(),
            content_sha256: artifact.content_sha256.clone(),
            status: artifact.status.clone(),
        })
        .collect()
}

pub fn role_group(kind: &str) -> &'static str {
    match kind {
        "hero_generate" => "hero",
        "judge_patch" | "verifier" | "red_team" | "meta_judge" => "judge",
        "literature_synthesis" => "research",
        "knowledge_curate" => "knowledge",
        _ => "other",
    }
}

fn storage_safe_summary(value: &str) -> String {
    value
        .replace("chain-of-thought", "private reasoning")
        .replace("chain_of_thought", "private_reasoning")
        .chars()
        .take(320)
        .collect()
}

fn arrayish_count(value: &serde_json::Value, keys: &[&str]) -> usize {
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

fn evidence_grounding_score(value: &serde_json::Value, evidence_refs: usize) -> f64 {
    let text = value.to_string().to_ascii_lowercase();
    let marker_bonus = ["sha256", "evidence", "doi", "arxiv", "citation", "source"]
        .iter()
        .filter(|marker| text.contains(**marker))
        .count();
    (normalized(evidence_refs, 4) * 0.70 + normalized(marker_bonus, 3) * 0.30).clamp(0.0, 1.0)
}

fn storage_safety_score(value: &serde_json::Value, summary: &str) -> f64 {
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

fn structural_score(
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

fn normalized(count: usize, target: usize) -> f64 {
    if target == 0 {
        return 1.0;
    }
    (count as f64 / target as f64).clamp(0.0, 1.0)
}

fn mean_metric(artifacts: &[HeroJudgeLaneArtifact], key: &str, default_score: f64) -> f64 {
    if artifacts.is_empty() {
        return default_score;
    }
    artifacts
        .iter()
        .map(|artifact| metric_value(artifact, key, default_score))
        .sum::<f64>()
        / artifacts.len() as f64
}

fn mean_metric_all(groups: &[&[HeroJudgeLaneArtifact]], key: &str, default_score: f64) -> f64 {
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

fn max_metric(artifacts: &[HeroJudgeLaneArtifact], key: &str, default_score: f64) -> f64 {
    artifacts
        .iter()
        .map(|artifact| metric_value(artifact, key, default_score))
        .max_by(f64::total_cmp)
        .unwrap_or(default_score)
}

fn metric_value(artifact: &HeroJudgeLaneArtifact, key: &str, default_score: f64) -> f64 {
    artifact
        .metrics
        .get(key)
        .copied()
        .unwrap_or(default_score)
        .clamp(0.0, 1.0)
}

fn search_receipt_score(receipts: &[HeroJudgeSearchReceipt]) -> f64 {
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

fn red_team_penalty(artifacts: &[HeroJudgeLaneArtifact]) -> f64 {
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

fn leak_status(artifact: &HeroJudgeLaneArtifact) -> String {
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
