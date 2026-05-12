//! Shared deterministic runner for the benchmark binaries.
//!
//! Usage:
//!     bench --candidate <name> [--out <path>] [--json]
//!
//! Reference candidates:
//!   baseline | reference_context_pack | reference_evidence_ledger |
//!   reference_claim_skeptic | arena_lane_00..arena_lane_19
//!
//! Always deterministic. Two invocations with identical input produce
//! byte-identical output (FNV-1a-hashed in `verify_determinism`).

use std::collections::BTreeMap;
use std::fs;
use std::process;

#[rustfmt::skip]
use crate::adapters::{baseline, reference_claim_skeptic, reference_context_pack, reference_evidence_ledger};
#[rustfmt::skip]
use crate::candidates::{arena, compression_first, hybrid_index, ledger_first, skeptic_dataset, temporal_graph};
use crate::json::{self, Json};
use crate::memory_api::axes_to_json;
use crate::runner_generated::run_generated_candidate;
use crate::runner_support::{accumulate, average, parse_args, run_fixture, weighted_fraction};
use crate::{AxisScores, MemorySystem, Split, SuiteConfig};

pub use crate::runner_support::DEFAULT_REFERENCE_CANDIDATES;

pub struct CandidateReport {
    pub name: String,
    pub total: f32,
    pub fixtures_run: u32,
    pub fixtures_passed: u32,
    pub json: String,
}

fn boxed_adapter(name: &str) -> Result<Box<dyn MemorySystem>, String> {
    match name {
        "baseline" => Ok(Box::new(baseline::Adapter::default())),
        "reference_context_pack" => Ok(Box::new(reference_context_pack::Adapter::default())),
        "reference_evidence_ledger" => Ok(Box::new(reference_evidence_ledger::Adapter::default())),
        "reference_claim_skeptic" => Ok(Box::new(reference_claim_skeptic::Adapter::default())),
        "exec" | "ledger_first" => Ok(Box::new(ledger_first::Adapter::default())),
        "hybrid_index" => Ok(Box::new(hybrid_index::Adapter::default())),
        "temporal_graph" => Ok(Box::new(temporal_graph::Adapter::default())),
        "compression_first" => Ok(Box::new(compression_first::Adapter::default())),
        "skeptic_dataset" => Ok(Box::new(skeptic_dataset::Adapter::default())),
        "arena_lane_00" => Ok(Box::new(arena::lane_00::Adapter::default())),
        "arena_lane_01" => Ok(Box::new(arena::lane_01::Adapter::default())),
        "arena_lane_02" => Ok(Box::new(arena::lane_02::Adapter::default())),
        "arena_lane_03" => Ok(Box::new(arena::lane_03::Adapter::default())),
        "arena_lane_04" => Ok(Box::new(arena::lane_04::Adapter::default())),
        "arena_lane_05" => Ok(Box::new(arena::lane_05::Adapter::default())),
        "arena_lane_06" => Ok(Box::new(arena::lane_06::Adapter::default())),
        "arena_lane_07" => Ok(Box::new(arena::lane_07::Adapter::default())),
        "arena_lane_08" => Ok(Box::new(arena::lane_08::Adapter::default())),
        "arena_lane_09" => Ok(Box::new(arena::lane_09::Adapter::default())),
        "arena_lane_10" => Ok(Box::new(arena::lane_10::Adapter::default())),
        "arena_lane_11" => Ok(Box::new(arena::lane_11::Adapter::default())),
        "arena_lane_12" => Ok(Box::new(arena::lane_12::Adapter::default())),
        "arena_lane_13" => Ok(Box::new(arena::lane_13::Adapter::default())),
        "arena_lane_14" => Ok(Box::new(arena::lane_14::Adapter::default())),
        "arena_lane_15" => Ok(Box::new(arena::lane_15::Adapter::default())),
        "arena_lane_16" => Ok(Box::new(arena::lane_16::Adapter::default())),
        "arena_lane_17" => Ok(Box::new(arena::lane_17::Adapter::default())),
        "arena_lane_18" => Ok(Box::new(arena::lane_18::Adapter::default())),
        "arena_lane_19" => Ok(Box::new(arena::lane_19::Adapter::default())),
        other => Err(format!("unknown candidate {:?}", other)),
    }
}

pub fn run_cli() {
    let (candidate, out_path, config) = parse_args();
    let report = match run_candidate_with_config(&candidate, &config) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(2);
        }
    };

    if let Some(p) = out_path {
        if let Some(parent) = std::path::Path::new(&p).parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::write(&p, &report.json) {
            eprintln!("bench: write {} failed: {}", p, e);
            process::exit(3);
        }
        eprintln!(
            "bench: candidate={} total={:.2} fixtures={}/{} -> {}",
            report.name, report.total, report.fixtures_passed, report.fixtures_run, p
        );
    } else {
        println!("{}", report.json);
    }
}

pub fn run_candidate(candidate: &str) -> Result<CandidateReport, String> {
    run_candidate_with_config(candidate, &SuiteConfig::default())
}

pub fn run_candidate_with_config(
    candidate: &str,
    config: &SuiteConfig,
) -> Result<CandidateReport, String> {
    let mut adapter = boxed_adapter(candidate)?;
    if config.split != Split::PublicSmoke {
        return run_generated_candidate(candidate, adapter.as_mut(), config);
    }
    let fixtures = crate::fixture::all();

    let mut axis_totals = AxisScores::default();
    let mut axis_counts = AxisScores::default();
    let mut fixtures_run = 0u32;
    let mut fixtures_passed = 0u32;
    let mut fixture_records: Vec<Json> = Vec::with_capacity(fixtures.len());

    for f in fixtures {
        fixtures_run += 1;
        let mut record = BTreeMap::new();
        record.insert("id".to_string(), Json::Int(f.id as i64));
        record.insert("block".to_string(), Json::Str(f.block.name().to_string()));
        record.insert("domain".to_string(), Json::Str(f.domain.name().to_string()));

        let result = run_fixture(adapter.as_mut(), f);

        let axes = if let Some(r) = result {
            (f.grade)(&r, &f.expected)
        } else {
            // Ingest-only fixtures: no query → no axis is exercised here.
            // All axes marked NaN so they're excluded from averaging.
            AxisScores {
                correctness: f32::NAN,
                provenance: f32::NAN,
                bitemporal_recall: f32::NAN,
                contradiction: f32::NAN,
                math_science: f32::NAN,
                english_discourse_coreference: f32::NAN,
                privacy_redaction: f32::NAN,
                procedural_skill: f32::NAN,
                feedback_adaptation: f32::NAN,
                determinism_rebuild: f32::NAN,
            }
        };

        // Compute weighted score for this fixture (ignoring NaN axes).
        let weighted = weighted_fraction(&axes);
        if weighted >= 0.50 {
            fixtures_passed += 1;
        }

        record.insert("axes".to_string(), axes_to_json(&axes));
        record.insert("weighted".to_string(), Json::Float(weighted as f64));
        fixture_records.push(Json::Object(record));

        accumulate(&mut axis_totals, &mut axis_counts, &axes);
    }

    let avg = average(&axis_totals, &axis_counts);
    // Total: weighted sum of axis averages, normalized to a 100-point scale.
    // Only axes that had ≥ 1 contributing fixture count toward the
    // weight-normalizer. This makes the total faithful to what the fixture
    // set actually exercised, not the rubric's theoretical maximum.
    let w = AxisScores::WEIGHTS;
    let pairs = [
        (avg.correctness, w.correctness, axis_counts.correctness),
        (avg.provenance, w.provenance, axis_counts.provenance),
        (
            avg.bitemporal_recall,
            w.bitemporal_recall,
            axis_counts.bitemporal_recall,
        ),
        (
            avg.contradiction,
            w.contradiction,
            axis_counts.contradiction,
        ),
        (avg.math_science, w.math_science, axis_counts.math_science),
        (
            avg.english_discourse_coreference,
            w.english_discourse_coreference,
            axis_counts.english_discourse_coreference,
        ),
        (
            avg.privacy_redaction,
            w.privacy_redaction,
            axis_counts.privacy_redaction,
        ),
        (
            avg.procedural_skill,
            w.procedural_skill,
            axis_counts.procedural_skill,
        ),
        (
            avg.feedback_adaptation,
            w.feedback_adaptation,
            axis_counts.feedback_adaptation,
        ),
        (
            avg.determinism_rebuild,
            w.determinism_rebuild,
            axis_counts.determinism_rebuild,
        ),
    ];
    let mut sum = 0.0_f32;
    let mut wsum = 0.0_f32;
    for (v, weight, c) in pairs {
        if c > 0.0 {
            sum += v * weight;
            wsum += weight;
        }
    }
    let total = if wsum > 0.0 { sum / wsum * 100.0 } else { 0.0 };

    let mut top = BTreeMap::new();
    top.insert("name".to_string(), Json::Str(candidate.to_string()));
    top.insert("total".to_string(), Json::Float(total as f64));
    top.insert("axes".to_string(), axes_to_json(&avg));
    top.insert("fixtures_run".to_string(), Json::Int(fixtures_run as i64));
    top.insert(
        "fixtures_passed".to_string(),
        Json::Int(fixtures_passed as i64),
    );
    top.insert("fixtures".to_string(), Json::Array(fixture_records));
    let s = Json::Object(top).to_string();

    Ok(CandidateReport {
        name: candidate.to_string(),
        total,
        fixtures_run,
        fixtures_passed,
        json: s,
    })
}

// Anchor the `json` re-export so callers retain the symbol path.
#[allow(dead_code)]
fn _anchor(_: json::Json) {}
