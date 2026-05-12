//! Deterministic memory benchmark harness library.
//!
//! The public API is intentionally re-exported from small modules so candidate
//! adapters can keep using `memory_benchmark::{Event, MemorySystem,
//! RecallResult}` while the harness grows generated suites and executable
//! oracles.

pub mod adapters;
pub mod candidates;
pub mod case;
pub mod chase_report;
pub mod corpus;
pub mod fixture;
pub mod generated;
pub mod grow_curriculum;
pub mod hash;
pub mod json;
pub(crate) mod json_parser;
pub mod memory_api;
pub mod oracle;
pub mod population_memory;
pub mod report;
pub mod result;
pub mod runner;
pub(crate) mod runner_generated;
pub mod runner_support;
pub mod scorer;
pub mod scoring;
pub mod triangulate;
pub mod types;

pub use case::{BenchCase, CaseOracle, EpisodeStep, OracleKind, Split, SuiteConfig};
pub use result::{
    Citation, ClaimRecord, ClaimStatus, ContextMetric, OmissionNote, RecallResult, Redaction,
    SkillCall,
};
pub use scoring::{AxisScores, ScoringAxis};
pub use types::{
    ClaimModality, Domain, Event, EventKind, Feedback, FixtureBlock, MemorySystem, Outcome,
    Pathology, PrivacyClass, PublicBench, Query, QueryIntent, Receipt, Source, TemporalLens,
    Tombstone, Warning,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_reexports_compile() {
        fn accepts_public_api(_: Event, _: &mut dyn MemorySystem, _: RecallResult) {}
        let _ = accepts_public_api;
    }

    #[test]
    fn axis_weights_sum_to_100() {
        let w = AxisScores::WEIGHTS;
        let s = w.correctness
            + w.provenance
            + w.bitemporal_recall
            + w.contradiction
            + w.math_science
            + w.english_discourse_coreference
            + w.privacy_redaction
            + w.procedural_skill
            + w.feedback_adaptation
            + w.determinism_rebuild;
        assert!((s - 100.0).abs() < 0.001, "weights sum to {}", s);
    }

    #[test]
    fn advanced_axis_weights_sum_to_100() {
        let s: f32 = AxisScores::ADVANCED_WEIGHTS.iter().map(|(_, w)| *w).sum();
        assert!((s - 100.0).abs() < 0.001, "weights sum to {}", s);
    }

    #[test]
    fn pathology_count_is_ten() {
        assert_eq!(Pathology::ALL.len(), 10);
    }

    #[test]
    fn domain_count_is_five() {
        assert_eq!(Domain::ALL.len(), 5);
    }

    #[test]
    fn no_branded_identifiers() {
        use std::path::Path;
        let retired_suffix = format!("{}{}{}", "_", "v", "3");
        let banned: Vec<String> = vec![
            format!("{}{}", "claude", retired_suffix),
            format!("{}{}", "codex", retired_suffix),
            format!("{}{}", "gemini", retired_suffix),
            format!("{}{}", "codex-", "memory"),
            format!("{}{}{}", "memory-", "v", "3"),
            format!("{}{}{}", "MG", "V", "3"),
            format!("{}{}", "Memory", "Gauntlet"),
            format!("{}{}{}", "mne", "mos_", "gauntlet"),
        ];
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut violations: Vec<String> = Vec::new();
        walk_dir(&src, &mut |p: &Path| {
            if p.extension().and_then(|e| e.to_str()) != Some("rs") {
                return;
            }
            let Ok(content) = std::fs::read_to_string(p) else {
                return;
            };
            for needle in &banned {
                if content.contains(needle) {
                    violations.push(format!(
                        "{}: contains banned identifier {:?}",
                        p.strip_prefix(env!("CARGO_MANIFEST_DIR"))
                            .unwrap_or(p)
                            .display(),
                        needle
                    ));
                }
            }
        });
        assert!(
            violations.is_empty(),
            "branded benchmark identifiers found:\n  {}",
            violations.join("\n  ")
        );
    }

    #[test]
    fn candidate_score_bands_stay_calibrated() {
        let baseline = crate::runner::run_candidate("baseline").expect("baseline report");
        assert!(
            (25.0..=75.0).contains(&baseline.total),
            "baseline score {} outside calibration band",
            baseline.total
        );

        for candidate in [
            "reference_context_pack",
            "reference_evidence_ledger",
            "reference_claim_skeptic",
        ] {
            let report = crate::runner::run_candidate(candidate).expect("reference report");
            assert!(
                (70.0..=90.0).contains(&report.total),
                "{} score {} outside calibration band",
                candidate,
                report.total
            );
        }
    }

    fn walk_dir(dir: &std::path::Path, visit: &mut dyn FnMut(&std::path::Path)) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk_dir(&p, visit);
            } else {
                visit(&p);
            }
        }
    }
}
