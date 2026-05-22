use super::model::{
    AcceptanceMetrics, AnswerKey, ArtifactProvenance, ContextPack, ContextPackProvenance,
    JudgeTrial, ModelDecision, ModelTrial, NumericTolerance, PaperChallenge, PaperRecord,
    PaperSection, RouteMetadata, SourcePublication, SupportRef, TokenUsage,
};
#[path = "json_helpers.rs"]
mod helpers;
#[path = "parse_structs.rs"]
mod structs;
#[path = "parse_support.rs"]
mod support;
use crate::json::{self, Json};
use crate::types::Domain;
use helpers::*;
use std::fs;
use std::path::{Path, PathBuf};
use structs::*;
use support::*;

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
