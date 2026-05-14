//! `Core::observe`, `Core::feedback`, `Core::forget` — write-path mutations
//! that append to the WAL and produce stable receipts. Split out from
//! `core/mod.rs` to keep each file under the shape audit cap.

use super::{
    modality_byte, outcome_byte, privacy_byte, Cell, Core, FeedbackSignal, Outcome, Receipt,
    StoredEvent, Tombstone,
};
use crate::concept::{attach_threshold, best_concept_match};
use crate::hash::fnv1a_hex;
use crate::index::{bigrams, minhash_sketch, tokenize, TokenId};
use crate::ledger::WalOp;
use crate::time::BENCH_NOW;

impl Core {
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
        let modality_byte = ev.claim_modality.map(modality_byte);
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
        // Derive the lane keys from the event before moving it into the cell so
        // the index updates are a single linear pass with no Option lookups.
        let subject_key = ev.subject.to_ascii_lowercase();
        let kind_key = ev.kind.clone();
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
        self.subject_index
            .entry(subject_key.clone())
            .or_default()
            .push(cell_idx);
        match kind_key.as_str() {
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
}
