use super::model::{
    stable_section_hash, BankValidation, PaperRecord, PRODUCTION_MANIFEST_SCHEMA_VERSION,
};
use super::parse::{collect_json_files, read_challenges, read_paper};
use crate::qbank_hash::sha256_hex;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::Path;

#[path = "validation_checks.rs"]
mod validation_checks;
#[path = "validation_order.rs"]
mod validation_order;
#[path = "validation_support.rs"]
mod validation_support;
use validation_checks::{
    validate_acceptance, validate_challenge_hash, validate_paper_presence,
    validate_production_challenge,
};
use validation_order::{challenge_order_plain, context_token_budget};

pub fn validate_bank(
    root: &Path,
    allow_empty: bool,
    top_n: usize,
    min_required_accepted: usize,
) -> Result<BankValidation, String> {
    let mut result = BankValidation {
        min_required_accepted,
        ..BankValidation::default()
    };
    let mut paper_paths = Vec::new();
    collect_json_files(&root.join("papers"), &mut paper_paths)?;
    let mut challenge_paths = Vec::new();
    collect_json_files(&root.join("challenges"), &mut challenge_paths)?;
    let mut rejected_paths = Vec::new();
    collect_json_files(&root.join("rejected"), &mut rejected_paths)?;
    let allow_fixture_qbank = env::var("memory_benchmark_dev_qbank").ok().as_deref() == Some("1");
    result.strict_production = !allow_fixture_qbank;
    match read_manifest_schema(root) {
        Ok(manifest_schema) => {
            result.manifest_schema = manifest_schema;
        }
        Err(err) => {
            result
                .warnings
                .push(format!("manifest schema unavailable: {err}"));
        }
    }

    let mut seen_publications = BTreeSet::new();
    let mut papers_by_hash = BTreeMap::new();
    for path in &paper_paths {
        match read_paper(path) {
            Ok(paper) => {
                if !seen_publications.insert(paper.publication_hash.clone()) {
                    result.duplicate_publications += 1;
                    result
                        .errors
                        .push(format!("duplicate publication {}", paper.publication_hash));
                }
                if !paper.redistributable {
                    result
                        .errors
                        .push(format!("non-redistributable paper {}", path.display()));
                }
                if !allow_fixture_qbank && paper_has_fixture_provenance(&paper) {
                    result
                        .errors
                        .push(format!("fixture provenance in paper {}", path.display()));
                }
                if !allow_fixture_qbank && paper.sections.is_empty() {
                    result.errors.push(format!(
                        "paper {} has no full-text sections",
                        path.display()
                    ));
                }
                for section in &paper.sections {
                    if !allow_fixture_qbank && section.text.trim().is_empty() {
                        result.errors.push(format!(
                            "{} section {} has empty full text",
                            path.display(),
                            section.section_id
                        ));
                    }
                    if !allow_fixture_qbank && section.section_hash.trim().is_empty() {
                        result.errors.push(format!(
                            "{} section {} missing section_hash",
                            path.display(),
                            section.section_id
                        ));
                    }
                    let expected = stable_section_hash(&section.text);
                    if !section.section_hash.is_empty() && section.section_hash != expected {
                        result.errors.push(format!(
                            "{} section {} hash mismatch",
                            path.display(),
                            section.section_id
                        ));
                    }
                }
                papers_by_hash.insert(paper.publication_hash.clone(), paper);
            }
            Err(err) => result.errors.push(err),
        }
    }

    let mut accepted = Vec::new();
    let mut seen_challenges = BTreeSet::new();
    let mut publication_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut domain_counts: BTreeMap<String, usize> = BTreeMap::new();
    for path in &challenge_paths {
        match read_challenges(path) {
            Ok(challenges) => {
                for challenge in challenges {
                    if !seen_challenges.insert(challenge.challenge_hash.clone()) {
                        result
                            .errors
                            .push(format!("duplicate challenge {}", challenge.challenge_hash));
                    }
                    if let Err(err) = validate_challenge_hash(&challenge) {
                        result.errors.push(format!("{}: {err}", path.display()));
                    }
                    if let Err(err) = validate_acceptance(&challenge) {
                        result.errors.push(format!("{}: {err}", path.display()));
                    }
                    if challenge.context_pack.estimated_tokens
                        > context_token_budget(&challenge.context_pack)
                    {
                        result.errors.push(format!(
                            "{}: context pack exceeds token limit",
                            path.display()
                        ));
                    }
                    if let Err(err) =
                        validate_paper_presence(&challenge, &papers_by_hash, allow_fixture_qbank)
                    {
                        result.errors.push(format!("{}: {err}", path.display()));
                    }
                    if !allow_fixture_qbank {
                        for err in validate_production_challenge(&challenge) {
                            result.errors.push(format!("{}: {err}", path.display()));
                        }
                    }
                    *publication_counts
                        .entry(challenge.publication_hash.clone())
                        .or_default() += 1;
                    *domain_counts.entry(challenge.domain.clone()).or_default() += 1;
                    accepted.push(challenge);
                }
            }
            Err(err) => result.errors.push(err),
        }
    }
    result.accepted_challenges = accepted.len();
    result.rejected_challenges = rejected_paths.len();
    result.unique_publications = publication_counts.len();
    result.distinct_domains = domain_counts.len();
    result.top_selected = accepted.len().min(top_n);
    result.max_publication_share = publication_counts.values().copied().max().unwrap_or(0) as f32
        / accepted.len().max(1) as f32;
    result.max_domain_share =
        domain_counts.values().copied().max().unwrap_or(0) as f32 / accepted.len().max(1) as f32;
    result.source_diversity = if accepted.is_empty() {
        0.0
    } else {
        result.unique_publications as f32 / accepted.len() as f32
    };
    accepted.sort_by(challenge_order_plain);
    let mut manifest_material = String::new();
    for challenge in accepted.iter().take(result.top_selected) {
        manifest_material.push_str(&challenge.challenge_hash);
        manifest_material.push('\n');
    }
    result.manifest_hash = sha256_hex(manifest_material.as_bytes());

    if !allow_empty && result.accepted_challenges == 0 {
        result
            .errors
            .push("bank has no accepted challenges".to_string());
    }
    let bank_is_empty =
        paper_paths.is_empty() && challenge_paths.is_empty() && rejected_paths.is_empty();
    if allow_fixture_qbank {
        result
            .warnings
            .push("dev_only fixture qbank mode enabled".to_string());
    } else if !(allow_empty && bank_is_empty && result.accepted_challenges == 0) {
        if result.manifest_schema != PRODUCTION_MANIFEST_SCHEMA_VERSION {
            result.errors.push(format!(
                "manifest schema is not production v3: {}",
                if result.manifest_schema.is_empty() {
                    "missing"
                } else {
                    result.manifest_schema.as_str()
                }
            ));
        }
        if result.accepted_challenges < min_required_accepted {
            result.errors.push(format!(
                "production bank has {} accepted challenges; need at least {}",
                result.accepted_challenges, min_required_accepted
            ));
        }
        let required_unique_publications = ((min_required_accepted as f32) * 0.34).ceil() as usize;
        if result.unique_publications < required_unique_publications {
            result.errors.push(format!(
                "production bank has {} unique publications; need at least {}",
                result.unique_publications, required_unique_publications
            ));
        }
        for (publication, count) in &publication_counts {
            if *count > 3 {
                result.errors.push(format!(
                    "publication {} exceeds 3 accepted challenges ({})",
                    publication, count
                ));
            }
        }
        if min_required_accepted >= 10
            && result.accepted_challenges >= 10
            && result.max_domain_share > 0.35
        {
            let mut worst = String::new();
            let mut worst_count = 0usize;
            for (domain, count) in &domain_counts {
                if *count > worst_count {
                    worst = domain.clone();
                    worst_count = *count;
                }
            }
            result.errors.push(format!(
                "domain {} exceeds 35% share ({:.1}%)",
                worst,
                result.max_domain_share * 100.0
            ));
        }
    }
    result.qbank_trusted = !allow_fixture_qbank
        && result.accepted_challenges >= min_required_accepted
        && result.manifest_schema == PRODUCTION_MANIFEST_SCHEMA_VERSION
        && result.errors.is_empty();
    Ok(result)
}

fn paper_has_fixture_provenance(paper: &PaperRecord) -> bool {
    paper.title.to_ascii_lowercase().contains("fixture")
        || paper.title.to_ascii_lowercase().contains("generated")
        || paper
            .dedupe_keys
            .iter()
            .chain(paper.source_ids.iter())
            .any(|value| {
                let lower = value.to_ascii_lowercase();
                lower.contains("fixture") || lower.contains("generated")
            })
        || paper
            .source_url
            .as_deref()
            .map(|url| {
                url.contains("example.invalid") || url.contains("qbank-smoke.openaccess.local")
            })
            .unwrap_or(false)
        || paper.retrieval_kinds.iter().any(|kind| {
            kind == "seed_fixture_bank" || kind == "generated" || kind.contains("smoke")
        })
}

fn read_manifest_schema(root: &Path) -> Result<String, String> {
    let path = root.join("manifests/latest.json");
    let text =
        std::fs::read_to_string(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let parsed =
        crate::json::parse(&text).map_err(|err| format!("parse {}: {err}", path.display()))?;
    let obj = match parsed {
        crate::json::Json::Object(obj) => obj,
        _ => return Err(format!("{}: manifest must be an object", path.display())),
    };
    Ok(match obj.get("schema_version") {
        Some(crate::json::Json::Str(value)) => value.clone(),
        _ => String::new(),
    })
}
