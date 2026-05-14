//! Paper ingestion — sections, claims, equations, theorems extracted into
//! StoredEvent streams.

use std::collections::BTreeMap;

use crate::core::{ClaimModality, PrivacyClass, SourceRef, StoredEvent};
use crate::ingest::equation::extract_equations;
use crate::ingest::theorem::extract_theorems;
use crate::ingest::IngestBackend;

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
        let tx_time = paper
            .published_at
            .clone()
            .unwrap_or_else(|| "2026-01-01T00:00:00Z".to_string());
        let valid_from = Some(tx_time.clone());
        let sources: Vec<SourceRef> = paper
            .sources
            .iter()
            .map(|s| SourceRef {
                uri: s.uri.clone(),
                citation: s.citation.clone(),
                quality: s.quality,
            })
            .collect();

        // Abstract event (one Claim)
        if !paper.abstract_text.is_empty() {
            events.push(StoredEvent {
                id: String::new(),
                kind: "Claim".to_string(),
                subject: paper.canonical_subject.clone(),
                body: format!("Abstract: {}", paper.abstract_text),
                tx_time: tx_time.clone(),
                valid_from: valid_from.clone(),
                valid_to: None,
                privacy_class: PrivacyClass::Public,
                claim_modality: Some(modality),
                tags: tags.clone(),
                sources: sources.clone(),
                supersedes: Vec::new(),
                contradicts: Vec::new(),
            });
        }

        // Per-section events
        for section in &paper.sections {
            events.push(StoredEvent {
                id: String::new(),
                kind: "Claim".to_string(),
                subject: paper.canonical_subject.clone(),
                body: section.text.clone(),
                tx_time: tx_time.clone(),
                valid_from: valid_from.clone(),
                valid_to: None,
                privacy_class: PrivacyClass::Public,
                claim_modality: Some(modality),
                tags: tags.clone(),
                sources: sources.clone(),
                supersedes: Vec::new(),
                contradicts: Vec::new(),
            });

            // Extract equations from the section
            for eq in extract_equations(&section.text) {
                events.push(StoredEvent {
                    id: String::new(),
                    kind: "Equation".to_string(),
                    subject: paper.canonical_subject.clone(),
                    body: format!(
                        "{} {} {} [{}]",
                        eq.lhs,
                        eq.op,
                        eq.rhs,
                        eq.units.as_deref().unwrap_or("")
                    ),
                    tx_time: tx_time.clone(),
                    valid_from: valid_from.clone(),
                    valid_to: None,
                    privacy_class: PrivacyClass::Public,
                    claim_modality: Some(modality),
                    tags: tags.clone(),
                    sources: sources.clone(),
                    supersedes: Vec::new(),
                    contradicts: Vec::new(),
                });
            }

            // Extract theorems from the section
            for thm in extract_theorems(&section.text) {
                events.push(StoredEvent {
                    id: String::new(),
                    kind: "Theorem".to_string(),
                    subject: paper.canonical_subject.clone(),
                    body: format!("{} {}: {}", thm.kind, thm.name, thm.statement),
                    tx_time: tx_time.clone(),
                    valid_from: valid_from.clone(),
                    valid_to: None,
                    privacy_class: PrivacyClass::Public,
                    claim_modality: Some(modality),
                    tags: tags.clone(),
                    sources: sources.clone(),
                    supersedes: Vec::new(),
                    contradicts: Vec::new(),
                });
            }
        }

        events
    }
}

/// Minimal JSON-line parser for StoredEvent shape. Handles the limited
/// surface produced by qbank-builder's `emit-cogcore` command. Cogcore is
/// zero-deps, so this is a small hand-rolled parser. Returns None on any
/// parse failure (caller logs/skips).
pub fn parse_jsonl_event(line: &str) -> Option<StoredEvent> {
    let line = line.trim();
    if line.is_empty() || !line.starts_with('{') {
        return None;
    }
    let map = parse_object(line)?;
    Some(StoredEvent {
        id: get_string(&map, "id").unwrap_or_default(),
        kind: get_string(&map, "kind").unwrap_or_else(|| "Claim".to_string()),
        subject: get_string(&map, "subject")?,
        body: get_string(&map, "body").unwrap_or_default(),
        tx_time: get_string(&map, "tx_time").unwrap_or_else(|| "2026-01-01T00:00:00Z".to_string()),
        valid_from: get_string(&map, "valid_from"),
        valid_to: get_string(&map, "valid_to"),
        privacy_class: get_string(&map, "privacy_class")
            .as_deref()
            .map(|s| match s {
                "Internal" => PrivacyClass::Internal,
                "Confidential" => PrivacyClass::Confidential,
                "Secret" => PrivacyClass::Secret,
                "Vault" => PrivacyClass::Vault,
                _ => PrivacyClass::Public,
            })
            .unwrap_or(PrivacyClass::Public),
        claim_modality: get_string(&map, "claim_modality")
            .as_deref()
            .and_then(|s| match s {
                "Observed" => Some(ClaimModality::Observed),
                "AssertedBySource" => Some(ClaimModality::AssertedBySource),
                "InferredByAgent" => Some(ClaimModality::InferredByAgent),
                "HumanApproved" => Some(ClaimModality::HumanApproved),
                "FormallyVerified" => Some(ClaimModality::FormallyVerified),
                _ => None,
            }),
        tags: get_string_array(&map, "tags").unwrap_or_default(),
        sources: get_source_array(&map, "sources").unwrap_or_default(),
        supersedes: get_string_array(&map, "supersedes").unwrap_or_default(),
        contradicts: get_string_array(&map, "contradicts").unwrap_or_default(),
    })
}

// ── Hand-rolled minimal JSON object parser ─────────────────────────────
// Handles: string values, null, bool, number (returned as string), nested
// objects (returned as raw &str slice), arrays of strings, arrays of
// objects with {uri, citation, quality}. Not a full JSON parser — only
// the StoredEvent shape.

fn parse_object(s: &str) -> Option<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    let s = s.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return None;
    }
    let inner = &s[1..s.len() - 1];
    let bytes = inner.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Skip whitespace and commas between entries.
        while i < bytes.len() && ((bytes[i] as char).is_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] != b'"' {
            return None;
        }
        let (key, after_key) = read_string(inner, i)?;
        i = after_key;
        // Skip whitespace and the colon separator.
        while i < bytes.len() && ((bytes[i] as char).is_whitespace() || bytes[i] == b':') {
            i += 1;
        }
        if i >= bytes.len() {
            return None;
        }
        let (val, after_val) = read_value(inner, i)?;
        map.insert(key, val);
        i = after_val;
    }
    Some(map)
}

fn read_string(s: &str, start: usize) -> Option<(String, usize)> {
    let bytes = s.as_bytes();
    if start >= bytes.len() || bytes[start] != b'"' {
        return None;
    }
    let mut i = start + 1;
    let mut out = String::new();
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                other => out.push(other as char),
            }
            i += 2;
        } else if bytes[i] == b'"' {
            return Some((out, i + 1));
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    None
}

fn read_value(s: &str, start: usize) -> Option<(String, usize)> {
    let bytes = s.as_bytes();
    let mut i = start;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    match bytes[i] {
        b'"' => {
            let (s_val, end) = read_string(s, i)?;
            Some((s_val, end))
        }
        b'n' => {
            if i + 4 <= bytes.len() && &s[i..i + 4] == "null" {
                Some(("null".to_string(), i + 4))
            } else {
                None
            }
        }
        b't' => {
            if i + 4 <= bytes.len() && &s[i..i + 4] == "true" {
                Some(("true".to_string(), i + 4))
            } else {
                None
            }
        }
        b'f' => {
            if i + 5 <= bytes.len() && &s[i..i + 5] == "false" {
                Some(("false".to_string(), i + 5))
            } else {
                None
            }
        }
        b'[' | b'{' => {
            // Capture balanced delimiters and return the raw slice. Strings
            // inside are skipped so brackets inside string literals do not
            // unbalance the depth counter.
            let open = bytes[i] as char;
            let close = if open == '[' { ']' } else { '}' };
            let mut depth = 1;
            let mut j = i + 1;
            let mut in_str = false;
            while j < bytes.len() && depth > 0 {
                let c = bytes[j] as char;
                if in_str {
                    if c == '\\' {
                        j += 2;
                        continue;
                    }
                    if c == '"' {
                        in_str = false;
                    }
                } else if c == '"' {
                    in_str = true;
                } else if c == open {
                    depth += 1;
                } else if c == close {
                    depth -= 1;
                }
                j += 1;
            }
            if depth != 0 {
                return None;
            }
            Some((s[i..j].to_string(), j))
        }
        _ => {
            // Number or bareword. Read until terminator.
            let mut j = i;
            while j < bytes.len() {
                let c = bytes[j] as char;
                if c == ',' || c == '}' || c == ']' || c.is_whitespace() {
                    break;
                }
                j += 1;
            }
            Some((s[i..j].to_string(), j))
        }
    }
}

fn get_string(map: &BTreeMap<String, String>, key: &str) -> Option<String> {
    let v = map.get(key)?;
    if v == "null" {
        return None;
    }
    Some(v.clone())
}

fn get_string_array(map: &BTreeMap<String, String>, key: &str) -> Option<Vec<String>> {
    let raw = map.get(key)?;
    if raw == "null" {
        return None;
    }
    if !raw.starts_with('[') || !raw.ends_with(']') {
        return None;
    }
    let inner = &raw[1..raw.len() - 1];
    let mut out = Vec::new();
    let bytes = inner.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && ((bytes[i] as char).is_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] != b'"' {
            return None;
        }
        let (s_val, end) = read_string(inner, i)?;
        out.push(s_val);
        i = end;
    }
    Some(out)
}

fn get_source_array(map: &BTreeMap<String, String>, key: &str) -> Option<Vec<SourceRef>> {
    let raw = map.get(key)?;
    if raw == "null" {
        return None;
    }
    if !raw.starts_with('[') || !raw.ends_with(']') {
        return None;
    }
    let inner = &raw[1..raw.len() - 1];
    let mut out = Vec::new();
    let bytes = inner.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && ((bytes[i] as char).is_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] != b'{' {
            return None;
        }
        let (obj_str, end) = read_value(inner, i)?;
        i = end;
        let obj_map = parse_object(&obj_str)?;
        out.push(SourceRef {
            uri: obj_map.get("uri").cloned().unwrap_or_default(),
            citation: obj_map.get("citation").cloned().unwrap_or_default(),
            quality: obj_map
                .get("quality")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.9),
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_backend_emits_section_events() {
        let paper = IngestedPaper {
            publication_hash: "h1".to_string(),
            title: "Neutrino Oscillation".to_string(),
            canonical_subject: "neutrino".to_string(),
            published_at: Some("2026-03-01T00:00:00Z".to_string()),
            redistributable: true,
            abstract_text: "We observe oscillation.".to_string(),
            sections: vec![PaperSection {
                section_id: "s1".to_string(),
                title: "Methods".to_string(),
                text: "We measure delta m^2 = 7.5e-5 [eV^2].".to_string(),
                section_hash: "sh1".to_string(),
            }],
            sources: vec![SourceSpec {
                uri: "doi:10.1234/x".to_string(),
                citation: "Smith 2026".to_string(),
                quality: 0.95,
            }],
            tags: vec!["arxiv".to_string()],
            dev_only: false,
        };
        let events = RuleBackend.ingest_paper(&paper);
        // Abstract + 1 section + at least 1 equation
        assert!(
            events.len() >= 2,
            "expected >=2 events, got {}",
            events.len()
        );
        assert!(events.iter().any(|e| e.kind == "Equation"));
        assert!(events.iter().all(|e| e.subject == "neutrino"));
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
        let paper = IngestedPaper {
            publication_hash: "h2".to_string(),
            title: "T".to_string(),
            canonical_subject: "sub".to_string(),
            published_at: None,
            redistributable: false,
            abstract_text: "abstract".to_string(),
            sections: vec![],
            sources: vec![],
            tags: vec![],
            dev_only: true,
        };
        let events = RuleBackend.ingest_paper(&paper);
        assert!(!events.is_empty());
        assert!(events
            .iter()
            .all(|e| e.tags.contains(&"dev_only".to_string())));
    }
}
