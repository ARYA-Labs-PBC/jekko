use super::paper_json_parse::parse_object;
use super::paper_json_support::{
    parse_json_classifiers, parse_source_array_with_default, parse_string_array_with_default,
    parse_string_or_default, parse_string_or_empty, parse_string_or_none,
};
use super::paper_support::build_stored_event;
use crate::core::StoredEvent;

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
    let (privacy_class, claim_modality) = parse_json_classifiers(&map);
    let id = parse_string_or_empty(&map, "id");
    let kind = parse_string_or_default(&map, "kind", "Claim");
    let subject = parse_string_or_none(&map, "subject")?;
    let body = parse_string_or_empty(&map, "body");
    let tx_time = parse_string_or_empty(&map, "tx_time");

    Some(build_stored_event(
        id,
        &kind,
        subject,
        body,
        tx_time,
        parse_string_or_none(&map, "valid_from"),
        parse_string_or_none(&map, "valid_to"),
        privacy_class,
        claim_modality,
        parse_string_array_with_default(&map, "tags"),
        parse_source_array_with_default(&map, "sources"),
    ))
}
