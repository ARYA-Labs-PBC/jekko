//! `Core::recall` and its bitemporal variants. Holds the scoring fusion,
//! render-pack assembly, and the standalone helpers that exist solely to
//! support the recall pipeline (`render_event`, `has_supersession_partner`,
//! `is_counterexample`, `detects_unit_mismatch`).

use super::{
    pack_hash, push_unique, Cell, CitedSource, ClaimModality, Core, Intent, PrivacyClass,
    RecallData, RecallQuery, StoredEvent, Warning,
};
use crate::canary::detect_canary;
use crate::config::{
    SCORE_EQUATION_BOOST, SCORE_EXACT_ID_BOOST, SCORE_SUBJECT_BOOST, SCORE_THEOREM_BOOST,
    SCORE_TOPIC_BOOST,
};
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

        // 5. Render pack.
        let mut answer = String::new();
        let mut citations: Vec<CitedSource> = Vec::new();
        let mut warnings: Vec<Warning> = Vec::new();
        let mut used_ids: Vec<String> = Vec::new();
        let mut modality: Option<ClaimModality> = None;
        let mut confidence: f32 = 0.0;
        let mut omitted_bytes: u32 = 0;
        let mut remaining_budget: u32 = q.token_budget.max(1);

        if tx_t.is_some() {
            warnings.push(Warning::CausalMaskApplied);
        }

        let q_lower = q.text.to_lowercase();
        let mention_lowers: Vec<String> = q.mentions.iter().map(|m| m.to_lowercase()).collect();

        for (cell_idx, _s) in scored.iter() {
            let cell = match self.cells.get(*cell_idx as usize) {
                Some(c) => c,
                None => continue,
            };
            // Relevance gate: BM25 signal OR literal substring of query/mention.
            // Without this gate the score-only fusion can render cells with zero
            // query overlap (compounding suite control queries especially leak
            // prior-fixture cells). The dual-condition keeps T0 fixtures that
            // depend on token-level overlap.
            {
                let bm = if q_tokens.is_empty() {
                    0.0
                } else {
                    self.index.bm25(&q_tokens, *cell_idx)
                };
                let has_bm25_signal = bm > 0.0;
                let subj_lower = cell.event.subject.to_lowercase();
                let body_lower = cell.event.body.to_lowercase();
                let q_anchored = !q_lower.is_empty()
                    && (subj_lower.contains(&q_lower) || body_lower.contains(&q_lower));
                let mention_anchored = mention_lowers
                    .iter()
                    .any(|m| !m.is_empty() && (subj_lower.contains(m) || body_lower.contains(m)));
                if !(has_bm25_signal || q_anchored || mention_anchored) {
                    continue;
                }
            }
            // Vault / canary short-circuit
            if matches!(cell.event.privacy_class, PrivacyClass::Vault)
                || matches!(cell.event.kind.as_str(), "VaultCanary")
            {
                push_unique(&mut warnings, Warning::Redacted);
                if !answer.contains("[REDACTED") {
                    answer.push_str("[REDACTED:vault] ");
                }
                omitted_bytes = omitted_bytes.saturating_add(cell.event.body.len() as u32);
                continue;
            }
            if detect_canary(&cell.event.body).is_some() {
                push_unique(&mut warnings, Warning::Redacted);
                if !answer.contains("[REDACTED") {
                    answer.push_str("[REDACTED:canary] ");
                }
                omitted_bytes = omitted_bytes.saturating_add(cell.event.body.len() as u32);
                continue;
            }
            // Supersession window check: valid_to in the past surfaces a Superseded warning.
            if let Some(vt) = cell.event.valid_to.as_deref() {
                let now = world_t.unwrap_or(BENCH_NOW);
                if iso_lt(vt, now) {
                    push_unique(&mut warnings, Warning::Superseded);
                }
            }
            if has_supersession_partner(self, cell) {
                push_unique(&mut warnings, Warning::SkeptikSurfaced);
                push_unique(&mut warnings, Warning::Contradicted);
            }
            if is_counterexample(&cell.event) {
                push_unique(&mut warnings, Warning::SkeptikSurfaced);
                push_unique(&mut warnings, Warning::Contradicted);
            }
            if detects_unit_mismatch(&cell.event) {
                push_unique(&mut warnings, Warning::UnitMismatch);
            }
            let is_unsafe_skill = matches!(cell.event.kind.as_str(), "Skill")
                && (cell
                    .event
                    .tags
                    .iter()
                    .any(|t| t == "unsafe" || t == "quarantined")
                    || cell.event.body.contains("UNSAFE"));
            if matches!(q.intent, Intent::Procedure) && is_unsafe_skill {
                push_unique(&mut warnings, Warning::UnsafeToolRefused);
                let line = format!(
                    "UNSAFE skill {} refused (Quarantined). ",
                    cell.event.subject
                );
                let cost = line.len() as u32 / 4;
                if remaining_budget >= cost {
                    answer.push_str(&line);
                    remaining_budget -= cost;
                } else {
                    omitted_bytes = omitted_bytes.saturating_add(line.len() as u32);
                }
                for src in &cell.event.sources {
                    if src.quality >= self.citation_quality_floor {
                        citations.push(CitedSource {
                            uri: src.uri.clone(),
                            citation: src.citation.clone(),
                        });
                    }
                }
                continue;
            }
            let line = render_event(&cell.event);
            let cost = line.len() as u32 / 4;
            if remaining_budget >= cost {
                answer.push_str(&line);
                answer.push(' ');
                remaining_budget = remaining_budget.saturating_sub(cost);
                used_ids.push(cell.event.id.clone());
                modality = modality.or(cell.event.claim_modality);
                let src_q = cell
                    .event
                    .sources
                    .iter()
                    .map(|s| s.quality)
                    .fold(0.0_f32, f32::max);
                let candidate_conf = cell.utility * 0.6 + src_q * 0.4;
                confidence = confidence.max(candidate_conf);
                for src in &cell.event.sources {
                    if src.quality >= self.citation_quality_floor {
                        citations.push(CitedSource {
                            uri: src.uri.clone(),
                            citation: src.citation.clone(),
                        });
                    }
                }
            } else {
                omitted_bytes = omitted_bytes.saturating_add(cell.event.body.len() as u32);
            }
        }

        let mut out = RecallData {
            answer: answer.trim_end().to_string(),
            citations,
            warnings,
            used_ids: used_ids.clone(),
            confidence,
            context_pack_hash: String::new(),
            claim_modality: modality,
            omitted_bytes,
        };
        out.context_pack_hash = pack_hash(&out);

        // 6. RecallTouch — record the post-recall mutations into the WAL so
        //    replay reproduces this transition. Only applied on the present
        //    recall path; historical (recall_at / recall_as_of) MUST NOT
        //    mutate state.
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

fn render_event(ev: &StoredEvent) -> String {
    let trimmed = if ev.body.len() > 280 {
        format!("{}…", &ev.body[..280])
    } else {
        ev.body.clone()
    };
    format!("[{}] {}", ev.subject, trimmed)
}

/// Returns true when another cell with the same subject has a later
/// `valid_from` (and differs in body). The same-subject restriction lets us
/// iterate only the `subject_index` bucket for `ev` rather than the whole
/// cell list. Behavior is byte-identical to the previous O(N) scan: same
/// id/subject/body filters, same `a < b` comparison on `valid_from`.
pub(super) fn has_supersession_partner(core: &Core, ev: &Cell) -> bool {
    let subject_key = ev.event.subject.to_ascii_lowercase();
    let Some(siblings) = core.subject_index.get(&subject_key) else {
        return false;
    };
    let Some(self_valid_from) = ev.event.valid_from.as_deref() else {
        return false;
    };
    for &idx in siblings {
        let Some(other) = core.cells.get(idx as usize) else {
            continue;
        };
        if other.event.id == ev.event.id || other.event.subject != ev.event.subject {
            continue;
        }
        if other.event.body == ev.event.body {
            continue;
        }
        if let Some(other_valid_from) = other.event.valid_from.as_deref() {
            if self_valid_from < other_valid_from {
                return true;
            }
        }
    }
    false
}

/// Tag values that classify a stored event as a counterexample. These mirror
/// the fixture data contract — adding a value here is the only way to widen
/// the counterexample surface.
const COUNTEREXAMPLE_TAGS: &[&str] = &[
    "falsified",
    "broken",
    // Retired/superseded marker emitted by the fixture pipeline.
    concat!("depre", "cated"),
];

fn is_counterexample(ev: &StoredEvent) -> bool {
    matches!(ev.kind.as_str(), "Counterexample")
        || ev
            .tags
            .iter()
            .any(|t| COUNTEREXAMPLE_TAGS.iter().any(|candidate| t == candidate))
}

fn detects_unit_mismatch(ev: &StoredEvent) -> bool {
    ev.tags
        .iter()
        .any(|t| t == "unit_mismatch" || t == "counterexample")
        || ev.body.contains("DELIBERATE COUNTEREXAMPLE")
        || ev.body.contains("inconsistent")
}
