use crate::RecallResult;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Economics {
    pub context_token_count: u32,
    pub retrieved_token_count: u32,
    pub state_bytes: u64,
    pub efficiency_score: f32,
}

pub fn economics_for(out: &RecallResult, budget_tokens: u32) -> Economics {
    let used = out.context_token_count;
    let efficiency_score = if budget_tokens == 0 {
        0.0
    } else {
        1.0 - (used.min(budget_tokens) as f32 / budget_tokens as f32 * 0.5)
    };
    Economics {
        context_token_count: used,
        retrieved_token_count: out.retrieved_token_count,
        state_bytes: out.state_bytes,
        efficiency_score,
    }
}
