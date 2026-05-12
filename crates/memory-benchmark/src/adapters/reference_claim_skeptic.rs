//! reference claim-skeptic reference adapter — 16-lane + ClaimModality + Skeptic.
//!
//! Reuses reference_context_pack core; emphasizes ClaimModality preservation (reference claim-skeptic
//! makes ClaimModality a first-class enum) and is more aggressive about
//! skeptic-surfaced warnings on any same-subject divergence (Skeptic Daemon).

use crate::{Event, Feedback, MemorySystem, Query, RecallResult, Receipt, Tombstone, Warning};

pub struct Adapter {
    inner: super::reference_context_pack::Adapter,
}

impl Default for Adapter {
    fn default() -> Self {
        Adapter {
            inner: super::reference_context_pack::Adapter::with_citation_quality_floor(1.0),
        }
    }
}

impl Adapter {
    fn skeptical_result(mut result: RecallResult) -> RecallResult {
        result.confidence = (result.confidence * 0.9).max(0.0);
        result
    }
}

impl MemorySystem for Adapter {
    fn name(&self) -> &'static str {
        "reference_claim_skeptic"
    }
    fn observe(&mut self, event: &Event) -> Receipt {
        self.inner.observe(event)
    }
    fn recall(&mut self, q: &Query) -> RecallResult {
        let mut r = self.inner.recall(q);
        // Skeptic Daemon — if any contradiction-bearing warning is present,
        // surface SkeptikSurfaced explicitly to the caller.
        if r.warnings.iter().any(|w| {
            matches!(
                w,
                Warning::Superseded | Warning::Contradicted | Warning::UnitMismatch
            )
        }) && !r.warnings.contains(&Warning::SkeptikSurfaced)
        {
            r.warnings.push(Warning::SkeptikSurfaced);
        }
        Self::skeptical_result(r)
    }
    fn recall_at(&mut self, q: &Query, world_time: &str) -> RecallResult {
        Self::skeptical_result(self.inner.recall_at(q, world_time))
    }
    fn recall_as_of(&mut self, q: &Query, tx_time: &str) -> RecallResult {
        Self::skeptical_result(self.inner.recall_as_of(q, tx_time))
    }
    fn feedback(&mut self, pack_id: &str, o: &Feedback) -> Receipt {
        self.inner.feedback(pack_id, o)
    }
    fn forget(&mut self, id: &str, reason: &str) -> Tombstone {
        self.inner.forget(id, reason)
    }
    fn rebuild(&mut self) -> Receipt {
        self.inner.rebuild()
    }
    fn export_state_hash(&self) -> String {
        format!("refskeptic:{}", self.inner.export_state_hash())
    }
}
