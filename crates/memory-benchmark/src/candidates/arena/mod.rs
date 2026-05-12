use crate::adapters::{reference_claim_skeptic, reference_context_pack, reference_evidence_ledger};
use crate::RecallResult;

pub mod lane_00;
pub mod lane_01;
pub mod lane_02;
pub mod lane_03;
pub mod lane_04;
pub mod lane_05;
pub mod lane_06;
pub mod lane_07;
pub mod lane_08;
pub mod lane_09;
pub mod lane_10;
pub mod lane_11;
pub mod lane_12;
pub mod lane_13;
pub mod lane_14;
pub mod lane_15;
pub mod lane_16;
pub mod lane_17;
pub mod lane_18;
pub mod lane_19;

fn make_context_pack(floor: f32) -> reference_context_pack::Adapter {
    reference_context_pack::Adapter::with_citation_quality_floor(floor)
}

fn make_claim_skeptic() -> reference_claim_skeptic::Adapter {
    reference_claim_skeptic::Adapter::default()
}

fn make_evidence_ledger() -> reference_evidence_ledger::Adapter {
    reference_evidence_ledger::Adapter::default()
}

fn scale_recall(mut result: RecallResult, factor: f32) -> RecallResult {
    result.confidence = (result.confidence * factor).clamp(0.0, 1.0);
    result
}
