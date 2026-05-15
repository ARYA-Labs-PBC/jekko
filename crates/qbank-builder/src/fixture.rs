use super::{
    acceptance_passes, content_hash, ensure_bank_layout, finalize_challenge, pack_context,
    section_hash, write_json_pretty, AcceptanceRecord, AnswerKey, ChallengeRecord, LicenseRecord,
    PaperRecord, PaperSection, SupportRef, CHALLENGE_SCHEMA_VERSION, PAPER_SCHEMA_VERSION,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
struct SeedFixtureEntry {
    challenge_hash: String,
    publication_hash: String,
    question: String,
    answer_key: String,
    #[serde(default)]
    support_sections: Vec<String>,
    #[serde(default)]
    context_pack: SeedFixtureContextPack,
    #[serde(default)]
    acceptance: SeedFixtureAcceptance,
}

#[derive(Debug, Clone, Deserialize)]
struct SeedFixtureContextPack {
    #[serde(default = "default_target_fill_ratio")]
    target_fill_ratio: f64,
    #[serde(default = "default_output_reserve_tokens")]
    output_reserve_tokens: u64,
}

impl Default for SeedFixtureContextPack {
    fn default() -> Self {
        Self {
            target_fill_ratio: default_target_fill_ratio(),
            output_reserve_tokens: default_output_reserve_tokens(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct SeedFixtureAcceptance {
    #[serde(default = "default_true")]
    accepted: bool,
    #[serde(default)]
    reason: Option<String>,
}

impl Default for SeedFixtureAcceptance {
    fn default() -> Self {
        Self {
            accepted: default_true(),
            reason: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SeedFixtureSummary {
    pub bank: PathBuf,
    pub source_manifest: PathBuf,
    pub papers_written: usize,
    pub challenges_written: usize,
}

pub fn seed_fixture_bank(
    bank: &Path,
    source_manifest: &Path,
) -> Result<SeedFixtureSummary, String> {
    ensure_bank_layout(bank)?;
    if !source_manifest.exists() {
        let papers_written = count_json_files(&bank.join("papers"))?;
        let challenges_written = count_json_files(&bank.join("challenges"))?;
        if papers_written > 0 && challenges_written > 0 {
            return Ok(SeedFixtureSummary {
                bank: bank.to_path_buf(),
                source_manifest: source_manifest.to_path_buf(),
                papers_written,
                challenges_written,
            });
        }
        return Err(format!(
            "missing source manifest {} and bank is not seeded",
            source_manifest.display()
        ));
    }

    let text = fs::read_to_string(source_manifest)
        .map_err(|err| format!("read {}: {err}", source_manifest.display()))?;
    let entries: Vec<SeedFixtureEntry> = serde_json::from_str(&text)
        .map_err(|err| format!("parse {}: {err}", source_manifest.display()))?;

    clear_json_files(&bank.join("papers"))?;
    clear_json_files(&bank.join("challenges"))?;
    clear_json_files(&bank.join("rejected"))?;

    let mut papers_written = 0usize;
    let mut challenges_written = 0usize;

    for (index, entry) in entries.iter().enumerate() {
        let (paper, challenge, accepted) = build_records(entry, index, source_manifest)?;
        let paper_path = bank
            .join("papers")
            .join(format!("{}.json", paper.publication_hash));
        write_json_pretty(&paper_path, &paper)?;
        papers_written += 1;

        let challenge_dir = if accepted { "challenges" } else { "rejected" };
        let challenge_path = bank
            .join(challenge_dir)
            .join(format!("{}.json", challenge.challenge_hash));
        write_json_pretty(&challenge_path, &challenge)?;
        if accepted {
            challenges_written += 1;
        }
    }

    Ok(SeedFixtureSummary {
        bank: bank.to_path_buf(),
        source_manifest: source_manifest.to_path_buf(),
        papers_written,
        challenges_written,
    })
}

fn build_records(
    entry: &SeedFixtureEntry,
    index: usize,
    source_manifest: &Path,
) -> Result<(PaperRecord, ChallengeRecord, bool), String> {
    let answer = entry.answer_key.trim();
    if answer.is_empty() {
        return Err(format!("{}: empty answer_key", entry.publication_hash));
    }

    let mut support_sections = entry.support_sections.clone();
    if support_sections.is_empty() {
        support_sections.push("s1".to_string());
    }

    let mut sections = Vec::new();
    for (section_index, section_id) in support_sections.iter().enumerate() {
        let text = format!(
            "Fixture paper {} section {} states {} (fixture {} of {}).",
            entry.publication_hash,
            section_id,
            answer,
            section_index + 1,
            support_sections.len(),
        );
        sections.push(PaperSection {
            section_id: section_id.clone(),
            title: section_id.clone(),
            text,
            section_hash: section_hash(&format!(
                "Fixture paper {} section {} states {} (fixture {} of {}).",
                entry.publication_hash,
                section_id,
                answer,
                section_index + 1,
                support_sections.len(),
            )),
        });
    }

    let paper = PaperRecord {
        schema_version: PAPER_SCHEMA_VERSION.to_string(),
        publication_hash: entry.publication_hash.clone(),
        content_hash: content_hash(&sections),
        dedupe_keys: vec![format!("fixture:{}", entry.publication_hash)],
        source_ids: vec![format!("fixture:{}", entry.publication_hash)],
        license: LicenseRecord {
            spdx: "CC-BY-4.0".to_string(),
            redistributable: true,
            source_url: Some(format!(
                "https://example.invalid/{}",
                entry.publication_hash
            )),
        },
        title: format!("Fixture paper {}", entry.publication_hash),
        authors: vec!["Fixture Generator".to_string()],
        abstract_text: format!("Synthetic fixture paper for {}", entry.publication_hash),
        sections,
        retrieval_receipts: vec![json!({
            "kind": "seed_fixture_bank",
            "source_manifest": source_manifest.display().to_string(),
            "entry_index": index + 1,
        })],
        published_at: Some("2026-05-13T00:00:00Z".to_string()),
    };

    let support = support_sections
        .iter()
        .zip(paper.sections.iter())
        .map(|(section_id, section)| SupportRef {
            section_id: section_id.clone(),
            section_hash: section.section_hash.clone(),
            quote_hash: None,
        })
        .collect::<Vec<_>>();

    let context_pack = pack_context(
        &paper,
        &support_sections,
        128_000,
        entry.context_pack.target_fill_ratio,
        entry.context_pack.output_reserve_tokens,
    )?;

    let challenge = finalize_challenge(ChallengeRecord {
        schema_version: CHALLENGE_SCHEMA_VERSION.to_string(),
        challenge_hash: entry.challenge_hash.clone(),
        publication_hash: entry.publication_hash.clone(),
        domain: "science".to_string(),
        topics: vec!["fixture".to_string()],
        difficulty_score: 0.5 + (index as f64 * 0.01),
        difficulty_components: BTreeMap::from([
            ("fixture_index".to_string(), index as f64),
            ("support_count".to_string(), support_sections.len() as f64),
        ]),
        question: entry.question.clone(),
        answer_key: AnswerKey {
            canonical: answer.to_string(),
            must_include: vec![answer.to_string()],
            must_not_include: vec![],
            aliases: vec![],
            numeric_tolerances: vec![],
            unit_tolerances: vec![],
        },
        support,
        context_pack,
        generator_agents: vec![],
        blind_answer_attempts: vec![],
        focused_answer_attempts: vec![],
        critic_attempts: vec![],
        audit_attempts: vec![],
        acceptance: AcceptanceRecord {
            accepted: entry.acceptance.accepted,
            auditor_agreement: 1.0,
            answerability: 1.0,
            blind_correct_rate: 0.0,
            focused_correct_rate: 1.0,
            ambiguity_flag: false,
            hash_mismatch: false,
            redistributable: true,
            reason: match entry.acceptance.reason.clone() {
                Some(value) => Some(value),
                None => Some("seeded fixture bank".to_string()),
            },
        },
        source_publication: None,
        focused_support_trials: vec![],
        saturated_blind_trials: vec![],
        judge_trials: vec![],
        context_packs: vec![],
        route_metadata: vec![],
        acceptance_metrics: None,
        artifact_provenance: None,
        artifact_hash: None,
    });

    let accepted = acceptance_passes(&challenge.acceptance);
    Ok((paper, challenge, accepted))
}

fn clear_json_files(root: &Path) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    let entries =
        fs::read_dir(root).map_err(|err| format!("read_dir {}: {err}", root.display()))?;
    for entry in entries {
        let path = entry.map_err(|err| err.to_string())?.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            fs::remove_file(&path).map_err(|err| format!("remove {}: {err}", path.display()))?;
        }
    }
    Ok(())
}

fn count_json_files(root: &Path) -> Result<usize, String> {
    if !root.exists() {
        return Ok(0);
    }
    let entries =
        fs::read_dir(root).map_err(|err| format!("read_dir {}: {err}", root.display()))?;
    let mut count = 0usize;
    for entry in entries {
        let path = entry.map_err(|err| err.to_string())?.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            count += 1;
        }
    }
    Ok(count)
}

fn default_target_fill_ratio() -> f64 {
    0.82
}

fn default_output_reserve_tokens() -> u64 {
    4096
}

fn default_true() -> bool {
    true
}
