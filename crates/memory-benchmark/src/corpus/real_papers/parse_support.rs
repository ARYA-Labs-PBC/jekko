use super::*;

pub(super) fn token_usage_from_object(
    obj: &std::collections::BTreeMap<String, Json>,
) -> Result<TokenUsage, String> {
    Ok(TokenUsage {
        prompt_tokens: optional_i64(obj, "prompt_tokens").unwrap_or(0).max(0) as u32,
        completion_tokens: optional_i64(obj, "completion_tokens").unwrap_or(0).max(0) as u32,
        total_tokens: optional_i64(obj, "total_tokens").unwrap_or(0).max(0) as u32,
    })
}

pub(super) fn parse_route_metadata_list(
    value: Option<&Json>,
) -> Result<Vec<RouteMetadata>, String> {
    match value {
        Some(Json::Array(items)) => items.iter().map(route_metadata_from_json).collect(),
        Some(Json::Object(_)) => {
            route_metadata_from_json(value.expect("value")).map(|item| vec![item])
        }
        Some(_) => Err("route_metadata must be an object or array".to_string()),
        None => Ok(Vec::new()),
    }
}

pub(super) fn token_usage_from_json(value: &Json) -> Result<TokenUsage, String> {
    let obj = as_object(value)?;
    token_usage_from_object(obj)
}

pub(super) fn acceptance_metrics_from_json(value: &Json) -> Result<AcceptanceMetrics, String> {
    let obj = as_object(value)?;
    Ok(AcceptanceMetrics {
        focused_agreement: optional_f32(obj, "focused_agreement").unwrap_or(0.0),
        focused_correct_rate: optional_f32(obj, "focused_correct_rate").unwrap_or(0.0),
        answerability: optional_f32(obj, "answerability").unwrap_or(0.0),
        saturated_blind_correct_rate: optional_f32(obj, "saturated_blind_correct_rate")
            .unwrap_or(1.0),
        saturated_mean_confidence: optional_f32(obj, "saturated_mean_confidence").unwrap_or(1.0),
    })
}

pub(super) fn artifact_provenance_from_json(value: &Json) -> Result<ArtifactProvenance, String> {
    let obj = as_object(value)?;
    Ok(ArtifactProvenance {
        run_id: required_string(obj, "run_id")?,
        reducer_version: required_string(obj, "reducer_version")?,
        agent_mode: optional_string(obj, "agent_mode"),
        fixture_provenance: optional_bool(obj, "fixture_provenance").unwrap_or(false),
        answer_leakage_detected: optional_bool(obj, "answer_leakage_detected").unwrap_or(true),
        license_ambiguous: optional_bool(obj, "license_ambiguous").unwrap_or(true),
    })
}

pub(super) fn parse_array<T>(
    obj: &std::collections::BTreeMap<String, Json>,
    key: &str,
    parse: fn(&Json) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    match obj.get(key) {
        Some(value) => required_array_value(value, key)?
            .iter()
            .map(parse)
            .collect(),
        None => Ok(Vec::new()),
    }
}

pub(super) fn answer_key_from_json(value: &Json) -> Result<AnswerKey, String> {
    let obj = as_object(value)?;
    let numeric_tolerances = match obj.get("numeric_tolerances") {
        Some(value) => required_array_value(value, "numeric_tolerances")?
            .iter()
            .map(numeric_tolerance_from_json)
            .collect::<Result<Vec<_>, String>>()?,
        None => Vec::new(),
    };
    Ok(AnswerKey {
        canonical: required_string(obj, "canonical")?,
        must_include: optional_string_array(obj, "must_include"),
        must_not_include: optional_string_array(obj, "must_not_include"),
        aliases: optional_string_array(obj, "aliases"),
        numeric_tolerances,
        unit_tolerances: optional_string_array(obj, "unit_tolerances"),
    })
}

pub(super) fn numeric_tolerance_from_json(value: &Json) -> Result<NumericTolerance, String> {
    let obj = as_object(value)?;
    Ok(NumericTolerance {
        value: required_f64(obj, "value")?,
        tolerance: required_f64(obj, "tolerance")?,
        unit: optional_string(obj, "unit"),
    })
}

pub(super) fn support_from_json(value: &Json) -> Result<SupportRef, String> {
    let obj = as_object(value)?;
    #[allow(clippy::manual_unwrap_or_default)]
    let section_hash = match optional_string(obj, "section_hash") {
        Some(section_hash) => section_hash,
        None => String::new(),
    };
    Ok(SupportRef {
        section_id: required_string(obj, "section_id")?,
        section_hash,
    })
}

pub(super) fn context_pack_from_json(value: &Json) -> Result<ContextPack, String> {
    let obj = as_object(value)?;
    let safe_window_tokens = match optional_i64(obj, "safe_window_tokens") {
        Some(value) => value as u32,
        None => 128000,
    };
    let target_fill_ratio = optional_f32(obj, "target_fill_ratio").unwrap_or(0.82);
    let output_reserve_tokens = optional_i64(obj, "output_reserve_tokens").unwrap_or(4096) as u32;
    let estimated_tokens = optional_i64(obj, "estimated_tokens").unwrap_or(0) as u32;
    Ok(ContextPack {
        safe_window_tokens,
        target_fill_ratio,
        output_reserve_tokens,
        estimated_tokens,
        target_section_ids: optional_string_array(obj, "target_section_ids"),
        distractor_section_ids: optional_string_array(obj, "distractor_section_ids"),
    })
}

fn receipt_array(obj: &std::collections::BTreeMap<String, Json>, key: &str) -> Vec<Json> {
    match obj
        .get(key)
        .and_then(|value| required_array_value(value, key).ok())
    {
        Some(items) => items.to_vec(),
        None => Vec::new(),
    }
}

pub(super) fn retrieval_receipts(obj: &std::collections::BTreeMap<String, Json>) -> Vec<Json> {
    receipt_array(obj, "retrieval_receipts")
}

pub(super) fn review_receipts(obj: &std::collections::BTreeMap<String, Json>) -> Vec<Json> {
    let receipts = receipt_array(obj, "review_receipts");
    if receipts.is_empty() {
        receipt_array(obj, "proof_receipts")
    } else {
        receipts
    }
}

pub(super) fn retrieval_kinds(obj: &std::collections::BTreeMap<String, Json>) -> Vec<String> {
    retrieval_receipts(obj)
        .iter()
        .filter_map(as_object_ok)
        .filter_map(|receipt| optional_string(receipt, "kind"))
        .collect()
}

pub(super) fn optional_bool(
    obj: &std::collections::BTreeMap<String, Json>,
    key: &str,
) -> Option<bool> {
    obj.get(key).and_then(as_bool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::{obj, s};

    #[test]
    fn support_from_json_defaults_missing_section_hash_to_empty_string() {
        let value = obj(&[("section_id", s("s1"))]);
        let support = support_from_json(&value).expect("parse support");
        assert_eq!(support.section_id, "s1");
        assert!(support.section_hash.is_empty());
    }
}
