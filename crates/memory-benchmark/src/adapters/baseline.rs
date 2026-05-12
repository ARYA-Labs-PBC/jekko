//! Baseline naïve memory adapter — deliberately weak.
//!
//! Stores events in a Vec, does linear search, NO bitemporal filter, NO
//! privacy redaction, NO supersession warnings, NO ClaimModality preservation.
//!
//! Target score on the gauntlet: [35, 75]. Proves the benchmark is non-trivial.

use crate::hash::fnv1a_hex;
use crate::memory_api::{event_canonical_id, pack_hash, BENCH_NOW};
use crate::{
    Citation, Event, Feedback, MemorySystem, OmissionNote, Query, RecallResult, Receipt, Tombstone,
    Warning,
};

pub struct Adapter {
    events: Vec<Event>,
    receipt_seq: u64,
}

impl Default for Adapter {
    fn default() -> Self {
        Adapter {
            events: Vec::new(),
            receipt_seq: 0,
        }
    }
}

impl Adapter {
    fn match_event(&self, e: &Event, query: &Query) -> bool {
        let q_lower = query.text.to_lowercase();
        let body_lower = e.body.to_lowercase();
        let subj_lower = e.subject.to_lowercase();
        if !q_lower.is_empty() && (body_lower.contains(&q_lower) || subj_lower.contains(&q_lower)) {
            return true;
        }
        for m in &query.mentions {
            let m_lower = m.to_lowercase();
            if body_lower.contains(&m_lower) || subj_lower.contains(&m_lower) {
                return true;
            }
        }
        false
    }

    fn naive_recall(&self, query: &Query) -> RecallResult {
        let mut answer = String::new();
        let mut citations: Vec<Citation> = Vec::new();
        let mut used_ids: Vec<String> = Vec::new();
        for e in &self.events {
            if self.match_event(e, query) {
                // Baseline: NO redaction, NO bitemporal filter, NO warnings.
                // Leaks vault canaries! Returns superseded claims! That's the point.
                answer.push_str(&format!("[{}] {} ", e.subject, e.body));
                used_ids.push(e.id.clone());
                for s in &e.sources {
                    citations.push(Citation::from_source(s));
                }
            }
        }
        let answer = answer.trim_end().to_string();
        let mut r = RecallResult::default();
        r.answer = answer;
        r.citations = citations;
        r.warnings = Vec::new(); // baseline emits no warnings
        r.omitted = Vec::<OmissionNote>::new();
        r.used_ids = used_ids;
        r.confidence = 0.5;
        r.context_pack_hash = String::new();
        r.claim_modality = None;
        r.context_pack_hash = pack_hash(&r);
        r
    }
}

impl MemorySystem for Adapter {
    fn name(&self) -> &'static str {
        "baseline"
    }
    fn observe(&mut self, event: &Event) -> Receipt {
        self.receipt_seq += 1;
        let mut e = event.clone();
        if e.id.is_empty() {
            e.id = event_canonical_id(&format!("{:?}", e.kind), &e.subject, &e.body, &e.tx_time);
        }
        let id = e.id.clone();
        self.events.push(e);
        Receipt {
            event_id: Some(id),
            mutation_id: format!("baseline-{:08}", self.receipt_seq),
            at: BENCH_NOW.to_string(),
            previous_hash: String::new(),
            hash: fnv1a_hex(&format!("baseline:{}", self.receipt_seq)),
        }
    }
    fn recall(&mut self, q: &Query) -> RecallResult {
        self.naive_recall(q)
    }
    // Baseline: no bitemporal — returns the same thing for at / as_of / current.
    fn recall_at(&mut self, q: &Query, _world: &str) -> RecallResult {
        self.naive_recall(q)
    }
    fn recall_as_of(&mut self, q: &Query, _tx: &str) -> RecallResult {
        self.naive_recall(q)
    }
    fn feedback(&mut self, _pack: &str, _o: &Feedback) -> Receipt {
        self.receipt_seq += 1;
        Receipt {
            event_id: None,
            mutation_id: format!("baseline-fb-{:08}", self.receipt_seq),
            at: BENCH_NOW.to_string(),
            previous_hash: String::new(),
            hash: fnv1a_hex(&format!("baseline-fb:{}", self.receipt_seq)),
        }
    }
    fn forget(&mut self, id: &str, reason: &str) -> Tombstone {
        // Baseline: tombstones but doesn't actually filter on recall (weakness).
        Tombstone {
            memory_id: id.to_string(),
            reason: reason.to_string(),
            deletion_proof: fnv1a_hex(id),
            deleted_at: BENCH_NOW.to_string(),
        }
    }
    fn rebuild(&mut self) -> Receipt {
        self.receipt_seq += 1;
        Receipt {
            event_id: None,
            mutation_id: format!("baseline-rebuild-{:08}", self.receipt_seq),
            at: BENCH_NOW.to_string(),
            previous_hash: String::new(),
            hash: fnv1a_hex(&format!("baseline-rebuild:{}", self.receipt_seq)),
        }
    }
    fn export_state_hash(&self) -> String {
        let mut buf = String::new();
        let mut ids: Vec<&str> = self.events.iter().map(|e| e.id.as_str()).collect();
        ids.sort();
        for id in &ids {
            buf.push_str(id);
            buf.push('|');
        }
        fnv1a_hex(&buf)
    }
}

#[allow(dead_code)]
fn _unused_warning() -> Warning {
    Warning::LowConfidence
}
