use super::{
    canonicalize_paper, ensure_bank_layout, license_is_redistributable, read_papers, sha256_hex,
    write_json_pretty, LicenseRecord, PaperRecord, PaperSection, PAPER_SCHEMA_VERSION,
};
use regex::Regex;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

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
    let existing = read_papers(&config.bank).unwrap_or_default();
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
        search_europe_pmc(&client, config.query.as_deref(), limit, max_search_pages).await?;
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
        let xml_url = europe_pmc_full_text_url(&candidate.pmcid);
        let xml = match fetch_text(&client, &xml_url).await {
            Ok(xml) => xml,
            Err(err) => {
                skipped += 1;
                *skip_reasons.entry("fetch_error".to_string()).or_insert(0) += 1;
                rows.push(json!({"pmcid": candidate.pmcid, "status": "fetch_error", "error": err}));
                continue;
            }
        };
        fetched += 1;
        match build_paper_from_europe_pmc_xml(&xml, &xml_url, candidate) {
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
                *skip_reasons.entry(skip_reason(&err)).or_insert(0) += 1;
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

async fn search_europe_pmc(
    client: &reqwest::Client,
    query: Option<&str>,
    limit: usize,
    max_search_pages: usize,
) -> Result<Vec<EuropePmcCandidate>, String> {
    let base_query = query
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("OPEN_ACCESS:y HAS_FT:y SRC:PMC");
    let url = "https://www.ebi.ac.uk/europepmc/webservices/rest/search";
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut cursor = "*".to_string();
    let page_size = limit.clamp(1, 100).to_string();
    for _page in 0..max_search_pages {
        let response = client
            .get(url)
            .query(&[
                ("query", base_query),
                ("format", "json"),
                ("resultType", "core"),
                ("pageSize", &page_size),
                ("cursorMark", &cursor),
            ])
            .send()
            .await
            .map_err(|err| format!("Europe PMC search request failed: {err}"))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|err| format!("Europe PMC search response read failed: {err}"))?;
        if !status.is_success() {
            return Err(format!(
                "Europe PMC search returned HTTP {}: {text}",
                status.as_u16()
            ));
        }
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|err| format!("Europe PMC search response is not JSON: {err}"))?;
        let results = value
            .pointer("/resultList/result")
            .and_then(|value| value.as_array())
            .ok_or("Europe PMC search response missing resultList.result")?;
        if results.is_empty() {
            break;
        }
        for candidate in results.iter().filter_map(candidate_from_search_result) {
            if seen.insert(candidate.pmcid.clone()) {
                out.push(candidate);
            }
        }
        let next_cursor = value
            .get("nextCursorMark")
            .and_then(|value| value.as_str())
            .unwrap_or(cursor.as_str());
        if next_cursor == cursor {
            break;
        }
        cursor = next_cursor.to_string();
    }
    Ok(out)
}

fn candidate_from_search_result(value: &serde_json::Value) -> Option<EuropePmcCandidate> {
    let pmcid = value
        .get("pmcid")
        .and_then(|value| value.as_str())
        .or_else(|| value.get("pmcidVersion").and_then(|value| value.as_str()))?
        .trim_start_matches("PMC_")
        .to_string();
    let pmcid = if pmcid.starts_with("PMC") {
        pmcid
    } else {
        format!("PMC{pmcid}")
    };
    let mut source_ids = vec![format!("PMCID:{pmcid}")];
    for key in ["doi", "pmid", "id"] {
        if let Some(value) = value.get(key).and_then(|value| value.as_str()) {
            source_ids.push(format!("{}:{}", key.to_ascii_uppercase(), value));
        }
    }
    Some(EuropePmcCandidate {
        pmcid,
        source_ids,
        title: optional_clean_field(value, "title"),
        abstract_text: optional_clean_field(value, "abstractText"),
        license: optional_clean_field(value, "license"),
        published_at: optional_clean_field(value, "firstPublicationDate")
            .or_else(|| optional_clean_field(value, "journalInfo.printPublicationDate")),
    })
}

fn optional_clean_field(value: &serde_json::Value, key: &str) -> Option<String> {
    let field = if key.contains('.') {
        let pointer = format!("/{}", key.replace('.', "/"));
        value.pointer(&pointer)
    } else {
        value.get(key)
    }?;
    field
        .as_str()
        .map(clean_text)
        .filter(|value| !value.is_empty())
}

fn europe_pmc_full_text_url(pmcid: &str) -> String {
    format!("https://www.ebi.ac.uk/europepmc/webservices/rest/{pmcid}/fullTextXML")
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| format!("fetch {url}: {err}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| format!("read {url}: {err}"))?;
    if !status.is_success() {
        return Err(format!("HTTP {} from {url}: {text}", status.as_u16()));
    }
    Ok(text)
}

fn build_paper_from_europe_pmc_xml(
    xml: &str,
    source_url: &str,
    candidate: &EuropePmcCandidate,
) -> Result<PaperRecord, String> {
    if !xml.contains("<article") || !xml.contains("<body") {
        return Err("malformed XML: missing article body".to_string());
    }
    let license = license_from_candidate_or_xml(candidate.license.as_deref(), xml, source_url)?;
    if !license_is_redistributable(&license) {
        return Err(format!("license {} is not redistributable", license.spdx));
    }
    let title = candidate
        .title
        .clone()
        .or_else(|| first_tag_text(xml, "article-title"))
        .ok_or("full-text XML is missing article title")?;
    let abstract_text = candidate
        .abstract_text
        .clone()
        .or_else(|| first_tag_text(xml, "abstract"))
        .unwrap_or_default();
    let mut sections = Vec::new();
    if !abstract_text.trim().is_empty() {
        sections.push(PaperSection {
            section_id: "abstract".to_string(),
            title: "Abstract".to_string(),
            text: abstract_text.clone(),
            section_hash: String::new(),
        });
    }
    for (index, (heading, text)) in body_sections(xml).into_iter().enumerate() {
        if text.chars().count() < 120 {
            continue;
        }
        sections.push(PaperSection {
            section_id: format!("s{}", index + 1),
            title: heading.unwrap_or_else(|| format!("Section {}", index + 1)),
            text,
            section_hash: String::new(),
        });
    }
    if sections
        .iter()
        .filter(|section| section.section_id != "abstract")
        .count()
        == 0
    {
        return Err("full-text XML has no non-empty body sections".to_string());
    }
    let mut dedupe_keys = candidate.source_ids.clone();
    dedupe_keys.push(format!("pmcid:{}", candidate.pmcid));
    dedupe_keys.push(format!("xml-sha256:{}", sha256_hex(xml.as_bytes())));
    canonicalize_paper(PaperRecord {
        schema_version: PAPER_SCHEMA_VERSION.to_string(),
        publication_hash: String::new(),
        content_hash: String::new(),
        dedupe_keys,
        source_ids: candidate.source_ids.clone(),
        license,
        title,
        authors: authors_from_xml(xml),
        abstract_text,
        sections,
        retrieval_receipts: vec![json!({
            "kind": "discover_full_text",
            "provider": "europe-pmc",
            "pmcid": candidate.pmcid,
            "source_url": source_url,
            "xml_sha256": sha256_hex(xml.as_bytes())
        })],
        published_at: candidate.published_at.clone(),
    })
}

fn skip_reason(error: &str) -> String {
    if error.contains("license") {
        "license_rejected".to_string()
    } else if error.contains("body") || error.contains("abstract") {
        "abstract_or_empty_body".to_string()
    } else if error.contains("malformed XML") {
        "malformed_xml".to_string()
    } else {
        "rejected".to_string()
    }
}

pub fn parse_europe_pmc_full_text_xml(xml: &str, source_url: &str) -> Result<PaperRecord, String> {
    let candidate = EuropePmcCandidate {
        pmcid: "PMCUNKNOWN".to_string(),
        source_ids: vec!["PMCID:PMCUNKNOWN".to_string()],
        title: None,
        abstract_text: None,
        license: None,
        published_at: None,
    };
    build_paper_from_europe_pmc_xml(xml, source_url, &candidate)
}

fn license_from_candidate_or_xml(
    candidate_license: Option<&str>,
    xml: &str,
    source_url: &str,
) -> Result<LicenseRecord, String> {
    let raw = candidate_license
        .map(str::to_string)
        .or_else(|| capture_attr(xml, "license-type"))
        .or_else(|| first_tag_text(xml, "license-p"))
        .ok_or("full-text XML has no machine-readable license")?;
    let spdx = normalize_license(&raw).ok_or_else(|| format!("ambiguous license: {raw}"))?;
    Ok(LicenseRecord {
        spdx,
        redistributable: true,
        source_url: Some(source_url.to_string()),
    })
}

fn normalize_license(raw: &str) -> Option<String> {
    let upper = raw.to_ascii_uppercase();
    if upper.contains("CC0") || upper.contains("PUBLIC DOMAIN") {
        Some("CC0-1.0".to_string())
    } else if upper.contains("CC-BY-SA") || upper.contains("CC BY-SA") {
        Some("CC-BY-SA-4.0".to_string())
    } else if upper.contains("CC-BY-4")
        || upper.contains("CC BY 4")
        || upper.contains("CC BY")
        || upper.trim() == "CC-BY"
    {
        Some("CC-BY-4.0".to_string())
    } else if upper.contains("CC-BY-3") || upper.contains("CC BY 3") {
        Some("CC-BY-3.0".to_string())
    } else {
        None
    }
}

fn body_sections(xml: &str) -> Vec<(Option<String>, String)> {
    let body = match first_tag_raw(xml, "body") {
        Some(body) => body,
        None => return Vec::new(),
    };
    let sec_re = Regex::new(r"(?is)<sec\b[^>]*>(.*?)</sec>").expect("valid section regex");
    let title_re = Regex::new(r"(?is)<title\b[^>]*>(.*?)</title>").expect("valid title regex");
    let mut sections = Vec::new();
    for capture in sec_re.captures_iter(&body) {
        let raw = capture.get(1).map(|item| item.as_str()).unwrap_or("");
        let title = title_re
            .captures(raw)
            .and_then(|item| item.get(1))
            .map(|item| clean_xml_text(item.as_str()))
            .filter(|value| !value.is_empty());
        let text = clean_xml_text(raw);
        if !text.is_empty() {
            sections.push((title, text));
        }
    }
    if sections.is_empty() {
        let text = clean_xml_text(&body);
        if !text.is_empty() {
            sections.push((Some("Body".to_string()), text));
        }
    }
    sections
}

fn authors_from_xml(xml: &str) -> Vec<String> {
    let contrib_re =
        Regex::new(r#"(?is)<contrib\b[^>]*contrib-type=['"]author['"][^>]*>(.*?)</contrib>"#)
            .expect("valid contrib regex");
    contrib_re
        .captures_iter(xml)
        .filter_map(|capture| capture.get(1).map(|item| clean_xml_text(item.as_str())))
        .filter(|value| !value.is_empty())
        .take(50)
        .collect()
}

fn first_tag_text(xml: &str, tag: &str) -> Option<String> {
    first_tag_raw(xml, tag)
        .map(|raw| clean_xml_text(&raw))
        .filter(|value| !value.is_empty())
}

fn first_tag_raw(xml: &str, tag: &str) -> Option<String> {
    let re = Regex::new(&format!(r"(?is)<{tag}\b[^>]*>(.*?)</{tag}>")).ok()?;
    re.captures(xml)
        .and_then(|capture| capture.get(1))
        .map(|item| item.as_str().to_string())
}

fn capture_attr(xml: &str, attr: &str) -> Option<String> {
    let re = Regex::new(&format!(r#"(?is)\b{attr}\s*=\s*['"]([^'"]+)['"]"#)).ok()?;
    re.captures(xml)
        .and_then(|capture| capture.get(1))
        .map(|item| clean_text(item.as_str()))
        .filter(|value| !value.is_empty())
}

fn clean_xml_text(input: &str) -> String {
    let tags = Regex::new(r"(?is)<[^>]+>").expect("valid tag regex");
    clean_text(&decode_entities(&tags.replace_all(input, " ")))
}

fn clean_text(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"
    <article>
      <front>
        <article-meta>
          <title-group><article-title>Example OA Article</article-title></title-group>
          <permissions><license license-type="CC-BY"><license-p>Creative Commons Attribution License</license-p></license></permissions>
          <abstract><p>This is the abstract.</p></abstract>
        </article-meta>
      </front>
      <body>
        <sec><title>Results</title><p>The measured flux was 42.7 microjoules after annealing, which anchors the recall question with a precise value and method detail.</p></sec>
      </body>
    </article>
    "#;

    #[test]
    fn parses_europe_pmc_xml_into_canonical_paper() {
        let paper = parse_europe_pmc_full_text_xml(XML, "https://example.org/fullTextXML").unwrap();
        assert_eq!(paper.title, "Example OA Article");
        assert_eq!(paper.license.spdx, "CC-BY-4.0");
        assert!(paper
            .sections
            .iter()
            .any(|section| section.section_id == "s1"));
        assert!(paper
            .sections
            .iter()
            .all(|section| !section.section_hash.is_empty()));
    }

    #[test]
    fn rejects_non_redistributable_license() {
        let xml = XML.replace("CC-BY", "publisher-specific");
        assert!(parse_europe_pmc_full_text_xml(&xml, "https://example.org/fullTextXML").is_err());
    }
}
