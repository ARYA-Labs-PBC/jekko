use super::support::{
    authors_from_xml, body_sections, candidate_from_search_result, first_tag_text,
    license_from_candidate_or_xml,
};
use super::*;
use serde_json::json;
use std::collections::BTreeSet;

pub(super) async fn search_europe_pmc(
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
        let next_cursor = match value.get("nextCursorMark").and_then(|value| value.as_str()) {
            Some(value) => value,
            None => cursor.as_str(),
        };
        if next_cursor == cursor {
            break;
        }
        cursor = next_cursor.to_string();
    }
    Ok(out)
}
pub(super) fn europe_pmc_full_text_url(pmcid: &str) -> String {
    format!("https://www.ebi.ac.uk/europepmc/webservices/rest/{pmcid}/fullTextXML")
}

pub(super) async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
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

pub(super) fn build_paper_from_europe_pmc_xml(
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
    let title = match candidate.title.clone() {
        Some(value) => value,
        None => match first_tag_text(xml, "article-title") {
            Some(value) => value,
            None => return Err("full-text XML is missing article title".to_string()),
        },
    };
    let abstract_text = match candidate.abstract_text.clone() {
        Some(value) => value,
        None => match first_tag_text(xml, "abstract") {
            Some(value) => value,
            None => String::new(),
        },
    };
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
            title: match heading {
                Some(value) => value,
                None => format!("Section {}", index + 1),
            },
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

pub(super) fn skip_reason(error: &str) -> String {
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
