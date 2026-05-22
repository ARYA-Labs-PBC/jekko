use crate::core::{ClaimModality, PrivacyClass, SourceRef, StoredEvent};
use crate::ingest::equation::EqAtom;
use crate::ingest::paper::PaperSection;
#[cfg(test)]
use crate::ingest::paper::{IngestedPaper, SourceSpec};
use crate::ingest::theorem::TheoremRef;

#[derive(Debug)]
pub(crate) struct PaperEventCtx {
    pub(crate) subject: String,
    pub(crate) modality: ClaimModality,
    pub(crate) tx_time: String,
    pub(crate) valid_from: Option<String>,
    pub(crate) sources: Vec<SourceRef>,
    pub(crate) tags: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_stored_event(
    id: String,
    kind: &str,
    subject: String,
    body: String,
    tx_time: String,
    valid_from: Option<String>,
    valid_to: Option<String>,
    privacy_class: PrivacyClass,
    claim_modality: Option<ClaimModality>,
    tags: Vec<String>,
    sources: Vec<SourceRef>,
) -> StoredEvent {
    StoredEvent {
        id,
        kind: kind.to_string(),
        subject,
        body,
        tx_time,
        valid_from,
        valid_to,
        privacy_class,
        claim_modality,
        tags,
        sources,
        supersedes: Vec::new(),
        contradicts: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_event(ctx: &PaperEventCtx, kind: &str, body: String) -> StoredEvent {
    build_stored_event(
        String::new(),
        kind,
        ctx.subject.clone(),
        body,
        ctx.tx_time.clone(),
        ctx.valid_from.clone(),
        None,
        PrivacyClass::Public,
        Some(ctx.modality),
        ctx.tags.clone(),
        ctx.sources.clone(),
    )
}

pub(crate) fn map_section_body(section: &PaperSection) -> String {
    section.text.clone()
}

pub(crate) fn map_equation_body(eq: &EqAtom) -> String {
    format!(
        "{} {} {} [{}]",
        eq.lhs,
        eq.op,
        eq.rhs,
        eq.units.as_deref().unwrap_or("")
    )
}

pub(crate) fn map_theorem_body(thm: &TheoremRef) -> String {
    format!("{} {}: {}", thm.kind, thm.name, thm.statement)
}

#[cfg(test)]
fn paper_source(uri: &str, citation: &str, quality: f32) -> SourceSpec {
    SourceSpec {
        uri: uri.to_string(),
        citation: citation.to_string(),
        quality,
    }
}

#[cfg(test)]
fn section(id: &str, title: &str, text: &str, section_hash: &str) -> PaperSection {
    PaperSection {
        section_id: id.to_string(),
        title: title.to_string(),
        text: text.to_string(),
        section_hash: section_hash.to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn paper_fixture(
    publication_hash: &str,
    title: &str,
    canonical_subject: &str,
    published_at: Option<&str>,
    redistributable: bool,
    abstract_text: &str,
    sections: Vec<PaperSection>,
    sources: Vec<SourceSpec>,
    tags: Vec<&str>,
    dev_only: bool,
) -> IngestedPaper {
    IngestedPaper {
        publication_hash: publication_hash.to_string(),
        title: title.to_string(),
        canonical_subject: canonical_subject.to_string(),
        published_at: published_at.map(str::to_string),
        redistributable,
        abstract_text: abstract_text.to_string(),
        sections,
        sources,
        tags: tags.into_iter().map(str::to_string).collect(),
        dev_only,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ClaimModality;
    use crate::ingest::paper::RuleBackend;
    use crate::ingest::paper_json::parse_jsonl_event;
    use crate::ingest::IngestBackend;

    #[test]
    fn rule_backend_emits_section_eq_theorem_claim_events() {
        let paper = paper_fixture(
            "h1",
            "Neutrino Oscillation",
            "neutrino",
            Some("2026-03-01T00:00:00Z"),
            true,
            "We observe oscillation.",
            vec![section(
                "s1",
                "Methods",
                "We measure delta m^2 = 7.5e-5 [eV^2]. Theorem 3.1: Every oscillation is periodic.",
                "sh1",
            )],
            vec![paper_source("doi:10.1234/x", "Smith 2026", 0.95)],
            vec!["arxiv"],
            false,
        );
        let events = RuleBackend.ingest_paper(&paper);
        assert_eq!(events.len(), 4);
        assert!(events.iter().any(|e| e.kind == "Equation"));
        assert!(events.iter().any(|e| e.kind == "Theorem"));
        assert!(events.iter().all(|e| e.subject == "neutrino"));
        assert!(events
            .iter()
            .all(|e| e.claim_modality == Some(ClaimModality::FormallyVerified)));
        assert!(events.iter().all(|e| e.tx_time == "2026-03-01T00:00:00Z"));
        assert!(events
            .iter()
            .all(|e| e.valid_from.as_deref() == Some("2026-03-01T00:00:00Z")));
        assert!(events.iter().all(|e| e.tags == vec!["arxiv".to_string()]));
        assert!(events
            .iter()
            .all(|e| e.sources.len() == 1 && e.sources[0].uri == "doi:10.1234/x"));
    }

    #[test]
    fn jsonl_parse_round_trip() {
        let line = r#"{"id":"","kind":"Claim","subject":"neutrino","body":"oscillation observed","tx_time":"2026-03-01T00:00:00Z","valid_from":"2026-01-01T00:00:00Z","valid_to":null,"privacy_class":"Public","claim_modality":"FormallyVerified","tags":["arxiv","neutrino"],"sources":[{"uri":"doi:10.1/x","citation":"Smith 2026","quality":0.95}],"supersedes":[],"contradicts":[]}"#;
        let event = parse_jsonl_event(line).expect("parse must succeed");
        assert_eq!(event.kind, "Claim");
        assert_eq!(event.subject, "neutrino");
        assert_eq!(event.body, "oscillation observed");
        assert_eq!(event.tags, vec!["arxiv", "neutrino"]);
        assert_eq!(event.sources.len(), 1);
        assert_eq!(event.sources[0].uri, "doi:10.1/x");
    }

    #[test]
    fn dev_only_paper_tagged() {
        let paper = paper_fixture(
            "h2",
            "T",
            "sub",
            None,
            false,
            "abstract",
            vec![],
            vec![],
            vec![],
            true,
        );
        let events = RuleBackend.ingest_paper(&paper);
        assert!(!events.is_empty());
        assert_eq!(events[0].tx_time, "");
        assert!(events[0].valid_from.is_none());
        assert!(events
            .iter()
            .all(|e| e.tags.contains(&"dev_only".to_string())));
    }

    #[test]
    fn abstract_section_eq_theorem_events_share_metadata() {
        let paper = paper_fixture(
            "h3",
            "Neutrino Phenomenology",
            "neutrino",
            Some("2026-03-02T12:34:56Z"),
            true,
            "We model neutrino transport.",
            vec![
                section(
                    "s2",
                    "Introduction",
                    "v = c. Theorem A: Light is fast. Lemma A: A supporting claim.",
                    "sh2",
                ),
                section("s3", "Methods", "x = 1", "sh3"),
            ],
            vec![paper_source("arxiv:2301.1", "Doe 2026", 0.99)],
            vec!["arxiv", "vibes"],
            false,
        );
        let events = RuleBackend.ingest_paper(&paper);
        assert!(events.len() >= 6);

        let first = &events[0];
        for e in &events[1..] {
            assert_eq!(e.subject, first.subject);
            assert_eq!(e.claim_modality, first.claim_modality);
            assert_eq!(e.tx_time, first.tx_time);
            assert_eq!(e.valid_from, first.valid_from);
            assert_eq!(e.tags, first.tags);
            assert_eq!(e.sources, first.sources);
        }
    }
}
