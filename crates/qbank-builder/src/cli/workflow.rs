use super::*;

pub fn publish_paper(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let input = path_value(args, "--input").ok_or("--input is required")?;
    ensure_bank_layout(&bank)?;
    let paper: PaperRecord = read_json(&input)?;
    let paper = canonicalize_paper(paper)?;
    let out = bank
        .join("papers")
        .join(format!("{}.json", paper.publication_hash));
    if out.exists() && !args.iter().any(|arg| arg == "--replace") {
        return Err(format!("paper already exists: {}", out.display()));
    }
    write_json_pretty(&out, &paper)
}

pub fn make_work(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let out = match path_value(args, "--out") {
        Some(value) => value,
        None => PathBuf::from(".jekko/daemon/paper-qbank/work.jsonl"),
    };
    let mut paper_paths = Vec::new();
    qbank_builder::collect_json_files(&bank.join("papers"), &mut paper_paths)?;
    let mut lines = String::new();
    let mode = match value(args, "--mode") {
        Some(value) if !value.is_empty() => value,
        _ => "dev-smoke".to_string(),
    };
    let production = matches!(
        mode.as_str(),
        "production-hard-recall" | "production-deep-stem-hard-recall"
    );
    if production && paper_paths.is_empty() {
        return Err(format!(
            "production make-work requires at least one paper JSON under {}",
            bank.join("papers").display()
        ));
    }
    for path in paper_paths {
        let paper: PaperRecord = read_json(&path)?;
        let kinds: &[&str] = if production {
            &[
                "generator",
                "focused_auditor",
                "saturated_answerer",
                "judge",
            ]
        } else {
            &["generator"]
        };
        for kind in kinds {
            let prompt = match *kind {
                "generator" => format!(
                    "Generate hard but answerable questions for '{}' using only checked redistributable paper sections. Return production QBank candidates without answer leakage.",
                    paper.title
                ),
                "focused_auditor" => format!(
                    "Audit candidate support for '{}' with focused context only. Include Jnoccio route metadata, model decisions, confidence, prompt hash, and context hash.",
                    paper.title
                ),
                "saturated_answerer" => format!(
                    "Blind-answer hard recall candidates for '{}' with saturated context and no answer key. Include route metadata, model decisions, token usage, confidence, and hashes.",
                    paper.title
                ),
                "judge" => format!(
                    "Reduce QBank tournament evidence for '{}' and accept only source-supported, hard, non-leaking production challenges.",
                    paper.title
                ),
                _ => unreachable!(),
            };
            let item = WorkItem {
                kind: (*kind).to_string(),
                publication_hash: paper.publication_hash.clone(),
                challenge_hash: None,
                prompt,
            };
            lines.push_str(&serde_json::to_string(&item).map_err(|err| err.to_string())?);
            lines.push('\n');
        }
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    std::fs::write(&out, lines).map_err(|err| format!("write {}: {err}", out.display()))
}

pub fn pack_context_command(args: &[String]) -> Result<(), String> {
    let input = path_value(args, "--paper").ok_or("--paper is required")?;
    let paper: PaperRecord = read_json(&input)?;
    let sections = match value(args, "--sections") {
        Some(values) if !values.is_empty() => values
            .split(',')
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let safe_window = match value(args, "--safe-window-tokens") {
        Some(value) => match value.parse::<u64>() {
            Ok(parsed) => parsed,
            Err(_) => 128_000,
        },
        None => 128_000,
    };
    let pack = qbank_builder::pack_context(&paper, &sections, safe_window, 0.82, 4096)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&pack).map_err(|err| err.to_string())?
    );
    Ok(())
}
