use crate::candidates::shared::{CandidateAdapter, CandidatePolicy};
use crate::RecallResult;

use super::{make_context_pack, scale_recall};

#[derive(Default)]
pub struct Policy;

impl CandidatePolicy for Policy {
    type Inner = crate::adapters::reference_context_pack::Adapter;

    fn name() -> &'static str {
        "arena_lane_00"
    }

    fn make_inner() -> Self::Inner {
        make_context_pack(0.00)
    }

    fn adjust_recall(result: RecallResult) -> RecallResult {
        scale_recall(result, 1.00)
    }
}

pub type Adapter = CandidateAdapter<Policy>;
