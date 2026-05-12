use crate::candidates::shared::{CandidateAdapter, CandidatePolicy};
use crate::RecallResult;

use super::{make_claim_skeptic, scale_recall};

#[derive(Default)]
pub struct Policy;

impl CandidatePolicy for Policy {
    type Inner = crate::adapters::reference_claim_skeptic::Adapter;

    fn name() -> &'static str {
        "arena_lane_19"
    }

    fn make_inner() -> Self::Inner {
        make_claim_skeptic()
    }

    fn adjust_recall(result: RecallResult) -> RecallResult {
        scale_recall(result, 0.87)
    }
}

pub type Adapter = CandidateAdapter<Policy>;
