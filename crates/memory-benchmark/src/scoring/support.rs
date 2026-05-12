use std::collections::BTreeSet;

use crate::RecallResult;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SupportScore {
    pub required_recall: f32,
    pub irrelevant_penalty: f32,
    pub citation_bloat: u32,
}

pub fn score_support(
    out: &RecallResult,
    required_ids: &[String],
    allowed_ids: &[String],
    max_used_ids: usize,
    max_context_tokens: u32,
) -> SupportScore {
    let required: BTreeSet<&str> = required_ids.iter().map(String::as_str).collect();
    let allowed: BTreeSet<&str> = allowed_ids.iter().map(String::as_str).collect();
    let used: BTreeSet<&str> = out.used_ids.iter().map(String::as_str).collect();
    let hits = required.iter().filter(|id| used.contains(**id)).count();
    let irrelevant = used
        .iter()
        .filter(|id| !allowed.is_empty() && !allowed.contains(**id))
        .count();
    SupportScore {
        required_recall: if required.is_empty() {
            1.0
        } else {
            hits as f32 / required.len() as f32
        },
        irrelevant_penalty: irrelevant as f32 / used.len().max(1) as f32,
        citation_bloat: (out.used_ids.len() > max_used_ids
            || out.context_token_count > max_context_tokens) as u32,
    }
}
