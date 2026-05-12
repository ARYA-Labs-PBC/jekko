//! Reference evidence-ledger adapter — durable memory lanes plus runtime overlays.
//!
//! Reuses reference_context_pack core logic but demotes inferred-source claims
//! to asserted-source claims for a different evidence policy.

use crate::{
    ClaimModality, Event, Feedback, MemorySystem, Query, RecallResult, Receipt, Tombstone, Warning,
};

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

impl MemorySystem for Adapter {
    fn name(&self) -> &'static str {
        "reference_evidence_ledger"
    }
    fn observe(&mut self, event: &Event) -> Receipt {
        self.inner.observe(event)
    }
    fn recall(&mut self, q: &Query) -> RecallResult {
        let mut r = self.inner.recall(q);
        if let Some(m) = r.claim_modality {
            r.claim_modality = match m {
                ClaimModality::Observed | ClaimModality::HumanApproved => Some(m),
                _ => Some(ClaimModality::AssertedBySource),
            };
        }
        r
    }
    fn recall_at(&mut self, q: &Query, world_time: &str) -> RecallResult {
        self.inner.recall_at(q, world_time)
    }
    fn recall_as_of(&mut self, q: &Query, tx_time: &str) -> RecallResult {
        let mut result = self.inner.recall_as_of(q, tx_time);
        result
            .warnings
            .retain(|warning| !matches!(warning, Warning::CausalMaskApplied | Warning::Superseded));
        result
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
        format!("refledger:{}", self.inner.export_state_hash())
    }
}
