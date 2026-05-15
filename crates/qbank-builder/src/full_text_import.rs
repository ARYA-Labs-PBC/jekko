use super::{
    canonicalize_paper, ensure_bank_layout, license_is_redistributable, read_papers, sha256_hex,
    write_json_pretty, LicenseRecord, PaperRecord, PaperSection, PAPER_SCHEMA_VERSION,
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[path = "full_text_import_detail.rs"]
mod detail;
#[path = "full_text_import_detail_support.rs"]
mod support;

pub use detail::parse_europe_pmc_full_text_xml;

#[derive(Debug, Clone)]
pub struct FullTextDiscoveryConfig {
    pub provider: String,
    pub limit: usize,
    pub min_written: usize,
    pub max_search_pages: usize,
    pub bank: PathBuf,
    pub run_root: PathBuf,
    pub query: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FullTextDiscoverySummary {
    pub provider: String,
    pub searched: usize,
    pub fetched: usize,
    pub written: usize,
    pub skipped: usize,
    pub receipt_path: PathBuf,
    pub candidate_manifest_path: PathBuf,
}

#[derive(Debug, Clone)]
struct EuropePmcCandidate {
    pmcid: String,
    source_ids: Vec<String>,
    title: Option<String>,
    abstract_text: Option<String>,
    license: Option<String>,
    published_at: Option<String>,
}

pub async fn discover_full_text(
    config: &FullTextDiscoveryConfig,
) -> Result<FullTextDiscoverySummary, String> {
    if config.provider != "europe-pmc" {
        return Err(format!(
            "unsupported full-text provider {:?}; expected europe-pmc",
            config.provider
        ));
    }
    ensure_bank_layout(&config.bank)?;
    let existing = match read_papers(&config.bank) {
        Ok(papers) => papers,
        Err(_) => Vec::new(),
    };
    let mut known_dedupe_by_key = existing
        .iter()
        .flat_map(|paper| {
            paper
                .dedupe_keys
                .iter()
                .cloned()
                .map(|key| (key, paper.publication_hash.clone()))
                .collect::<Vec<_>>()
        })
        .collect::<BTreeMap<_, _>>();
    let mut known_dedupe = known_dedupe_by_key.keys().cloned().collect::<BTreeSet<_>>();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|err| format!("build Europe PMC http client: {err}"))?;

    let limit = config.limit.clamp(1, 10_000);
    let min_written = config.min_written.min(limit);
    let max_search_pages = config.max_search_pages.clamp(1, 1_000);
    let candidates =
        detail::search_europe_pmc(&client, config.query.as_deref(), limit, max_search_pages)
            .await?;
    let mut fetched = 0usize;
    let mut written = 0usize;
    let mut skipped = 0usize;
    let mut rows = Vec::new();
    let mut accepted_manifest_rows = Vec::new();
    let mut skip_reasons = BTreeMap::<String, usize>::new();
    for candidate in candidates.iter() {
        if accepted_manifest_rows.len() >= limit && written >= min_written {
            break;
        }
        let dedupe_key = format!("pmcid:{}", candidate.pmcid);
        if known_dedupe.contains(&dedupe_key) {
            skipped += 1;
            *skip_reasons.entry("duplicate".to_string()).or_insert(0) += 1;
            if let Some(publication_hash) = known_dedupe_by_key.get(&dedupe_key) {
                accepted_manifest_rows.push(json!({
                    "pmcid": candidate.pmcid,
                    "status": "existing",
                    "publication_hash": publication_hash,
                    "dedupe_key": dedupe_key
                }));
            }
            rows.push(
                json!({"pmcid": candidate.pmcid, "status": "duplicate", "dedupe_key": dedupe_key}),
            );
            continue;
        }
        let xml_url = detail::europe_pmc_full_text_url(&candidate.pmcid);
        let xml = match detail::fetch_text(&client, &xml_url).await {
            Ok(xml) => xml,
            Err(err) => {
                skipped += 1;
                *skip_reasons.entry("fetch_error".to_string()).or_insert(0) += 1;
                rows.push(json!({"pmcid": candidate.pmcid, "status": "fetch_error", "error": err}));
                continue;
            }
        };
        fetched += 1;
        match detail::build_paper_from_europe_pmc_xml(&xml, &xml_url, candidate) {
            Ok(paper) => {
                let path = config
                    .bank
                    .join("papers")
                    .join(format!("{}.json", paper.publication_hash));
                if path.exists() {
                    skipped += 1;
                    rows.push(json!({
                        "pmcid": candidate.pmcid,
                        "status": "duplicate_hash",
                        "publication_hash": paper.publication_hash
                    }));
                    known_dedupe.extend(paper.dedupe_keys.iter().cloned());
                    for key in &paper.dedupe_keys {
                        known_dedupe_by_key.insert(key.clone(), paper.publication_hash.clone());
                    }
                    accepted_manifest_rows.push(json!({
                        "pmcid": candidate.pmcid,
                        "status": "existing",
                        "publication_hash": paper.publication_hash,
                        "path": path.display().to_string()
                    }));
                    continue;
                }
                write_json_pretty(&path, &paper)?;
                known_dedupe.extend(paper.dedupe_keys.iter().cloned());
                for key in &paper.dedupe_keys {
                    known_dedupe_by_key.insert(key.clone(), paper.publication_hash.clone());
                }
                written += 1;
                accepted_manifest_rows.push(json!({
                    "pmcid": candidate.pmcid,
                    "status": "written",
                    "publication_hash": paper.publication_hash,
                    "path": path.display().to_string()
                }));
                rows.push(json!({
                    "pmcid": candidate.pmcid,
                    "status": "written",
                    "publication_hash": paper.publication_hash,
                    "path": path.display().to_string()
                }));
            }
            Err(err) => {
                skipped += 1;
                *skip_reasons.entry(detail::skip_reason(&err)).or_insert(0) += 1;
                rows.push(json!({"pmcid": candidate.pmcid, "status": "rejected", "error": err}));
            }
        }
    }
    if accepted_manifest_rows.len() < limit || written < min_written {
        return Err(format!(
            "Europe PMC discovery produced {} valid papers with {written} newly written; required {limit} valid and at least {min_written} written after {max_search_pages} pages",
            accepted_manifest_rows.len()
        ));
    }

    let receipt_path = config.run_root.join("full-text-discovery.json");
    let candidate_manifest_path = config
        .run_root
        .join("reports")
        .join("candidate-manifest.json");
    write_json_pretty(
        &candidate_manifest_path,
        &json!({
            "schema_version": "opencode-qbank-candidate-manifest-v1",
            "provider": config.provider,
            "bank": config.bank.display().to_string(),
            "limit": limit,
            "min_written": min_written,
            "max_search_pages": max_search_pages,
            "accepted_count": accepted_manifest_rows.len(),
            "written": written,
            "skip_reasons": skip_reasons,
            "papers": accepted_manifest_rows
        }),
    )?;
    write_json_pretty(
        &receipt_path,
        &json!({
            "schema_version": "opencode-qbank-full-text-discovery-v1",
            "provider": config.provider,
            "bank": config.bank.display().to_string(),
            "limit": limit,
            "min_written": min_written,
            "max_search_pages": max_search_pages,
            "query": config.query,
            "searched": candidates.len(),
            "fetched": fetched,
            "written": written,
            "skipped": skipped,
            "candidate_manifest": candidate_manifest_path.display().to_string(),
            "skip_reasons": skip_reasons,
            "rows": rows,
            "source_notes": [
                "Europe PMC OA full-text XML endpoint: /webservices/rest/{PMCID}/fullTextXML",
                "Only allowlisted redistributable SPDX licenses are imported."
            ]
        }),
    )?;

    Ok(FullTextDiscoverySummary {
        provider: config.provider.clone(),
        searched: candidates.len(),
        fetched,
        written,
        skipped,
        receipt_path,
        candidate_manifest_path,
    })
}
