//! Deterministic Hero/Judge scoring, artifact, and parser helpers.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use jekko_store::db::Db;
use serde_json::json;

use crate::daemon_store;
use crate::hashing::{sha256_hex, sha256_json};
use crate::hero_judge::{
    FrontierScore, HeroJudgeConfig, HeroJudgeLaneArtifact, HeroJudgeLaneMetric,
    HeroJudgeQualityMetric, HeroJudgeQualityTrend, HeroJudgeReviewCard, HeroJudgeRunbook,
    HeroJudgeSearchReceipt, HeroJudgeSeriesRow, KnowledgeEntry, PromotionDecision, PromptVariant,
};
use crate::model_client::kind_label;
use crate::model_policy::ModelTaskKind;
use crate::reasoning::{
    stable_reasoning_hash, AdvancedReasoningConfig, EvidenceLevel, MemoryCapsule,
    ReasoningArtifact, ReasoningArtifactKind, ReasoningRole,
};

pub(crate) fn reduce_generation(
    run_id: &str,
    generation: usize,
    heroes: &[HeroJudgeLaneArtifact],
    verifier_score: f64,
    red_team: &[HeroJudgeLaneArtifact],
    config: &HeroJudgeConfig,
) -> PromotionDecision {
    let red_team_penalty = red_team_penalty(red_team);
    let mut best: Option<(&HeroJudgeLaneArtifact, f64, String)> = None;
    for hero in heroes {
        let leak = leak_status(hero);
        let mut score =
            (hero.score * 0.70 + verifier_score * 0.25 - red_team_penalty).clamp(0.0, 1.0);
        if config.promotion.canary_replay && leak != "clean" {
            score = 0.0;
        }
        if config.promotion.anti_leak && leak != "clean" {
            score = 0.0;
        }
        if best
            .as_ref()
            .is_none_or(|(_, best_score, _)| score > *best_score)
        {
            best = Some((hero, score, leak));
        }
    }
    let Some((winner, score, leak)) = best else {
        return PromotionDecision {
            run_id: run_id.to_string(),
            generation,
            winner_candidate_id: None,
            winner_prompt_id: None,
            score: 0.0,
            promoted: false,
            reason: "no hero candidates".to_string(),
        };
    };
    let promoted = score >= config.promotion.min_score && leak == "clean";
    PromotionDecision {
        run_id: run_id.to_string(),
        generation,
        winner_candidate_id: Some(winner.id.clone()),
        winner_prompt_id: Some(format!("prompt-{}", winner.id)),
        score,
        promoted,
        reason: if promoted {
            "passed deterministic host score, canary replay, and anti-leak gates".to_string()
        } else if leak != "clean" {
            format!("rejected by anti-leak gate: {leak}")
        } else {
            format!(
                "score {:.3} below promotion gate {:.3}",
                score, config.promotion.min_score
            )
        },
    }
}

pub(crate) fn scoreboard_for_generation(
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

pub(crate) fn knowledge_entry(
    generation: usize,
    decision: &PromotionDecision,
    evidence: &[crate::evidence::LoadedEvidence],
) -> KnowledgeEntry {
    let status = if decision.promoted {
        "verified"
    } else {
        "rejected"
    };
    let claim = if decision.promoted {
        format!(
            "Generation {generation} prompt variant {} passed the deterministic OpenQG promotion gates.",
            decision
                .winner_prompt_id
                .as_deref()
                .unwrap_or("unknown-prompt")
        )
    } else {
        format!(
            "Generation {generation} prompt variant was not promoted: {}.",
            decision.reason
        )
    };
    let mut entry = KnowledgeEntry {
        id: format!("knowledge-g{generation:03}"),
        generation,
        status: status.to_string(),
        claim,
        evidence_refs: evidence.iter().map(|item| item.id.clone()).collect(),
        source_candidate_id: decision.winner_candidate_id.clone(),
        content_sha256: String::new(),
    };
    entry.content_sha256 = sha256_json(&entry, "knowledge_entry");
    entry
}

pub(crate) fn persist_knowledge_capsule(
    db: &Db,
    run_id: &str,
    entry: &KnowledgeEntry,
) -> Result<()> {
    let config = AdvancedReasoningConfig::default();
    let mut artifact = ReasoningArtifact::new(
        format!("artifact-{}", entry.id),
        run_id,
        ReasoningRole::MemoryCurator,
        ReasoningArtifactKind::MemoryCapsule,
        format!("Hero/Judge {}", entry.id),
        entry.claim.clone(),
        EvidenceLevel::ExternalGrounding,
        if entry.status == "verified" { 0.8 } else { 0.6 },
        serde_json::to_value(entry)?,
    );
    artifact.prepare_for_storage(&config);
    daemon_store::persist_reasoning_artifact(db, run_id, &artifact)?;
    let memory = MemoryCapsule {
        id: entry.id.clone(),
        run_id: run_id.to_string(),
        artifact_id: artifact.id,
        scope: "openqg".to_string(),
        status: entry.status.clone(),
        summary: entry.claim.clone(),
        evidence_level: EvidenceLevel::ExternalGrounding,
        confidence: if entry.status == "verified" { 0.8 } else { 0.6 },
        payload_json: serde_json::to_value(entry)?,
        content_hash: stable_reasoning_hash(entry),
    };
    daemon_store::persist_memory_capsule(db, run_id, &memory)?;
    Ok(())
}

pub(crate) fn seed_prompt_lineage(objective: &str, config: &HeroJudgeConfig) -> Vec<PromptVariant> {
    let hero_seed = format!("hero seed: {objective}");
    let judge_seed = format!("judge seed: {objective}");
    vec![
        PromptVariant {
            id: "hero-seed".to_string(),
            role: "hero".to_string(),
            generation: 0,
            parent_id: None,
            summary: "Seed hero prompt for OpenQG theory candidate generation.".to_string(),
            prompt_sha256: sha256_hex(hero_seed.as_bytes()),
            score: 0.0,
            status: "seed".to_string(),
        },
        PromptVariant {
            id: "judge-seed".to_string(),
            role: "judge".to_string(),
            generation: 0,
            parent_id: None,
            summary: format!(
                "Seed judge prompt with promotion threshold {:.3}.",
                config.promotion.min_score
            ),
            prompt_sha256: sha256_hex(judge_seed.as_bytes()),
            score: 0.0,
            status: "seed".to_string(),
        },
    ]
}

pub(crate) fn prompt_for(
    role: &str,
    objective: &str,
    generation: usize,
    evidence: &[crate::evidence::LoadedEvidence],
    receipts: &[crate::hero_judge::HeroJudgeSearchReceipt],
) -> String {
    let evidence_refs = evidence
        .iter()
        .map(|item| format!("{}:{}:{}", item.id, item.role, item.sha256))
        .collect::<Vec<_>>()
        .join(", ");
    let research_refs = receipts
        .iter()
        .map(|receipt| format!("{}:{}:{}", receipt.id, receipt.status, receipt.url_count))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Role: {role}. Objective: {objective}. Generation: {generation}. Evidence: [{evidence_refs}]. Research receipts: [{research_refs}]. Return only compact JSON with summary:string, claims:string[], questions:string[], rubric:string[], evidence_refs:string[], score:number. Do not return private reasoning. Hero lanes must submit artifact-first theory prospects: formal objects, assumptions, derivation gaps, falsifier questions, constants ledger, extraction map, and unsupported rows as compact claims/questions. Judge, verifier, red-team, and meta lanes must improve rubric calibration, leakage detection, hidden-parameter checks, prior-art discipline, extraction validity, and reviewer questions. Reward honest structural progress; do not over-score ideas that only repackage known theory or quote observed constants."
    )
}

pub(crate) fn summary_from_value(
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
            kind_label(kind)
        ),
    }
}

pub(crate) fn synthetic_lane_value(kind: ModelTaskKind, generation: usize) -> serde_json::Value {
    json!({
        "summary": format!("deterministic {} summary", kind_label(kind)),
        "claims": ["bounded evidence", "canary checked", "promotion-gated"],
        "questions": ["What falsifiable signal would move this theory up or down?"],
        "rubric": ["evidence grounding", "falsifiability", "calibration"],
        "evidence_refs": ["deterministic-local-evidence"],
        "score": lane_default_score(kind, generation),
    })
}

pub(crate) fn parse_substitute_lane_value(
    kind: ModelTaskKind,
    generation: usize,
) -> serde_json::Value {
    json!({
        "summary": format!("live {} response completed but required storage-safe JSON substitute", kind_label(kind)),
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

pub(crate) fn score_from_value(
    kind: ModelTaskKind,
    generation: usize,
    value: &serde_json::Value,
) -> f64 {
    match value.get("score").and_then(serde_json::Value::as_f64) {
        Some(score) => score.clamp(0.0, 1.0),
        None => lane_default_score(kind, generation),
    }
}

pub(crate) fn lane_default_score(kind: ModelTaskKind, generation: usize) -> f64 {
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

pub(crate) fn average_score(artifacts: &[HeroJudgeLaneArtifact], default_score: f64) -> f64 {
    if artifacts.is_empty() {
        return default_score;
    }
    artifacts.iter().map(|artifact| artifact.score).sum::<f64>() / artifacts.len() as f64
}

pub(crate) fn run_objective(runbook: &HeroJudgeRunbook, config: &HeroJudgeConfig) -> String {
    match config
        .objective
        .clone()
        .or_else(|| runbook.job.as_ref().map(|job| job.objective.clone()))
    {
        Some(objective) => objective,
        None => "Evolve OpenQG hero and judge prompts".to_string(),
    }
}

pub(crate) fn validate_config(config: &HeroJudgeConfig) -> Result<()> {
    if config.generations == 0 {
        anyhow::bail!("hero_judge.generations must be at least 1");
    }
    if config.population.hero_lanes == 0 {
        anyhow::bail!("hero_judge.population.hero_lanes must be at least 1");
    }
    if config.population.judge_lanes == 0 {
        anyhow::bail!("hero_judge.population.judge_lanes must be at least 1");
    }
    if config.budgets.model_calls == 0 {
        anyhow::bail!("hero_judge.budgets.model_calls must be at least 1");
    }
    if !config.promotion.min_score.is_finite() {
        anyhow::bail!("hero_judge.promotion.min_score must be finite");
    }
    Ok(())
}

pub(crate) fn zyal_yaml_body(text: &str) -> Result<String> {
    let mut lines = text.lines();
    let Some(first) = lines.next() else {
        anyhow::bail!("empty ZYAL document");
    };
    if !first.starts_with("<<<ZYAL ") {
        return Ok(text.to_string());
    }
    let mut body = Vec::new();
    for line in lines {
        if line.starts_with("<<<END_ZYAL ") {
            return Ok(body.join("\n"));
        }
        body.push(line);
    }
    anyhow::bail!("missing END_ZYAL sentinel")
}

pub(crate) fn write_json_pretty<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn write_jsonl<T: serde::Serialize>(path: &Path, values: &[T]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    for value in values {
        writeln!(file, "{}", serde_json::to_string(value)?)?;
    }
    Ok(())
}

pub(crate) fn rounded(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

pub(crate) fn lane_quality_metrics(
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

pub(crate) struct GenerationMetricInputs<'a> {
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

pub(crate) fn generation_quality_metric(
    input: GenerationMetricInputs<'_>,
) -> HeroJudgeQualityMetric {
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

pub(crate) fn quality_trend(
    run_id: &str,
    metrics: &[HeroJudgeQualityMetric],
) -> HeroJudgeQualityTrend {
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

pub fn write_quality_csv(path: &Path, metrics: &[HeroJudgeQualityMetric]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    writeln!(
        file,
        "run_id,generation,theory_quality_index,question_quality_index,rubric_quality_index,judge_calibration_index,evidence_grounding_index,verifier_confidence,red_team_resilience,promotion_score,overall_quality_index,delta_overall_quality,frontier_quality_index,delta_frontier_quality,promoted,hero_candidate_count,judge_patch_count,research_receipt_count,knowledge_entry_count"
    )?;
    for metric in metrics {
        writeln!(
            file,
            "{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{}",
            metric.run_id,
            metric.generation,
            metric.theory_quality_index,
            metric.question_quality_index,
            metric.rubric_quality_index,
            metric.judge_calibration_index,
            metric.evidence_grounding_index,
            metric.verifier_confidence,
            metric.red_team_resilience,
            metric.promotion_score,
            metric.overall_quality_index,
            metric.delta_overall_quality,
            metric.frontier_quality_index,
            metric.delta_frontier_quality,
            metric.promoted,
            metric.hero_candidate_count,
            metric.judge_patch_count,
            metric.research_receipt_count,
            metric.knowledge_entry_count,
        )?;
    }
    Ok(())
}

pub(crate) fn role_group(kind: &str) -> &'static str {
    match kind {
        "hero_generate" => "hero",
        "judge_patch" | "verifier" | "red_team" | "meta_judge" => "judge",
        "literature_synthesis" => "research",
        "knowledge_curate" => "knowledge",
        _ => "other",
    }
}

pub(crate) fn lane_metric_records(
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

pub(crate) fn review_cards(groups: &[&[HeroJudgeLaneArtifact]]) -> Vec<HeroJudgeReviewCard> {
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

pub(crate) fn reviewer_questions() -> Vec<String> {
    vec![
        "Are hero artifacts becoming more derivable, falsifiable, and evidence-grounded across generations?".to_string(),
        "Are judge artifacts becoming better calibrated, less gameable, and more explicit about leakage, hidden parameters, and extraction maps?".to_string(),
        "Does the retained frontier improve without storing private reasoning or importing fixture constants?".to_string(),
    ]
}

pub fn write_lane_metrics_csv(path: &Path, metrics: &[HeroJudgeLaneMetric]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    writeln!(
        file,
        "run_id,generation,role_group,kind,artifact_id,lane,score,claim_quality,question_quality,rubric_quality,evidence_grounding,structural_completeness,storage_safety,claim_count,question_count,rubric_item_count,status,model_receipt_id,content_sha256"
    )?;
    for metric in metrics {
        writeln!(
            file,
            "{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.0},{:.0},{:.0},{},{},{}",
            metric.run_id,
            metric.generation,
            metric.role_group,
            metric.kind,
            metric.artifact_id,
            metric.lane,
            metric.score,
            metric.claim_quality,
            metric.question_quality,
            metric.rubric_quality,
            metric.evidence_grounding,
            metric.structural_completeness,
            metric.storage_safety,
            metric.claim_count,
            metric.question_count,
            metric.rubric_item_count,
            metric.status,
            metric.model_receipt_id,
            metric.content_sha256,
        )?;
    }
    Ok(())
}

pub fn write_series_summary_csv(path: &Path, rows: &[HeroJudgeSeriesRow]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    writeln!(
        file,
        "series_id,trial_index,run_id,generation,theory_quality_index,question_quality_index,rubric_quality_index,judge_calibration_index,evidence_grounding_index,verifier_confidence,red_team_resilience,promotion_score,overall_quality_index,delta_overall_quality,frontier_quality_index,delta_frontier_quality,promoted,frontier_winner,model_calls_used,model_call_budget,search_receipt_count,hero_lane_mean,judge_lane_mean,quality_metrics_sha256,lane_metrics_sha256,reviewer_packet_sha256,promotion_decision_sha256,search_receipts_sha256"
    )?;
    for row in rows {
        writeln!(
            file,
            "{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{},{:.3},{:.3},{},{},{},{},{}",
            csv_cell(&row.series_id),
            row.trial_index,
            csv_cell(&row.run_id),
            row.generation,
            row.theory_quality_index,
            row.question_quality_index,
            row.rubric_quality_index,
            row.judge_calibration_index,
            row.evidence_grounding_index,
            row.verifier_confidence,
            row.red_team_resilience,
            row.promotion_score,
            row.overall_quality_index,
            row.delta_overall_quality,
            row.frontier_quality_index,
            row.delta_frontier_quality,
            row.promoted,
            csv_cell(row.frontier_winner.as_deref().unwrap_or("")),
            row.model_calls_used,
            row.model_call_budget,
            row.search_receipt_count,
            row.hero_lane_mean,
            row.judge_lane_mean,
            csv_cell(&row.quality_metrics_sha256),
            csv_cell(&row.lane_metrics_sha256),
            csv_cell(&row.reviewer_packet_sha256),
            csv_cell(&row.promotion_decision_sha256),
            csv_cell(&row.search_receipts_sha256),
        )?;
    }
    Ok(())
}

fn csv_cell(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anti_leak_rejects_hidden_canaries() {
        let config = HeroJudgeConfig::default();
        let decision = reduce_generation(
            "run",
            1,
            &[HeroJudgeLaneArtifact {
                id: "hero-leak".into(),
                generation: 1,
                kind: "hero_generate".into(),
                lane: 1,
                model_receipt_id: "receipt".into(),
                summary: "uses HIDDEN_CANARY hidden constant".into(),
                content_sha256: "hash".into(),
                score: 0.99,
                metrics: std::collections::BTreeMap::new(),
                status: "complete".into(),
            }],
            0.99,
            &[],
            &config,
        );
        assert!(!decision.promoted);
        assert!(decision.reason.contains("anti-leak"));
    }

    #[test]
    fn red_team_quality_does_not_force_zero_resilience() {
        let hero = HeroJudgeLaneArtifact {
            id: "hero".into(),
            generation: 1,
            kind: "hero_generate".into(),
            lane: 1,
            model_receipt_id: "receipt".into(),
            summary: "clean candidate".into(),
            content_sha256: "hash".into(),
            score: 0.90,
            metrics: BTreeMap::new(),
            status: "complete".into(),
        };
        let red_team = HeroJudgeLaneArtifact {
            id: "red".into(),
            generation: 1,
            kind: "red_team".into(),
            lane: 1,
            model_receipt_id: "receipt".into(),
            summary: "strong adversarial critique".into(),
            content_sha256: "hash".into(),
            score: 0.95,
            metrics: BTreeMap::new(),
            status: "complete".into(),
        };
        let decision = PromotionDecision {
            run_id: "run".into(),
            generation: 1,
            winner_candidate_id: Some("hero".into()),
            winner_prompt_id: Some("prompt-hero".into()),
            score: 0.87,
            promoted: true,
            reason: "promoted".into(),
        };
        let metric = generation_quality_metric(GenerationMetricInputs {
            run_id: "run",
            generation: 1,
            literature: &[],
            heroes: &[hero],
            judges: &[],
            verifiers: &[],
            red_team: &[red_team],
            meta: &[],
            decision: &decision,
            search_receipts: &[],
            previous_overall: None,
            previous_frontier: None,
            knowledge_entry_count: 1,
        });
        assert!(metric.red_team_resilience > 0.0);
    }
}
