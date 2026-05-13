//! Lightweight concept and topic structures.
//!
//! Phase 2 keeps the data shapes minimal: a `Concept` is a cluster of
//! cells (by index) and a `Topic` is a set of concepts plus the strength
//! formula state. The promotion algorithm in `Core::consolidate` runs
//! offline; the hot recall path only reads these structures.

use crate::config::{CONCEPT_ATTACH_TAU, CONCEPT_FORM_TAU};
use crate::index::{jaccard_minhash, TokenId};

pub type ConceptId = u32;
pub type TopicId = u32;

#[derive(Clone)]
pub struct Concept {
    pub id: ConceptId,
    pub label: String,
    pub kernel_tokens: Vec<TokenId>,
    pub minhash: [u32; 8],
    pub member_cells: Vec<u32>,
}

#[derive(Clone)]
pub struct Topic {
    pub id: TopicId,
    pub label: String,
    pub concepts: Vec<ConceptId>,
    pub strength: f32,
    pub half_life_hours: f32,
    pub last_update_tx: String,
    pub contradiction_pressure: f32,
    pub stats: TopicStats,
}

#[derive(Clone, Default)]
pub struct TopicStats {
    pub recall_count: u32,
    pub success_count: u32,
    pub failure_count: u32,
    pub recent_observes: u32,
    pub distinct_subjects: u32,
    pub avg_source_quality: f32,
}

pub fn best_concept_match(sketch: &[u32; 8], concepts: &[Concept]) -> Option<(ConceptId, f32)> {
    let mut best: Option<(ConceptId, f32)> = None;
    for c in concepts {
        let j = jaccard_minhash(sketch, &c.minhash);
        if let Some((_, prev)) = best {
            if j > prev {
                best = Some((c.id, j));
            }
        } else if j > 0.0 {
            best = Some((c.id, j));
        }
    }
    best
}

pub fn attach_threshold() -> f32 {
    CONCEPT_ATTACH_TAU
}

pub fn formation_threshold() -> f32 {
    CONCEPT_FORM_TAU
}
