//! Cognitive memory core — Phase 2.
//!
//! Substantive memory engine: append-only WAL ledger, BM25-lite inverted
//! index, MinHash concept attachment, topic strength formula, Hebbian
//! co-activation matrix, FSRS-on-cells, and a `WalOp::RecallTouch` step
//! that records hot-path mutations so `rebuild()` is byte-identical.
//!
//! Public API surface unchanged from Phase 1 so the benchmark adapter
//! continues to work without modification.

use std::collections::BTreeMap;

use crate::budget::Budget;
use crate::concept::{Concept, ConceptId, Topic, TopicId};
use crate::config::DEFAULT_CITATION_QUALITY_FLOOR;
use crate::hash::{fnv1a_hex, fnv1a_seq_hex};
use crate::hebb::Hebb;
use crate::index::{Interner, InvertedIndex, TokenId};
use crate::ledger::Wal;
use crate::time::BENCH_NOW;

mod consolidate;
mod observe;
mod recall;
mod state;

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
pub(super) struct Cell {
    pub(super) event: StoredEvent,
    pub(super) tokens: Vec<TokenId>,
    pub(super) sketch: [u32; 8],
    pub(super) strength: f32,
    pub(super) half_life_hours: f32,
    pub(super) last_recall_tx: String,
    pub(super) recall_count: u32,
    pub(super) success_count: u32,
    pub(super) failure_count: u32,
    pub(super) utility: f32,
    pub(super) concept_id: Option<ConceptId>,
}

pub struct Core {
    pub(super) cells: Vec<Cell>,
    pub(super) by_id: BTreeMap<String, u32>,
    pub(super) exact_id_index: BTreeMap<String, u32>,
    pub(super) subject_index: BTreeMap<String, Vec<u32>>,
    pub(super) equation_lane: BTreeMap<String, Vec<u32>>,
    pub(super) theorem_lane: BTreeMap<String, Vec<u32>>,
    pub(super) tombstones: BTreeMap<String, Tombstone>,
    pub(super) interner: Interner,
    pub(super) index: InvertedIndex,
    pub(super) hebb: Hebb,
    pub(super) concepts: Vec<Concept>,
    pub(super) topics: Vec<Topic>,
    pub(super) topic_lookup: BTreeMap<String, TopicId>,
    pub(super) receipt_seq: u64,
    pub(super) last_receipt_hash: String,
    pub(super) citation_quality_floor: f32,
    pub(super) wal: Wal,
    pub(super) consolidation_budget: Budget,
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
            consolidation_budget: Budget::ZERO,
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

    /// Configure the LLM/embedding budget consulted by future
    /// `consolidate()` calls. Defaults to [`Budget::ZERO`], which keeps
    /// cogcore on the deterministic rule-based path. Hosts that wire a
    /// non-default `ConsolidationBackend` (e.g. ZYAL-mediated Jnoccio)
    /// can override this before running consolidation.
    pub fn set_consolidation_budget(&mut self, budget: Budget) {
        self.consolidation_budget = budget;
    }

    pub fn canonical_event_id(kind: &str, subject: &str, body: &str, tx_time: &str) -> String {
        fnv1a_seq_hex(&[kind, subject, body, tx_time])
    }

    pub(super) fn next_receipt(&mut self, event_id: Option<&str>, kind: &str) -> Receipt {
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
        self.topic_lookup
            .insert(consolidate::topic_key(label), id);
        id
    }
}

pub(super) fn push_unique(warnings: &mut Vec<Warning>, w: Warning) {
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

pub(super) fn privacy_byte(p: PrivacyClass) -> u8 {
    match p {
        PrivacyClass::Public => 0,
        PrivacyClass::Internal => 1,
        PrivacyClass::Confidential => 2,
        PrivacyClass::Secret => 3,
        PrivacyClass::Vault => 4,
    }
}

pub(super) fn privacy_from_byte(b: u8) -> PrivacyClass {
    match b {
        1 => PrivacyClass::Internal,
        2 => PrivacyClass::Confidential,
        3 => PrivacyClass::Secret,
        4 => PrivacyClass::Vault,
        _ => PrivacyClass::Public,
    }
}

pub(super) fn modality_byte(m: ClaimModality) -> u8 {
    match m {
        ClaimModality::Observed => 0,
        ClaimModality::AssertedBySource => 1,
        ClaimModality::InferredByAgent => 2,
        ClaimModality::HumanApproved => 3,
        ClaimModality::FormallyVerified => 4,
    }
}

pub(super) fn modality_from_byte(b: u8) -> ClaimModality {
    match b {
        1 => ClaimModality::AssertedBySource,
        2 => ClaimModality::InferredByAgent,
        3 => ClaimModality::HumanApproved,
        4 => ClaimModality::FormallyVerified,
        _ => ClaimModality::Observed,
    }
}

pub(super) fn outcome_byte(o: Outcome) -> u8 {
    match o {
        Outcome::TaskSuccess => 0,
        Outcome::TaskFailure => 1,
        Outcome::Verified => 2,
        Outcome::Falsified => 3,
        Outcome::Ignored => 4,
    }
}

pub(super) fn outcome_from_byte(b: u8) -> Outcome {
    match b {
        1 => Outcome::TaskFailure,
        2 => Outcome::Verified,
        3 => Outcome::Falsified,
        4 => Outcome::Ignored,
        _ => Outcome::TaskSuccess,
    }
}

#[cfg(test)]
mod tests;
