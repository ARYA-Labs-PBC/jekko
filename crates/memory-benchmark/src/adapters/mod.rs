//! Reference implementations of `MemorySystem`.
//!
//! Three named reference adapters demonstrate the contract the benchmark
//! enforces. They emphasize different design facets so reviewers can see
//! per-axis tradeoffs in the comparison matrix:
//!
//! * `reference_context_pack` — bitemporal + ContextPack-style citation,
//!   causal-mask + privacy redaction. This is the shared core; the other
//!   two are diversification points off it.
//! * `reference_evidence_ledger` — ReviewState-style modality demotion;
//!   demotes inferred-source claims to `AssertedBySource`.
//! * `reference_claim_skeptic` — aggressive contradiction surfacing;
//!   emits `SkeptikSurfaced` on any superseded-or-contradicted warning.
//!
//! The `baseline` adapter is intentionally weak (calibration anchor).

pub mod baseline;
pub mod reference_claim_skeptic;
pub mod reference_context_pack;
pub mod reference_evidence_ledger;
