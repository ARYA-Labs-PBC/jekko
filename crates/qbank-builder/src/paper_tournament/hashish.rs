use super::*;

pub fn final_paper_challenge_artifact_hash(
    artifact: &FinalPaperChallengeArtifact,
) -> Result<String, String> {
    let mut clone = artifact.clone();
    clone.artifact_hash.clear();
    let json = serde_json::to_vec(&clone).map_err(|err| err.to_string())?;
    Ok(sha256_hex(&json))
}

pub(crate) fn run_id(run_root: &Path) -> String {
    run_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("paper-qbank")
        .to_string()
}

pub(crate) fn domain_for_paper(paper: &PaperRecord) -> String {
    let key = match paper.source_ids.first().map(String::as_str) {
        Some(value) => value,
        None => match paper.dedupe_keys.first().map(String::as_str) {
            Some(value) => value,
            None => "",
        },
    };
    let index = key
        .rsplit('-')
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    match index % 4 {
        0 => "materials-science",
        1 => "computational-biology",
        2 => "applied-physics",
        _ => "systems-neuroscience",
    }
    .to_string()
}

pub(crate) fn smoke_paper(index: usize) -> PaperRecord {
    let labels = [
        ("cobalt memory", "cobalt", 42.7),
        ("rhodium lattice", "rhodium", 37.4),
        ("silicon enzyme", "silicon", 58.2),
        ("nickel cortex", "nickel", 26.9),
        ("argon polymer", "argon", 73.5),
        ("boron synapse", "boron", 19.8),
        ("titanium graph", "titanium", 64.1),
        ("xenon channel", "xenon", 31.6),
        ("iridium genome", "iridium", 88.3),
        ("gallium matrix", "gallium", 47.9),
    ];
    let (study, marker, value) = labels[index % labels.len()];
    let anchor = format!(
        "The calibrated recall anchor for the {study} study is {marker}-{} at {value:.1} microjoules after the third annealing pass",
        index + 17
    );
    let long_result = format!(
        "{anchor}. This result is reported as the decisive condition because earlier passes remained unstable under distractor load. The authors state that the anchor should be treated as a paper-local constant, not as a general material property. The evaluation section repeats that {marker}-{} at {value:.1} microjoules is the only setting that survives the saturated recall test with all controls held fixed.",
        index + 17
    );
    let slug = study.replace(' ', "-");
    PaperRecord {
        schema_version: PAPER_SCHEMA_VERSION.to_string(),
        publication_hash: String::new(),
        content_hash: String::new(),
        dedupe_keys: vec![format!("doi:10.5555/{slug}-{index}")],
        source_ids: vec![format!("doi:10.5555/{slug}-{index}")],
        license: LicenseRecord {
            spdx: "CC-BY-4.0".to_string(),
            redistributable: true,
            source_url: Some(format!(
                "https://qbank-smoke.openaccess.local/papers/{slug}-{index}"
            )),
        },
        title: format!("{} Calibration Study", title_case(study)),
        authors: vec!["QBank Smoke Authors".to_string()],
        abstract_text: format!(
            "A redistributable {study} study used for local paper tournament smoke validation."
        ),
        sections: vec![
            PaperSection {
                section_id: "abstract".to_string(),
                title: "Abstract".to_string(),
                text: format!(
                    "This {study} study evaluates recall anchors under saturated context pressure."
                ),
                section_hash: String::new(),
            },
            PaperSection {
                section_id: "results".to_string(),
                title: "Results".to_string(),
                text: long_result,
                section_hash: String::new(),
            },
            PaperSection {
                section_id: "methods".to_string(),
                title: "Methods".to_string(),
                text: "The method used three annealing passes, fixed control loads, and randomized distractor paragraphs to measure recall stability. Every measurement was repeated under the same public-license protocol so that downstream benchmark records can retain the full body text."
                    .to_string(),
                section_hash: String::new(),
            },
        ],
        retrieval_receipts: vec![json!({
            "kind": "paper_tournament_smoke",
            "retrieved_at": "2026-05-13T00:00:00Z",
            "license_spdx": "CC-BY-4.0",
            "smoke_index": index
        })],
        published_at: Some("2026-05-13T00:00:00Z".to_string()),
    }
}

pub(crate) fn title_case(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!(
                    "{}{}",
                    first.to_uppercase(),
                    chars.as_str().to_ascii_lowercase()
                ),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
