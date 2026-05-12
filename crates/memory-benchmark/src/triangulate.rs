//! Prompt-scoring vs executable-bench triangulation.
//!
//! Detects when LLM judges and the deterministic harness disagree, surfacing
//! axes where the rubric or fixture set may be biased.
//! The module is intentionally tiny so the comparison logic stays easy to
//! audit and reuse from other reporting paths.

pub struct AxisDivergence {
    pub axis: String,
    pub exec_score: f32,
    pub prompt_score: f32,
    pub divergence: f32,
    pub flagged: bool, // true if |diff| > 0.20
}

pub fn divergence(prompt: f32, exec: f32) -> AxisDivergence {
    AxisDivergence {
        axis: String::new(),
        exec_score: exec,
        prompt_score: prompt,
        divergence: (prompt - exec).abs(),
        flagged: (prompt - exec).abs() > 0.20,
    }
}
