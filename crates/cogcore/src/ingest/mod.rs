//! Cogcore ingest pipeline — convert structured paper input into
//! `StoredEvent` streams that the `Core::observe` API consumes.
//!
//! Phase 1 ships a rule-based default (`RuleBackend`) that handles:
//! - Plain-text section bodies → Claim events
//! - Equations parsed from LaTeX-ish patterns → Equation events with SI units
//! - Theorem headers (Theorem / Lemma / Proposition / Corollary) → Theorem events
//!
//! Phase 7+ may wire a Jnoccio-mediated `LlmBackend` behind a ZYAL contract;
//! cogcore stays zero-dep here.

pub mod equation;
pub mod paper;
mod paper_json;
mod paper_json_parse;
mod paper_json_support;
mod paper_support;
pub mod theorem;

pub use equation::EqAtom;
pub use paper::{IngestedPaper, PaperSection, RuleBackend, SourceSpec};
pub use paper_json::parse_jsonl_event;
pub use theorem::TheoremRef;

use crate::core::StoredEvent;

/// Pluggable extractor. Production cogcore uses `RuleBackend`. Tests may
/// use a fixture backend. ZYAL-mediated LLM backends sit at this
/// layer in Phase 7+.
pub trait IngestBackend {
    /// Convert one paper into a stream of events. Implementations must be
    /// deterministic: same input → same output bytes.
    fn ingest_paper(&self, paper: &IngestedPaper) -> Vec<StoredEvent>;
}

#[cfg(test)]
mod mod_tests {
    use super::*;

    #[test]
    fn rule_backend_constructs() {
        let _ = RuleBackend;
    }
}
