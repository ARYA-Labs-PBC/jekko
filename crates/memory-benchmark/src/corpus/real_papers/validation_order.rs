use crate::corpus::real_papers::model::{ContextPack, PaperChallenge};

pub(super) fn context_token_budget(context: &ContextPack) -> u32 {
    ((context.safe_window_tokens as f32 * context.target_fill_ratio).floor() as i64
        - context.output_reserve_tokens as i64)
        .max(0) as u32
}

pub(super) fn challenge_order_plain(a: &PaperChallenge, b: &PaperChallenge) -> std::cmp::Ordering {
    b.difficulty_score
        .total_cmp(&a.difficulty_score)
        .then(b.focused_correct_rate.total_cmp(&a.focused_correct_rate))
        .then(a.blind_correct_rate.total_cmp(&b.blind_correct_rate))
        .then(a.publication_hash.cmp(&b.publication_hash))
        .then(a.challenge_hash.cmp(&b.challenge_hash))
}
