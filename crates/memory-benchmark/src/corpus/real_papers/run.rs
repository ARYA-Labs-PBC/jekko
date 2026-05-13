#[path = "score.rs"]
mod score;

pub(crate) use score::{challenge_order, observe_paper};

use super::model::{LoadedChallenge, PaperChallenge};
use super::parse::{load_all_challenges, load_paper, load_selection};
use crate::json::Json;
use crate::memory_api::axes_to_json;
use crate::runner::CandidateReport;
use crate::runner_support::{accumulate, average, weighted_fraction};
use crate::{AxisScores, MemorySystem, Query, QueryIntent, SuiteConfig};
use std::collections::BTreeMap;
use std::env;
use std::path::Path;

pub fn default_bank_path() -> &'static Path {
    Path::new("crates/memory-benchmark/data/real-paper-bank")
}

pub fn load_challenges(root: &Path) -> Result<Vec<PaperChallenge>, String> {
    Ok(load_bank(root, &SuiteConfig::default())?
        .into_iter()
        .map(|loaded| loaded.challenge)
        .collect())
}

pub fn load_bank(root: &Path, config: &SuiteConfig) -> Result<Vec<LoadedChallenge>, String> {
    let mut loaded = Vec::new();
    for challenge in load_all_challenges(root)? {
        if let Some(topic) = config.qbank_topic_focus.as_deref() {
            if !challenge
                .topics
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(topic))
            {
                continue;
            }
        }
        let paper = load_paper(root, &challenge.publication_hash).ok();
        loaded.push(LoadedChallenge { challenge, paper });
    }
    loaded.sort_by(challenge_order);

    if let Some(path) = config.qbank_selection_path.as_deref() {
        let selected = load_selection(Path::new(path))?;
        loaded.retain(|item| selected.contains(&item.challenge.challenge_hash));
    }
    if config.qbank_top_n > 0 && loaded.len() > config.qbank_top_n {
        loaded.truncate(config.qbank_top_n);
    }
    Ok(loaded)
}

pub(crate) fn allow_fixture_qbank() -> bool {
    env::var("memory_benchmark_dev_qbank").ok().as_deref() == Some("1")
}

pub fn run_candidate(
    candidate: &str,
    adapter: &mut dyn MemorySystem,
    bank: &Path,
    config: &SuiteConfig,
) -> Result<CandidateReport, String> {
    let loaded = load_bank(bank, config)?;
    let dev_only = allow_fixture_qbank();
    if loaded.is_empty() {
        return Err(format!(
            "no accepted challenge JSON found under {}",
            bank.display()
        ));
    }
    if loaded.len() < 50 && !dev_only {
        return Err(format!(
            "real-papers bank at {} has only {} accepted challenges (need 50 unless memory_benchmark_dev_qbank=1)",
            bank.display(),
            loaded.len()
        ));
    }

    let mut axis_totals = AxisScores::default();
    let mut axis_counts = AxisScores::default();
    let mut fixtures_passed = 0u32;
    let mut fixture_records = Vec::new();

    for (index, loaded_challenge) in loaded.iter().enumerate() {
        observe_paper(adapter, loaded_challenge)?;
        let challenge = &loaded_challenge.challenge;
        let query = Query {
            text: challenge.question.clone(),
            intent: QueryIntent::Fact,
            mentions: vec![challenge.publication_hash.clone()],
            token_budget: config.context_budget,
        };
        let result = adapter.recall(&query);
        let axes = score::grade_answer(&result.answer, &result.used_ids, challenge);
        let weighted = weighted_fraction(&axes);
        if weighted >= 0.50 {
            fixtures_passed += 1;
        }
        accumulate(&mut axis_totals, &mut axis_counts, &axes);

        let mut record = BTreeMap::new();
        record.insert("id".to_string(), Json::Int((index + 1) as i64));
        record.insert(
            "challenge_hash".to_string(),
            Json::Str(challenge.challenge_hash.clone()),
        );
        record.insert(
            "publication_hash".to_string(),
            Json::Str(challenge.publication_hash.clone()),
        );
        record.insert("domain".to_string(), Json::Str(challenge.domain.clone()));
        record.insert(
            "difficulty_score".to_string(),
            Json::Float(challenge.difficulty_score as f64),
        );
        record.insert("axes".to_string(), axes_to_json(&axes));
        record.insert("weighted".to_string(), Json::Float(weighted as f64));
        fixture_records.push(Json::Object(record));
    }

    let avg = average(&axis_totals, &axis_counts);
    let total = score::weighted_average_total(&avg, &axis_counts);
    let mut top = BTreeMap::new();
    top.insert("name".to_string(), Json::Str(candidate.to_string()));
    top.insert("suite".to_string(), Json::Str("real-papers".to_string()));
    top.insert(
        "paper_bank".to_string(),
        Json::Str(bank.display().to_string()),
    );
    top.insert(
        "qbank_top_n".to_string(),
        Json::Int(config.qbank_top_n as i64),
    );
    top.insert("dev_only".to_string(), Json::Bool(dev_only));
    top.insert(
        "qbank_trusted".to_string(),
        Json::Bool(!dev_only && loaded.len() >= 50),
    );
    top.insert("total".to_string(), Json::Float(total as f64));
    top.insert("axes".to_string(), axes_to_json(&avg));
    top.insert("fixtures_run".to_string(), Json::Int(loaded.len() as i64));
    top.insert(
        "fixtures_passed".to_string(),
        Json::Int(fixtures_passed as i64),
    );
    top.insert("fixtures".to_string(), Json::Array(fixture_records));
    let json = Json::Object(top).to_string();

    Ok(CandidateReport {
        name: candidate.to_string(),
        total,
        fixtures_run: loaded.len() as u32,
        fixtures_passed,
        json,
    })
}
