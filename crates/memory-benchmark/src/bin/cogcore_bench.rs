use memory_benchmark::adapters::cogcore_adapter;
use memory_benchmark::json::{self, Json};
use memory_benchmark::{ClaimModality, Event, EventKind, MemorySystem, PrivacyClass, Source};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    if let Err(err) = run() {
        eprintln!("cogcore_bench: {err}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    let events_path = value(&args, "--events").ok_or("--events is required")?;
    let out = value(&args, "--out");
    let candidate = value(&args, "--candidate").unwrap_or_else(|| "cogcore".to_string());
    if candidate != "cogcore" {
        return Err(format!(
            "--candidate {candidate:?} is not supported by cogcore_bench"
        ));
    }

    let text =
        fs::read_to_string(&events_path).map_err(|err| format!("read {events_path}: {err}"))?;
    let mut adapter = cogcore_adapter::Adapter::default();
    let mut event_count = 0_i64;
    let mut dev_only = false;
    let mut last_receipt = String::new();
    for (line_idx, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let event = parse_event_line(line)
            .map_err(|err| format!("parse {} line {}: {err}", events_path, line_idx + 1))?;
        if event
            .tags
            .iter()
            .any(|tag| tag == "dev_only" || tag == "dev-only")
        {
            dev_only = true;
        }
        let receipt = adapter.observe(&event);
        last_receipt = receipt.hash;
        event_count += 1;
    }
    let state_hash = adapter.export_state_hash();

    let mut top = Json::obj();
    top.insert("name".to_string(), Json::Str("cogcore_bench".to_string()));
    top.insert("candidate".to_string(), Json::Str(candidate));
    top.insert("events".to_string(), Json::Str(events_path.clone()));
    top.insert("event_count".to_string(), Json::Int(event_count));
    top.insert("dev_only".to_string(), Json::Bool(dev_only));
    top.insert("state_hash".to_string(), Json::Str(state_hash));
    top.insert("last_receipt_hash".to_string(), Json::Str(last_receipt));
    let payload = Json::Object(top).to_string();

    if let Some(path) = out {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create {}: {err}", parent.display()))?;
        }
        fs::write(&path, format!("{payload}\n")).map_err(|err| format!("write {path}: {err}"))?;
    } else {
        println!("{payload}");
    }
    Ok(())
}

fn parse_event_line(line: &str) -> Result<Event, String> {
    let Json::Object(obj) = json::parse(line)? else {
        return Err("event line must be a JSON object".to_string());
    };
    Ok(Event {
        id: str_field(&obj, "id")?,
        kind: parse_kind(&str_field(&obj, "kind")?)?,
        subject: str_field(&obj, "subject")?,
        body: str_field(&obj, "body")?,
        sources: parse_sources(&obj)?,
        valid_from: opt_str_field(&obj, "valid_from")?,
        valid_to: opt_str_field(&obj, "valid_to")?,
        tx_time: str_field(&obj, "tx_time")?,
        event_time: None,
        observation_time: None,
        review_time: None,
        policy_time: None,
        dependencies: Vec::new(),
        supersedes: arr_str_field(&obj, "supersedes")?,
        contradicts: arr_str_field(&obj, "contradicts")?,
        derived_from: Vec::new(),
        namespace: Some("opencode.qbank.cogcore_events".to_string()),
        privacy_class: parse_privacy(&str_field(&obj, "privacy_class")?)?,
        claim_modality: opt_str_field(&obj, "claim_modality")?
            .as_deref()
            .map(parse_modality)
            .transpose()?,
        tags: arr_str_field(&obj, "tags")?,
    })
}

fn str_field(obj: &BTreeMap<String, Json>, key: &str) -> Result<String, String> {
    match obj.get(key) {
        Some(Json::Str(value)) => Ok(value.clone()),
        _ => Err(format!("missing string field {key:?}")),
    }
}

fn opt_str_field(obj: &BTreeMap<String, Json>, key: &str) -> Result<Option<String>, String> {
    match obj.get(key) {
        Some(Json::Str(value)) => Ok(Some(value.clone())),
        Some(Json::Null) | None => Ok(None),
        _ => Err(format!("field {key:?} must be string or null")),
    }
}

fn arr_str_field(obj: &BTreeMap<String, Json>, key: &str) -> Result<Vec<String>, String> {
    match obj.get(key) {
        Some(Json::Array(items)) => items
            .iter()
            .map(|item| match item {
                Json::Str(value) => Ok(value.clone()),
                _ => Err(format!("field {key:?} must contain only strings")),
            })
            .collect(),
        Some(Json::Null) | None => Ok(Vec::new()),
        _ => Err(format!("field {key:?} must be an array")),
    }
}

fn parse_sources(obj: &BTreeMap<String, Json>) -> Result<Vec<Source>, String> {
    match obj.get("sources") {
        Some(Json::Array(items)) => items.iter().map(parse_source).collect(),
        Some(Json::Null) | None => Ok(Vec::new()),
        _ => Err("field \"sources\" must be an array".to_string()),
    }
}

fn parse_source(value: &Json) -> Result<Source, String> {
    let Json::Object(obj) = value else {
        return Err("source must be a JSON object".to_string());
    };
    Ok(Source {
        uri: str_field(obj, "uri")?,
        citation: str_field(obj, "citation")?,
        quality: number_field(obj, "quality")? as f32,
    })
}

fn number_field(obj: &BTreeMap<String, Json>, key: &str) -> Result<f64, String> {
    match obj.get(key) {
        Some(Json::Float(value)) => Ok(*value),
        Some(Json::Int(value)) => Ok(*value as f64),
        _ => Err(format!("missing number field {key:?}")),
    }
}

fn parse_kind(value: &str) -> Result<EventKind, String> {
    match value {
        "Observation" => Ok(EventKind::Observation),
        "Claim" => Ok(EventKind::Claim),
        "Equation" => Ok(EventKind::Equation),
        "Theorem" => Ok(EventKind::Theorem),
        "Skill" => Ok(EventKind::Skill),
        "Resource" => Ok(EventKind::Resource),
        "Dataset" => Ok(EventKind::Dataset),
        "Experiment" => Ok(EventKind::Experiment),
        "Hypothesis" => Ok(EventKind::Hypothesis),
        "Counterexample" => Ok(EventKind::Counterexample),
        "Lesson" => Ok(EventKind::Lesson),
        "Question" => Ok(EventKind::Question),
        other => Err(format!("unsupported event kind {other:?}")),
    }
}

fn parse_privacy(value: &str) -> Result<PrivacyClass, String> {
    match value {
        "Public" => Ok(PrivacyClass::Public),
        "Internal" => Ok(PrivacyClass::Internal),
        "Confidential" => Ok(PrivacyClass::Confidential),
        "Secret" => Ok(PrivacyClass::Secret),
        "Vault" => Ok(PrivacyClass::Vault),
        other => Err(format!("unsupported privacy class {other:?}")),
    }
}

fn parse_modality(value: &str) -> Result<ClaimModality, String> {
    match value {
        "Observed" => Ok(ClaimModality::Observed),
        "AssertedBySource" => Ok(ClaimModality::AssertedBySource),
        "InferredByAgent" => Ok(ClaimModality::InferredByAgent),
        "HumanApproved" => Ok(ClaimModality::HumanApproved),
        "FormallyVerified" => Ok(ClaimModality::FormallyVerified),
        other => Err(format!("unsupported claim modality {other:?}")),
    }
}

fn value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stored_event_json_line() {
        let line = r#"{"body":"Alpha equals one.","claim_modality":"AssertedBySource","contradicts":[],"id":"","kind":"Claim","privacy_class":"Public","sources":[{"citation":"Alpha :: Result","quality":0.95,"uri":"qbank://paper/p/s1"}],"subject":"Alpha Paper","supersedes":[],"tags":["qbank","paper-section","topic:alpha"],"tx_time":"2026-01-01T00:00:00Z","valid_from":"2026-01-01T00:00:00Z","valid_to":null}"#;
        let event = parse_event_line(line).expect("event");
        assert_eq!(event.id, "");
        assert_eq!(event.subject, "Alpha Paper");
        assert_eq!(event.sources[0].quality, 0.95);
        assert_eq!(event.claim_modality, Some(ClaimModality::AssertedBySource));
        assert!(event.tags.contains(&"topic:alpha".to_string()));
    }

    #[test]
    fn rejects_answer_key_metadata_shape() {
        let line = r#"{"answer_key":"do not ingest","body":"Alpha","claim_modality":"AssertedBySource","contradicts":[],"id":"","kind":"Claim","privacy_class":"Public","sources":[],"subject":"Alpha","supersedes":[],"tags":[],"tx_time":"2026-01-01T00:00:00Z","valid_from":null,"valid_to":null}"#;
        let event = parse_event_line(line).expect("event");
        assert!(!event.body.contains("do not ingest"));
    }
}
