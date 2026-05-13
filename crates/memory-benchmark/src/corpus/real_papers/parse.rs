use super::model::{AnswerKey, ContextPack, NumericTolerance, PaperChallenge, PaperRecord, PaperSection, SupportRef};
#[path = "json_helpers.rs"]
mod helpers;
use helpers::*;
use crate::json::{self, Json};
use crate::types::Domain;
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
        out.push(read_challenge(&file)?);
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

pub(crate) fn read_challenge(file: &Path) -> Result<PaperChallenge, String> {
    let text =
        fs::read_to_string(file).map_err(|err| format!("read {}: {}", file.display(), err))?;
    let parsed = json::parse(&text).map_err(|err| format!("parse {}: {}", file.display(), err))?;
    challenge_from_json(&parsed).map_err(|err| format!("{}: {}", file.display(), err))
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
    let context_pack = match obj.get("context_pack").map(context_pack_from_json).transpose()? {
        Some(pack) => pack,
        None => ContextPack::default(),
    };
    Ok(PaperChallenge {
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
    })
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
