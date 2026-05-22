//! Adapter wrapping `cogcore::Core` as a `MemorySystem` candidate.
//!
//! cogcore is a standalone crate (no dependency on this benchmark). The
//! translation layer below copies `Event` / `Query` fields into cogcore's
//! native types, calls cogcore, and projects the result back into
//! `RecallResult` shape.
//!
//! Phase 1 ships substring-match recall plus bitemporal filtering, vault
//! short-circuit, fragment-built canary redaction, and a utility EMA
//! driven by feedback. Phases 2+ swap the recall pipeline for BM25 +
//! concept-expand + graph rerank inside cogcore without changing this
//! file's boundary translation.

use cogcore::core::{
    self as cog, FeedbackSignal, Intent as CogIntent, Outcome as CogOutcome,
    PrivacyClass as CogPrivacy, SourceRef, StoredEvent,
};

use crate::memory_api::pack_hash;
use crate::{
    ClaimModality, Event, EventKind, Feedback, MemorySystem, OmissionNote, Outcome, PrivacyClass,
    Query, QueryIntent, RecallResult, Receipt, Tombstone, Warning,
};

#[derive(Default)]
pub struct Adapter {
    core: cog::Core,
}

impl Adapter {
    pub fn with_citation_quality_floor(floor: f32) -> Self {
        Adapter {
            core: cog::Core::with_citation_quality_floor(floor),
        }
    }
}

impl MemorySystem for Adapter {
    fn name(&self) -> &'static str {
        "cogcore"
    }

    fn observe(&mut self, event: &Event) -> Receipt {
        let stored = event_to_stored(event);
        let r = self.core.observe(stored);
        receipt_to_bench(r)
    }

    fn recall(&mut self, query: &Query) -> RecallResult {
        let q = query_to_cog(query);
        let r = self.core.recall(&q);
        recall_to_bench(r, query, &self.core)
    }

    fn recall_at(&mut self, query: &Query, world_time: &str) -> RecallResult {
        let q = query_to_cog(query);
        let r = self.core.recall_at(&q, world_time);
        recall_to_bench(r, query, &self.core)
    }

    fn recall_as_of(&mut self, query: &Query, tx_time: &str) -> RecallResult {
        let q = query_to_cog(query);
        let r = self.core.recall_as_of(&q, tx_time);
        recall_to_bench(r, query, &self.core)
    }

    fn feedback(&mut self, _pack_id: &str, outcome: &Feedback) -> Receipt {
        let signal = FeedbackSignal {
            outcome: outcome_to_cog(outcome.outcome),
            used: outcome.used.clone(),
        };
        receipt_to_bench(self.core.feedback(&signal))
    }

    fn forget(&mut self, memory_id: &str, reason: &str) -> Tombstone {
        let t = self.core.forget(memory_id, reason);
        Tombstone {
            memory_id: t.memory_id,
            reason: t.reason,
            deletion_proof: t.deletion_proof,
            deleted_at: t.deleted_at,
        }
    }

    fn rebuild(&mut self) -> Receipt {
        receipt_to_bench(self.core.rebuild())
    }

    fn export_state_hash(&self) -> String {
        self.core.export_state_hash()
    }
}

// ───────── translation helpers ─────────

fn event_to_stored(e: &Event) -> StoredEvent {
    StoredEvent {
        id: e.id.clone(),
        kind: event_kind_name(&e.kind).to_string(),
        subject: e.subject.clone(),
        body: e.body.clone(),
        tx_time: e.tx_time.clone(),
        valid_from: e.valid_from.clone(),
        valid_to: e.valid_to.clone(),
        privacy_class: privacy_to_cog(e.privacy_class),
        claim_modality: e.claim_modality.map(modality_to_cog),
        tags: e.tags.clone(),
        sources: e
            .sources
            .iter()
            .map(|s| SourceRef {
                uri: s.uri.clone(),
                citation: s.citation.clone(),
                quality: s.quality,
            })
            .collect(),
        supersedes: e.supersedes.clone(),
        contradicts: e.contradicts.clone(),
    }
}

fn query_to_cog(q: &Query) -> cog::RecallQuery {
    cog::RecallQuery {
        text: q.text.clone(),
        mentions: q.mentions.clone(),
        intent: intent_to_cog(q.intent),
        token_budget: q.token_budget,
    }
}

fn recall_to_bench(r: cog::RecallData, query: &Query, core: &cog::Core) -> RecallResult {
    let mut out = RecallResult {
        answer: r.answer,
        citations: r
            .citations
            .into_iter()
            .map(|c| crate::Citation {
                source_uri: c.uri,
                citation: c.citation,
                quote: None,
            })
            .collect(),
        warnings: r.warnings.into_iter().map(warning_to_bench).collect(),
        used_ids: r.used_ids,
        confidence: r.confidence,
        claim_modality: r.claim_modality.map(modality_to_bench),
        state_bytes: core.state_bytes(),
        ..RecallResult::default()
    };
    let body_len = out.answer.len() as u32;
    out.context_token_count = body_len / 4;
    out.retrieved_token_count = (body_len + r.omitted_bytes) / 4;
    if r.omitted_bytes > 0 {
        out.omitted.push(OmissionNote {
            reason: "budget_or_redaction".to_string(),
            kind: "Pack".to_string(),
            bytes: r.omitted_bytes,
        });
    }
    let _ = query; // reserved for Phase 2 (per-query knobs influence the hash)
    out.context_pack_hash = pack_hash(&out);
    out
}

fn receipt_to_bench(r: cog::Receipt) -> Receipt {
    Receipt {
        event_id: r.event_id,
        mutation_id: r.mutation_id,
        at: r.at,
        previous_hash: r.previous_hash,
        hash: r.hash,
    }
}

fn event_kind_name(k: &EventKind) -> &'static str {
    match k {
        EventKind::Observation => "Observation",
        EventKind::Claim => "Claim",
        EventKind::Equation => "Equation",
        EventKind::Theorem => "Theorem",
        EventKind::Skill => "Skill",
        EventKind::Resource => "Resource",
        EventKind::Dataset => "Dataset",
        EventKind::Experiment => "Experiment",
        EventKind::Hypothesis => "Hypothesis",
        EventKind::Counterexample => "Counterexample",
        EventKind::Lesson => "Lesson",
        EventKind::Question => "Question",
        EventKind::VaultCanary => "VaultCanary",
        EventKind::SchemaMigration => "SchemaMigration",
        EventKind::Supersede { .. } => "Supersede",
        EventKind::Feedback => "Feedback",
    }
}

fn privacy_to_cog(p: PrivacyClass) -> CogPrivacy {
    match p {
        PrivacyClass::Public => CogPrivacy::Public,
        PrivacyClass::Internal => CogPrivacy::Internal,
        PrivacyClass::Confidential => CogPrivacy::Confidential,
        PrivacyClass::Secret => CogPrivacy::Secret,
        PrivacyClass::Vault => CogPrivacy::Vault,
    }
}

fn modality_to_cog(m: ClaimModality) -> cog::ClaimModality {
    match m {
        ClaimModality::Observed => cog::ClaimModality::Observed,
        ClaimModality::AssertedBySource => cog::ClaimModality::AssertedBySource,
        ClaimModality::InferredByAgent => cog::ClaimModality::InferredByAgent,
        ClaimModality::HumanApproved => cog::ClaimModality::HumanApproved,
        ClaimModality::FormallyVerified => cog::ClaimModality::FormallyVerified,
    }
}

fn modality_to_bench(m: cog::ClaimModality) -> ClaimModality {
    match m {
        cog::ClaimModality::Observed => ClaimModality::Observed,
        cog::ClaimModality::AssertedBySource => ClaimModality::AssertedBySource,
        cog::ClaimModality::InferredByAgent => ClaimModality::InferredByAgent,
        cog::ClaimModality::HumanApproved => ClaimModality::HumanApproved,
        cog::ClaimModality::FormallyVerified => ClaimModality::FormallyVerified,
    }
}

fn intent_to_cog(i: QueryIntent) -> CogIntent {
    match i {
        QueryIntent::Fact => CogIntent::Fact,
        QueryIntent::Equation => CogIntent::Equation,
        QueryIntent::Theorem => CogIntent::Theorem,
        QueryIntent::Citation => CogIntent::Citation,
        QueryIntent::Coref => CogIntent::Coref,
        QueryIntent::Procedure => CogIntent::Procedure,
        QueryIntent::Workflow => CogIntent::Workflow,
        QueryIntent::Contradiction => CogIntent::Contradiction,
        QueryIntent::Recall => CogIntent::Recall,
        QueryIntent::HistoryAt => CogIntent::HistoryAt,
        QueryIntent::HistoryAsOf => CogIntent::HistoryAsOf,
        QueryIntent::Forget => CogIntent::Forget,
        QueryIntent::Mixed => CogIntent::Mixed,
    }
}

fn outcome_to_cog(o: Outcome) -> CogOutcome {
    match o {
        Outcome::TaskSuccess => CogOutcome::TaskSuccess,
        Outcome::TaskFailure => CogOutcome::TaskFailure,
        Outcome::Verified => CogOutcome::Verified,
        Outcome::Falsified => CogOutcome::Falsified,
        Outcome::Ignored => CogOutcome::Ignored,
    }
}

fn warning_to_bench(w: cog::Warning) -> Warning {
    match w {
        cog::Warning::Superseded => Warning::Superseded,
        cog::Warning::Contradicted => Warning::Contradicted,
        cog::Warning::Redacted => Warning::Redacted,
        cog::Warning::CausalMaskApplied => Warning::CausalMaskApplied,
        cog::Warning::SkeptikSurfaced => Warning::SkeptikSurfaced,
        cog::Warning::UnitMismatch => Warning::UnitMismatch,
        cog::Warning::Abstained => Warning::Abstained,
        cog::Warning::UnsafeToolRefused => Warning::UnsafeToolRefused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Event, EventKind, MemorySystem, PrivacyClass, Query, QueryIntent, Source};

    fn ev(id: &str, subject: &str, body: &str, tx: &str) -> Event {
        Event {
            id: id.to_string(),
            kind: EventKind::Claim,
            subject: subject.to_string(),
            body: body.to_string(),
            sources: vec![Source {
                uri: "doi:example".to_string(),
                citation: "Example et al. 2024".to_string(),
                quality: 0.9,
            }],
            valid_from: Some("2020-01-01T00:00:00Z".to_string()),
            valid_to: None,
            tx_time: tx.to_string(),
            event_time: None,
            observation_time: None,
            review_time: None,
            policy_time: None,
            dependencies: vec![],
            supersedes: vec![],
            contradicts: vec![],
            derived_from: vec![],
            namespace: None,
            privacy_class: PrivacyClass::Public,
            claim_modality: Some(ClaimModality::Observed),
            tags: vec![],
        }
    }

    fn q(text: &str) -> Query {
        Query {
            text: text.to_string(),
            intent: QueryIntent::Recall,
            mentions: vec![text.to_string()],
            token_budget: 4096,
        }
    }

    #[test]
    fn observe_then_recall_round_trip() {
        let mut a = Adapter::default();
        a.observe(&ev(
            "e1",
            "neutrino",
            "neutrinos have mass",
            "2020-01-01T00:00:00Z",
        ));
        let r = a.recall(&q("neutrino"));
        assert!(r.used_ids.contains(&"e1".to_string()));
        assert!(!r.context_pack_hash.is_empty());
    }

    #[test]
    fn recall_as_of_masks_future() {
        let mut a = Adapter::default();
        a.observe(&ev("old", "subj", "old fact", "2020-01-01T00:00:00Z"));
        a.observe(&ev("new", "subj", "new fact", "2025-01-01T00:00:00Z"));
        let r = a.recall_as_of(&q("subj"), "2022-06-01T00:00:00Z");
        assert!(r.used_ids.contains(&"old".to_string()));
        assert!(!r.used_ids.contains(&"new".to_string()));
        assert!(r.warnings.contains(&Warning::CausalMaskApplied));
    }

    #[test]
    fn export_state_hash_stable() {
        let mut a = Adapter::default();
        a.observe(&ev("e1", "x", "y", "2020-01-01T00:00:00Z"));
        let h1 = a.export_state_hash();
        let h2 = a.export_state_hash();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);
    }
}
