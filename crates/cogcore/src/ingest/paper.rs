//! Paper ingestion — sections, claims, equations, theorems extracted into
//! `StoredEvent` streams.

use crate::core::{ClaimModality, SourceRef, StoredEvent};
use crate::ingest::equation::extract_equations;
use crate::ingest::theorem::extract_theorems;
use crate::ingest::IngestBackend;

use super::paper_support::{
    emit_event, map_equation_body, map_section_body, map_theorem_body, PaperEventCtx,
};

/// Cogcore-internal mirror of qbank-builder's `PaperRecord`. Keeping this
/// in cogcore avoids a path dep on the qbank-builder crate (which uses
/// `serde+regex+sha2` and would leak into cogcore's zero-dep contract).
#[derive(Debug, Clone)]
pub struct IngestedPaper {
    /// Hash that identifies this publication (content-addressed).
    pub publication_hash: String,
    /// Human-readable paper title.
    pub title: String,
    /// Canonical subject the events should attach to (e.g. "neutrino").
    pub canonical_subject: String,
    /// Optional ISO-8601 publication timestamp; controls `tx_time` and modality.
    pub published_at: Option<String>,
    /// True when the paper is open-licensed and may be redistributed.
    pub redistributable: bool,
    /// Paper abstract text — emitted as a single `Claim` event.
    pub abstract_text: String,
    /// Ordered list of paper sections — each emits a `Claim` plus extracted atoms.
    pub sections: Vec<PaperSection>,
    /// Source descriptors attached to every emitted event.
    pub sources: Vec<SourceSpec>,
    /// Free-form tags carried verbatim onto every emitted event.
    pub tags: Vec<String>,
    /// True when the paper is gated behind a developer-only license bucket.
    pub dev_only: bool,
}

/// A single section of an `IngestedPaper`.
#[derive(Debug, Clone)]
pub struct PaperSection {
    /// Stable identifier (e.g. `s1`, `methods`).
    pub section_id: String,
    /// Section heading.
    pub title: String,
    /// Body text of the section.
    pub text: String,
    /// Content-addressed hash of the section body.
    pub section_hash: String,
}

/// Source descriptor used as input to ingest. Mirrors `SourceRef` without
/// taking a dependency on `core::SourceRef` for the surface API (which lets
/// the qbank-builder crate stay in its own dep universe).
#[derive(Debug, Clone)]
pub struct SourceSpec {
    /// Identifier URI (DOI, arXiv ID, etc.).
    pub uri: String,
    /// Human-readable citation string.
    pub citation: String,
    /// Provenance quality score in `[0.0, 1.0]`.
    pub quality: f32,
}

/// Rule-based ingest backend. Stateless and deterministic.
#[derive(Debug, Default)]
pub struct RuleBackend;

impl IngestBackend for RuleBackend {
    fn ingest_paper(&self, paper: &IngestedPaper) -> Vec<StoredEvent> {
        let mut events = Vec::new();
        let mut tags = paper.tags.clone();
        if paper.dev_only {
            tags.push("dev_only".to_string());
        }
        let modality = if paper.redistributable && paper.published_at.is_some() {
            ClaimModality::FormallyVerified
        } else {
            ClaimModality::AssertedBySource
        };
        let (tx_time, valid_from) = if let Some(ts) = paper.published_at.clone() {
            (ts.clone(), Some(ts))
        } else {
            (String::new(), None)
        };
        let sources: Vec<SourceRef> = paper
            .sources
            .iter()
            .map(|s| SourceRef {
                uri: s.uri.clone(),
                citation: s.citation.clone(),
                quality: s.quality,
            })
            .collect();

        let ctx = PaperEventCtx {
            subject: paper.canonical_subject.clone(),
            modality,
            tx_time,
            valid_from,
            sources,
            tags,
        };

        // Abstract event (one Claim)
        if !paper.abstract_text.is_empty() {
            events.push(emit_event(
                &ctx,
                "Claim",
                format!("Abstract: {}", paper.abstract_text),
            ));
        }

        // Per-section events
        for section in &paper.sections {
            events.push(emit_event(&ctx, "Claim", map_section_body(section)));

            // Extract equations from the section
            for eq in extract_equations(&section.text) {
                events.push(emit_event(&ctx, "Equation", map_equation_body(&eq)));
            }

            // Extract theorems from the section
            for thm in extract_theorems(&section.text) {
                events.push(emit_event(&ctx, "Theorem", map_theorem_body(&thm)));
            }
        }

        events
    }
}
