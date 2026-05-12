//! Adversarial curriculum-growth proposal emitter.
//!
//! When any candidate scores > 0.85 on an axis, this module proposes 3 harder
//! fixtures in that axis. Proposals are written to a JSON file but **not**
//! auto-applied — preserves run determinism.
//! The structure stays deliberately small so generated curricula can layer on
//! top without coupling proposal logic to fixture generation.

pub struct CurriculumProposal {
    pub axis: String,
    pub current_max_score: f32,
    pub suggested_fixture_summary: String,
    pub rationale: String,
}
