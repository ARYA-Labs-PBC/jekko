use super::*;
use regex::Regex;

pub(super) fn candidate_from_search_result(
    value: &serde_json::Value,
) -> Option<EuropePmcCandidate> {
    let pmcid = match value.get("pmcid").and_then(|value| value.as_str()) {
        Some(value) => value,
        None => value.get("pmcidVersion").and_then(|value| value.as_str())?,
    }
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
        published_at: match optional_clean_field(value, "firstPublicationDate") {
            Some(value) => Some(value),
            None => optional_clean_field(value, "journalInfo.printPublicationDate"),
        },
    })
}

pub(super) fn optional_clean_field(value: &serde_json::Value, key: &str) -> Option<String> {
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

pub(super) fn license_from_candidate_or_xml(
    candidate_license: Option<&str>,
    xml: &str,
    source_url: &str,
) -> Result<LicenseRecord, String> {
    let raw = match candidate_license.map(str::to_string) {
        Some(value) => Some(value),
        None => capture_attr(xml, "license-type"),
    };
    let raw = match raw {
        Some(value) => Some(value),
        None => first_tag_text(xml, "license-p"),
    }
    .ok_or("full-text XML has no machine-readable license")?;
    let spdx = match normalize_license(&raw) {
        Some(value) => value,
        None => return Err(format!("ambiguous license: {raw}")),
    };
    Ok(LicenseRecord {
        spdx,
        redistributable: true,
        source_url: Some(source_url.to_string()),
    })
}

pub(super) fn normalize_license(raw: &str) -> Option<String> {
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

pub(super) fn body_sections(xml: &str) -> Vec<(Option<String>, String)> {
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

pub(super) fn authors_from_xml(xml: &str) -> Vec<String> {
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

pub(super) fn first_tag_text(xml: &str, tag: &str) -> Option<String> {
    first_tag_raw(xml, tag)
        .map(|raw| clean_xml_text(&raw))
        .filter(|value| !value.is_empty())
}

pub(super) fn first_tag_raw(xml: &str, tag: &str) -> Option<String> {
    let re = Regex::new(&format!(r"(?is)<{tag}\b[^>]*>(.*?)</{tag}>")).ok()?;
    re.captures(xml)
        .and_then(|capture| capture.get(1))
        .map(|item| item.as_str().to_string())
}

pub(super) fn capture_attr(xml: &str, attr: &str) -> Option<String> {
    let re = Regex::new(&format!(r#"(?is)\b{attr}\s*=\s*['"]([^'"]+)['"]"#)).ok()?;
    re.captures(xml)
        .and_then(|capture| capture.get(1))
        .map(|item| clean_text(item.as_str()))
        .filter(|value| !value.is_empty())
}

pub(super) fn clean_xml_text(input: &str) -> String {
    let tags = Regex::new(r"(?is)<[^>]+>").expect("valid tag regex");
    clean_text(&decode_entities(&tags.replace_all(input, " ")))
}

pub(super) fn clean_text(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn decode_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}
