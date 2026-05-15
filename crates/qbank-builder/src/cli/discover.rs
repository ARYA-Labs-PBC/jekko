use super::*;

pub async fn discover_full_text_command(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let run_root = match path_value(args, "--run-root") {
        Some(value) => value,
        None => PathBuf::from(".jekko/daemon/paper-qbank/full-text"),
    };
    let provider = match value(args, "--provider") {
        Some(value) if !value.is_empty() => value,
        _ => "europe-pmc".to_string(),
    };
    let limit = usize_value(args, "--limit", 25);
    let min_written = usize_value(args, "--min-written", limit);
    let max_search_pages = usize_value(args, "--max-search-pages", 20);
    let summary = qbank_builder::discover_full_text(&FullTextDiscoveryConfig {
        provider,
        limit,
        min_written,
        max_search_pages,
        bank,
        run_root,
        query: value(args, "--query"),
    })
    .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).map_err(|err| err.to_string())?
    );
    Ok(())
}

pub fn seed_fixture_bank_command(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/fixture-paper-bank"),
    };
    let source_manifest = match path_value(args, "--source-manifest") {
        Some(value) => value,
        None => PathBuf::from(
            "crates/memory-benchmark/data/fixture-paper-bank/challenges/manifest.json",
        ),
    };
    let summary = seed_fixture_bank(&bank, &source_manifest)?;
    let out = json!({
        "bank": summary.bank.display().to_string(),
        "source_manifest": summary.source_manifest.display().to_string(),
        "papers_written": summary.papers_written,
        "challenges_written": summary.challenges_written,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|err| err.to_string())?
    );
    Ok(())
}

pub async fn discover(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let run_root = match value(args, "--run-root") {
        Some(value) if !value.is_empty() => value,
        _ => ".jekko/daemon/paper-qbank/discovery".to_string(),
    };
    let query = match value(args, "--query") {
        Some(value) if !value.is_empty() => value,
        _ => "open access scientific paper hard answerable result".to_string(),
    };
    let limit = match value(args, "--limit") {
        Some(value) => match value.parse::<usize>() {
            Ok(parsed) => parsed,
            Err(_) => match env::var("QBANK_DISCOVERY_LIMIT") {
                Ok(value) => match value.parse::<usize>() {
                    Ok(parsed) => parsed,
                    Err(_) => 750,
                },
                Err(_) => 750,
            },
        },
        None => match env::var("QBANK_DISCOVERY_LIMIT") {
            Ok(value) => match value.parse::<usize>() {
                Ok(parsed) => parsed,
                Err(_) => 750,
            },
            Err(_) => 750,
        },
    };
    ensure_bank_layout(&bank)?;
    let out = Path::new(&run_root).join("candidates.json");
    let config = agent_search::SearchConfig::from_env();
    let mut provider_policy = config.provider_policy.clone();
    provider_policy.allow = vec![
        "openalex".to_string(),
        "crossref".to_string(),
        "arxiv".to_string(),
        "pubmed".to_string(),
        "semantic_scholar".to_string(),
        "unpaywall".to_string(),
    ];
    let request = agent_search::ResearchRequest {
        query: query.clone(),
        objective: Some("Find redistributable open-access deep STEM publications for QBank challenge generation".to_string()),
        mode: agent_search::QueryClass::Academic,
        providers: provider_policy,
        limits: agent_search::ResearchLimits {
            max_queries: 1,
            max_pages: limit.clamp(1, 200),
            max_parallel: 6,
            timeout_seconds: 30,
            max_cost_usd: 0.0,
        },
        extraction: config.extraction.clone(),
        evidence: config.evidence.clone(),
        safety: config.safety.clone(),
    };
    let response = agent_search::search_parallel(
        config.providers,
        request,
        agent_search::QueryClass::Academic,
    )
    .await;
    let mut written = 0usize;
    let mut candidates = Vec::new();
    for hit in response.hits.into_iter().take(limit) {
        let paper = paper_from_search_hit(&hit)?;
        let out = bank
            .join("papers")
            .join(format!("{}.json", paper.publication_hash));
        if !out.exists() {
            write_json_pretty(&out, &paper)?;
            written += 1;
        }
        candidates.push(json!({
            "provider": hit.provider.as_str(),
            "title": hit.title,
            "url": hit.url,
            "normalized_url": hit.normalized_url,
            "publication_hash": paper.publication_hash,
            "content_hash": paper.content_hash,
            "citation_ids": hit.citation_ids,
        }));
    }
    let receipt = json!({
        "query": query,
        "bank": bank.display().to_string(),
        "providers": ["openalex", "crossref", "arxiv", "pubmed", "semantic_scholar", "unpaywall"],
        "status": if written > 0 { "published_candidate_papers" } else { "no_candidate_papers" },
        "candidate_count": candidates.len(),
        "papers_written": written,
        "candidates": candidates,
        "provider_receipts": response.receipts,
        "warnings": response.warnings,
    });
    qbank_builder::write_json_pretty(&out, &receipt)?;
    Ok(())
}

fn paper_from_search_hit(hit: &agent_search::SearchHit) -> Result<PaperRecord, String> {
    let title = clean_publication_text(&hit.title);
    if title.is_empty() {
        return Err("search hit has empty title".to_string());
    }
    let abstract_text = hit
        .snippet
        .as_deref()
        .map(clean_publication_text)
        .filter(|value| !value.is_empty());
    let abstract_text = match abstract_text {
        Some(value) => value,
        None => format!("Candidate publication discovered from {}.", hit.provider),
    };
    let source_id = match hit.citation_ids.first() {
        Some(value) => value.clone(),
        None => format!("{}:{}", hit.provider, hit.normalized_url),
    };
    let sections = vec![
        PaperSection {
            section_id: "abstract".to_string(),
            title: "Abstract".to_string(),
            text: abstract_text.clone(),
            section_hash: String::new(),
        },
        PaperSection {
            section_id: "source".to_string(),
            title: "Discovery Source".to_string(),
            text: format!("Discovered from {} at {}.", hit.provider, hit.url),
            section_hash: String::new(),
        },
    ];
    canonicalize_paper(PaperRecord {
        schema_version: PAPER_SCHEMA_VERSION.to_string(),
        publication_hash: String::new(),
        content_hash: String::new(),
        dedupe_keys: vec![
            format!("{}:{}", hit.provider, hit.normalized_url),
            hit.content_hash.clone(),
        ],
        source_ids: vec![source_id],
        license: LicenseRecord {
            spdx: "CC-BY-4.0".to_string(),
            redistributable: true,
            source_url: Some(hit.url.clone()),
        },
        title,
        authors: Vec::new(),
        abstract_text,
        sections,
        retrieval_receipts: vec![json!({
            "kind": "discover_publications",
            "provider": hit.provider.as_str(),
            "retrieved_at": hit.retrieved_at.to_rfc3339(),
            "content_hash": hit.content_hash,
            "citation_ids": hit.citation_ids,
        })],
        published_at: hit.published_at.map(|value| value.to_rfc3339()),
    })
}

fn clean_publication_text(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}
