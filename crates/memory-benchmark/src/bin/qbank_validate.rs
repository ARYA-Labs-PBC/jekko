//! Validate checked-in real-paper QBank artifacts.

use memory_benchmark::corpus::real_papers::{default_bank_path, validate_bank};
use memory_benchmark::json::{self, Json};
use std::env;
use std::path::PathBuf;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let bank = value(&args, "--bank")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_bank_path().to_path_buf());
    let allow_empty = args.iter().any(|arg| arg == "--allow-empty");
    let top_n = value(&args, "--top-n")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100);
    let min_required_accepted = value(&args, "--min-accepted")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(500);
    let validation = match validate_bank(&bank, allow_empty, top_n, min_required_accepted) {
        Ok(validation) => validation,
        Err(err) => {
            eprintln!("qbank_validate: {err}");
            process::exit(2);
        }
    };
    let failed = !validation.errors.is_empty();
    let mut top = Json::obj();
    top.insert("bank".to_string(), Json::Str(bank.display().to_string()));
    top.insert(
        "accepted_challenges".to_string(),
        Json::Int(validation.accepted_challenges as i64),
    );
    top.insert(
        "rejected_challenges".to_string(),
        Json::Int(validation.rejected_challenges as i64),
    );
    top.insert(
        "duplicate_publications".to_string(),
        Json::Int(validation.duplicate_publications as i64),
    );
    top.insert(
        "top_selected".to_string(),
        Json::Int(validation.top_selected as i64),
    );
    top.insert(
        "manifest_hash".to_string(),
        Json::Str(validation.manifest_hash),
    );
    top.insert(
        "manifest_schema".to_string(),
        Json::Str(validation.manifest_schema),
    );
    top.insert(
        "strict_production".to_string(),
        Json::Bool(validation.strict_production),
    );
    top.insert(
        "qbank_trusted".to_string(),
        Json::Bool(validation.qbank_trusted),
    );
    top.insert(
        "min_required_accepted".to_string(),
        Json::Int(validation.min_required_accepted as i64),
    );
    top.insert(
        "unique_publications".to_string(),
        Json::Int(validation.unique_publications as i64),
    );
    top.insert(
        "distinct_domains".to_string(),
        Json::Int(validation.distinct_domains as i64),
    );
    top.insert(
        "max_publication_share".to_string(),
        Json::Float(validation.max_publication_share as f64),
    );
    top.insert(
        "max_domain_share".to_string(),
        Json::Float(validation.max_domain_share as f64),
    );
    top.insert(
        "source_diversity".to_string(),
        Json::Float(validation.source_diversity as f64),
    );
    top.insert(
        "dev_only".to_string(),
        Json::Bool(env::var("memory_benchmark_dev_qbank").ok().as_deref() == Some("1")),
    );
    top.insert(
        "errors".to_string(),
        json::arr_str(validation.errors.iter().cloned()),
    );
    top.insert(
        "warnings".to_string(),
        json::arr_str(validation.warnings.iter().cloned()),
    );
    println!("{}", Json::Object(top));
    if failed {
        process::exit(1);
    }
}

fn value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}
