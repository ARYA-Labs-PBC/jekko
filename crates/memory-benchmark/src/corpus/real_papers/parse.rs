use super::model::{
    AcceptanceMetrics, AnswerKey, ArtifactProvenance, ContextPack, ContextPackProvenance,
    JudgeTrial, ModelDecision, ModelTrial, NumericTolerance, PaperChallenge, PaperRecord,
    PaperSection, RouteMetadata, SourcePublication, SupportRef, TokenUsage,
};
#[path = "json_helpers.rs"]
mod helpers;
use crate::json::{self, Json};
use crate::types::Domain;
use helpers::*;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn load_all_challenges(root: &Path) -> Result<Vec<PaperChallenge>, String> {
    let challenge_root = if root.ends_with("challenges") {
        root.to_path_buf()
    } else {
        root.join("challenges")
    };
    let mut files = Vec::new();
    collect_json_files(&challenge_root, &mut files)?;
    let mut out = Vec::new();
    for file in files {
        out.extend(read_challenges(&file)?);
    }
    Ok(out)
}

pub(crate) fn load_paper(root: &Path, publication_hash: &str) -> Result<PaperRecord, String> {
    let paper_path = root.join("papers").join(format!("{publication_hash}.json"));
    read_paper(&paper_path)
}

pub(crate) fn read_paper(file: &Path) -> Result<PaperRecord, String> {
    let text =
        fs::read_to_string(file).map_err(|err| format!("read {}: {}", file.display(), err))?;
    let parsed = json::parse(&text).map_err(|err| format!("parse {}: {}", file.display(), err))?;
    paper_from_json(&parsed).map_err(|err| format!("{}: {}", file.display(), err))
}

#[allow(dead_code)]
pub(crate) fn read_challenge(file: &Path) -> Result<PaperChallenge, String> {
    let mut challenges = read_challenges(file)?;
    match challenges.len() {
        1 => Ok(challenges.remove(0)),
        0 => Err(format!("{}: no challenges found", file.display())),
        _ => Err(format!(
            "{}: expected a single challenge object, found {}",
            file.display(),
            challenges.len()
        )),
    }
}

pub(crate) fn read_challenges(file: &Path) -> Result<Vec<PaperChallenge>, String> {
    let text =
        fs::read_to_string(file).map_err(|err| format!("read {}: {}", file.display(), err))?;
    let parsed = json::parse(&text).map_err(|err| format!("parse {}: {}", file.display(), err))?;
    match &parsed {
        Json::Array(items) => items
            .iter()
            .map(|item| {
                challenge_from_json(item).map_err(|err| format!("{}: {}", file.display(), err))
            })
            .collect(),
        _ => challenge_from_json(&parsed)
            .map(|challenge| vec![challenge])
            .map_err(|err| format!("{}: {}", file.display(), err)),
    }
}

pub(crate) fn collect_json_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    let entries =
        fs::read_dir(root).map_err(|err| format!("read_dir {}: {}", root.display(), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

pub(crate) fn load_selection(path: &Path) -> Result<std::collections::BTreeSet<String>, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect())
}

fn paper_from_json(value: &Json) -> Result<PaperRecord, String> {
    let obj = as_object(value)?;
    let license_obj = obj.get("license").and_then(as_object_ok);
    let license_spdx = match license_obj.and_then(|license| license.get("spdx").and_then(as_str)) {
        Some(value) => value.to_string(),
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
    let title = match optional_string(obj, "title") {
        Some(title) if !title.trim().is_empty() => title,
        _ => "untitled".to_string(),
    };
    Ok(PaperRecord {
        publication_hash: required_string(obj, "publication_hash")?,
        title,
        license_spdx,
        redistributable,
        dedupe_keys: optional_string_array(obj, "dedupe_keys"),
        source_ids: optional_string_array(obj, "source_ids"),
        source_url: license_obj.and_then(|license| optional_string(license, "source_url")),
        retrieval_kinds: retrieval_kinds(obj),
        sections,
    })
}

fn section_from_json(value: &Json) -> Result<PaperSection, String> {
    let obj = as_object(value)?;
    let section_id = required_string(obj, "section_id")?;
    let title = match optional_string(obj, "title") {
        Some(title) if !title.trim().is_empty() => title,
        _ => section_id.clone(),
    };
    let section_hash = match optional_string(obj, "section_hash") {
        Some(value) => value,
        None => String::new(),
    };
    Ok(PaperSection {
        section_id,
        title,
        text: required_string(obj, "text")?,
        section_hash,
    })
}

fn challenge_from_json(value: &Json) -> Result<PaperChallenge, String> {
    let obj = as_object(value)?;
    let schema_version = optional_string(obj, "schema_version")
        .unwrap_or_else(|| "opencode-qbank-challenge-v1".to_string());
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
    let support = if let Some(value) = obj.get("support") {
        required_array_value(value, "support")?
            .iter()
            .map(support_from_json)
            .collect::<Result<Vec<_>, _>>()?
    } else {
        required_array(obj, "support_sections")?
            .iter()
            .filter_map(as_str)
            .map(|section_id| SupportRef {
                section_id: section_id.to_string(),
                section_hash: String::new(),
            })
            .collect()
    };
    let domain = match optional_string(obj, "domain") {
        Some(domain) if !domain.trim().is_empty() => domain,
        _ => Domain::Science.name().to_string(),
    };
    let topics = optional_string_array(obj, "topics");
    let difficulty_score = match optional_f32(obj, "difficulty_score") {
        Some(value) => value,
        None => 0.0,
    };
    let answerability = match optional_f32(acceptance, "answerability") {
        Some(value) => value,
        None => 1.0,
    };
    let focused_correct_rate = match optional_f32(acceptance, "focused_correct_rate") {
        Some(value) => value,
        None => 1.0,
    };
    let blind_correct_rate = match optional_f32(acceptance, "blind_correct_rate") {
        Some(value) => value,
        None => 0.0,
    };
    let context_pack = match obj
        .get("context_pack")
        .map(context_pack_from_json)
        .transpose()?
    {
        Some(pack) => pack,
        None => ContextPack::default(),
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

fn source_publication_from_json(value: &Json) -> Result<SourcePublication, String> {
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

fn model_trial_from_json(value: &Json) -> Result<ModelTrial, String> {
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

fn judge_trial_from_json(value: &Json) -> Result<JudgeTrial, String> {
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

fn context_pack_provenance_from_json(value: &Json) -> Result<ContextPackProvenance, String> {
    let obj = as_object(value)?;
    Ok(ContextPackProvenance {
        kind: required_string(obj, "kind")?,
        context_hash: required_string(obj, "context_hash")?,
        prompt_hash: required_string(obj, "prompt_hash")?,
        section_ids: optional_string_array(obj, "section_ids"),
        estimated_tokens: optional_i64(obj, "estimated_tokens").unwrap_or(0).max(0) as u32,
    })
}

fn route_metadata_from_json(value: &Json) -> Result<RouteMetadata, String> {
    let obj = as_object(value)?;
    let token_usage = obj
        .get("token_usage")
        .and_then(as_object_ok)
        .map(|usage| token_usage_from_object(usage))
        .transpose()?;
    Ok(RouteMetadata {
        request_id: required_string(obj, "request_id")?,
        provider: optional_string(obj, "provider").unwrap_or_default(),
        model: optional_string(obj, "model").unwrap_or_default(),
        route_mode: optional_string(obj, "route_mode"),
        route_confidence: optional_f32(obj, "route_confidence")
            .or_else(|| optional_f32(obj, "confidence")),
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

fn model_decision_from_json(value: &Json) -> Result<ModelDecision, String> {
    let obj = as_object(value)?;
    Ok(ModelDecision {
        model_id: required_string(obj, "model_id")?,
        configured_score: optional_f32(obj, "configured_score").unwrap_or(0.0),
        selection_score: optional_f32(obj, "selection_score").unwrap_or(0.0),
        latency_ms: optional_i64(obj, "latency_ms").unwrap_or(0).max(0) as u64,
        status: optional_string(obj, "status").unwrap_or_default(),
        output_hash: optional_string(obj, "output_hash"),
        selected: optional_bool(obj, "selected").unwrap_or(false),
        token_usage: token_usage_from_json(required(obj, "token_usage")?)?,
    })
}

fn token_usage_from_object(
    obj: &std::collections::BTreeMap<String, Json>,
) -> Result<TokenUsage, String> {
    Ok(TokenUsage {
        prompt_tokens: optional_i64(obj, "prompt_tokens").unwrap_or(0).max(0) as u32,
        completion_tokens: optional_i64(obj, "completion_tokens").unwrap_or(0).max(0) as u32,
        total_tokens: optional_i64(obj, "total_tokens").unwrap_or(0).max(0) as u32,
    })
}

fn parse_route_metadata_list(value: Option<&Json>) -> Result<Vec<RouteMetadata>, String> {
    match value {
        Some(Json::Array(items)) => items.iter().map(route_metadata_from_json).collect(),
        Some(Json::Object(_)) => {
            route_metadata_from_json(value.expect("value")).map(|item| vec![item])
        }
        Some(_) => Err("route_metadata must be an object or array".to_string()),
        None => Ok(Vec::new()),
    }
}

fn token_usage_from_json(value: &Json) -> Result<TokenUsage, String> {
    let obj = as_object(value)?;
    token_usage_from_object(obj)
}

fn acceptance_metrics_from_json(value: &Json) -> Result<AcceptanceMetrics, String> {
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

fn artifact_provenance_from_json(value: &Json) -> Result<ArtifactProvenance, String> {
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

fn parse_array<T>(
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

fn answer_key_from_json(value: &Json) -> Result<AnswerKey, String> {
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

fn numeric_tolerance_from_json(value: &Json) -> Result<NumericTolerance, String> {
    let obj = as_object(value)?;
    Ok(NumericTolerance {
        value: required_f64(obj, "value")?,
        tolerance: required_f64(obj, "tolerance")?,
        unit: optional_string(obj, "unit"),
    })
}

fn support_from_json(value: &Json) -> Result<SupportRef, String> {
    let obj = as_object(value)?;
    let section_hash = match optional_string(obj, "section_hash") {
        Some(value) => value,
        None => String::new(),
    };
    Ok(SupportRef {
        section_id: required_string(obj, "section_id")?,
        section_hash,
    })
}

fn context_pack_from_json(value: &Json) -> Result<ContextPack, String> {
    let obj = as_object(value)?;
    let safe_window_tokens = match optional_i64(obj, "safe_window_tokens") {
        Some(value) => value as u32,
        None => 128000,
    };
    let target_fill_ratio = match optional_f32(obj, "target_fill_ratio") {
        Some(value) => value,
        None => 0.82,
    };
    let output_reserve_tokens = match optional_i64(obj, "output_reserve_tokens") {
        Some(value) => value as u32,
        None => 4096,
    };
    let estimated_tokens = match optional_i64(obj, "estimated_tokens") {
        Some(value) => value as u32,
        None => 0,
    };
    Ok(ContextPack {
        safe_window_tokens,
        target_fill_ratio,
        output_reserve_tokens,
        estimated_tokens,
        target_section_ids: optional_string_array(obj, "target_section_ids"),
        distractor_section_ids: optional_string_array(obj, "distractor_section_ids"),
    })
}

fn retrieval_kinds(obj: &std::collections::BTreeMap<String, Json>) -> Vec<String> {
    match obj
        .get("retrieval_receipts")
        .and_then(|value| required_array_value(value, "retrieval_receipts").ok())
    {
        Some(items) => items
            .iter()
            .filter_map(as_object_ok)
            .filter_map(|receipt| optional_string(receipt, "kind"))
            .collect(),
        None => Vec::new(),
    }
}

fn optional_bool(obj: &std::collections::BTreeMap<String, Json>, key: &str) -> Option<bool> {
    obj.get(key).and_then(as_bool)
}
