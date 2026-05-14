//! Deterministic tuning constants for `cogcore`.
//!
//! The chase loop mutates this file in future optimization rounds, so the
//! numbers stay centralized instead of being scattered through the engine.

pub const DEFAULT_CITATION_QUALITY_FLOOR: f32 = 0.85;

pub const CONCEPT_ATTACH_TAU: f32 = 0.30;
pub const CONCEPT_FORM_TAU: f32 = 0.55;
pub const CONCEPT_MIN_MEMBERS: usize = 3;
pub const CONCEPT_KERNEL_LIMIT: usize = 15;
pub const CONCEPT_CONFLICT_THRESHOLD: f32 = 0.35;

pub const TOPIC_RECENCY_WEIGHT: f32 = 0.20;
pub const TOPIC_RECURRENCE_WEIGHT: f32 = 0.18;
pub const TOPIC_UTILITY_WEIGHT: f32 = 0.12;
pub const TOPIC_NOVELTY_WEIGHT: f32 = 0.08;
pub const TOPIC_SOURCE_QUALITY_WEIGHT: f32 = 0.10;
pub const TOPIC_RECALL_SUCCESS_WEIGHT: f32 = 0.20;
pub const TOPIC_CONTRADICTION_WEIGHT: f32 = 0.30;

pub const HEBB_ETA_RECALL: f32 = 0.05;
pub const HEBB_ETA_SUCCESS: f32 = 0.15;
pub const HEBB_ETA_FALSIFY: f32 = 0.20;
pub const HEBB_ETA_FAILURE: f32 = 0.05;
pub const HEBB_ETA_IGNORE: f32 = 0.02;
pub const HEBB_PRUNE_BELOW: f32 = 0.02;
pub const HEBB_CAP_PAIRS: usize = 64;

pub const SCORE_EXACT_ID_BOOST: f32 = 0.45;
pub const SCORE_SUBJECT_BOOST: f32 = 0.60;
pub const SCORE_TOPIC_BOOST: f32 = 0.20;
pub const SCORE_EQUATION_BOOST: f32 = 0.20;
pub const SCORE_THEOREM_BOOST: f32 = 0.20;
pub const TOPIC_EMERGENCE_WEIGHT: f32 = 0.10;
