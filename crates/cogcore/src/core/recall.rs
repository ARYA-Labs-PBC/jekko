//! `Core::recall` and its bitemporal variants. Holds the scoring fusion,
//! render-pack assembly, and the standalone helpers that exist solely to
//! support the recall pipeline (`render_event`, `has_supersession_partner`,
//! `is_counterexample`, `detects_unit_mismatch`).

use super::{Core, Intent, RecallData, RecallQuery};
use crate::config::{
    SCORE_EQUATION_BOOST, SCORE_EXACT_ID_BOOST, SCORE_SUBJECT_BOOST, SCORE_THEOREM_BOOST,
    SCORE_TOPIC_BOOST,
};
use crate::core::recall_render::{has_supersession_partner, render_recall_data};
use crate::index::{tokenize, TokenId};
use crate::ledger::WalOp;
use crate::time::{iso_lt, BENCH_NOW};

impl Core {
    pub fn recall(&mut self, q: &RecallQuery) -> RecallData {
        self.run_recall(q, None, None, true)
    }

    pub fn recall_at(&mut self, q: &RecallQuery, world_time: &str) -> RecallData {
        self.run_recall(q, Some(world_time), None, false)
    }

    pub fn recall_as_of(&mut self, q: &RecallQuery, tx_time: &str) -> RecallData {
        self.run_recall(q, None, Some(tx_time), false)
    }

    fn run_recall(
        &mut self,
        q: &RecallQuery,
        world_t: Option<&str>,
        tx_t: Option<&str>,
        mutate: bool,
    ) -> RecallData {
        // 1. Build query token list (use existing intern table; do not learn
        //    new tokens on the read side — that would alter projection hashes).
        let mut q_tokens: Vec<TokenId> = Vec::new();
        for raw in tokenize(&q.text) {
            if let Some(id) = self.interner.lookup(&raw) {
                q_tokens.push(id);
            }
        }
        for m in &q.mentions {
            for raw in tokenize(m) {
                if let Some(id) = self.interner.lookup(&raw) {
                    q_tokens.push(id);
                }
            }
        }
        q_tokens.sort();
        q_tokens.dedup();

        // 2. Candidate pool: BM25 hits unioned with the substring sweep used
        //    when the inverted index hasn't observed any query tokens yet.
        let mut candidates: std::collections::BTreeSet<u32> = self
            .index
            .candidate_cells(&q_tokens, 256)
            .into_iter()
            .collect();
        if let Some(idx) = self.exact_id_index.get(&q.text).copied() {
            candidates.insert(idx);
        }
        for mention in &q.mentions {
            if let Some(indices) = self.subject_index.get(&mention.to_ascii_lowercase()) {
                for idx in indices {
                    candidates.insert(*idx);
                }
            }
        }
        if matches!(q.intent, Intent::Equation) {
            for mention in &q.mentions {
                if let Some(indices) = self.equation_lane.get(&mention.to_ascii_lowercase()) {
                    for idx in indices {
                        candidates.insert(*idx);
                    }
                }
            }
        }
        if matches!(q.intent, Intent::Theorem) {
            for mention in &q.mentions {
                if let Some(indices) = self.theorem_lane.get(&mention.to_ascii_lowercase()) {
                    for idx in indices {
                        candidates.insert(*idx);
                    }
                }
            }
        }
        if candidates.is_empty() {
            let q_lower = q.text.to_lowercase();
            for (i, cell) in self.cells.iter().enumerate() {
                if !q_lower.is_empty()
                    && (cell.event.subject.to_lowercase().contains(&q_lower)
                        || cell.event.body.to_lowercase().contains(&q_lower))
                {
                    candidates.insert(i as u32);
                }
                for m in &q.mentions {
                    let ml = m.to_lowercase();
                    if cell.event.subject.to_lowercase().contains(&ml)
                        || cell.event.body.to_lowercase().contains(&ml)
                    {
                        candidates.insert(i as u32);
                    }
                }
            }
        }

        // 3. Score each candidate.
        let mut scored: Vec<(u32, f32)> = Vec::with_capacity(candidates.len());
        let cand_vec: Vec<u32> = candidates.iter().copied().collect();
        for cell_idx in &cand_vec {
            let cell = match self.cells.get(*cell_idx as usize) {
                Some(c) => c,
                None => continue,
            };
            if self.tombstones.contains_key(&cell.event.id) {
                continue;
            }
            // Bitemporal filtering
            if let Some(t) = tx_t {
                if iso_lt(t, &cell.event.tx_time) {
                    continue;
                }
            }
            if let Some(w) = world_t {
                if let Some(vf) = cell.event.valid_from.as_deref() {
                    if iso_lt(w, vf) {
                        continue;
                    }
                }
                if let Some(vt) = cell.event.valid_to.as_deref() {
                    if !iso_lt(w, vt) {
                        continue;
                    }
                }
            }
            let bm = if q_tokens.is_empty() {
                0.0
            } else {
                self.index.bm25(&q_tokens, *cell_idx)
            };
            let subj_lower = cell.event.subject.to_lowercase();
            let q_lower = q.text.to_lowercase();
            let subj_match = if !q_lower.is_empty() && subj_lower.contains(&q_lower) {
                1.0
            } else {
                0.0
            };
            let mention_match = q
                .mentions
                .iter()
                .any(|m| subj_lower.contains(&m.to_lowercase()));
            let mut score = 1.0 * bm
                + SCORE_SUBJECT_BOOST * subj_match
                + 0.4 * (mention_match as i32 as f32)
                + 0.5 * cell.strength
                + 0.4 * cell.utility;
            let src_q = cell
                .event
                .sources
                .iter()
                .map(|s| s.quality)
                .fold(0.0_f32, f32::max);
            score += 0.3 * src_q;
            if self
                .exact_id_index
                .get(&q.text)
                .is_some_and(|idx| *idx == *cell_idx)
            {
                score += SCORE_EXACT_ID_BOOST;
            }
            let subject_key = cell.event.subject.to_ascii_lowercase();
            if self.topic_lookup.contains_key(&subject_key) {
                score += SCORE_TOPIC_BOOST;
            }
            if matches!(q.intent, Intent::Equation) && cell.event.kind == "Equation" {
                score += SCORE_EQUATION_BOOST;
            }
            if matches!(q.intent, Intent::Theorem) && cell.event.kind == "Theorem" {
                score += SCORE_THEOREM_BOOST;
            }
            if has_supersession_partner(self, cell) {
                score -= 0.4;
            }
            scored.push((*cell_idx, score));
        }
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });

        // 4. Graph rerank: top-32 boost via Hebbian neighbors.
        let top_pool: Vec<u32> = scored.iter().take(32).map(|(c, _)| *c).collect();
        for (cell_idx, s) in scored.iter_mut().take(32) {
            *s += 0.15 * self.hebb.boost_against(*cell_idx, &top_pool);
        }
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });

        let (out, used_ids) = render_recall_data(self, q, &q_tokens, &scored, world_t, tx_t);
        if mutate && !used_ids.is_empty() {
            self.wal.append(WalOp::RecallTouch {
                used_ids: used_ids.clone(),
                tx_time: BENCH_NOW.to_string(),
            });
            self.apply_recall_touch(&used_ids, BENCH_NOW);
        }
        out
    }
}
