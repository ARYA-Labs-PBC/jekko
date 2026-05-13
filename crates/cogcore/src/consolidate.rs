//! Offline consolidation pipeline.
//!
//! Pluggable backends produce deterministic state mutations:
//! - `RuleBackend` (default): rule-based passes, zero LLM, byte-stable
//! - `JnoccioBackend` (Phase 7+): periodic LLM summarization through
//!    ZYAL-mediated Jnoccio. Deferred until a Rust Jnoccio client exists
//!    OR the benchmark wires a ZYAL-driven consolidation receipt path.
//!
//! All passes must be deterministic given `(state, budget, BENCH_NOW)`.

use crate::budget::Budget;
use crate::concept::Topic;
use crate::core::StoredEvent;
use crate::ingest::equation::EqAtom;

/// Result of an offline consolidation pass.
#[derive(Debug, Clone, Default)]
pub struct ConsolidationReport {
    /// Number of concepts promoted from new clusters in this pass.
    pub concepts_promoted: u32,
    /// Number of topic merges performed.
    pub topics_merged: u32,
    /// Number of equations flagged with unit-consistency verdicts.
    pub equations_flagged: u32,
    /// Number of cells pruned by utility decay or supersession.
    pub cells_pruned: u32,
    /// LLM calls actually issued (always 0 for `RuleBackend`).
    pub llm_calls_made: u32,
}

/// Synthesized lesson from a topic summary (output of LLM enrich pass).
#[derive(Debug, Clone)]
pub struct SynthesizedLesson {
    /// Topic the lesson summarizes.
    pub topic_id: u32,
    /// Topic label used when surfacing the lesson.
    pub label: String,
    /// Human-readable summary body produced by the backend.
    pub summary_body: String,
    /// Cell ids that informed the lesson — used for citation receipts.
    pub source_cell_ids: Vec<String>,
}

/// Verdict from an equation unit consistency check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitVerdict {
    /// Units are recognized and dimensionally consistent.
    Consistent,
    /// Units are recognized but dimensionally inconsistent.
    Inconsistent,
    /// Units could not be confirmed (unknown unit string or absent).
    Unverifiable,
}

/// Adversarial-claim flag from contradiction detection.
#[derive(Debug, Clone)]
pub struct AdversarialFlag {
    /// Cell that was flagged.
    pub cell_id: String,
    /// Short, deterministic explanation of why the flag fired.
    pub reason: String,
    /// High-quality peer cells the flagged cell contradicts.
    pub conflicting_peers: Vec<String>,
}

/// Pluggable consolidation backend. Production benchmark uses
/// `RuleBackend`; ZYAL workflows may configure other backends via the
/// host runner. All implementations must be deterministic.
pub trait ConsolidationBackend {
    /// Summarize a topic's member cells into a lesson. Default
    /// implementation returns `None` (no synthesis). LLM-backed
    /// implementations call out behind a `Budget` gate.
    fn summarize_topic(
        &mut self,
        topic: &Topic,
        members: &[&StoredEvent],
        budget: &mut Budget,
    ) -> Option<SynthesizedLesson> {
        let _ = (topic, members, budget);
        None
    }

    /// Verify dimensional consistency of an equation in context.
    fn verify_equation_units(
        &mut self,
        eq: &EqAtom,
        context: &[&StoredEvent],
        budget: &mut Budget,
    ) -> Option<UnitVerdict> {
        let _ = (eq, context, budget);
        None
    }

    /// Detect adversarial claims among a cell's peers.
    fn detect_adversarial_claim(
        &mut self,
        cell: &StoredEvent,
        peers: &[&StoredEvent],
        budget: &mut Budget,
    ) -> Option<AdversarialFlag> {
        let _ = (cell, peers, budget);
        None
    }
}

/// Deterministic rule-based consolidation backend. Default for benchmarks.
/// No LLM calls. No embedding calls. No clock reads.
#[derive(Debug, Default)]
pub struct RuleBackend;

impl ConsolidationBackend for RuleBackend {
    fn summarize_topic(
        &mut self,
        topic: &Topic,
        members: &[&StoredEvent],
        budget: &mut Budget,
    ) -> Option<SynthesizedLesson> {
        // Rule-based summary: take the highest-source-quality member's
        // body as the canonical statement. No LLM call.
        if members.is_empty() {
            return None;
        }
        let _ = budget; // RuleBackend uses no budget
        let mut best: Option<(&StoredEvent, f32)> = None;
        for member in members.iter() {
            let q = member
                .sources
                .iter()
                .map(|s| s.quality)
                .fold(0.0_f32, f32::max);
            match best {
                None => best = Some((*member, q)),
                Some((_, current_q)) if q > current_q => best = Some((*member, q)),
                _ => {}
            }
        }
        let (lead, _) = best?;
        Some(SynthesizedLesson {
            topic_id: topic.id,
            label: topic.label.clone(),
            summary_body: format!(
                "Topic {} synthesized from {} member cells. Lead claim: {}",
                topic.label,
                members.len(),
                lead.body
            ),
            source_cell_ids: members.iter().map(|m| m.id.clone()).collect(),
        })
    }

    fn verify_equation_units(
        &mut self,
        eq: &EqAtom,
        context: &[&StoredEvent],
        budget: &mut Budget,
    ) -> Option<UnitVerdict> {
        let _ = (context, budget);
        // Rule: if units field is present and matches a known SI unit,
        // verdict is Consistent. If present but unknown, Unverifiable.
        // If absent, Unverifiable.
        match eq.units.as_deref() {
            None => Some(UnitVerdict::Unverifiable),
            Some(u) if is_known_si_unit(u) => Some(UnitVerdict::Consistent),
            Some(_) => Some(UnitVerdict::Unverifiable),
        }
    }

    fn detect_adversarial_claim(
        &mut self,
        cell: &StoredEvent,
        peers: &[&StoredEvent],
        budget: &mut Budget,
    ) -> Option<AdversarialFlag> {
        let _ = budget;
        // Rule: a cell with low source.quality (< 0.5) whose `contradicts`
        // list references a high-quality peer is flagged.
        let cell_quality = cell
            .sources
            .iter()
            .map(|s| s.quality)
            .fold(0.0_f32, f32::max);
        if cell_quality >= 0.5 {
            return None;
        }
        let mut conflicting: Vec<String> = Vec::new();
        for peer in peers.iter() {
            if !cell.contradicts.contains(&peer.id) {
                continue;
            }
            let peer_q = peer
                .sources
                .iter()
                .map(|s| s.quality)
                .fold(0.0_f32, f32::max);
            if peer_q >= 0.85 {
                conflicting.push(peer.id.clone());
            }
        }
        if conflicting.is_empty() {
            return None;
        }
        Some(AdversarialFlag {
            cell_id: cell.id.clone(),
            reason: format!(
                "low-quality cell (q={cell_quality:.2}) contradicts high-quality peer(s)"
            ),
            conflicting_peers: conflicting,
        })
    }
}

fn is_known_si_unit(u: &str) -> bool {
    matches!(
        u.trim(),
        "kg" | "m" | "s" | "A" | "K" | "mol" | "cd"
        | "m/s" | "m/s^2" | "J" | "W" | "N" | "Pa" | "Hz"
        | "eV" | "keV" | "MeV" | "GeV" | "TeV"
        | "eV^2" | "GeV^2"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ClaimModality, PrivacyClass, SourceRef};

    fn ev(id: &str, q: f32, contradicts: Vec<String>) -> StoredEvent {
        StoredEvent {
            id: id.to_string(),
            kind: "Claim".to_string(),
            subject: "topic".to_string(),
            body: format!("body of {id}"),
            tx_time: "2026-01-01T00:00:00Z".to_string(),
            valid_from: None,
            valid_to: None,
            privacy_class: PrivacyClass::Public,
            claim_modality: Some(ClaimModality::Observed),
            tags: Vec::new(),
            sources: vec![SourceRef {
                uri: format!("doi:{id}"),
                citation: format!("Source {id}"),
                quality: q,
            }],
            supersedes: Vec::new(),
            contradicts,
        }
    }

    #[test]
    fn rule_backend_summarize_picks_highest_quality_member() {
        let mut backend = RuleBackend::default();
        let mut budget = Budget::ZERO;
        let a = ev("a", 0.7, vec![]);
        let b = ev("b", 0.95, vec![]); // highest
        let c = ev("c", 0.5, vec![]);
        let members = vec![&a, &b, &c];
        let topic = crate::concept::Topic {
            id: 1,
            label: "test-topic".to_string(),
            concepts: Vec::new(),
            strength: 0.5,
            half_life_hours: 24.0,
            last_update_tx: "2026-01-01T00:00:00Z".to_string(),
            contradiction_pressure: 0.0,
            stats: crate::topic::empty_stats(),
        };
        let lesson = backend.summarize_topic(&topic, &members, &mut budget).unwrap();
        assert!(lesson.summary_body.contains("body of b"));
        assert_eq!(lesson.source_cell_ids.len(), 3);
    }

    #[test]
    fn rule_backend_verifies_known_si_units() {
        let mut backend = RuleBackend::default();
        let mut budget = Budget::ZERO;
        let eq = EqAtom {
            lhs: "E".to_string(),
            op: "=".to_string(),
            rhs: "mc^2".to_string(),
            units: Some("J".to_string()),
        };
        assert_eq!(
            backend.verify_equation_units(&eq, &[], &mut budget),
            Some(UnitVerdict::Consistent)
        );
    }

    #[test]
    fn rule_backend_unverifiable_on_unknown_unit() {
        let mut backend = RuleBackend::default();
        let mut budget = Budget::ZERO;
        let eq = EqAtom {
            lhs: "X".to_string(),
            op: "=".to_string(),
            rhs: "Y".to_string(),
            units: Some("zorkbargs".to_string()),
        };
        assert_eq!(
            backend.verify_equation_units(&eq, &[], &mut budget),
            Some(UnitVerdict::Unverifiable)
        );
    }

    #[test]
    fn rule_backend_unverifiable_when_no_units() {
        let mut backend = RuleBackend::default();
        let mut budget = Budget::ZERO;
        let eq = EqAtom {
            lhs: "X".to_string(),
            op: "=".to_string(),
            rhs: "Y".to_string(),
            units: None,
        };
        assert_eq!(
            backend.verify_equation_units(&eq, &[], &mut budget),
            Some(UnitVerdict::Unverifiable)
        );
    }

    #[test]
    fn rule_backend_flags_low_quality_contradiction() {
        let mut backend = RuleBackend::default();
        let mut budget = Budget::ZERO;
        let bad = ev("bad", 0.2, vec!["good".to_string()]);
        let good = ev("good", 0.95, vec![]);
        let peers = vec![&good];
        let flag = backend.detect_adversarial_claim(&bad, &peers, &mut budget);
        assert!(flag.is_some());
        let f = flag.unwrap();
        assert_eq!(f.cell_id, "bad");
        assert_eq!(f.conflicting_peers, vec!["good".to_string()]);
    }

    #[test]
    fn rule_backend_skips_high_quality_cell() {
        let mut backend = RuleBackend::default();
        let mut budget = Budget::ZERO;
        let good = ev("good", 0.95, vec!["other".to_string()]);
        let other = ev("other", 0.95, vec![]);
        let peers = vec![&other];
        assert!(backend.detect_adversarial_claim(&good, &peers, &mut budget).is_none());
    }

    #[test]
    fn default_backend_no_op_methods_return_none() {
        struct StubBackend;
        impl ConsolidationBackend for StubBackend {}
        let mut backend = StubBackend;
        let mut budget = Budget::ZERO;
        let cell = ev("x", 0.9, vec![]);
        assert!(backend.detect_adversarial_claim(&cell, &[], &mut budget).is_none());
        let eq = EqAtom {
            lhs: "a".to_string(),
            op: "=".to_string(),
            rhs: "b".to_string(),
            units: None,
        };
        assert!(backend.verify_equation_units(&eq, &[], &mut budget).is_none());
    }
}
