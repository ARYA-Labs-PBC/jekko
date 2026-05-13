use super::{sha256_hex, AcceptanceRecord, ChallengeRecord, ContextPack, PaperRecord};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn token_estimate(text: &str) -> u64 {
    ((text.chars().count() as u64) + 3) / 4
}

pub fn pack_context(
    paper: &PaperRecord,
    selected_section_ids: &[String],
    safe_window_tokens: u64,
    target_fill_ratio: f64,
    output_reserve_tokens: u64,
) -> Result<ContextPack, String> {
    let budget = ((safe_window_tokens as f64 * target_fill_ratio).floor() as i64
        - output_reserve_tokens as i64)
        .max(0) as u64;
    let selected: BTreeSet<&str> = selected_section_ids.iter().map(String::as_str).collect();
    let mut estimated = 0_u64;
    let mut targets = Vec::new();
    let mut distractors = Vec::new();
    for section in &paper.sections {
        let cost = token_estimate(&section.text);
        if estimated + cost > budget {
            continue;
        }
        estimated += cost;
        if selected.contains(section.section_id.as_str()) {
            targets.push(section.section_id.clone());
        } else {
            distractors.push(section.section_id.clone());
        }
    }
    if targets.is_empty() {
        return Err("context pack does not include any target section".to_string());
    }
    Ok(ContextPack {
        safe_window_tokens,
        target_fill_ratio,
        output_reserve_tokens,
        estimated_tokens: estimated,
        target_section_ids: targets,
        distractor_section_ids: distractors,
    })
}

pub fn acceptance_passes(acceptance: &AcceptanceRecord) -> bool {
    acceptance.accepted
        && acceptance.auditor_agreement >= 0.75
        && acceptance.answerability >= 0.90
        && acceptance.blind_correct_rate <= 0.50
        && acceptance.focused_correct_rate >= 0.90
        && !acceptance.ambiguity_flag
        && !acceptance.hash_mismatch
        && acceptance.redistributable
}

pub fn challenge_sort_key(
    challenge: &ChallengeRecord,
) -> (
    std::cmp::Reverse<i64>,
    std::cmp::Reverse<i64>,
    i64,
    String,
    String,
) {
    (
        std::cmp::Reverse((challenge.difficulty_score * 1_000_000.0).round() as i64),
        std::cmp::Reverse((challenge.acceptance.focused_correct_rate * 1_000_000.0).round() as i64),
        (challenge.acceptance.blind_correct_rate * 1_000_000.0).round() as i64,
        challenge.publication_hash.clone(),
        challenge.challenge_hash.clone(),
    )
}

pub fn sorted_challenges(mut challenges: Vec<ChallengeRecord>) -> Vec<ChallengeRecord> {
    challenges.sort_by_key(challenge_sort_key);
    challenges
}

pub fn bank_subdir(bank: &Path, name: &str) -> PathBuf {
    bank.join(name)
}

pub fn ensure_bank_layout(bank: &Path) -> Result<(), String> {
    for dir in ["papers", "challenges", "rejected", "manifests"] {
        fs::create_dir_all(bank.join(dir)).map_err(|err| format!("create {dir}: {err}"))?;
    }
    Ok(())
}

pub fn write_json_pretty<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    fs::write(path, format!("{json}\n")).map_err(|err| format!("write {}: {err}", path.display()))
}

pub fn read_json<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("parse {}: {err}", path.display()))
}

pub fn read_challenges(root: &Path) -> Result<Vec<ChallengeRecord>, String> {
    let challenge_root = root.join("challenges");
    let mut paths = Vec::new();
    collect_json_files(&challenge_root, &mut paths)?;
    let mut out = Vec::new();
    for path in paths {
        if path.file_name().and_then(|name| name.to_str()) == Some("manifest.json") {
            continue;
        }
        out.push(read_json(&path)?);
    }
    Ok(out)
}

pub fn read_papers(root: &Path) -> Result<Vec<PaperRecord>, String> {
    let paper_root = root.join("papers");
    let mut paths = Vec::new();
    collect_json_files(&paper_root, &mut paths)?;
    let mut out = Vec::new();
    for path in paths {
        out.push(read_json(&path)?);
    }
    Ok(out)
}

pub fn collect_json_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    let entries =
        fs::read_dir(root).map_err(|err| format!("read_dir {}: {err}", root.display()))?;
    for entry in entries {
        let path = entry.map_err(|err| err.to_string())?.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

pub fn manifest_hash(paths: &[PathBuf]) -> Result<String, String> {
    let mut material = String::new();
    for path in paths {
        let text =
            fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
        material.push_str(&path.display().to_string());
        material.push('\0');
        material.push_str(&sha256_hex(text.as_bytes()));
        material.push('\n');
    }
    Ok(sha256_hex(material.as_bytes()))
}
