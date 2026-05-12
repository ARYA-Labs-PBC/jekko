use super::*;

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
    assert!((s - 100.0).abs() < 0.001, "weights sum to {}, not 100", s);
}

#[test]
fn pathology_count_is_ten() {
    assert_eq!(Pathology::ALL.len(), 10);
}

#[test]
fn domain_count_is_five() {
    assert_eq!(Domain::ALL.len(), 5);
}

/// Guard against drift back to branded spec identifiers.
///
/// Walks every `.rs` file in `src/` and asserts none contain the
/// retired names. The banned strings are constructed from fragments
/// at runtime so this test's own source doesn't trip the check.
#[test]
fn no_branded_identifiers() {
    use std::path::Path;
    // Build banned strings from fragments so they don't appear verbatim
    // in this source file.
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
        // Skip this lib_tests.rs itself — the fragments above are inert but
        // string comparison on this file would still match `lib.rs`.
        if p.file_name().and_then(|n| n.to_str()) == Some("lib_tests.rs") {
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
        (35.0..=75.0).contains(&baseline.total),
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
            (70.0..=88.0).contains(&report.total),
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
