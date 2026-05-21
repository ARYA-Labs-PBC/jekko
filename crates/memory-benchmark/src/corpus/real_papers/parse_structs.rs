use super::*;

pub(super) fn paper_from_json(value: &Json) -> Result<PaperRecord, String> {
    let obj = as_object(value)?;
    let license_obj = obj.get("license").and_then(as_object_ok);
    let license_spdx = match license_obj.and_then(|license| license.get("spdx").and_then(as_str)) {
        Some(spdx) => spdx.to_string(),
        None => "NOASSERTION".to_string(),
    };
    let redistributable = matches!(
        license_obj.and_then(|license| license.get("redistributable").and_then(as_bool)),
        Some(true)
    );
    let sections = required_array(obj, "sections")?
        .iter()
        .map(section_from_json)
        .collect::<Result<Vec<_>, _>>()?;
    let retrieval_receipts = retrieval_receipts(obj);
    let review_receipts = review_receipts(obj);
    let title = match optional_string(obj, "title") {
        Some(title) if !title.trim().is_empty() => title,
        Some(_) | None => "untitled".to_string(),
    };
    Ok(PaperRecord {
        publication_hash: required_string(obj, "publication_hash")?,
        title,
        license_spdx,
        redistributable,
        dedupe_keys: optional_string_array(obj, "dedupe_keys"),
        source_ids: optional_string_array(obj, "source_ids"),
        source_url: license_obj.and_then(|license| optional_string(license, "source_url")),
        retrieval_receipts: retrieval_receipts.clone(),
        review_receipts,
        retrieval_kinds: retrieval_kinds(obj),
        sections,
    })
}

pub(super) fn section_from_json(value: &Json) -> Result<PaperSection, String> {
    let obj = as_object(value)?;
    let section_id = required_string(obj, "section_id")?;
    let title = match optional_string(obj, "title") {
        Some(title) if !title.trim().is_empty() => title,
        _ => section_id.clone(),
    };
    #[allow(clippy::manual_unwrap_or_default)]
    let section_hash = match optional_string(obj, "section_hash") {
        Some(hash) => hash,
        None => String::new(),
    };
    Ok(PaperSection {
        section_id,
        title,
        text: required_string(obj, "text")?,
        section_hash,
    })
}

enum ChallengeSchemaVersion {
    Declared(String),
    LegacyMissingProvenance,
}

fn challenge_schema_version(
    obj: &std::collections::BTreeMap<String, Json>,
) -> ChallengeSchemaVersion {
    match optional_string(obj, "schema_version") {
        Some(value) if !value.trim().is_empty() => ChallengeSchemaVersion::Declared(value),
        Some(_) => ChallengeSchemaVersion::LegacyMissingProvenance,
        None => ChallengeSchemaVersion::LegacyMissingProvenance,
    }
}

pub(super) fn challenge_from_json(value: &Json) -> Result<PaperChallenge, String> {
    let obj = as_object(value)?;
    let schema_version = match challenge_schema_version(obj) {
        ChallengeSchemaVersion::Declared(value) => value,
        ChallengeSchemaVersion::LegacyMissingProvenance => {
            "opencode-qbank-challenge-v1".to_string()
        }
    };
    let acceptance = as_object(required(obj, "acceptance")?)?;
    let accepted = matches!(acceptance.get("accepted").and_then(as_bool), Some(true));
    if !accepted {
        return Err("challenge is not accepted".to_string());
    }
    let answer_key = match required(obj, "answer_key")? {
        Json::Str(value) => AnswerKey {
            canonical: value.clone(),
            must_include: Vec::new(),
            must_not_include: Vec::new(),
            aliases: Vec::new(),
            numeric_tolerances: Vec::new(),
            unit_tolerances: Vec::new(),
        },
        other => answer_key_from_json(other)?,
    };
    let support = match obj.get("support") {
        Some(value) => required_array_value(value, "support")?
            .iter()
            .map(support_from_json)
            .collect::<Result<Vec<_>, _>>()?,
        None => required_array(obj, "support_sections")?
            .iter()
            .filter_map(as_str)
            .map(|section_id| SupportRef {
                section_id: section_id.to_string(),
                section_hash: String::new(),
            })
            .collect(),
    };
    let domain = match optional_string(obj, "domain") {
        Some(domain) if !domain.trim().is_empty() => domain,
        Some(_) | None => Domain::Science.name().to_string(),
    };
    let topics = optional_string_array(obj, "topics");
    let difficulty_score = optional_f32(obj, "difficulty_score").unwrap_or(0.0);
    let answerability = optional_f32(acceptance, "answerability").unwrap_or(1.0);
    let focused_correct_rate = optional_f32(acceptance, "focused_correct_rate").unwrap_or(1.0);
    let blind_correct_rate = optional_f32(acceptance, "blind_correct_rate").unwrap_or(0.0);
    let context_pack = match obj
        .get("context_pack")
        .map(context_pack_from_json)
        .transpose()?
    {
        Some(context_pack) => context_pack,
        None => empty_context_pack(),
    };
    Ok(PaperChallenge {
        schema_version,
        challenge_hash: required_string(obj, "challenge_hash")?,
        publication_hash: required_string(obj, "publication_hash")?,
        domain,
        topics,
        difficulty_score,
        answerability,
        focused_correct_rate,
        blind_correct_rate,
        question: required_string(obj, "question")?,
        answer_key,
        support,
        context_pack,
        source_publication: obj
            .get("source_publication")
            .map(source_publication_from_json)
            .transpose()?,
        focused_support_trials: parse_array(obj, "focused_support_trials", model_trial_from_json)?,
        saturated_blind_trials: parse_array(obj, "saturated_blind_trials", model_trial_from_json)?,
        judge_trials: parse_array(obj, "judge_trials", judge_trial_from_json)?,
        context_packs: parse_array(obj, "context_packs", context_pack_provenance_from_json)?,
        route_metadata: parse_route_metadata_list(obj.get("route_metadata"))?,
        acceptance_metrics: obj
            .get("acceptance_metrics")
            .map(acceptance_metrics_from_json)
            .transpose()?,
        artifact_provenance: obj
            .get("artifact_provenance")
            .map(artifact_provenance_from_json)
            .transpose()?,
    })
}

pub(super) fn source_publication_from_json(value: &Json) -> Result<SourcePublication, String> {
    let obj = as_object(value)?;
    Ok(SourcePublication {
        publication_hash: required_string(obj, "publication_hash")?,
        content_hash: required_string(obj, "content_hash")?,
        license_spdx: required_string(obj, "license_spdx")?,
        redistributable: optional_bool(obj, "redistributable").unwrap_or(false),
        source_url: optional_string(obj, "source_url"),
        section_hashes: optional_string_array(obj, "section_hashes"),
    })
}

pub(super) fn model_trial_from_json(value: &Json) -> Result<ModelTrial, String> {
    let obj = as_object(value)?;
    Ok(ModelTrial {
        agent_id: required_string(obj, "agent_id")?,
        phase: required_string(obj, "phase")?,
        correct: optional_bool(obj, "correct").unwrap_or(false),
        answerability: optional_f32(obj, "answerability").unwrap_or(0.0),
        supported: optional_bool(obj, "supported").unwrap_or(false),
        confidence: optional_f32(obj, "confidence").unwrap_or(-1.0),
        prompt_hash: required_string(obj, "prompt_hash")?,
        context_hash: required_string(obj, "context_hash")?,
        route_metadata: route_metadata_from_json(required(obj, "route_metadata")?)?,
        token_usage: token_usage_from_json(required(obj, "token_usage")?)?,
    })
}

pub(super) fn judge_trial_from_json(value: &Json) -> Result<JudgeTrial, String> {
    let obj = as_object(value)?;
    Ok(JudgeTrial {
        agent_id: required_string(obj, "agent_id")?,
        accepted: optional_bool(obj, "accepted").unwrap_or(false),
        confidence: optional_f32(obj, "confidence").unwrap_or(-1.0),
        rationale_hash: required_string(obj, "rationale_hash")?,
        route_metadata: route_metadata_from_json(required(obj, "route_metadata")?)?,
        token_usage: token_usage_from_json(required(obj, "token_usage")?)?,
    })
}

pub(super) fn context_pack_provenance_from_json(
    value: &Json,
) -> Result<ContextPackProvenance, String> {
    let obj = as_object(value)?;
    Ok(ContextPackProvenance {
        kind: required_string(obj, "kind")?,
        context_hash: required_string(obj, "context_hash")?,
        prompt_hash: required_string(obj, "prompt_hash")?,
        section_ids: optional_string_array(obj, "section_ids"),
        estimated_tokens: optional_i64(obj, "estimated_tokens").unwrap_or(0).max(0) as u32,
    })
}

pub(super) fn route_metadata_from_json(value: &Json) -> Result<RouteMetadata, String> {
    let obj = as_object(value)?;
    let token_usage = obj
        .get("token_usage")
        .and_then(as_object_ok)
        .map(token_usage_from_object)
        .transpose()?;
    #[allow(clippy::manual_unwrap_or_default)]
    let provider = match optional_string(obj, "provider") {
        Some(provider) => provider,
        None => String::new(),
    };
    #[allow(clippy::manual_unwrap_or_default)]
    let model = match optional_string(obj, "model") {
        Some(model) => model,
        None => String::new(),
    };
    let route_confidence = match optional_f32(obj, "route_confidence") {
        Some(value) => Some(value),
        None => optional_f32(obj, "confidence"),
    };
    Ok(RouteMetadata {
        request_id: required_string(obj, "request_id")?,
        provider,
        model,
        route_mode: optional_string(obj, "route_mode"),
        route_confidence,
        primary_model_id: optional_string(obj, "primary_model_id"),
        backup_model_ids: optional_string_array(obj, "backup_model_ids"),
        fusion_model_id: optional_string(obj, "fusion_model_id"),
        winner_model_id: optional_string(obj, "winner_model_id"),
        prompt_hash: optional_string(obj, "prompt_hash"),
        context_hash: optional_string(obj, "context_hash"),
        receipts_hash: optional_string(obj, "receipts_hash"),
        token_usage,
        model_decisions_hash: optional_string(obj, "model_decisions_hash"),
        model_decisions: parse_array(obj, "model_decisions", model_decision_from_json)?,
    })
}

pub(super) fn model_decision_from_json(value: &Json) -> Result<ModelDecision, String> {
    let obj = as_object(value)?;
    #[allow(clippy::manual_unwrap_or_default)]
    let status = match optional_string(obj, "status") {
        Some(status) => status,
        None => String::new(),
    };
    Ok(ModelDecision {
        model_id: required_string(obj, "model_id")?,
        configured_score: optional_f32(obj, "configured_score").unwrap_or(0.0),
        selection_score: optional_f32(obj, "selection_score").unwrap_or(0.0),
        latency_ms: optional_i64(obj, "latency_ms").unwrap_or(0).max(0) as u64,
        status,
        output_hash: optional_string(obj, "output_hash"),
        selected: optional_bool(obj, "selected").unwrap_or(false),
        token_usage: token_usage_from_json(required(obj, "token_usage")?)?,
    })
}

fn empty_context_pack() -> ContextPack {
    ContextPack {
        safe_window_tokens: 0,
        target_fill_ratio: 0.0,
        output_reserve_tokens: 0,
        estimated_tokens: 0,
        target_section_ids: Vec::new(),
        distractor_section_ids: Vec::new(),
    }
}
