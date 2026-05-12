use crate::candidates::shared::{CandidateAdapter, CandidatePolicy};
use crate::RecallResult;

use super::{make_evidence_ledger, scale_recall};

#[derive(Default)]
pub struct Policy;

impl CandidatePolicy for Policy {
    type Inner = crate::adapters::reference_evidence_ledger::Adapter;

    fn name() -> &'static str {
        "arena_lane_11"
    }

    fn make_inner() -> Self::Inner {
        make_evidence_ledger()
    }

    fn adjust_recall(result: RecallResult) -> RecallResult {
        scale_recall(result, 0.95)
    }
}

pub type Adapter = CandidateAdapter<Policy>;
