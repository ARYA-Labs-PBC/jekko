//! Delta computation for score / findings / caps vs baseline.

use ratatui::style::Color;

use super::style::{BLUE, GOLD, GREEN, RED};

/// Delta direction enum mirrors `delta.ts::DeltaDirection`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeltaDirection {
    /// Metric improved vs baseline.
    Improving,
    /// Metric regressed vs baseline.
    Worsening,
    /// No change.
    Neutral,
    /// One side missing.
    Unknown,
}

impl DeltaDirection {
    pub(super) fn color(self) -> Color {
        match self {
            DeltaDirection::Improving => GREEN,
            DeltaDirection::Worsening => RED,
            DeltaDirection::Neutral => BLUE,
            DeltaDirection::Unknown => GOLD,
        }
    }
}

/// Whether a metric is "lower-is-better" (findings, caps) or "higher-is-better"
/// (score, level).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum DeltaMetric {
    /// Score is higher-is-better.
    Score,
    /// Findings counter — lower is better.
    Findings,
    /// Caps applied — lower is better.
    Caps,
    /// Hard findings — lower is better.
    Hard,
    /// Soft findings — lower is better.
    Soft,
    /// Conformance level — higher is better.
    Level,
}

impl DeltaMetric {
    fn lower_is_better(self) -> bool {
        !matches!(self, DeltaMetric::Score | DeltaMetric::Level)
    }
}

/// One delta computation result. Ports the `DeltaResult` struct in
/// `delta.ts`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeltaResult {
    /// `current - baseline`. None when either side is missing.
    pub delta: Option<f64>,
    /// Improvement/regression direction.
    pub direction: DeltaDirection,
    /// Glyph the panel draws beside the number (`▲`, `▼`, `=`, `-`).
    pub glyph: &'static str,
}

/// Compute the delta between `current` and `baseline` for a given metric.
///
/// Ports `delta.ts::delta`.
pub fn compute_delta(
    current: Option<f64>,
    baseline: Option<f64>,
    metric: DeltaMetric,
) -> DeltaResult {
    match (current, baseline) {
        (Some(c), Some(b)) => {
            let diff = c - b;
            if diff == 0.0 {
                return DeltaResult {
                    delta: Some(0.0),
                    direction: DeltaDirection::Neutral,
                    glyph: "=",
                };
            }
            let improving = if metric.lower_is_better() {
                diff < 0.0
            } else {
                diff > 0.0
            };
            let big = diff.abs() >= 10.0;
            let glyph = match (improving, big) {
                (true, true) => "▲▲",
                (true, false) => "▲",
                (false, true) => "▼▼",
                (false, false) => "▼",
            };
            DeltaResult {
                delta: Some(diff),
                direction: if improving {
                    DeltaDirection::Improving
                } else {
                    DeltaDirection::Worsening
                },
                glyph,
            }
        }
        _ => DeltaResult {
            delta: None,
            direction: DeltaDirection::Unknown,
            glyph: "-",
        },
    }
}

/// Format a [`DeltaResult`] like `+3 ▲` or `= 0`. Ports `delta.ts::formatDelta`.
pub fn format_delta(result: &DeltaResult) -> String {
    match result.delta {
        None => "-".to_string(),
        Some(0.0) => "= 0".to_string(),
        Some(d) => {
            let sign = if d > 0.0 { "+" } else { "" };
            // Treat near-integer deltas as whole numbers for terseness.
            if (d - d.round()).abs() < 1e-9 {
                format!("{}{} {}", sign, d.round() as i64, result.glyph)
            } else {
                format!("{}{:.1} {}", sign, d, result.glyph)
            }
        }
    }
}
