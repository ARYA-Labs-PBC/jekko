use super::{
    content_hash, license_is_redistributable, CanonicalPaperText, PaperRecord, PaperTextSection,
};

pub fn canonical_paper_text(paper: &PaperRecord, non_production: bool) -> CanonicalPaperText {
    let full_text = paper
        .sections
        .iter()
        .map(|section| format!("{}\n{}", section.title, section.text))
        .collect::<Vec<_>>()
        .join("\n\n");
    let mut source_urls = Vec::new();
    if let Some(url) = paper.license.source_url.clone() {
        source_urls.push(url);
    }
    CanonicalPaperText {
        title: paper.title.clone(),
        abstract_text: paper.abstract_text.clone(),
        full_text,
        sections: paper
            .sections
            .iter()
            .map(|section| PaperTextSection {
                section_id: section.section_id.clone(),
                title: section.title.clone(),
                text: section.text.clone(),
                section_hash: section.section_hash.clone(),
            })
            .collect(),
        source_urls,
        license_spdx: paper.license.spdx.clone(),
        redistributable: paper.license.redistributable,
        content_hash: content_hash(&paper.sections),
        non_production,
    }
}

pub fn validate_full_text_paper(
    paper: &PaperRecord,
    strict_production: bool,
) -> Result<(), String> {
    if !license_is_redistributable(&paper.license) {
        return Err(format!(
            "paper {} has non-allowlisted license {}",
            paper.publication_hash, paper.license.spdx
        ));
    }
    if paper.retrieval_receipts.is_empty() {
        return Err(format!(
            "paper {} is missing retrieval receipts",
            paper.publication_hash
        ));
    }
    let body_sections = paper
        .sections
        .iter()
        .filter(|section| {
            let id = section.section_id.to_ascii_lowercase();
            let title = section.title.to_ascii_lowercase();
            id != "abstract" && title != "abstract" && id != "source"
        })
        .collect::<Vec<_>>();
    if body_sections.is_empty() {
        return Err(format!(
            "paper {} has no full body sections",
            paper.publication_hash
        ));
    }
    let body_chars = body_sections
        .iter()
        .map(|section| section.text.trim().chars().count())
        .sum::<usize>();
    if strict_production && body_chars < 200 {
        return Err(format!(
            "paper {} looks abstract-only or snippet-only",
            paper.publication_hash
        ));
    }
    for section in &paper.sections {
        if section.section_id.trim().is_empty() || section.section_hash.trim().is_empty() {
            return Err(format!(
                "paper {} has section without id/hash",
                paper.publication_hash
            ));
        }
        if section.text.trim().is_empty() {
            return Err(format!(
                "paper {} section {} is empty",
                paper.publication_hash, section.section_id
            ));
        }
    }
    if strict_production {
        let source_url = paper.license.source_url.as_deref().unwrap_or("");
        if source_url.trim().is_empty() {
            return Err(format!(
                "paper {} is missing source URL",
                paper.publication_hash
            ));
        }
        if source_url.contains("example.invalid")
            || source_url.contains("qbank-smoke.openaccess.local")
        {
            return Err(format!(
                "paper {} uses a fixture source URL",
                paper.publication_hash
            ));
        }
        if paper
            .retrieval_receipts
            .iter()
            .any(|receipt| receipt.to_string().contains("seed_fixture_bank"))
        {
            return Err(format!(
                "paper {} has fixture retrieval provenance",
                paper.publication_hash
            ));
        }
    }
    Ok(())
}
