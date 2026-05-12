use std::env;
use std::process;

use crate::fixture::{Fixture, Setup, SetupEvent};
use crate::{
    AxisScores, ClaimModality, Event, EventKind, Feedback, MemorySystem, Outcome, PrivacyClass,
    Query, RecallResult, Source, Split, SuiteConfig, TemporalLens,
};

/// Replayable proof command pinned alongside `gate_findings` so audit
/// receipts can be reproduced from a single line. Surfaced as a JSON field
/// downstream so reviewers do not have to accept summarized claims.
pub const GATE_REPLAY_CMD: &str = "rtk just memory-benchmark-fast";

/// Canonical reference-candidate ordering used by the population reducer
/// and by tests that need a deterministic enumeration of the built-in
/// adapters. The order is contract; do not sort or reorder.
pub const DEFAULT_REFERENCE_CANDIDATES: &[&str] = &[
    "baseline",
    "reference_context_pack",
    "reference_evidence_ledger",
    "reference_claim_skeptic",
];

pub fn parse_args() -> (String, Option<String>, SuiteConfig) {
    let mut candidate = String::new();
    let mut out: Option<String> = None;
    let mut config = SuiteConfig::default();
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--candidate" => {
                candidate = match args.get(i + 1) {
                    Some(value) => value.clone(),
                    None => String::new(),
                };
                i += 2;
            }
            "--out" => {
                out = args.get(i + 1).cloned();
                i += 2;
            }
            "--suite" => {
                if let Some(value) = args.get(i + 1) {
                    config.split = match value.as_str() {
                        "public" => Split::PublicSmoke,
                        "generated" => Split::PublicGenerated,
                        "stress" => Split::Stress,
                        _ => config.split,
                    };
                }
                i += 2;
            }
            "--split" => {
                if let Some(value) = args.get(i + 1) {
                    config.split = match value.as_str() {
                        "public-dev" => Split::PublicGenerated,
                        "private" => Split::PrivateGenerated,
                        "stress" => Split::Stress,
                        "public" => Split::PublicSmoke,
                        _ => config.split,
                    };
                }
                i += 2;
            }
            "--seed" => {
                if let Some(value) = args.get(i + 1) {
                    config.seed_label = value.clone();
                }
                i += 2;
            }
            "--fixtures" => {
                if let Some(value) = args.get(i + 1).and_then(|v| v.parse::<usize>().ok()) {
                    config.fixture_count = value;
                }
                i += 2;
            }
            "--difficulty" => {
                if let Some(value) = args.get(i + 1).and_then(|v| v.parse::<u8>().ok()) {
                    config.difficulty = value.clamp(1, 5);
                }
                i += 2;
            }
            "--context-budget" => {
                if let Some(value) = args.get(i + 1).and_then(|v| v.parse::<u32>().ok()) {
                    config.context_budget = value;
                }
                i += 2;
            }
            "--json" => {
                i += 1;
            }
            "--help" | "-h" => {
                eprintln!(
                    "bench --candidate <name> [--suite public|generated|stress] [--seed label] [--fixtures n] [--out path] [--json]\n  candidate in {{baseline, reference_context_pack, reference_evidence_ledger, reference_claim_skeptic,\n    ledger_first, hybrid_index, temporal_graph, compression_first, skeptic_dataset,\n    arena_lane_00, arena_lane_01, arena_lane_02, arena_lane_03, arena_lane_04,\n    arena_lane_05, arena_lane_06, arena_lane_07, arena_lane_08, arena_lane_09,\n    arena_lane_10, arena_lane_11, arena_lane_12, arena_lane_13, arena_lane_14,\n    arena_lane_15, arena_lane_16, arena_lane_17, arena_lane_18, arena_lane_19}}"
                );
                process::exit(0);
            }
            _ => {
                i += 1;
            }
        }
    }
    if candidate.is_empty() {
        eprintln!("bench: --candidate <name> is required");
        process::exit(2);
    }
    (candidate, out, config)
}

fn privacy_of(s: &str) -> PrivacyClass {
    match s {
        "Public" => PrivacyClass::Public,
        "Internal" => PrivacyClass::Internal,
        "Confidential" => PrivacyClass::Confidential,
        "Secret" => PrivacyClass::Secret,
        "Vault" => PrivacyClass::Vault,
        _ => PrivacyClass::Internal,
    }
}

fn kind_of(s: &str) -> EventKind {
    match s {
        "Claim" => EventKind::Claim,
        "Observation" => EventKind::Observation,
        "Equation" => EventKind::Equation,
        "Theorem" => EventKind::Theorem,
        "Skill" => EventKind::Skill,
        "Resource" => EventKind::Resource,
        "Dataset" => EventKind::Dataset,
        "Experiment" => EventKind::Experiment,
        "Hypothesis" => EventKind::Hypothesis,
        "Counterexample" => EventKind::Counterexample,
        "Lesson" => EventKind::Lesson,
        "Question" => EventKind::Question,
        "VaultCanary" => EventKind::VaultCanary,
        "SchemaMigration" => EventKind::SchemaMigration,
        _ => EventKind::Claim,
    }
}

fn modality_of(s: &str) -> Option<ClaimModality> {
    Some(match s {
        "Observed" => ClaimModality::Observed,
        "AssertedBySource" => ClaimModality::AssertedBySource,
        "InferredByAgent" => ClaimModality::InferredByAgent,
        "HumanApproved" => ClaimModality::HumanApproved,
        "FormallyVerified" => ClaimModality::FormallyVerified,
        _ => return None,
    })
}

fn outcome_of(s: &str) -> Outcome {
    match s {
        "TaskSuccess" => Outcome::TaskSuccess,
        "TaskFailure" => Outcome::TaskFailure,
        "Verified" => Outcome::Verified,
        "Falsified" => Outcome::Falsified,
        _ => Outcome::Ignored,
    }
}

fn setup_event_to_event(se: &SetupEvent) -> Event {
    Event {
        id: se.id.to_string(),
        kind: kind_of(se.kind),
        subject: se.subject.to_string(),
        body: se.body.to_string(),
        sources: vec![Source {
            uri: se.source_uri.to_string(),
            citation: se.source_citation.to_string(),
            quality: se.source_quality,
        }],
        valid_from: se.valid_from.map(|s| s.to_string()),
        valid_to: se.valid_to.map(|s| s.to_string()),
        tx_time: se.tx_time.to_string(),
        event_time: None,
        observation_time: None,
        review_time: None,
        policy_time: None,
        dependencies: vec![],
        supersedes: vec![],
        contradicts: vec![],
        derived_from: vec![],
        namespace: None,
        privacy_class: privacy_of(se.privacy),
        claim_modality: se.claim_modality.and_then(modality_of),
        tags: se.tags.iter().map(|s| s.to_string()).collect(),
    }
}

fn build_query(f: &Fixture) -> Option<Query> {
    f.query_text.map(|t| Query {
        text: t.to_string(),
        intent: f.query_intent,
        mentions: f.query_mentions.iter().map(|s| s.to_string()).collect(),
        token_budget: 4096,
    })
}

pub(crate) fn run_fixture(adapter: &mut dyn MemorySystem, f: &Fixture) -> Option<RecallResult> {
    match &f.setup {
        Setup::NoSetup => {}
        Setup::Observe(events) => {
            for se in *events {
                let e = setup_event_to_event(se);
                let _ = adapter.observe(&e);
            }
        }
        Setup::Feedback {
            outcome_kind,
            used_event_ids,
            reason,
        } => {
            let fb = Feedback {
                outcome: outcome_of(outcome_kind),
                used: used_event_ids.iter().map(|s| s.to_string()).collect(),
                reason: Some(reason.to_string()),
            };
            let _ = adapter.feedback("pack-fixture", &fb);
        }
        Setup::Rebuild => {
            let _ = adapter.rebuild();
        }
        Setup::Forget { memory_id, reason } => {
            let _ = adapter.forget(memory_id, reason);
        }
    }

    let q = build_query(f)?;
    let result = match f.lens {
        TemporalLens::Current => adapter.recall(&q),
        TemporalLens::At => adapter.recall_at(&q, f.world_time.unwrap_or("")),
        TemporalLens::AsOf => adapter.recall_as_of(&q, f.tx_time.unwrap_or("")),
        TemporalLens::AtAsOf => adapter.recall_at(&q, f.world_time.unwrap_or("")),
        TemporalLens::NoQuery => return None,
    };
    Some(result)
}

pub(crate) fn add_if_active(total: &mut f32, count: &mut f32, value: f32) {
    if !value.is_nan() {
        *total += value;
        *count += 1.0;
    }
}

pub(crate) fn accumulate(totals: &mut AxisScores, counts: &mut AxisScores, a: &AxisScores) {
    add_if_active(
        &mut totals.correctness,
        &mut counts.correctness,
        a.correctness,
    );
    add_if_active(&mut totals.provenance, &mut counts.provenance, a.provenance);
    add_if_active(
        &mut totals.bitemporal_recall,
        &mut counts.bitemporal_recall,
        a.bitemporal_recall,
    );
    add_if_active(
        &mut totals.contradiction,
        &mut counts.contradiction,
        a.contradiction,
    );
    add_if_active(
        &mut totals.math_science,
        &mut counts.math_science,
        a.math_science,
    );
    add_if_active(
        &mut totals.english_discourse_coreference,
        &mut counts.english_discourse_coreference,
        a.english_discourse_coreference,
    );
    add_if_active(
        &mut totals.privacy_redaction,
        &mut counts.privacy_redaction,
        a.privacy_redaction,
    );
    add_if_active(
        &mut totals.procedural_skill,
        &mut counts.procedural_skill,
        a.procedural_skill,
    );
    add_if_active(
        &mut totals.feedback_adaptation,
        &mut counts.feedback_adaptation,
        a.feedback_adaptation,
    );
    add_if_active(
        &mut totals.determinism_rebuild,
        &mut counts.determinism_rebuild,
        a.determinism_rebuild,
    );
}

pub(crate) fn weighted_fraction(a: &AxisScores) -> f32 {
    let w = AxisScores::WEIGHTS;
    let mut sum = 0.0_f32;
    let mut wsum = 0.0_f32;
    let pairs = [
        (a.correctness, w.correctness),
        (a.provenance, w.provenance),
        (a.bitemporal_recall, w.bitemporal_recall),
        (a.contradiction, w.contradiction),
        (a.math_science, w.math_science),
        (
            a.english_discourse_coreference,
            w.english_discourse_coreference,
        ),
        (a.privacy_redaction, w.privacy_redaction),
        (a.procedural_skill, w.procedural_skill),
        (a.feedback_adaptation, w.feedback_adaptation),
        (a.determinism_rebuild, w.determinism_rebuild),
    ];
    for (v, weight) in pairs {
        if !v.is_nan() {
            sum += v * weight;
            wsum += weight;
        }
    }
    if wsum > 0.0 {
        sum / wsum
    } else {
        0.5
    }
}

pub(crate) fn average(t: &AxisScores, c: &AxisScores) -> AxisScores {
    let safe = |a: f32, b: f32| if b > 0.0 { a / b } else { 0.0 };
    AxisScores {
        correctness: safe(t.correctness, c.correctness),
        provenance: safe(t.provenance, c.provenance),
        bitemporal_recall: safe(t.bitemporal_recall, c.bitemporal_recall),
        contradiction: safe(t.contradiction, c.contradiction),
        math_science: safe(t.math_science, c.math_science),
        english_discourse_coreference: safe(
            t.english_discourse_coreference,
            c.english_discourse_coreference,
        ),
        privacy_redaction: safe(t.privacy_redaction, c.privacy_redaction),
        procedural_skill: safe(t.procedural_skill, c.procedural_skill),
        feedback_adaptation: safe(t.feedback_adaptation, c.feedback_adaptation),
        determinism_rebuild: safe(t.determinism_rebuild, c.determinism_rebuild),
    }
}
