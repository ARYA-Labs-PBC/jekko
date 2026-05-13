//! Verify that benchmark JSON is byte-identical across repeated runs.

use std::env;

use memory_benchmark::runner::{
    run_candidate, run_candidate_with_config, DEFAULT_REFERENCE_CANDIDATES,
};
use memory_benchmark::{Split, SuiteConfig};

fn main() {
    let mut single: Option<String> = None;
    let mut config = SuiteConfig::default();
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--candidate" => {
                single = args.get(i + 1).cloned();
                i += 2;
            }
            "--suite" => {
                if let Some(value) = args.get(i + 1) {
                    config.split = match value.as_str() {
                        "public-generated" => Split::PublicGenerated,
                        "generated" => Split::PublicGenerated,
                        "private-generated" => Split::PrivateGenerated,
                        "stress" => Split::Stress,
                        "public" => Split::PublicSmoke,
                        "real-papers" => Split::RealPapers,
                        "compounding" => Split::PublicCompounding,
                        "hardening" => Split::PublicHardening,
                        _ => config.split,
                    };
                }
                i += 2;
            }
            "--paper-bank" => {
                config.paper_bank_path = args.get(i + 1).cloned();
                i += 2;
            }
            "--qbank-top-n" => {
                if let Some(value) = args.get(i + 1).and_then(|v| v.parse::<usize>().ok()) {
                    config.qbank_top_n = value;
                }
                i += 2;
            }
            "--qbank-selection" => {
                config.qbank_selection_path = args.get(i + 1).cloned();
                i += 2;
            }
            "--context-budget" => {
                if let Some(value) = args.get(i + 1).and_then(|v| v.parse::<u32>().ok()) {
                    config.context_budget = value;
                }
                i += 2;
            }
            "--seed" => {
                if let Some(value) = args.get(i + 1) {
                    config.seed_label = value.clone();
                }
                i += 2;
            }
            "--fixtures" => {
                if let Some(value) = args.get(i + 1).and_then(|v| v.parse::<usize>().ok()) {
                    config.fixture_count = value;
                }
                i += 2;
            }
            _ => i += 1,
        }
    }

    let candidates: Vec<&str> = match single.as_deref() {
        Some(candidate) => vec![candidate],
        None => DEFAULT_REFERENCE_CANDIDATES.to_vec(),
    };

    let mut ok = true;
    for candidate in candidates {
        let first = if config.split == Split::PublicSmoke {
            run_candidate(candidate)
        } else {
            run_candidate_with_config(candidate, &config)
        }
        .unwrap_or_else(|error| {
            eprintln!("verify_determinism: {}: {}", candidate, error);
            std::process::exit(2);
        });
        let second = if config.split == Split::PublicSmoke {
            run_candidate(candidate)
        } else {
            run_candidate_with_config(candidate, &config)
        }
        .unwrap_or_else(|error| {
            eprintln!("verify_determinism: {}: {}", candidate, error);
            std::process::exit(2);
        });

        if first.json == second.json {
            eprintln!(
                "verify_determinism: OK candidate={} bytes={}",
                candidate,
                first.json.len()
            );
        } else {
            eprintln!(
                "verify_determinism: MISMATCH candidate={} ({} vs {} bytes)",
                candidate,
                first.json.len(),
                second.json.len()
            );
            ok = false;
        }
    }

    std::process::exit(if ok { 0 } else { 1 });
}
