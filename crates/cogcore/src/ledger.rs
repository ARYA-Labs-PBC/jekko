//! Append-only WAL ledger.
//!
//! Phase 2 ships an in-memory ledger; the disk-backed variant is a swap
//! behind `StorageBackend::Disk` in Phase 6+. Every state-mutating
//! operation appends an entry; the receipt chain is `hash = fnv1a(prev ||
//! seq || op_repr)`. Recall mutations (`RecallTouch`) are recorded so
//! `rebuild()` reproduces the live state byte-for-byte.

use crate::hash::fnv1a_seq_hex;

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum WalOp {
    Observe {
        event_id: String,
        kind: String,
        subject: String,
        body: String,
        tx_time: String,
        valid_from: Option<String>,
        valid_to: Option<String>,
        privacy_class: u8,
        claim_modality: Option<u8>,
        tags: Vec<String>,
        sources: Vec<(String, String, f32)>,
        supersedes: Vec<String>,
        contradicts: Vec<String>,
    },
    Tombstone {
        event_id: String,
        reason: String,
    },
    Feedback {
        outcome: u8,
        used: Vec<String>,
    },
    RecallTouch {
        used_ids: Vec<String>,
        tx_time: String,
    },
}

#[derive(Debug, Clone)]
pub struct WalEntry {
    pub seq: u64,
    pub prev_hash: String,
    pub hash: String,
    pub op: WalOp,
}

#[derive(Default)]
pub struct Wal {
    entries: Vec<WalEntry>,
    last_hash: String,
}

impl Wal {
    pub fn append(&mut self, op: WalOp) -> WalEntry {
        let seq = self.entries.len() as u64 + 1;
        let repr = op_repr(&op);
        let prev = self.last_hash.clone();
        let hash = fnv1a_seq_hex(&[&prev, &seq.to_string(), &repr]);
        let entry = WalEntry {
            seq,
            prev_hash: prev,
            hash: hash.clone(),
            op,
        };
        self.last_hash = hash;
        self.entries.push(entry.clone());
        entry
    }

    pub fn entries(&self) -> &[WalEntry] {
        &self.entries
    }

    pub fn last_hash(&self) -> &str {
        &self.last_hash
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn op_repr(op: &WalOp) -> String {
    match op {
        WalOp::Observe {
            event_id,
            kind,
            subject,
            body,
            tx_time,
            ..
        } => format!("OBS:{event_id}:{kind}:{subject}:{body}:{tx_time}"),
        WalOp::Tombstone { event_id, reason } => format!("TOMB:{event_id}:{reason}"),
        WalOp::Feedback { outcome, used } => {
            format!("FB:{outcome}:{}", used.join(","))
        }
        WalOp::RecallTouch { used_ids, tx_time } => {
            format!("RT:{}:{tx_time}", used_ids.join(","))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_advances_chain() {
        let mut w = Wal::default();
        let a = w.append(WalOp::RecallTouch {
            used_ids: vec!["e1".into()],
            tx_time: "2026-05-12T00:00:00Z".into(),
        });
        let b = w.append(WalOp::Tombstone {
            event_id: "e1".into(),
            reason: "x".into(),
        });
        assert_ne!(a.hash, b.hash);
        assert_eq!(b.prev_hash, a.hash);
    }
}
