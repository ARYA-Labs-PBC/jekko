//! Helper functions used across adapters and binaries.
//!
//! Deterministic: no `Instant::now()`, no `Utc::now()`, no random.

use crate::hash::{fnv1a_hex, fnv1a_seq_hex};
use crate::json::{self, Json};
use crate::{
    AxisScores, Citation, ClaimModality, OmissionNote, RecallResult, Redaction, SkillCall, Source,
    Warning,
};

/// Canonical "now" used by deterministic bench runs. Fixture-controlled.
/// Adapters should never call `std::time::SystemTime::now()` in the hot path.
pub const BENCH_NOW: &str = "2026-05-12T00:00:00Z";

/// Canonical content-hash of a RecallResult. Used both for determinism checks
/// and as the `context_pack_hash` field on the result itself.
pub fn pack_hash(r: &RecallResult) -> String {
    let canonical = r.to_canonical_json();
    fnv1a_hex(&canonical)
}

impl RecallResult {
    pub fn empty() -> Self {
        RecallResult::default()
    }

    pub fn to_canonical_json(&self) -> String {
        let mut o = Json::obj();
        o.insert("answer".to_string(), Json::Str(self.answer.clone()));
        o.insert(
            "citations".to_string(),
            Json::Array(self.citations.iter().map(Citation::to_json).collect()),
        );
        o.insert(
            "warnings".to_string(),
            Json::Array(
                self.warnings
                    .iter()
                    .map(|w| Json::Str(w.name().to_string()))
                    .collect(),
            ),
        );
        o.insert(
            "claims".to_string(),
            Json::Array(
                self.claims
                    .iter()
                    .map(|claim| {
                        json::obj(&[
                            ("id", Json::Str(claim.id.clone())),
                            ("text", Json::Str(claim.text.clone())),
                            ("status", Json::Str(format!("{:?}", claim.status))),
                            ("support", json::arr_str(claim.support.clone())),
                        ])
                    })
                    .collect(),
            ),
        );
        o.insert(
            "omitted".to_string(),
            Json::Array(self.omitted.iter().map(OmissionNote::to_json).collect()),
        );
        o.insert(
            "redactions".to_string(),
            Json::Array(self.redactions.iter().map(Redaction::to_json).collect()),
        );
        o.insert(
            "skill_calls".to_string(),
            Json::Array(self.skill_calls.iter().map(SkillCall::to_json).collect()),
        );
        o.insert(
            "used_ids".to_string(),
            Json::Array(self.used_ids.iter().map(|s| Json::Str(s.clone())).collect()),
        );
        o.insert(
            "excluded_ids".to_string(),
            Json::Array(
                self.excluded_ids
                    .iter()
                    .map(|s| Json::Str(s.clone()))
                    .collect(),
            ),
        );
        o.insert(
            "derived_from".to_string(),
            Json::Array(
                self.derived_from
                    .iter()
                    .map(|s| Json::Str(s.clone()))
                    .collect(),
            ),
        );
        o.insert(
            "confidence".to_string(),
            Json::Float(self.confidence as f64),
        );
        o.insert(
            "context_token_count".to_string(),
            Json::Int(self.context_token_count as i64),
        );
        o.insert(
            "retrieved_token_count".to_string(),
            Json::Int(self.retrieved_token_count as i64),
        );
        o.insert(
            "state_bytes".to_string(),
            Json::Int(self.state_bytes as i64),
        );
        o.insert(
            "claim_modality".to_string(),
            match self.claim_modality {
                Some(m) => Json::Str(modality_name(m).to_string()),
                None => Json::Null,
            },
        );
        // context_pack_hash deliberately omitted — it's a hash OF this struct.
        Json::Object(o).to_string()
    }
}

impl Citation {
    pub fn from_source(s: &Source) -> Self {
        Citation {
            source_uri: s.uri.clone(),
            citation: s.citation.clone(),
            quote: None,
        }
    }
    pub fn to_json(&self) -> Json {
        let mut o = Json::obj();
        o.insert("source_uri".to_string(), Json::Str(self.source_uri.clone()));
        o.insert("citation".to_string(), Json::Str(self.citation.clone()));
        o.insert(
            "quote".to_string(),
            match &self.quote {
                Some(q) => Json::Str(q.clone()),
                None => Json::Null,
            },
        );
        Json::Object(o)
    }
}

impl OmissionNote {
    pub fn to_json(&self) -> Json {
        let mut o = Json::obj();
        o.insert("reason".to_string(), Json::Str(self.reason.clone()));
        o.insert("kind".to_string(), Json::Str(self.kind.clone()));
        o.insert("bytes".to_string(), Json::Int(self.bytes as i64));
        Json::Object(o)
    }
}

impl Redaction {
    pub fn to_json(&self) -> Json {
        json::obj(&[
            ("channel", Json::Str(self.channel.clone())),
            ("reason", Json::Str(self.reason.clone())),
            (
                "evidence_id",
                self.evidence_id
                    .as_ref()
                    .map(|id| Json::Str(id.clone()))
                    .unwrap_or(Json::Null),
            ),
        ])
    }
}

impl SkillCall {
    pub fn to_json(&self) -> Json {
        json::obj(&[
            ("name", Json::Str(self.name.clone())),
            ("args_hash", Json::Str(self.args_hash.clone())),
            ("refused", Json::Bool(self.refused)),
        ])
    }
}

impl Warning {
    pub fn name(&self) -> &'static str {
        match self {
            Warning::Superseded => "superseded",
            Warning::Contradicted => "contradicted",
            Warning::LowConfidence => "low_confidence",
            Warning::Redacted => "redacted",
            Warning::CausalMaskApplied => "causal_mask_applied",
            Warning::UntrustedInstructionLikeContent => "untrusted_instruction_like_content",
            Warning::SkeptikSurfaced => "skeptic_surfaced",
            Warning::UnitMismatch => "unit_mismatch",
            Warning::SchemaMigrated => "schema_migrated",
            Warning::DependencyInvalidated => "dependency_invalidated",
            Warning::CitationUnsupported => "citation_unsupported",
            Warning::CitationBloated => "citation_bloated",
            Warning::CompressionDrift => "compression_drift",
            Warning::PrivacyTransformBlocked => "privacy_transform_blocked",
            Warning::UnsafeToolRefused => "unsafe_tool_refused",
            Warning::Abstained => "abstained",
            Warning::BeliefTimeApplied => "belief_time_applied",
        }
    }
}

pub fn modality_name(m: ClaimModality) -> &'static str {
    match m {
        ClaimModality::Observed => "observed",
        ClaimModality::AssertedBySource => "asserted_by_source",
        ClaimModality::InferredByAgent => "inferred_by_agent",
        ClaimModality::HumanApproved => "human_approved",
        ClaimModality::FormallyVerified => "formally_verified",
    }
}

/// Compare two RFC3339-ish ISO strings ASCII-lex (works because we use
/// `YYYY-MM-DDThh:mm:ssZ` everywhere; lexicographic order = chronological).
pub fn iso_le(a: &str, b: &str) -> bool {
    a <= b
}
pub fn iso_lt(a: &str, b: &str) -> bool {
    a < b
}
pub fn iso_ge(a: &str, b: &str) -> bool {
    a >= b
}

/// Stable id for an event derived from its contents.
pub fn event_canonical_id(kind: &str, subject: &str, body: &str, tx_time: &str) -> String {
    fnv1a_seq_hex(&[kind, subject, body, tx_time])
}

/// Convert AxisScores into a sorted JSON object (for reports).
pub fn axes_to_json(a: &AxisScores) -> Json {
    json::obj(&[
        ("correctness", Json::Float(a.correctness as f64)),
        ("provenance", Json::Float(a.provenance as f64)),
        ("bitemporal_recall", Json::Float(a.bitemporal_recall as f64)),
        ("contradiction", Json::Float(a.contradiction as f64)),
        ("math_science", Json::Float(a.math_science as f64)),
        (
            "english_discourse_coreference",
            Json::Float(a.english_discourse_coreference as f64),
        ),
        ("privacy_redaction", Json::Float(a.privacy_redaction as f64)),
        ("procedural_skill", Json::Float(a.procedural_skill as f64)),
        (
            "feedback_adaptation",
            Json::Float(a.feedback_adaptation as f64),
        ),
        (
            "determinism_rebuild",
            Json::Float(a.determinism_rebuild as f64),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_lex_matches_chrono() {
        assert!(iso_lt("2024-01-01T00:00:00Z", "2024-01-02T00:00:00Z"));
        assert!(iso_lt("2024-01-01T00:00:00Z", "2025-01-01T00:00:00Z"));
        assert!(iso_ge("2026-05-12T00:00:00Z", BENCH_NOW));
    }

    #[test]
    fn event_id_is_deterministic() {
        let a = event_canonical_id("Claim", "neutrino", "has mass", "2026-01-01T00:00:00Z");
        let b = event_canonical_id("Claim", "neutrino", "has mass", "2026-01-01T00:00:00Z");
        assert_eq!(a, b);
    }
}
