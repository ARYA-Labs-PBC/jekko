//! Runtime for ZYAL Hero/Judge prompt evolution.

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use jekko_store::db::Db;
use serde_json::json;

use crate::daemon_store;
use crate::events::{EventKind, EventSink};
use crate::hashing::sha256_json;
use crate::hero_judge::{
    HeroJudgeLaneArtifact, HeroJudgeLaneMetric, HeroJudgeQualityMetric, HeroJudgeReviewerPacket,
    HeroJudgeRunSummary, HeroJudgeRunbook, PromotionDecision, PromptVariant,
};
use crate::hero_judge_eval::{
    average_score, generation_quality_metric, knowledge_entry, lane_metric_records,
    lane_quality_metrics, parse_substitute_lane_value, persist_knowledge_capsule, prompt_for,
    quality_trend, reduce_generation, review_cards, reviewer_questions, rounded, run_objective,
    score_from_value, scoreboard_for_generation, seed_prompt_lineage, summary_from_value,
    synthetic_lane_value, validate_config, write_json_pretty, write_jsonl, write_lane_metrics_csv,
    write_quality_csv, zyal_yaml_body, GenerationMetricInputs,
};
use crate::hero_judge_search::{load_hero_judge_evidence, run_research};
use crate::model_client::{kind_label, ModelCallReceipt, ModelClient};
use crate::model_policy::ModelTaskKind;

/// Parse a ZYAL Hero/Judge runbook.
pub fn read_hero_judge_runbook(path: &Path) -> Result<HeroJudgeRunbook> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let yaml = zyal_yaml_body(&text).with_context(|| format!("parse {}", path.display()))?;
    let runbook: HeroJudgeRunbook =
        serde_yaml::from_str(&yaml).with_context(|| format!("decode {}", path.display()))?;
    validate_config(&runbook.hero_judge)?;
    Ok(runbook)
}

/// Run one Hero/Judge evolution workflow with the default DB.
pub async fn run_hero_judge_run(
    repo: &Path,
    run_id: &str,
    zyal_path: &Path,
    runbook: HeroJudgeRunbook,
    max_generations: Option<usize>,
    live_search: bool,
    model_client: &dyn ModelClient,
) -> Result<HeroJudgeRunSummary> {
    let db = daemon_store::open_db(repo)?;
    run_hero_judge_run_with_db(
        repo,
        run_id,
        zyal_path,
        runbook,
        max_generations,
        live_search,
        model_client,
        &db,
    )
    .await
}

/// Run one Hero/Judge evolution workflow with a caller-supplied DB.
pub async fn run_hero_judge_run_with_db(
    repo: &Path,
    run_id: &str,
    zyal_path: &Path,
    runbook: HeroJudgeRunbook,
    max_generations: Option<usize>,
    live_search: bool,
    model_client: &dyn ModelClient,
    db: &Db,
) -> Result<HeroJudgeRunSummary> {
    let config = runbook.hero_judge.clone();
    let generations = config.effective_generations(max_generations);
    let output_dir = repo.join(config.output_root()).join(run_id);
    fs::create_dir_all(&output_dir).with_context(|| format!("mkdir {}", output_dir.display()))?;
    let sink = EventSink::open(repo, run_id)?;
    daemon_store::ensure_daemon_run(
        db,
        repo,
        run_id,
        json!({
            "kind": "zyal_hero_judge",
            "zyal_path": zyal_path.display().to_string(),
            "hero_judge": config,
            "live_call_budget": {
                "max_calls": runbook.hero_judge.budgets.model_calls,
                "max_parallel": runbook.hero_judge.population.max_parallel,
                "require_live": false,
            },
        }),
    )?;
    sink.emit(
        EventKind::RunStarted,
        json!({"workflow": "zyal_hero_judge", "generations": generations}),
    )?;

    let evidence = load_hero_judge_evidence(repo, &config)?;
    let objective = run_objective(&runbook, &config);
    let search_receipts = run_research(repo, &objective, &config, live_search).await?;
    write_json_pretty(
        &output_dir.join("search").join("receipts.json"),
        &search_receipts,
    )?;
    for receipt in &search_receipts {
        sink.emit(
            EventKind::ResearchReceipt,
            json!({"id": receipt.id, "provider": receipt.provider, "status": receipt.status}),
        )?;
    }

    let mut prompt_lineage = seed_prompt_lineage(&objective, &config);
    let mut scoreboard = Vec::new();
    let mut knowledge = Vec::new();
    let mut quality_metrics: Vec<HeroJudgeQualityMetric> = Vec::new();
    let mut lane_metrics: Vec<HeroJudgeLaneMetric> = Vec::new();
    let mut reviewer_cards = Vec::new();
    let mut model_calls_used = 0_usize;
    let mut last_model_kind = None;
    let mut last_decision = PromotionDecision {
        run_id: run_id.to_string(),
        generation: 0,
        winner_candidate_id: None,
        winner_prompt_id: None,
        score: 0.0,
        promoted: false,
        reason: "no generation completed".to_string(),
    };

    let mut frontier_parent = Some("hero-seed".to_string());
    for generation in 1..=generations {
        daemon_store::mark_daemon_run(
            db,
            run_id,
            "running",
            &format!("hero_judge_generation_{generation}"),
            None,
        )?;
        sink.emit(
            EventKind::HeroJudgeGeneration,
            json!({"generation": generation}),
        )?;
        let gen_dir = output_dir.join(format!("generation-{generation:03}"));
        fs::create_dir_all(&gen_dir).with_context(|| format!("mkdir {}", gen_dir.display()))?;
        let evolution_context =
            evolution_context(generation, &last_decision, quality_metrics.last());

        let literature_prompt = with_evolution_context(
            prompt_for(
                "literature synthesis",
                &objective,
                generation,
                &evidence,
                &search_receipts,
            ),
            &evolution_context,
        );
        let literature = run_lane_group(
            repo,
            run_id,
            db,
            &sink,
            model_client,
            ModelTaskKind::LiteratureSynthesis,
            generation,
            config.population.literature_lanes,
            &literature_prompt,
        )
        .await?;
        model_calls_used += literature.len();
        write_json_pretty(&gen_dir.join("literature.json"), &literature)?;

        let hero_prompt = with_evolution_context(
            prompt_for(
                "hero candidate",
                &objective,
                generation,
                &evidence,
                &search_receipts,
            ),
            &evolution_context,
        );
        let heroes = run_lane_group(
            repo,
            run_id,
            db,
            &sink,
            model_client,
            ModelTaskKind::HeroGenerate,
            generation,
            config.population.hero_lanes,
            &hero_prompt,
        )
        .await?;
        model_calls_used += heroes.len();
        for hero in &heroes {
            sink.emit(
                EventKind::HeroCandidate,
                json!({"id": hero.id, "generation": generation, "score": rounded(hero.score)}),
            )?;
            prompt_lineage.push(PromptVariant {
                id: format!("prompt-{}", hero.id),
                role: "hero".to_string(),
                generation,
                parent_id: frontier_parent.clone(),
                summary: hero.summary.clone(),
                prompt_sha256: hero.content_sha256.clone(),
                score: hero.score,
                status: "candidate".to_string(),
            });
        }
        write_json_pretty(&gen_dir.join("hero-candidates.json"), &heroes)?;

        let judge_prompt = with_evolution_context(
            prompt_for(
                "judge patch",
                &objective,
                generation,
                &evidence,
                &search_receipts,
            ),
            &evolution_context,
        );
        let judges = run_lane_group(
            repo,
            run_id,
            db,
            &sink,
            model_client,
            ModelTaskKind::JudgePatch,
            generation,
            config.population.judge_lanes,
            &judge_prompt,
        )
        .await?;
        model_calls_used += judges.len();
        for judge in &judges {
            sink.emit(
                EventKind::JudgePatch,
                json!({"id": judge.id, "generation": generation}),
            )?;
            prompt_lineage.push(PromptVariant {
                id: format!("prompt-{}", judge.id),
                role: "judge".to_string(),
                generation,
                parent_id: Some("judge-seed".to_string()),
                summary: judge.summary.clone(),
                prompt_sha256: judge.content_sha256.clone(),
                score: judge.score,
                status: "candidate".to_string(),
            });
        }
        write_json_pretty(&gen_dir.join("judge-patches.json"), &judges)?;

        let verifier_prompt = with_evolution_context(
            prompt_for(
                "verifier",
                &objective,
                generation,
                &evidence,
                &search_receipts,
            ),
            &evolution_context,
        );
        let verifiers = run_lane_group(
            repo,
            run_id,
            db,
            &sink,
            model_client,
            ModelTaskKind::Verifier,
            generation,
            config.population.verifier_lanes,
            &verifier_prompt,
        )
        .await?;
        model_calls_used += verifiers.len();
        let verifier_score = average_score(&verifiers, 0.84);
        sink.emit(
            EventKind::VerifierScore,
            json!({"generation": generation, "score": rounded(verifier_score)}),
        )?;
        write_json_pretty(&gen_dir.join("verifier-scores.json"), &verifiers)?;

        let red_team_prompt = with_evolution_context(
            prompt_for(
                "red team",
                &objective,
                generation,
                &evidence,
                &search_receipts,
            ),
            &evolution_context,
        );
        let red_team = run_lane_group(
            repo,
            run_id,
            db,
            &sink,
            model_client,
            ModelTaskKind::RedTeam,
            generation,
            config.population.red_team_lanes,
            &red_team_prompt,
        )
        .await?;
        model_calls_used += red_team.len();
        write_json_pretty(&gen_dir.join("red-team.json"), &red_team)?;

        let meta_prompt = with_evolution_context(
            prompt_for(
                "meta judge reducer",
                &objective,
                generation,
                &evidence,
                &search_receipts,
            ),
            &evolution_context,
        );
        let meta = run_lane_group(
            repo,
            run_id,
            db,
            &sink,
            model_client,
            ModelTaskKind::MetaJudge,
            generation,
            1,
            &meta_prompt,
        )
        .await?;
        model_calls_used += meta.len();
        write_json_pretty(&gen_dir.join("meta-judge.json"), &meta)?;

        let decision = reduce_generation(
            run_id,
            generation,
            &heroes,
            verifier_score,
            &red_team,
            &config,
        );
        sink.emit(
            EventKind::PromotionDecision,
            json!({"generation": generation, "promoted": decision.promoted, "score": rounded(decision.score)}),
        )?;
        if let Some(winner) = decision.winner_candidate_id.as_deref() {
            frontier_parent = Some(format!("prompt-{winner}"));
        }
        scoreboard.extend(scoreboard_for_generation(
            generation,
            &heroes,
            verifier_score,
            &red_team,
            &decision,
        ));
        write_json_pretty(&gen_dir.join("promotion-decision.json"), &decision)?;
        last_decision = decision;

        let knowledge_prompt = with_evolution_context(
            prompt_for(
                "knowledge curator",
                &objective,
                generation,
                &evidence,
                &search_receipts,
            ),
            &evolution_context,
        );
        let curated = run_lane_group(
            repo,
            run_id,
            db,
            &sink,
            model_client,
            ModelTaskKind::KnowledgeCurate,
            generation,
            1,
            &knowledge_prompt,
        )
        .await?;
        model_calls_used += curated.len();
        last_model_kind = Some(kind_label(ModelTaskKind::KnowledgeCurate).to_string());
        let entry = knowledge_entry(generation, &last_decision, &evidence);
        persist_knowledge_capsule(db, run_id, &entry)?;
        sink.emit(
            EventKind::KnowledgeCompounded,
            json!({"id": entry.id, "status": entry.status}),
        )?;
        knowledge.push(entry);

        let previous_overall = quality_metrics
            .last()
            .map(|metric| metric.overall_quality_index);
        let previous_frontier = quality_metrics
            .last()
            .map(|metric| metric.frontier_quality_index);
        let quality_metric = generation_quality_metric(GenerationMetricInputs {
            run_id,
            generation,
            literature: &literature,
            heroes: &heroes,
            judges: &judges,
            verifiers: &verifiers,
            red_team: &red_team,
            meta: &meta,
            decision: &last_decision,
            search_receipts: &search_receipts,
            previous_overall,
            previous_frontier,
            knowledge_entry_count: knowledge.len(),
        });
        sink.emit(
            EventKind::HeroJudgeGeneration,
            json!({
                "generation": generation,
                "overall_quality_index": quality_metric.overall_quality_index,
                "theory_quality_index": quality_metric.theory_quality_index,
                "question_quality_index": quality_metric.question_quality_index,
                "rubric_quality_index": quality_metric.rubric_quality_index,
                "delta_overall_quality": quality_metric.delta_overall_quality,
                "frontier_quality_index": quality_metric.frontier_quality_index,
                "delta_frontier_quality": quality_metric.delta_frontier_quality,
            }),
        )?;
        write_json_pretty(&gen_dir.join("quality-metrics.json"), &quality_metric)?;
        quality_metrics.push(quality_metric);
        let generation_lane_metrics = lane_metric_records(
            run_id,
            &[
                &literature,
                &heroes,
                &judges,
                &verifiers,
                &red_team,
                &meta,
                &curated,
            ],
        );
        write_jsonl(
            &gen_dir.join("lane-metrics.jsonl"),
            &generation_lane_metrics,
        )?;
        lane_metrics.extend(generation_lane_metrics);
        reviewer_cards.extend(review_cards(&[
            &literature,
            &heroes,
            &judges,
            &verifiers,
            &red_team,
            &meta,
            &curated,
        ]));
    }

    let prompt_lineage_json = output_dir.join("prompt_lineage.json");
    let frontier_scoreboard_json = output_dir.join("frontier_scoreboard.json");
    let promotion_decision_json = output_dir.join("promotion-decision.json");
    let knowledge_compound_jsonl = output_dir.join("knowledge_compound.jsonl");
    let search_receipts_json = output_dir.join("search").join("receipts.json");
    let quality_metrics_jsonl = output_dir.join("quality_metrics.jsonl");
    let quality_metrics_csv = output_dir.join("quality_metrics.csv");
    let quality_trend_json = output_dir.join("quality_trend.json");
    let lane_metrics_jsonl = output_dir.join("lane_metrics.jsonl");
    let lane_metrics_csv = output_dir.join("lane_metrics.csv");
    let hero_metrics_csv = output_dir.join("hero_metrics.csv");
    let judge_metrics_csv = output_dir.join("judge_metrics.csv");
    let reviewer_packet_json = output_dir.join("reviewer_packet.json");
    let complete_ok = output_dir.join("complete.ok");
    write_json_pretty(&prompt_lineage_json, &prompt_lineage)?;
    write_json_pretty(&frontier_scoreboard_json, &scoreboard)?;
    write_json_pretty(&promotion_decision_json, &last_decision)?;
    write_jsonl(&knowledge_compound_jsonl, &knowledge)?;
    write_jsonl(&quality_metrics_jsonl, &quality_metrics)?;
    write_quality_csv(&quality_metrics_csv, &quality_metrics)?;
    write_jsonl(&lane_metrics_jsonl, &lane_metrics)?;
    write_lane_metrics_csv(&lane_metrics_csv, &lane_metrics)?;
    write_lane_metrics_csv(
        &hero_metrics_csv,
        &filter_lane_metrics(&lane_metrics, "hero"),
    )?;
    write_lane_metrics_csv(
        &judge_metrics_csv,
        &filter_lane_metrics(&lane_metrics, "judge"),
    )?;
    write_json_pretty(
        &quality_trend_json,
        &quality_trend(run_id, &quality_metrics),
    )?;
    write_json_pretty(
        &reviewer_packet_json,
        &HeroJudgeReviewerPacket {
            run_id: run_id.to_string(),
            objective: objective.clone(),
            reviewer_questions: reviewer_questions(),
            quality_metrics: quality_metrics.clone(),
            promotion_decision: last_decision.clone(),
            cards: reviewer_cards,
        },
    )?;
    fs::write(&complete_ok, b"ok\n").with_context(|| format!("write {}", complete_ok.display()))?;

    let summary = HeroJudgeRunSummary {
        run_id: run_id.to_string(),
        output_dir,
        generation: generations,
        hero_lane_count: config.population.hero_lanes,
        judge_lane_count: config.population.judge_lanes,
        frontier_winner: last_decision.winner_candidate_id.clone(),
        knowledge_entry_count: knowledge.len(),
        search_receipt_count: search_receipts.len(),
        last_promotion_decision: last_decision,
        model_calls_used,
        model_call_budget: config.budgets.model_calls,
        last_model_kind,
        prompt_lineage_json,
        frontier_scoreboard_json,
        promotion_decision_json,
        knowledge_compound_jsonl,
        search_receipts_json,
        quality_metrics_jsonl,
        quality_metrics_csv,
        quality_trend_json,
        lane_metrics_jsonl,
        lane_metrics_csv,
        hero_metrics_csv,
        judge_metrics_csv,
        reviewer_packet_json,
        complete_ok,
    };
    daemon_store::record_daemon_exit_result(db, run_id, serde_json::to_value(&summary)?)?;
    daemon_store::mark_daemon_run(db, run_id, "complete", "complete", None)?;
    sink.emit(
        EventKind::RunFinished,
        json!({"workflow": "zyal_hero_judge", "status": "complete"}),
    )?;
    Ok(summary)
}

async fn run_lane_group(
    repo: &Path,
    run_id: &str,
    db: &Db,
    sink: &EventSink,
    model_client: &dyn ModelClient,
    kind: ModelTaskKind,
    generation: usize,
    count: usize,
    base_prompt: &str,
) -> Result<Vec<HeroJudgeLaneArtifact>> {
    let mut artifacts = Vec::new();
    for lane in 1..=count.max(1) {
        let prompt = format!(
            "{base_prompt}\nLane: {lane}\nReturn exactly one compact JSON object under 700 tokens with summary, claims, questions, rubric, evidence_refs, and score. No markdown, no commentary, and no raw reasoning."
        );
        let (receipt, value) = complete_hero_json(
            repo,
            run_id,
            db,
            sink,
            model_client,
            kind,
            generation,
            &prompt,
        )
        .await?;
        let summary = summary_from_value(kind, generation, lane, &value);
        let score = score_from_value(kind, generation, &value);
        let metrics = lane_quality_metrics(kind, &value, &summary, score);
        artifacts.push(HeroJudgeLaneArtifact {
            id: format!("{}-g{generation:03}-l{lane:02}", kind_label(kind)),
            generation,
            kind: kind_label(kind).to_string(),
            lane,
            model_receipt_id: receipt.id,
            content_sha256: sha256_json(&value, "hero_judge_artifact"),
            summary,
            score,
            metrics,
            status: "complete".to_string(),
        });
    }
    Ok(artifacts)
}

async fn complete_hero_json(
    repo: &Path,
    run_id: &str,
    db: &Db,
    sink: &EventSink,
    model_client: &dyn ModelClient,
    kind: ModelTaskKind,
    generation: usize,
    prompt: &str,
) -> Result<(ModelCallReceipt, serde_json::Value)> {
    let mut last_error = None;
    for attempt in 1..=3 {
        sink.emit(
            EventKind::ModelAttempt,
            json!({"kind": kind_label(kind), "attempt": attempt}),
        )?;
        let receipt = model_client.complete(kind, prompt, repo).await?;
        daemon_store::persist_model_receipt(db, run_id, &receipt)?;
        sink.emit(
            EventKind::ModelOutcome,
            json!({"kind": receipt.kind, "model": receipt.model, "success": receipt.success}),
        )?;
        if receipt.budget_used.is_some() || receipt.budget_remaining.is_some() {
            sink.emit(
                EventKind::LiveBudget,
                json!({
                    "used": receipt.budget_used.unwrap_or(0),
                    "remaining": receipt.budget_remaining.unwrap_or(0),
                }),
            )?;
        }
        if !receipt.success {
            let error = match receipt.error.clone() {
                Some(error) => error,
                None => "unknown model failure".to_string(),
            };
            if timeout_model_error(&error) {
                return Ok((receipt, parse_substitute_lane_value(kind, generation)));
            }
            if attempt < 3 && retryable_model_error(&error) {
                last_error = Some(error);
                continue;
            }
            daemon_store::mark_daemon_run(db, run_id, "blocked", &receipt.kind, Some(&error))?;
            return Err(anyhow!("model call failed: {error}"));
        }
        let text = match receipt.response.as_deref() {
            Some(text) => text,
            None => "",
        };
        match parse_model_json(text) {
            Ok(value) => return Ok((receipt, value)),
            Err(_err) if receipt.provider == "fake" => {
                return Ok((receipt, synthetic_lane_value(kind, generation)));
            }
            Err(_err) if text.trim().is_empty() => {
                last_error = Some("empty model response".to_string());
            }
            Err(_err) => return Ok((receipt, parse_substitute_lane_value(kind, generation))),
        }
    }
    let error = match last_error {
        Some(error) => error,
        None => "invalid model JSON".to_string(),
    };
    daemon_store::mark_daemon_run(db, run_id, "blocked", "hero_judge_model_json", Some(&error))?;
    Err(anyhow!("hero/judge model JSON parse failed: {error}"))
}

fn retryable_model_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    !lower.contains("live call budget exhausted")
        && !lower.contains("no provider configured")
        && !lower.contains("missing credential")
        && !lower.contains("deterministic model receipt rejected")
}

fn timeout_model_error(error: &str) -> bool {
    error.to_ascii_lowercase().contains("timed out")
}

fn parse_model_json(text: &str) -> serde_json::Result<serde_json::Value> {
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(value) => Ok(value),
        Err(primary) => {
            let Some(start) = text.find('{') else {
                return Err(primary);
            };
            let Some(end) = text.rfind('}') else {
                return Err(primary);
            };
            serde_json::from_str::<serde_json::Value>(&text[start..=end])
        }
    }
}

fn with_evolution_context(mut prompt: String, context: &str) -> String {
    prompt.push_str("\nEvolution context: ");
    prompt.push_str(context);
    prompt.push_str(" Improve over the retained frontier without inventing evidence.");
    prompt
}

fn evolution_context(
    generation: usize,
    decision: &PromotionDecision,
    previous_metric: Option<&HeroJudgeQualityMetric>,
) -> String {
    let Some(metric) = previous_metric else {
        return "Initial generation; establish baseline theory, question, and rubric quality."
            .to_string();
    };
    format!(
        "Previous generation {} frontier prompt {:?}; prior overall {:.3}, frontier {:.3}, theory {:.3}, questions {:.3}, rubric {:.3}. Target measurable gains in the weakest metric while preserving anti-leak and evidence gates.",
        generation.saturating_sub(1),
        decision.winner_prompt_id,
        metric.overall_quality_index,
        metric.frontier_quality_index,
        metric.theory_quality_index,
        metric.question_quality_index,
        metric.rubric_quality_index,
    )
}

fn filter_lane_metrics(
    metrics: &[HeroJudgeLaneMetric],
    role_group: &str,
) -> Vec<HeroJudgeLaneMetric> {
    metrics
        .iter()
        .filter(|metric| metric.role_group == role_group)
        .cloned()
        .collect()
}
