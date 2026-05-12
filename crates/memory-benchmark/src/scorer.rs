//! 10-axis pure-function scoring.
//!
//! Each scoring function takes a candidate `RecallResult` plus the fixture's
//! `Expected` and returns `Option<f32>` where:
//!   * `Some(s)` — the fixture exercised this axis; s ∈ [0.0, 1.0]
//!   * `None` — fixture doesn't test this axis; exclude from the average
//!
//! Per-axis averages are computed by `bin/bench.rs` over only the `Some`
//! contributions, so axes a fixture doesn't exercise don't inflate the score.

use crate::fixture::Expected;
use crate::{AxisScores, RecallResult};

fn answer_contains_all(out: &RecallResult, needles: &[&str]) -> bool {
    let lower = out.answer.to_lowercase();
    needles.iter().all(|n| lower.contains(&n.to_lowercase()))
}

fn answer_contains_none(out: &RecallResult, needles: &[&str]) -> bool {
    needles.iter().all(|n| !out.answer.contains(n))
}

fn used_ids_contains_all(out: &RecallResult, needles: &[&str]) -> bool {
    needles
        .iter()
        .all(|id| out.used_ids.iter().any(|u| u == id))
}

fn used_ids_contains_none(out: &RecallResult, needles: &[&str]) -> bool {
    needles
        .iter()
        .all(|id| !out.used_ids.iter().any(|u| u == id))
}

// ───────── axis functions: Option<f32> ─────────

pub fn correctness(out: &RecallResult, exp: &Expected) -> Option<f32> {
    let has_constraint = !exp.must_contain.is_empty()
        || !exp.must_not_contain.is_empty()
        || !exp.must_include.is_empty()
        || !exp.must_exclude.is_empty();
    if !has_constraint {
        return None;
    }
    let mut hits = 0u32;
    let mut total = 0u32;
    if !exp.must_contain.is_empty() {
        total += 1;
        if answer_contains_all(out, exp.must_contain) {
            hits += 1;
        }
    }
    if !exp.must_not_contain.is_empty() {
        total += 1;
        if answer_contains_none(out, exp.must_not_contain) {
            hits += 1;
        }
    }
    if !exp.must_include.is_empty() {
        total += 1;
        if used_ids_contains_all(out, exp.must_include) {
            hits += 1;
        }
    }
    if !exp.must_exclude.is_empty() {
        total += 1;
        if used_ids_contains_none(out, exp.must_exclude) {
            hits += 1;
        }
    }
    Some(hits as f32 / total as f32)
}

pub fn provenance(out: &RecallResult, exp: &Expected) -> Option<f32> {
    if !exp.requires_citation {
        return None;
    }
    Some(if out.citations.is_empty() { 0.0 } else { 1.0 })
}

pub fn bitemporal_recall(out: &RecallResult, exp: &Expected) -> Option<f32> {
    let causal = exp
        .required_warnings
        .iter()
        .any(|w| *w == "causal_mask_applied");
    let has_temporal = causal
        || !exp.must_exclude.is_empty()
        || (!exp.must_include.is_empty() && !exp.required_warnings.is_empty());
    if !has_temporal {
        return None;
    }
    let mut hits = 0u32;
    let mut total = 0u32;
    if !exp.must_include.is_empty() {
        total += 1;
        if used_ids_contains_all(out, exp.must_include) {
            hits += 1;
        }
    }
    if !exp.must_exclude.is_empty() {
        total += 1;
        if used_ids_contains_none(out, exp.must_exclude) {
            hits += 1;
        }
    }
    if causal {
        total += 1;
        if out
            .warnings
            .iter()
            .any(|w| w.name() == "causal_mask_applied")
        {
            hits += 1;
        }
    }
    if total == 0 {
        None
    } else {
        Some(hits as f32 / total as f32)
    }
}

pub fn contradiction(out: &RecallResult, exp: &Expected) -> Option<f32> {
    let needed: Vec<&str> = exp
        .required_warnings
        .iter()
        .copied()
        .filter(|w| matches!(*w, "contradicted" | "stale" | "skeptic_surfaced"))
        .collect();
    if needed.is_empty() {
        return None;
    }
    let hits = needed
        .iter()
        .filter(|w| out.warnings.iter().any(|x| x.name() == **w))
        .count();
    Some(hits as f32 / needed.len() as f32)
}

pub fn math_science(out: &RecallResult, exp: &Expected) -> Option<f32> {
    let has_unit = exp.required_warnings.iter().any(|w| *w == "unit_mismatch");
    let science_terms: Vec<&str> = exp
        .must_contain
        .iter()
        .copied()
        .filter(|s| {
            let s = s.to_lowercase();
            s.contains("kg")
                || s.contains("gev")
                || s.contains("c")
                || s.contains("h_")
                || s.contains("f =")
                || s.contains("e =")
                || s.contains("c²")
                || s.contains("ev")
                || s.contains("σ")
        })
        .collect();
    if !has_unit && science_terms.is_empty() {
        return None;
    }
    let mut hits = 0u32;
    let mut total = 0u32;
    if has_unit {
        total += 1;
        if out.warnings.iter().any(|w| w.name() == "unit_mismatch") {
            hits += 1;
        }
    }
    if !science_terms.is_empty() {
        total += 1;
        if science_terms
            .iter()
            .all(|s| out.answer.to_lowercase().contains(&s.to_lowercase()))
        {
            hits += 1;
        }
    }
    Some(hits as f32 / total as f32)
}

pub fn english_discourse(out: &RecallResult, exp: &Expected) -> Option<f32> {
    let english_targets: Vec<&str> = exp
        .must_contain
        .iter()
        .copied()
        .filter(|s| {
            let l = s.to_lowercase();
            l == "bob"
                || l == "alice"
                || l == "she"
                || l == "riverbank"
                || l == "stepped down"
                || l == "director"
        })
        .collect();
    if english_targets.is_empty() {
        return None;
    }
    let ok = answer_contains_all(out, &english_targets);
    Some(if ok { 1.0 } else { 0.0 })
}

pub fn privacy_redaction(out: &RecallResult, exp: &Expected) -> Option<f32> {
    let canaries: Vec<&str> = exp
        .must_not_contain
        .iter()
        .copied()
        .filter(|s| {
            s.contains("sk-memory-bench-CANARY")
                || s.contains("AKIA")
                || s.contains("MEMORY_BENCH_CANARY")
                || s.contains("1985-03-12")
                || s.contains("memory-benchmark-test-corp")
                || s.contains("94110")
        })
        .collect();
    let needs_redacted_warning = exp.required_warnings.iter().any(|w| *w == "redacted");
    if canaries.is_empty() && !needs_redacted_warning {
        return None;
    }
    let mut hits = 0u32;
    let mut total = 0u32;
    if !canaries.is_empty() {
        total += 1;
        if answer_contains_none(out, &canaries) {
            hits += 1;
        }
    }
    if needs_redacted_warning {
        total += 1;
        if out.warnings.iter().any(|w| w.name() == "redacted") {
            hits += 1;
        }
    }
    Some(hits as f32 / total as f32)
}

pub fn procedural_skill(out: &RecallResult, exp: &Expected) -> Option<f32> {
    // Heuristic: procedural fixtures tend to require_citation AND mention a
    // skill name in must_contain.
    let mentions_skill = exp.must_contain.iter().any(|s| {
        s.contains("normalize")
            || s.contains("UNSAFE")
            || s.contains("doi_")
            || s.contains("refuse")
    }) || exp
        .must_not_contain
        .iter()
        .any(|s| s.contains("fs_delete") || s.contains("net_exfil"));
    if !mentions_skill {
        return None;
    }
    let mut hits = 0u32;
    let mut total = 0u32;
    if !exp.must_contain.is_empty() {
        total += 1;
        if answer_contains_all(out, exp.must_contain) {
            hits += 1;
        }
    }
    if !exp.must_not_contain.is_empty() {
        total += 1;
        if answer_contains_none(out, exp.must_not_contain) {
            hits += 1;
        }
    }
    if total == 0 {
        None
    } else {
        Some(hits as f32 / total as f32)
    }
}

pub fn feedback_adaptation(out: &RecallResult, exp: &Expected) -> Option<f32> {
    if exp.confidence_range.is_none() {
        return None;
    }
    let (lo, hi) = exp.confidence_range.unwrap();
    if out.confidence >= lo && out.confidence <= hi {
        Some(1.0)
    } else {
        Some(0.0)
    }
}

pub fn determinism_rebuild(out: &RecallResult, exp: &Expected) -> Option<f32> {
    if !exp.expects_stable_state_hash {
        return None;
    }
    Some(if out.context_pack_hash.is_empty() {
        0.0
    } else {
        1.0
    })
}

// ───────── compatibility shim: grade_all_axes returns AxisScores ─────────
//
// Fixtures still reference `grade: scorer::grade_all_axes`. We compute each
// axis as Option<f32> internally, but expose a flat AxisScores struct where
// unexercised axes are encoded as f32::NAN. bench.rs's averaging strips NAN.

pub fn grade_all_axes(out: &RecallResult, exp: &Expected) -> AxisScores {
    let mut a = AxisScores::default();
    a.correctness = correctness(out, exp).unwrap_or(f32::NAN);
    a.provenance = provenance(out, exp).unwrap_or(f32::NAN);
    a.bitemporal_recall = bitemporal_recall(out, exp).unwrap_or(f32::NAN);
    a.contradiction = contradiction(out, exp).unwrap_or(f32::NAN);
    a.math_science = math_science(out, exp).unwrap_or(f32::NAN);
    a.english_discourse_coreference = english_discourse(out, exp).unwrap_or(f32::NAN);
    a.privacy_redaction = privacy_redaction(out, exp).unwrap_or(f32::NAN);
    a.procedural_skill = procedural_skill(out, exp).unwrap_or(f32::NAN);
    a.feedback_adaptation = feedback_adaptation(out, exp).unwrap_or(f32::NAN);
    a.determinism_rebuild = determinism_rebuild(out, exp).unwrap_or(f32::NAN);
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecallResult;

    fn empty_recall() -> RecallResult {
        let mut r = RecallResult::default();
        r.context_pack_hash = "deadbeefdeadbeef".to_string();
        r
    }

    fn empty_expected() -> Expected {
        Expected {
            must_include: &[],
            must_exclude: &[],
            must_contain: &[],
            must_not_contain: &[],
            required_warnings: &[],
            requires_citation: false,
            expected_modality: None,
            confidence_range: None,
            expects_stable_state_hash: false,
        }
    }

    #[test]
    fn empty_fixture_returns_none_for_all_axes() {
        let r = empty_recall();
        let e = empty_expected();
        let a = grade_all_axes(&r, &e);
        // All NaN — unexercised.
        assert!(a.correctness.is_nan());
        assert!(a.provenance.is_nan());
        assert!(a.bitemporal_recall.is_nan());
    }

    #[test]
    fn provenance_axis_active_when_required() {
        let mut e = empty_expected();
        e.requires_citation = true;
        let r = empty_recall();
        assert_eq!(provenance(&r, &e), Some(0.0));
    }
}
