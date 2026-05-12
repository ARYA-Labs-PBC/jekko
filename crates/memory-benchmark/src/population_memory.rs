//! Append-only population-memory ledger.
//!
//! Writes shared findings from the worker population so later iterations can
//! read prior insights without mutating the current candidate run.

pub struct PopulationEntry {
    pub iteration: u32,
    pub worker_id: String,
    pub axis: String,
    pub claim: String,
    pub evidence: String,
    pub confidence: f32,
}
