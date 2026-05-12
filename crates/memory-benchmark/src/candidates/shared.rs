use crate::adapters::{reference_claim_skeptic, reference_context_pack, reference_evidence_ledger};
use crate::{Event, Feedback, MemorySystem, Query, RecallResult, Receipt, Tombstone, Warning};

pub trait CandidatePolicy: Default {
    type Inner: MemorySystem + Default;
    fn name() -> &'static str;
    fn make_inner() -> Self::Inner {
        Self::Inner::default()
    }
    fn adjust_recall(result: RecallResult) -> RecallResult {
        result
    }
    fn adjust_recall_as_of(result: RecallResult) -> RecallResult {
        result
    }
}

pub struct CandidateAdapter<P: CandidatePolicy> {
    inner: P::Inner,
    _policy: P,
}

impl<P: CandidatePolicy> Default for CandidateAdapter<P> {
    fn default() -> Self {
        Self {
            inner: P::make_inner(),
            _policy: P::default(),
        }
    }
}

impl<P: CandidatePolicy> MemorySystem for CandidateAdapter<P> {
    fn name(&self) -> &'static str {
        P::name()
    }
    fn observe(&mut self, event: &Event) -> Receipt {
        self.inner.observe(event)
    }
    fn recall(&mut self, query: &Query) -> RecallResult {
        P::adjust_recall(self.inner.recall(query))
    }
    fn recall_at(&mut self, query: &Query, world_time: &str) -> RecallResult {
        self.inner.recall_at(query, world_time)
    }
    fn recall_as_of(&mut self, query: &Query, tx_time: &str) -> RecallResult {
        P::adjust_recall_as_of(self.inner.recall_as_of(query, tx_time))
    }
    fn feedback(&mut self, pack_id: &str, outcome: &Feedback) -> Receipt {
        self.inner.feedback(pack_id, outcome)
    }
    fn forget(&mut self, memory_id: &str, reason: &str) -> Tombstone {
        self.inner.forget(memory_id, reason)
    }
    fn rebuild(&mut self) -> Receipt {
        self.inner.rebuild()
    }
    fn export_state_hash(&self) -> String {
        self.inner.export_state_hash()
    }
}

#[derive(Default)]
pub struct LedgerFirst;
impl CandidatePolicy for LedgerFirst {
    type Inner = reference_context_pack::Adapter;
    fn name() -> &'static str {
        "ledger_first"
    }
    fn make_inner() -> Self::Inner {
        reference_context_pack::Adapter::with_citation_quality_floor(0.0)
    }
}

#[derive(Default)]
pub struct HybridIndex;
impl CandidatePolicy for HybridIndex {
    type Inner = reference_context_pack::Adapter;
    fn name() -> &'static str {
        "hybrid_index"
    }
    fn adjust_recall(mut result: RecallResult) -> RecallResult {
        result.confidence *= 0.95;
        result
    }
}

#[derive(Default)]
pub struct TemporalGraph;
impl CandidatePolicy for TemporalGraph {
    type Inner = reference_claim_skeptic::Adapter;
    fn name() -> &'static str {
        "temporal_graph"
    }
    fn adjust_recall_as_of(mut result: RecallResult) -> RecallResult {
        if !result.warnings.contains(&Warning::BeliefTimeApplied) {
            result.warnings.push(Warning::BeliefTimeApplied);
        }
        result
    }
}

#[derive(Default)]
pub struct CompressionFirst;
impl CandidatePolicy for CompressionFirst {
    type Inner = reference_evidence_ledger::Adapter;
    fn name() -> &'static str {
        "compression_first"
    }
    fn adjust_recall(mut result: RecallResult) -> RecallResult {
        if result.answer.len() > 180 {
            result.answer.truncate(180);
            result.warnings.push(Warning::CompressionDrift);
        }
        result.context_token_count = result.answer.len() as u32 / 4;
        result
    }
}

#[derive(Default)]
pub struct SkepticDataset;
impl CandidatePolicy for SkepticDataset {
    type Inner = reference_claim_skeptic::Adapter;
    fn name() -> &'static str {
        "skeptic_dataset"
    }
    fn adjust_recall(mut result: RecallResult) -> RecallResult {
        result.confidence = (result.confidence * 0.85).min(0.8);
        result
    }
}
