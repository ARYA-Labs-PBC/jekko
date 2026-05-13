//! Deterministic patch templates for the AutoResearch chase loop.

const DEFAULT_ATTACH: f32 = 0.30;
const DEFAULT_FORM: f32 = 0.55;
const DEFAULT_RECENCY: f32 = 0.20;
const DEFAULT_RECURRENCE: f32 = 0.18;
const DEFAULT_UTILITY: f32 = 0.12;
const DEFAULT_NOVELTY: f32 = 0.08;
const DEFAULT_SOURCE_Q: f32 = 0.10;
const DEFAULT_RECALL_SUCCESS: f32 = 0.20;
const DEFAULT_CONTRADICTION: f32 = 0.30;
const DEFAULT_ETA_RECALL: f32 = 0.05;
const DEFAULT_ETA_SUCCESS: f32 = 0.15;
const DEFAULT_ETA_FALSIFY: f32 = 0.20;
const DEFAULT_ETA_FAILURE: f32 = 0.05;
const DEFAULT_ETA_IGNORE: f32 = 0.02;
const DEFAULT_PRUNE_BELOW: f32 = 0.02;

fn tune(base: f32, gauss: f32, min: f32, max: f32) -> f32 {
    let adjusted = base + gauss * 0.05;
    adjusted.clamp(min, max)
}

pub fn render_config_patch(worker_id: u32, cycle_id: &str, gauss: f32) -> String {
    let attach = tune(DEFAULT_ATTACH, gauss, 0.15, 0.45);
    let form = tune(DEFAULT_FORM, gauss * 0.5, 0.35, 0.75);
    let recency = tune(DEFAULT_RECENCY, gauss * 0.4, 0.10, 0.35);
    let recurrence = tune(DEFAULT_RECURRENCE, gauss * 0.35, 0.10, 0.30);
    let utility = tune(DEFAULT_UTILITY, gauss * 0.3, 0.05, 0.25);
    let novelty = tune(DEFAULT_NOVELTY, gauss * 0.2, 0.02, 0.18);
    let source_q = tune(DEFAULT_SOURCE_Q, gauss * 0.2, 0.05, 0.25);
    let recall_success = tune(DEFAULT_RECALL_SUCCESS, gauss * 0.35, 0.10, 0.35);
    let contradiction = tune(DEFAULT_CONTRADICTION, gauss * 0.25, 0.10, 0.50);
    let eta_recall = tune(DEFAULT_ETA_RECALL, gauss * 0.10, 0.01, 0.15);
    let eta_success = tune(DEFAULT_ETA_SUCCESS, gauss * 0.12, 0.05, 0.25);
    let eta_falsify = tune(DEFAULT_ETA_FALSIFY, gauss * 0.12, 0.05, 0.30);
    let eta_failure = tune(DEFAULT_ETA_FAILURE, gauss * 0.08, 0.02, 0.15);
    let eta_ignore = tune(DEFAULT_ETA_IGNORE, gauss * 0.05, 0.01, 0.08);
    let prune_below = tune(DEFAULT_PRUNE_BELOW, gauss * 0.03, 0.005, 0.08);

    format!(
        "//! AutoResearch worker patch for cycle {cycle_id}, worker {worker_id}.\n\
//! Generated deterministically from the GA proposal.\n\
\n\
pub const DEFAULT_CITATION_QUALITY_FLOOR: f32 = 0.85;\n\
\n\
pub const CONCEPT_ATTACH_TAU: f32 = {attach:.4};\n\
pub const CONCEPT_FORM_TAU: f32 = {form:.4};\n\
pub const CONCEPT_MIN_MEMBERS: usize = 3;\n\
pub const CONCEPT_KERNEL_LIMIT: usize = 15;\n\
pub const CONCEPT_CONFLICT_THRESHOLD: f32 = 0.35;\n\
\n\
pub const TOPIC_RECENCY_WEIGHT: f32 = {recency:.4};\n\
pub const TOPIC_RECURRENCE_WEIGHT: f32 = {recurrence:.4};\n\
pub const TOPIC_UTILITY_WEIGHT: f32 = {utility:.4};\n\
pub const TOPIC_NOVELTY_WEIGHT: f32 = {novelty:.4};\n\
pub const TOPIC_SOURCE_QUALITY_WEIGHT: f32 = {source_q:.4};\n\
pub const TOPIC_RECALL_SUCCESS_WEIGHT: f32 = {recall_success:.4};\n\
pub const TOPIC_CONTRADICTION_WEIGHT: f32 = {contradiction:.4};\n\
\n\
pub const HEBB_ETA_RECALL: f32 = {eta_recall:.4};\n\
pub const HEBB_ETA_SUCCESS: f32 = {eta_success:.4};\n\
pub const HEBB_ETA_FALSIFY: f32 = {eta_falsify:.4};\n\
pub const HEBB_ETA_FAILURE: f32 = {eta_failure:.4};\n\
pub const HEBB_ETA_IGNORE: f32 = {eta_ignore:.4};\n\
pub const HEBB_PRUNE_BELOW: f32 = {prune_below:.4};\n\
pub const HEBB_CAP_PAIRS: usize = 64;\n\
\n\
pub const SCORE_EXACT_ID_BOOST: f32 = 0.45;\n\
pub const SCORE_SUBJECT_BOOST: f32 = 0.60;\n\
pub const SCORE_TOPIC_BOOST: f32 = 0.20;\n\
pub const SCORE_EQUATION_BOOST: f32 = 0.20;\n\
pub const SCORE_THEOREM_BOOST: f32 = 0.20;\n\
pub const TOPIC_EMERGENCE_WEIGHT: f32 = 0.10;\n",
    )
}
