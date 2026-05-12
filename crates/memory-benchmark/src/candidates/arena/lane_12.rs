use crate::candidates::shared::{CandidateAdapter, CandidatePolicy};
use crate::RecallResult;
use crate::Warning;

use super::{make_evidence_ledger, scale_recall};

#[derive(Default)]
pub struct Policy;

impl CandidatePolicy for Policy {
    type Inner = crate::adapters::reference_evidence_ledger::Adapter;

    fn name() -> &'static str {
        "arena_lane_12"
    }

    fn make_inner() -> Self::Inner {
        make_evidence_ledger()
    }

    fn adjust_recall(mut result: RecallResult) -> RecallResult {
        if result.answer.len() > 96 {
            result.answer.truncate(96);
            result.warnings.push(Warning::CompressionDrift);
        }
        result.context_token_count = result.answer.len() as u32 / 4;
        scale_recall(result, 0.94)
    }
}

pub type Adapter = CandidateAdapter<Policy>;
