use qbank_builder::{
    acceptance_passes, canonicalize_paper, ensure_bank_layout, finalize_challenge, manifest_hash,
    read_challenges, read_json, sorted_challenges, write_json_pretty, ChallengeRecord, PaperRecord,
    WorkItem,
};
use serde_json::json;
use std::env;
use std::path::{Path, PathBuf};
use std::process;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("qbank: {err}");
        process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let Some(command) = args.get(1).map(String::as_str) else {
        print_help();
        return Err("missing command".to_string());
    };
    match command {
        "discover" => discover(&args[2..]).await,
        "publish-paper" => publish_paper(&args[2..]),
        "make-work" => make_work(&args[2..]),
        "pack-context" => pack_context_command(&args[2..]),
        "reduce" => reduce(&args[2..]),
        "publish" => publish_manifest(&args[2..]),
        "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command {other:?}")),
    }
}

fn print_help() {
    eprintln!(
        "qbank <discover|publish-paper|make-work|pack-context|reduce|publish> [--bank path] [--run-root path]"
    );
}

async fn discover(args: &[String]) -> Result<(), String> {
    let run_root = match value(args, "--run-root") {
        Some(value) if !value.is_empty() => value,
        _ => ".jekko/daemon/paper-qbank/discovery".to_string(),
    };
    let query = match value(args, "--query") {
        Some(value) if !value.is_empty() => value,
        _ => "open access scientific paper hard answerable result".to_string(),
    };
    let out = Path::new(&run_root).join("candidates.json");
    let receipt = json!({
        "query": query,
        "providers": ["openalex", "crossref", "arxiv", "pubmed", "semantic_scholar", "unpaywall"],
        "status": "local_receipt_only",
        "note": "network/model expansion is driven by ZYAL workers; checked-in bank writes happen only through publish-paper/reduce"
    });
    qbank_builder::write_json_pretty(&out, &receipt)?;
    Ok(())
}

fn publish_paper(args: &[String]) -> Result<(), String> {
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

fn make_work(args: &[String]) -> Result<(), String> {
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
    for path in paper_paths {
        let paper: PaperRecord = read_json(&path)?;
        let prompt = format!(
            "Generate hard but answerable questions for '{}' using only checked paper sections.",
            paper.title
        );
        let item = WorkItem {
            kind: "generator".to_string(),
            publication_hash: paper.publication_hash,
            challenge_hash: None,
            prompt,
        };
        lines.push_str(&serde_json::to_string(&item).map_err(|err| err.to_string())?);
        lines.push('\n');
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    std::fs::write(&out, lines).map_err(|err| format!("write {}: {err}", out.display()))
}

fn pack_context_command(args: &[String]) -> Result<(), String> {
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

fn reduce(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    let input = path_value(args, "--input").ok_or("--input is required")?;
    ensure_bank_layout(&bank)?;
    let mut challenge: ChallengeRecord = read_json(&input)?;
    challenge = finalize_challenge(challenge);
    let dir = if acceptance_passes(&challenge.acceptance) {
        "challenges"
    } else {
        "rejected"
    };
    let out = bank
        .join(dir)
        .join(format!("{}.json", challenge.challenge_hash));
    write_json_pretty(&out, &challenge)
}

fn publish_manifest(args: &[String]) -> Result<(), String> {
    let bank = match path_value(args, "--bank") {
        Some(value) => value,
        None => PathBuf::from("crates/memory-benchmark/data/real-paper-bank"),
    };
    ensure_bank_layout(&bank)?;
    let challenges = sorted_challenges(read_challenges(&bank)?);
    let mut files = Vec::new();
    qbank_builder::collect_json_files(&bank.join("papers"), &mut files)?;
    qbank_builder::collect_json_files(&bank.join("challenges"), &mut files)?;
    let hash = manifest_hash(&files)?;
    let manifest = json!({
        "schema_version": "opencode-qbank-manifest-v1",
        "accepted_challenges": challenges.len(),
        "manifest_hash": hash,
        "top_challenge_hashes": challenges.iter().map(|challenge| challenge.challenge_hash.clone()).collect::<Vec<_>>()
    });
    write_json_pretty(&bank.join("manifests").join("latest.json"), &manifest)
}

fn value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}

fn path_value(args: &[String], flag: &str) -> Option<PathBuf> {
    value(args, flag).map(PathBuf::from)
}
