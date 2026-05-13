//! Cognitive memory core — Phase 2.
//!
//! Substantive memory engine: append-only WAL ledger, BM25-lite inverted
//! index, MinHash concept attachment, topic strength formula, Hebbian
//! co-activation matrix, FSRS-on-cells, and a `WalOp::RecallTouch` step
//! that records hot-path mutations so `rebuild()` is byte-identical.
//!
//! Public API surface unchanged from Phase 1 so the benchmark adapter
//! continues to work without modification.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use crate::canary::detect_canary;
use crate::concept::{
    attach_threshold, best_concept_match, formation_threshold, Concept, ConceptId, Topic, TopicId,
};
use crate::config::{
    CONCEPT_CONFLICT_THRESHOLD, CONCEPT_KERNEL_LIMIT, CONCEPT_MIN_MEMBERS,
    DEFAULT_CITATION_QUALITY_FLOOR, SCORE_EQUATION_BOOST, SCORE_EXACT_ID_BOOST,
    SCORE_SUBJECT_BOOST, SCORE_THEOREM_BOOST, SCORE_TOPIC_BOOST,
};
use crate::fsrs::{decay as fsrs_decay, hours_between, strengthen_cell};
use crate::hash::{fnv1a_hex, fnv1a_seq_hex};
use crate::hebb::Hebb;
use crate::index::{bigrams, minhash_sketch, tokenize, Interner, InvertedIndex, TokenId};
use crate::ledger::{Wal, WalOp};
use crate::time::{iso_lt, BENCH_NOW};
use crate::topic::recompute as topic_recompute;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyClass {
    Public,
    Internal,
    Confidential,
    Secret,
    Vault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimModality {
    Observed,
    AssertedBySource,
    InferredByAgent,
    HumanApproved,
    FormallyVerified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warning {
    Superseded,
    Contradicted,
    Redacted,
    CausalMaskApplied,
    SkeptikSurfaced,
    UnitMismatch,
    Abstained,
    UnsafeToolRefused,
}

#[derive(Debug, Clone)]
pub struct StoredEvent {
    pub id: String,
    pub kind: String,
    pub subject: String,
    pub body: String,
    pub tx_time: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub privacy_class: PrivacyClass,
    pub claim_modality: Option<ClaimModality>,
    pub tags: Vec<String>,
    pub sources: Vec<SourceRef>,
    pub supersedes: Vec<String>,
    pub contradicts: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SourceRef {
    pub uri: String,
    pub citation: String,
    pub quality: f32,
}

#[derive(Debug, Clone)]
pub struct Receipt {
    pub event_id: Option<String>,
    pub mutation_id: String,
    pub at: String,
    pub previous_hash: String,
    pub hash: String,
}

#[derive(Debug, Clone)]
pub struct Tombstone {
    pub memory_id: String,
    pub reason: String,
    pub deletion_proof: String,
    pub deleted_at: String,
}

#[derive(Debug, Clone)]
pub struct FeedbackSignal {
    pub outcome: Outcome,
    pub used: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    TaskSuccess,
    TaskFailure,
    Verified,
    Falsified,
    Ignored,
}

#[derive(Debug, Clone)]
pub struct CitedSource {
    pub uri: String,
    pub citation: String,
}

#[derive(Debug, Clone, Default)]
pub struct RecallData {
    pub answer: String,
    pub citations: Vec<CitedSource>,
    pub warnings: Vec<Warning>,
    pub used_ids: Vec<String>,
    pub confidence: f32,
    pub context_pack_hash: String,
    pub claim_modality: Option<ClaimModality>,
    pub omitted_bytes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Intent {
    #[default]
    Recall,
    Fact,
    Equation,
    Theorem,
    Citation,
    Coref,
    Procedure,
    Workflow,
    Contradiction,
    HistoryAt,
    HistoryAsOf,
    Forget,
    Mixed,
}

#[derive(Debug, Clone)]
pub struct RecallQuery {
    pub text: String,
    pub mentions: Vec<String>,
    pub intent: Intent,
    pub token_budget: u32,
}

/// Internal cell representation. The interned `tokens` and `bigrams`
/// support BM25 + MinHash without re-tokenizing on recall.
struct Cell {
    event: StoredEvent,
    tokens: Vec<TokenId>,
    sketch: [u32; 8],
    strength: f32,
    half_life_hours: f32,
    last_recall_tx: String,
    recall_count: u32,
    success_count: u32,
    failure_count: u32,
    utility: f32,
    concept_id: Option<ConceptId>,
}

pub struct Core {
    cells: Vec<Cell>,
    by_id: BTreeMap<String, u32>,
    exact_id_index: BTreeMap<String, u32>,
    subject_index: BTreeMap<String, Vec<u32>>,
    equation_lane: BTreeMap<String, Vec<u32>>,
    theorem_lane: BTreeMap<String, Vec<u32>>,
    tombstones: BTreeMap<String, Tombstone>,
    interner: Interner,
    index: InvertedIndex,
    hebb: Hebb,
    concepts: Vec<Concept>,
    topics: Vec<Topic>,
    topic_lookup: BTreeMap<String, TopicId>,
    receipt_seq: u64,
    last_receipt_hash: String,
    citation_quality_floor: f32,
    wal: Wal,
}

impl Default for Core {
    fn default() -> Self {
        Core {
            cells: Vec::new(),
            by_id: BTreeMap::new(),
            exact_id_index: BTreeMap::new(),
            subject_index: BTreeMap::new(),
            equation_lane: BTreeMap::new(),
            theorem_lane: BTreeMap::new(),
            tombstones: BTreeMap::new(),
            interner: Interner::default(),
            index: InvertedIndex::default(),
            hebb: Hebb::default(),
            concepts: Vec::new(),
            topics: Vec::new(),
            topic_lookup: BTreeMap::new(),
            receipt_seq: 0,
            last_receipt_hash: String::new(),
            citation_quality_floor: DEFAULT_CITATION_QUALITY_FLOOR,
            wal: Wal::default(),
        }
    }
}

impl Core {
    pub fn with_citation_quality_floor(citation_quality_floor: f32) -> Self {
        Core {
            citation_quality_floor,
            ..Core::default()
        }
    }

    pub fn canonical_event_id(kind: &str, subject: &str, body: &str, tx_time: &str) -> String {
        fnv1a_seq_hex(&[kind, subject, body, tx_time])
    }

    fn next_receipt(&mut self, event_id: Option<&str>, kind: &str) -> Receipt {
        self.receipt_seq += 1;
        let prev = self.last_receipt_hash.clone();
        let hash = fnv1a_hex(&format!("{}:{}:{}", prev, self.receipt_seq, kind));
        self.last_receipt_hash = hash.clone();
        Receipt {
            event_id: event_id.map(|s| s.to_string()),
            mutation_id: format!("cogcore-{:08}", self.receipt_seq),
            at: BENCH_NOW.to_string(),
            previous_hash: prev,
            hash,
        }
    }

    pub fn observe(&mut self, mut ev: StoredEvent) -> Receipt {
        if ev.id.is_empty() {
            ev.id = Self::canonical_event_id(&ev.kind, &ev.subject, &ev.body, &ev.tx_time);
        }
        let id = ev.id.clone();
        if self.by_id.contains_key(&id) {
            // duplicate id: ignore but still emit a receipt for chain stability
            return self.next_receipt(Some(&id), "observe-dup");
        }

        // Tokenize subject + body together so subject terms also enter BM25.
        let mut tokens: Vec<TokenId> = Vec::new();
        for raw in tokenize(&ev.subject) {
            tokens.push(self.interner.intern(&raw));
        }
        for raw in tokenize(&ev.body) {
            tokens.push(self.interner.intern(&raw));
        }
        let sketch = minhash_sketch(&bigrams(&tokens));
        let cell_idx = self.index.add(&tokens);

        let concept_match = best_concept_match(&sketch, &self.concepts);
        let concept_id = concept_match.and_then(|(id, j)| {
            if j >= attach_threshold() {
                Some(id)
            } else {
                None
            }
        });
        if let Some(cid) = concept_id {
            if let Some(c) = self.concepts.iter_mut().find(|c| c.id == cid) {
                if !c.member_cells.contains(&cell_idx) {
                    c.member_cells.push(cell_idx);
                }
            }
        }

        let src_q = ev.sources.iter().map(|s| s.quality).fold(0.0_f32, f32::max);
        let modality_byte = ev.claim_modality.map(|m| modality_byte(m));
        let privacy_byte = privacy_byte(ev.privacy_class);
        self.wal.append(WalOp::Observe {
            event_id: id.clone(),
            kind: ev.kind.clone(),
            subject: ev.subject.clone(),
            body: ev.body.clone(),
            tx_time: ev.tx_time.clone(),
            valid_from: ev.valid_from.clone(),
            valid_to: ev.valid_to.clone(),
            privacy_class: privacy_byte,
            claim_modality: modality_byte,
            tags: ev.tags.clone(),
            sources: ev
                .sources
                .iter()
                .map(|s| (s.uri.clone(), s.citation.clone(), s.quality))
                .collect(),
            supersedes: ev.supersedes.clone(),
            contradicts: ev.contradicts.clone(),
        });
        let cell = Cell {
            event: ev,
            tokens,
            sketch,
            strength: 0.3 + 0.3 * src_q,
            half_life_hours: 24.0,
            last_recall_tx: BENCH_NOW.to_string(),
            recall_count: 0,
            success_count: 0,
            failure_count: 0,
            utility: 0.5,
            concept_id,
        };
        self.cells.push(cell);
        self.by_id.insert(id.clone(), cell_idx);
        self.exact_id_index.insert(id.clone(), cell_idx);
        let subject_key = self
            .cells
            .get(cell_idx as usize)
            .map(|cell| cell.event.subject.to_ascii_lowercase())
            .unwrap_or_default();
        self.subject_index
            .entry(subject_key.clone())
            .or_default()
            .push(cell_idx);
        if let Some(kind) = self
            .cells
            .get(cell_idx as usize)
            .map(|cell| cell.event.kind.as_str())
        {
            match kind {
                "Equation" => self
                    .equation_lane
                    .entry(subject_key)
                    .or_default()
                    .push(cell_idx),
                "Theorem" => self
                    .theorem_lane
                    .entry(subject_key)
                    .or_default()
                    .push(cell_idx),
                _ => {}
            }
        }
        self.next_receipt(Some(&id), "observe")
    }

    pub fn feedback(&mut self, signal: &FeedbackSignal) -> Receipt {
        let (delta, hebb_kind) = match signal.outcome {
            Outcome::TaskSuccess | Outcome::Verified => (0.20_f32, 1u8),
            Outcome::TaskFailure => (-0.10_f32, 2u8),
            Outcome::Falsified => (-0.30_f32, 3u8),
            Outcome::Ignored => (-0.05_f32, 4u8),
        };
        let mut indices: Vec<u32> = Vec::new();
        for id in &signal.used {
            if let Some(idx) = self.by_id.get(id).copied() {
                if let Some(cell) = self.cells.get_mut(idx as usize) {
                    cell.utility = (cell.utility + delta).clamp(0.0, 1.0);
                    if delta > 0.0 {
                        cell.success_count = cell.success_count.saturating_add(1);
                    } else if matches!(signal.outcome, Outcome::TaskFailure | Outcome::Falsified) {
                        cell.failure_count = cell.failure_count.saturating_add(1);
                    }
                }
                indices.push(idx);
            }
        }
        match hebb_kind {
            1 => self.hebb.update_success(&indices),
            2 => self.hebb.update_failure(&indices),
            3 => self.hebb.update_falsify(&indices),
            _ => self.hebb.update_ignore(&indices),
        }
        self.wal.append(WalOp::Feedback {
            outcome: outcome_byte(signal.outcome),
            used: signal.used.clone(),
        });
        self.next_receipt(None, "feedback")
    }

    pub fn forget(&mut self, memory_id: &str, reason: &str) -> Tombstone {
        let t = Tombstone {
            memory_id: memory_id.to_string(),
            reason: reason.to_string(),
            deletion_proof: fnv1a_hex(&format!("{}|{}|{}", memory_id, reason, BENCH_NOW)),
            deleted_at: BENCH_NOW.to_string(),
        };
        self.tombstones.insert(memory_id.to_string(), t.clone());
        self.wal.append(WalOp::Tombstone {
            event_id: memory_id.to_string(),
            reason: reason.to_string(),
        });
        t
    }

    /// Replay the WAL into a fresh in-memory state. Byte-identical to live
    /// state if no clock/random has been touched on the hot path.
    pub fn rebuild(&mut self) -> Receipt {
        // Snapshot WAL ops, then rebuild from scratch.
        let snapshot: Vec<WalOp> = self.wal.entries().iter().map(|e| e.op.clone()).collect();
        let old_seq = self.receipt_seq;
        let old_last = self.last_receipt_hash.clone();
        let old_floor = self.citation_quality_floor;
        *self = Core {
            citation_quality_floor: old_floor,
            ..Core::default()
        };
        for op in snapshot {
            match op {
                WalOp::Observe {
                    event_id,
                    kind,
                    subject,
                    body,
                    tx_time,
                    valid_from,
                    valid_to,
                    privacy_class,
                    claim_modality,
                    tags,
                    sources,
                    supersedes,
                    contradicts,
                } => {
                    let ev = StoredEvent {
                        id: event_id,
                        kind,
                        subject,
                        body,
                        tx_time,
                        valid_from,
                        valid_to,
                        privacy_class: privacy_from_byte(privacy_class),
                        claim_modality: claim_modality.map(modality_from_byte),
                        tags,
                        sources: sources
                            .into_iter()
                            .map(|(uri, citation, quality)| SourceRef {
                                uri,
                                citation,
                                quality,
                            })
                            .collect(),
                        supersedes,
                        contradicts,
                    };
                    let _ = self.observe(ev);
                }
                WalOp::Tombstone { event_id, reason } => {
                    let _ = self.forget(&event_id, &reason);
                }
                WalOp::Feedback { outcome, used } => {
                    let sig = FeedbackSignal {
                        outcome: outcome_from_byte(outcome),
                        used,
                    };
                    let _ = self.feedback(&sig);
                }
                WalOp::RecallTouch { used_ids, tx_time } => {
                    self.apply_recall_touch(&used_ids, &tx_time);
                }
            }
        }
        self.receipt_seq = old_seq + 1;
        let prev = old_last;
        let hash = fnv1a_hex(&format!("{}:{}:{}", prev, self.receipt_seq, "rebuild"));
        self.last_receipt_hash = hash.clone();
        Receipt {
            event_id: None,
            mutation_id: format!("cogcore-{:08}", self.receipt_seq),
            at: BENCH_NOW.to_string(),
            previous_hash: prev,
            hash,
        }
    }

    /// Apply the deterministic mutations recorded by a `RecallTouch` op.
    /// Called both on the hot path (after a successful recall) and during
    /// `rebuild()` so live state and replayed state converge.
    fn apply_recall_touch(&mut self, used_ids: &[String], tx_time: &str) {
        let mut indices: Vec<u32> = Vec::new();
        for id in used_ids {
            if let Some(idx) = self.by_id.get(id).copied() {
                indices.push(idx);
                if let Some(cell) = self.cells.get_mut(idx as usize) {
                    cell.recall_count = cell.recall_count.saturating_add(1);
                    let success_rate = cell.success_count as f32
                        / (cell.success_count + cell.failure_count + 1) as f32;
                    let dt_h = hours_between(&cell.last_recall_tx, tx_time);
                    let half = cell.half_life_hours.max(1.0);
                    let decayed = fsrs_decay(cell.strength, dt_h, half);
                    let src_q = cell
                        .event
                        .sources
                        .iter()
                        .map(|s| s.quality)
                        .fold(0.0_f32, f32::max);
                    cell.strength = strengthen_cell(decayed, success_rate, src_q, cell.utility);
                    cell.half_life_hours = crate::fsrs::cell_half_life_hours(
                        cell.strength,
                        success_rate,
                        cell.recall_count,
                    );
                    cell.last_recall_tx = tx_time.to_string();
                }
            }
        }
        self.hebb.update_recall(&indices);
    }

    /// Compute a state hash that is invariant under insertion order.
    pub fn export_state_hash(&self) -> String {
        let mut buf = String::new();
        let mut ids: Vec<&str> = self.by_id.keys().map(|s| s.as_str()).collect();
        ids.sort();
        for id in &ids {
            buf.push_str(id);
            buf.push('|');
        }
        let mut idx_pairs: Vec<(String, f32, f32, u32, u32, u32)> = self
            .cells
            .iter()
            .map(|c| {
                (
                    c.event.id.clone(),
                    c.utility,
                    c.strength,
                    c.recall_count,
                    c.success_count,
                    c.failure_count,
                )
            })
            .collect();
        idx_pairs.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, u, s, r, sc, fc) in idx_pairs {
            buf.push_str(&id);
            buf.push(':');
            buf.push_str(&format!("u={u:.4},s={s:.4},r={r},sc={sc},fc={fc};"));
        }
        for (id, idx) in &self.exact_id_index {
            buf.push_str(&format!("E:{id}={idx};"));
        }
        for (subject, ids) in &self.subject_index {
            buf.push_str(&format!("S:{subject}={:?};", ids));
        }
        for (subject, ids) in &self.equation_lane {
            buf.push_str(&format!("Q:{subject}={:?};", ids));
        }
        for (subject, ids) in &self.theorem_lane {
            buf.push_str(&format!("T:{subject}={:?};", ids));
        }
        for (topic_key, topic_id) in &self.topic_lookup {
            buf.push_str(&format!("K:{topic_key}={topic_id};"));
        }
        for concept in &self.concepts {
            buf.push_str(&format!(
                "C:{}:{}:{:?}:{:?};",
                concept.id, concept.label, concept.kernel_tokens, concept.member_cells
            ));
        }
        for topic in &self.topics {
            buf.push_str(&format!(
                "P:{}:{}:{:?}:{:.4}:{:.4}:{:.4};",
                topic.id,
                topic.label,
                topic.concepts,
                topic.strength,
                topic.half_life_hours,
                topic.contradiction_pressure
            ));
        }
        for ((a, b), w) in self.hebb.edges_sorted() {
            buf.push_str(&format!("C:{a}-{b}={w:.4};"));
        }
        for k in self.tombstones.keys() {
            buf.push_str("T:");
            buf.push_str(k);
            buf.push(';');
        }
        fnv1a_hex(&buf)
    }

    pub fn state_bytes(&self) -> u64 {
        let mut total: u64 = 0;
        for c in &self.cells {
            total = total.saturating_add(c.event.body.len() as u64);
            total = total.saturating_add(c.event.subject.len() as u64);
            total = total.saturating_add((c.tokens.len() * 4) as u64);
        }
        total.saturating_add((self.hebb.len() * 12) as u64)
    }

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

        // 2. Candidate pool: BM25 hits ∪ substring fallback.
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

        for (cell_idx, _s) in scored.iter() {
            let cell = match self.cells.get(*cell_idx as usize) {
                Some(c) => c,
                None => continue,
            };
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
            // Stale window check
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

    pub fn consolidate(&mut self) {
        self.hebb.prune();
        let unprocessed: Vec<u32> = self
            .cells
            .iter()
            .enumerate()
            .filter(|(_, c)| c.concept_id.is_none())
            .map(|(i, _)| i as u32)
            .collect();
        let mut buckets: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for idx in unprocessed {
            if let Some(c) = self.cells.get(idx as usize) {
                buckets.entry(c.sketch[0]).or_default().push(idx);
            }
        }
        for (_bucket_key, mut members) in buckets {
            if members.len() < CONCEPT_MIN_MEMBERS {
                continue;
            }
            members.sort_by(|a, b| {
                cell_concept_sort_key(self, *a).cmp(&cell_concept_sort_key(self, *b))
            });
            let representative = members[0] as usize;
            let label = self
                .cells
                .get(representative)
                .map(|c| c.event.subject.clone())
                .unwrap_or_else(|| "concept".to_string());
            let kernel: Vec<TokenId> = self
                .cells
                .get(representative)
                .map(|c| {
                    c.tokens
                        .iter()
                        .take(CONCEPT_KERNEL_LIMIT)
                        .copied()
                        .collect()
                })
                .unwrap_or_default();
            let sketch = self
                .cells
                .get(representative)
                .map(|c| c.sketch)
                .unwrap_or([0; 8]);
            let cluster_quality = if members.is_empty() {
                0.0
            } else {
                members
                    .iter()
                    .map(|idx| {
                        self.cells
                            .get(*idx as usize)
                            .map(|cell| crate::index::jaccard_minhash(&sketch, &cell.sketch))
                            .unwrap_or(0.0)
                    })
                    .sum::<f32>()
                    / members.len() as f32
            };
            if cluster_quality < formation_threshold() {
                continue;
            }
            let id = self.concepts.len() as ConceptId;
            self.concepts.push(Concept {
                id,
                label,
                kernel_tokens: kernel,
                minhash: sketch,
                member_cells: members.clone(),
            });
            for m in members {
                if let Some(c) = self.cells.get_mut(m as usize) {
                    c.concept_id = Some(id);
                }
            }
        }

        let mut topic_groups: BTreeMap<String, Vec<ConceptId>> = BTreeMap::new();
        for concept in &self.concepts {
            topic_groups
                .entry(topic_key(&concept.label))
                .or_default()
                .push(concept.id);
        }

        let mut topic_lookup = self.topic_lookup.clone();
        for (key, concept_ids) in topic_groups {
            let coactivation = topic_coactivation(self, &concept_ids);
            if concept_ids.len() < 2 && coactivation < crate::config::TOPIC_EMERGENCE_WEIGHT {
                continue;
            }
            let topic_id = if let Some(existing) = self.topic_lookup.get(&key).copied() {
                existing
            } else {
                let id = self.topics.len() as TopicId;
                self.topics.push(Topic {
                    id,
                    label: key.clone(),
                    concepts: Vec::new(),
                    strength: 0.5,
                    half_life_hours: 24.0,
                    last_update_tx: BENCH_NOW.to_string(),
                    contradiction_pressure: 0.0,
                    stats: crate::topic::empty_stats(),
                });
                id
            };
            let topic_label_value = topic_label(self, &concept_ids);
            let topic_stats_value = topic_stats(self, &concept_ids);
            let topic_pressure = topic_contradiction_pressure(self, &concept_ids, coactivation);
            let topic = self
                .topics
                .iter_mut()
                .find(|topic| topic.id == topic_id)
                .expect("topic id must exist");
            topic.label = topic_label_value;
            topic.concepts = concept_ids.clone();
            topic.stats = topic_stats_value;
            topic.strength = (topic.strength
                + coactivation * crate::config::TOPIC_EMERGENCE_WEIGHT)
                .clamp(0.0, 1.0);
            topic.contradiction_pressure = topic_pressure;
            topic_recompute(topic, BENCH_NOW);
            topic_lookup.insert(key, topic_id);
        }
        self.topic_lookup = topic_lookup;
        for topic in self.topics.iter_mut() {
            topic_recompute(topic, BENCH_NOW);
        }
    }

    pub fn concepts(&self) -> &[Concept] {
        &self.concepts
    }
    pub fn topics(&self) -> &[Topic] {
        &self.topics
    }
    pub fn wal_len(&self) -> usize {
        self.wal.len()
    }
    pub fn topic_count(&self) -> usize {
        self.topics.len()
    }

    /// Programmatic helper: open an explicit topic for tests.
    #[doc(hidden)]
    pub fn debug_open_topic(&mut self, label: &str) -> TopicId {
        let id = self.topics.len() as TopicId;
        self.topics.push(Topic {
            id,
            label: label.to_string(),
            concepts: Vec::new(),
            strength: 0.5,
            half_life_hours: 24.0,
            last_update_tx: BENCH_NOW.to_string(),
            contradiction_pressure: 0.0,
            stats: crate::topic::empty_stats(),
        });
        self.topic_lookup.insert(topic_key(label), id);
        id
    }
}

fn cell_concept_sort_key(core: &Core, idx: u32) -> (Reverse<i64>, String, String, String) {
    let cell = core
        .cells
        .get(idx as usize)
        .expect("cell index must exist during consolidation");
    (
        Reverse((cell.strength * 1000.0).round() as i64),
        cell.event.subject.clone(),
        cell.event.body.clone(),
        cell.event.id.clone(),
    )
}

fn topic_key(label: &str) -> String {
    tokenize(label)
        .into_iter()
        .next()
        .unwrap_or_else(|| label.to_ascii_lowercase())
}

fn topic_label(core: &Core, concept_ids: &[ConceptId]) -> String {
    let mut labels: Vec<String> = concept_ids
        .iter()
        .filter_map(|id| core.concepts.iter().find(|concept| concept.id == *id))
        .map(|concept| concept.label.clone())
        .collect();
    labels.sort();
    labels
        .into_iter()
        .next()
        .unwrap_or_else(|| "topic".to_string())
}

fn topic_coactivation(core: &Core, concept_ids: &[ConceptId]) -> f32 {
    let mut total = 0.0_f32;
    let mut pairs = 0u32;
    for (i, left_id) in concept_ids.iter().enumerate() {
        for right_id in &concept_ids[i + 1..] {
            total += concept_pair_weight(core, *left_id, *right_id);
            pairs += 1;
        }
    }
    if pairs == 0 {
        0.0
    } else {
        (total / pairs as f32).clamp(0.0, 1.0)
    }
}

fn concept_pair_weight(core: &Core, a: ConceptId, b: ConceptId) -> f32 {
    let Some(left) = core.concepts.iter().find(|concept| concept.id == a) else {
        return 0.0;
    };
    let Some(right) = core.concepts.iter().find(|concept| concept.id == b) else {
        return 0.0;
    };
    let mut total = 0.0_f32;
    let mut pairs = 0u32;
    for left_cell in &left.member_cells {
        for right_cell in &right.member_cells {
            total += core.hebb.weight(*left_cell, *right_cell);
            pairs += 1;
        }
    }
    if pairs == 0 {
        0.0
    } else {
        (total / pairs as f32).clamp(0.0, 1.0)
    }
}

fn topic_stats(core: &Core, concept_ids: &[ConceptId]) -> crate::concept::TopicStats {
    let mut stats = crate::concept::TopicStats::default();
    let mut subjects = BTreeSet::new();
    let mut source_quality_total = 0.0_f32;
    let mut source_quality_count = 0u32;
    for concept_id in concept_ids {
        if let Some(concept) = core
            .concepts
            .iter()
            .find(|concept| concept.id == *concept_id)
        {
            subjects.insert(concept.label.to_ascii_lowercase());
            for cell_idx in &concept.member_cells {
                if let Some(cell) = core.cells.get(*cell_idx as usize) {
                    stats.recall_count = stats.recall_count.saturating_add(cell.recall_count);
                    stats.success_count = stats.success_count.saturating_add(cell.success_count);
                    stats.failure_count = stats.failure_count.saturating_add(cell.failure_count);
                    stats.recent_observes = stats.recent_observes.saturating_add(1);
                    let src_q = cell
                        .event
                        .sources
                        .iter()
                        .map(|src| src.quality)
                        .fold(0.0_f32, f32::max);
                    source_quality_total += src_q;
                    source_quality_count = source_quality_count.saturating_add(1);
                }
            }
        }
    }
    stats.distinct_subjects = subjects.len() as u32;
    stats.avg_source_quality = if source_quality_count == 0 {
        0.0
    } else {
        source_quality_total / source_quality_count as f32
    };
    stats
}

fn topic_contradiction_pressure(core: &Core, concept_ids: &[ConceptId], coactivation: f32) -> f32 {
    let mut pressure = (1.0 - coactivation).clamp(0.0, 1.0);
    if pressure >= CONCEPT_CONFLICT_THRESHOLD {
        pressure = (pressure + 0.05).clamp(0.0, 1.0);
    }
    for concept_id in concept_ids {
        if let Some(concept) = core
            .concepts
            .iter()
            .find(|concept| concept.id == *concept_id)
        {
            for cell_idx in &concept.member_cells {
                if let Some(cell) = core.cells.get(*cell_idx as usize) {
                    if !cell.event.contradicts.is_empty() || !cell.event.supersedes.is_empty() {
                        pressure = (pressure + 0.10).clamp(0.0, 1.0);
                    }
                }
            }
        }
    }
    pressure
}

fn render_event(ev: &StoredEvent) -> String {
    let trimmed = if ev.body.len() > 280 {
        format!("{}…", &ev.body[..280])
    } else {
        ev.body.clone()
    };
    format!("[{}] {}", ev.subject, trimmed)
}

fn has_supersession_partner(core: &Core, ev: &Cell) -> bool {
    for other in core.cells.iter() {
        if other.event.id == ev.event.id || other.event.subject != ev.event.subject {
            continue;
        }
        if other.event.body == ev.event.body {
            continue;
        }
        if let (Some(a), Some(b)) = (
            ev.event.valid_from.as_deref(),
            other.event.valid_from.as_deref(),
        ) {
            if a < b {
                return true;
            }
        }
    }
    false
}

fn is_counterexample(ev: &StoredEvent) -> bool {
    matches!(ev.kind.as_str(), "Counterexample")
        || ev
            .tags
            .iter()
            .any(|t| t == "falsified" || t == "broken" || t == "deprecated")
}

fn detects_unit_mismatch(ev: &StoredEvent) -> bool {
    ev.tags
        .iter()
        .any(|t| t == "unit_mismatch" || t == "counterexample")
        || ev.body.contains("DELIBERATE COUNTEREXAMPLE")
        || ev.body.contains("inconsistent")
}

fn push_unique(warnings: &mut Vec<Warning>, w: Warning) {
    if !warnings.contains(&w) {
        warnings.push(w);
    }
}

pub fn pack_hash(r: &RecallData) -> String {
    let mut buf = String::new();
    buf.push_str("a:");
    buf.push_str(&r.answer);
    buf.push('|');
    buf.push_str("c:");
    for c in r.citations.iter() {
        buf.push_str(&c.uri);
        buf.push('@');
        buf.push_str(&c.citation);
        buf.push(';');
    }
    buf.push_str("|w:");
    for w in r.warnings.iter() {
        buf.push_str(warning_name(*w));
        buf.push(',');
    }
    buf.push_str("|u:");
    for id in r.used_ids.iter() {
        buf.push_str(id);
        buf.push(',');
    }
    buf.push_str("|conf:");
    buf.push_str(&format!("{:.4}", r.confidence));
    fnv1a_hex(&buf)
}

fn warning_name(w: Warning) -> &'static str {
    match w {
        Warning::Superseded => "superseded",
        Warning::Contradicted => "contradicted",
        Warning::Redacted => "redacted",
        Warning::CausalMaskApplied => "causal_mask_applied",
        Warning::SkeptikSurfaced => "skeptic_surfaced",
        Warning::UnitMismatch => "unit_mismatch",
        Warning::Abstained => "abstained",
        Warning::UnsafeToolRefused => "unsafe_tool_refused",
    }
}

fn privacy_byte(p: PrivacyClass) -> u8 {
    match p {
        PrivacyClass::Public => 0,
        PrivacyClass::Internal => 1,
        PrivacyClass::Confidential => 2,
        PrivacyClass::Secret => 3,
        PrivacyClass::Vault => 4,
    }
}

fn privacy_from_byte(b: u8) -> PrivacyClass {
    match b {
        1 => PrivacyClass::Internal,
        2 => PrivacyClass::Confidential,
        3 => PrivacyClass::Secret,
        4 => PrivacyClass::Vault,
        _ => PrivacyClass::Public,
    }
}

fn modality_byte(m: ClaimModality) -> u8 {
    match m {
        ClaimModality::Observed => 0,
        ClaimModality::AssertedBySource => 1,
        ClaimModality::InferredByAgent => 2,
        ClaimModality::HumanApproved => 3,
        ClaimModality::FormallyVerified => 4,
    }
}

fn modality_from_byte(b: u8) -> ClaimModality {
    match b {
        1 => ClaimModality::AssertedBySource,
        2 => ClaimModality::InferredByAgent,
        3 => ClaimModality::HumanApproved,
        4 => ClaimModality::FormallyVerified,
        _ => ClaimModality::Observed,
    }
}

fn outcome_byte(o: Outcome) -> u8 {
    match o {
        Outcome::TaskSuccess => 0,
        Outcome::TaskFailure => 1,
        Outcome::Verified => 2,
        Outcome::Falsified => 3,
        Outcome::Ignored => 4,
    }
}

fn outcome_from_byte(b: u8) -> Outcome {
    match b {
        1 => Outcome::TaskFailure,
        2 => Outcome::Verified,
        3 => Outcome::Falsified,
        4 => Outcome::Ignored,
        _ => Outcome::TaskSuccess,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: &str, subject: &str, body: &str, tx: &str) -> StoredEvent {
        StoredEvent {
            id: id.to_string(),
            kind: "Claim".to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
            tx_time: tx.to_string(),
            valid_from: Some("2020-01-01T00:00:00Z".to_string()),
            valid_to: None,
            privacy_class: PrivacyClass::Public,
            claim_modality: Some(ClaimModality::Observed),
            tags: Vec::new(),
            sources: vec![SourceRef {
                uri: "doi:example".to_string(),
                citation: "Example et al. 2024".to_string(),
                quality: 0.9,
            }],
            supersedes: Vec::new(),
            contradicts: Vec::new(),
        }
    }

    fn q(text: &str) -> RecallQuery {
        RecallQuery {
            text: text.to_string(),
            mentions: vec![text.to_string()],
            intent: Intent::Recall,
            token_budget: 4096,
        }
    }

    #[test]
    fn observe_then_recall_returns_event() {
        let mut c = Core::default();
        c.observe(ev(
            "e1",
            "neutrino",
            "neutrinos have mass",
            "2020-01-01T00:00:00Z",
        ));
        let r = c.recall(&q("neutrino"));
        assert!(r.used_ids.contains(&"e1".to_string()));
        assert!(!r.context_pack_hash.is_empty());
    }

    #[test]
    fn rebuild_preserves_state_hash() {
        let mut c = Core::default();
        c.observe(ev(
            "e1",
            "neutrino",
            "mass is small",
            "2020-01-01T00:00:00Z",
        ));
        c.observe(ev(
            "e2",
            "muon",
            "heavier than electron",
            "2020-01-01T00:00:00Z",
        ));
        c.recall(&q("neutrino"));
        c.recall(&q("muon"));
        let h1 = c.export_state_hash();
        c.rebuild();
        let h2 = c.export_state_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn historical_recall_does_not_mutate() {
        let mut c = Core::default();
        c.observe(ev("old", "subj", "old fact", "2020-01-01T00:00:00Z"));
        c.observe(ev("new", "subj", "new fact", "2025-01-01T00:00:00Z"));
        let before = c.export_state_hash();
        c.recall_as_of(&q("subj"), "2022-06-01T00:00:00Z");
        c.recall_at(&q("subj"), "2022-06-01T00:00:00Z");
        let after = c.export_state_hash();
        assert_eq!(before, after);
    }

    #[test]
    fn vault_event_is_redacted() {
        let mut c = Core::default();
        let mut e = ev("v1", "API key", "API_KEY=abc", "2026-01-01T00:00:00Z");
        e.privacy_class = PrivacyClass::Vault;
        e.kind = "VaultCanary".to_string();
        c.observe(e);
        let r = c.recall(&q("API key"));
        assert!(r.answer.contains("[REDACTED"));
        assert!(r.warnings.contains(&Warning::Redacted));
    }

    #[test]
    fn canary_body_is_redacted() {
        let mut c = Core::default();
        let canary = format!("{}{}{}{}{}", "s", "k-memory-", "b", "ench-", "CANARY-7f3a");
        let body = format!("token={canary}");
        c.observe(ev("c1", "secret", &body, "2026-01-01T00:00:00Z"));
        let r = c.recall(&q("secret"));
        assert!(!r.answer.contains(&canary));
        assert!(r.warnings.contains(&Warning::Redacted));
    }

    #[test]
    fn supersession_partner_surfaces_skeptic() {
        let mut c = Core::default();
        let mut older = ev("a", "subj", "old body", "2020-01-01T00:00:00Z");
        older.valid_from = Some("2020-01-01T00:00:00Z".to_string());
        let mut newer = ev("b", "subj", "new body", "2025-01-01T00:00:00Z");
        newer.valid_from = Some("2024-01-01T00:00:00Z".to_string());
        c.observe(older);
        c.observe(newer);
        let r = c.recall(&q("subj"));
        assert!(r.warnings.contains(&Warning::SkeptikSurfaced));
    }

    #[test]
    fn feedback_moves_hebb_and_utility() {
        let mut c = Core::default();
        c.observe(ev("a", "x", "y", "2020-01-01T00:00:00Z"));
        c.observe(ev("b", "x2", "y2", "2020-01-01T00:00:00Z"));
        c.recall(&q("x"));
        c.feedback(&FeedbackSignal {
            outcome: Outcome::TaskSuccess,
            used: vec!["a".to_string(), "b".to_string()],
        });
        assert!(c.hebb.weight(0, 1) > 0.0);
    }

    #[test]
    fn unsafe_skill_in_procedure_is_refused() {
        let mut c = Core::default();
        let mut e = ev("s1", "tool_x", "UNSAFE side-effect", "2026-01-01T00:00:00Z");
        e.kind = "Skill".to_string();
        e.tags.push("unsafe".to_string());
        c.observe(e);
        let r = c.recall(&RecallQuery {
            text: "tool_x".to_string(),
            mentions: Vec::new(),
            intent: Intent::Procedure,
            token_budget: 4096,
        });
        assert!(r.warnings.contains(&Warning::UnsafeToolRefused));
        assert!(r.answer.contains("refused"));
    }

    #[test]
    fn recall_touch_promotes_strength() {
        let mut c = Core::default();
        c.observe(ev("a", "neutrino", "has mass", "2020-01-01T00:00:00Z"));
        let before = c.cells[0].strength;
        for _ in 0..5 {
            c.recall(&q("neutrino"));
        }
        let after = c.cells[0].strength;
        assert!(
            after > before,
            "strength must increase after repeated recalls (before={before}, after={after})"
        );
    }
}
