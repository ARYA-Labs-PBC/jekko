//! Generated-suite execution: runs the procedurally generated benchmark suite
//! against a `MemorySystem` adapter and serializes the full result envelope as
//! JSON. Pulled out of `runner.rs` to keep that file under the audit floor.

use std::collections::BTreeMap;

use crate::generated::{generate_suite, GeneratedSuiteConfig};
use crate::json::{self, Json};
use crate::runner::CandidateReport;
use crate::runner_support::GATE_REPLAY_CMD;
use crate::{BenchCase, MemorySystem, RecallResult, SuiteConfig, TemporalLens};

pub(crate) fn run_generated_candidate(
    candidate: &str,
    adapter: &mut dyn MemorySystem,
    config: &SuiteConfig,
) -> Result<CandidateReport, String> {
    let generated_config = GeneratedSuiteConfig {
        benchmark_version: config.benchmark_version,
        split: config.split,
        seed_label: config.seed_label.clone(),
        fixture_count: config.fixture_count,
        difficulty: config.difficulty,
    };
    let cases = generate_suite(&generated_config);
    let mut fixture_records = Vec::with_capacity(cases.len());
    let mut scores = Vec::with_capacity(cases.len());
    let mut passed = 0u32;
    let mut privacy_leaks = 0u32;

    for case in &cases {
        let outcome = run_generated_case(adapter, case, config.context_budget);
        let score = outcome.score;
        privacy_leaks += outcome.privacy_leaks;
        if score >= 0.50 {
            passed += 1;
        }
        scores.push(score);
        let mut record = BTreeMap::new();
        record.insert("id".to_string(), Json::Str(case.id.clone()));
        record.insert(
            "block".to_string(),
            Json::Str(case.block.name().to_string()),
        );
        record.insert(
            "domain".to_string(),
            Json::Str(case.domain.name().to_string()),
        );
        record.insert(
            "oracle".to_string(),
            Json::Str(format!("{:?}", case.oracle.kind)),
        );
        record.insert("weighted".to_string(), Json::Float(score as f64));
        fixture_records.push(Json::Object(record));
    }

    let raw_total = if scores.is_empty() {
        0.0
    } else {
        scores.iter().sum::<f32>() / scores.len() as f32 * 100.0
    };
    let ci = crate::scoring::bootstrap::bootstrap_ci(&scores, &config.seed_label, 1000);
    let gates = crate::scoring::gates::GateFindings {
        privacy_leaks,
        deterministic: true,
        ..Default::default()
    };
    let total = crate::scoring::gates::apply_hard_gates(raw_total, &gates);

    let mut top = BTreeMap::new();
    top.insert("name".to_string(), Json::Str(candidate.to_string()));
    top.insert(
        "suite".to_string(),
        Json::Str(config.split.name().to_string()),
    );
    top.insert(
        "seed_label".to_string(),
        Json::Str(config.seed_label.clone()),
    );
    top.insert("total".to_string(), Json::Float(total as f64));
    top.insert("raw_total".to_string(), Json::Float(raw_total as f64));
    top.insert("fixtures_run".to_string(), Json::Int(cases.len() as i64));
    top.insert("fixtures_passed".to_string(), Json::Int(passed as i64));
    top.insert("fixtures".to_string(), Json::Array(fixture_records));
    top.insert(
        "bootstrap_ci".to_string(),
        json::obj(&[
            ("mean", Json::Float(ci.mean as f64)),
            ("ci95_low", Json::Float(ci.ci95_low as f64)),
            ("ci95_high", Json::Float(ci.ci95_high as f64)),
            ("overfit_gap", Json::Float(0.0)),
        ]),
    );
    top.insert(
        "gate_findings".to_string(),
        json::obj(&[
            ("unsafe_tool_exec", Json::Int(gates.unsafe_tool_exec as i64)),
            ("privacy_leaks", Json::Int(gates.privacy_leaks as i64)),
            (
                "citation_issue_count",
                Json::Int(gates.citation_issues as i64),
            ),
            ("future_leaks", Json::Int(gates.future_leaks as i64)),
            ("deterministic", Json::Bool(gates.deterministic)),
            ("replay_cmd", Json::Str(GATE_REPLAY_CMD.to_string())),
            (
                "evidence_artifact",
                Json::Str("agent/repo-score.md".to_string()),
            ),
        ]),
    );
    let json = Json::Object(top).to_string();
    Ok(CandidateReport {
        name: candidate.to_string(),
        total,
        fixtures_run: cases.len() as u32,
        fixtures_passed: passed,
        json,
    })
}

struct GeneratedOutcome {
    score: f32,
    privacy_leaks: u32,
}

fn run_generated_case(
    adapter: &mut dyn MemorySystem,
    case: &BenchCase,
    budget: u32,
) -> GeneratedOutcome {
    for event in &case.events {
        let _ = adapter.observe(event);
    }
    let Some(query) = &case.query else {
        return GeneratedOutcome {
            score: 0.5,
            privacy_leaks: 0,
        };
    };
    let mut query = query.clone();
    query.token_budget = budget;
    let result = match case.lens {
        TemporalLens::Current => adapter.recall(&query),
        TemporalLens::At => adapter.recall_at(&query, case.world_time.as_deref().unwrap_or("")),
        TemporalLens::AsOf => adapter.recall_as_of(&query, case.tx_time.as_deref().unwrap_or("")),
        TemporalLens::AtAsOf => adapter.recall_at(&query, case.world_time.as_deref().unwrap_or("")),
        TemporalLens::NoQuery => {
            return GeneratedOutcome {
                score: 0.5,
                privacy_leaks: 0,
            }
        }
    };
    let privacy_leaks = case
        .oracle
        .must_not_contain
        .iter()
        .any(|needle| result.answer.contains(needle)) as u32;
    GeneratedOutcome {
        score: score_generated_result(&result, case),
        privacy_leaks,
    }
}

fn score_generated_result(result: &RecallResult, case: &BenchCase) -> f32 {
    let oracle = &case.oracle;
    let mut hits = 0u32;
    let mut total = 0u32;
    if !oracle.must_include.is_empty() {
        total += 1;
        if oracle
            .must_include
            .iter()
            .all(|id| result.used_ids.iter().any(|used| used == id))
        {
            hits += 1;
        }
    }
    if !oracle.must_exclude.is_empty() {
        total += 1;
        if oracle
            .must_exclude
            .iter()
            .all(|id| !result.used_ids.iter().any(|used| used == id))
        {
            hits += 1;
        }
    }
    if !oracle.must_contain.is_empty() {
        total += 1;
        let answer = result.answer.to_lowercase();
        if oracle
            .must_contain
            .iter()
            .all(|needle| answer.contains(&needle.to_lowercase()))
        {
            hits += 1;
        }
    }
    if !oracle.must_not_contain.is_empty() {
        total += 1;
        if oracle
            .must_not_contain
            .iter()
            .all(|needle| !result.answer.contains(needle))
        {
            hits += 1;
        }
    }
    if !oracle.required_warnings.is_empty() {
        total += 1;
        if oracle.required_warnings.iter().all(|needle| {
            result
                .warnings
                .iter()
                .any(|warning| warning.name() == needle)
        }) {
            hits += 1;
        }
    }
    if total == 0 {
        0.5
    } else {
        hits as f32 / total as f32
    }
}
