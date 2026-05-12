//! `dump_tasks` — emit JSONL task records for the .zyal fan_out.
//!
//! Subcommands:
//!     dump_tasks prompt [--population N] [--out PATH]
//!         Emit N × axes tasks to instruct judge workers what to score.
//!
//!     dump_tasks exec [--population N] [--out PATH]
//!         Emit N × fixtures tasks for the executable benchmark.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;

use memory_benchmark::fixture;
use memory_benchmark::json::{self, Json};
use memory_benchmark::AxisScores;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode: String = match args.get(1) {
        Some(arg) => arg.clone(),
        None => "prompt".to_string(),
    };
    let mut population: usize = 20;
    let mut out: Option<String> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--population" => {
                population = args
                    .get(i + 1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(population);
                i += 2;
            }
            "--out" => {
                out = args.get(i + 1).cloned();
                i += 2;
            }
            _ => i += 1,
        }
    }

    let mut buf = String::new();
    match mode.as_str() {
        "prompt" => emit_prompt_tasks(&mut buf, population),
        "exec" => emit_exec_tasks(&mut buf, population),
        other => {
            eprintln!(
                "dump_tasks: unknown mode {:?} (expected 'prompt' | 'exec')",
                other
            );
            std::process::exit(2);
        }
    }

    match out {
        Some(p) => {
            if let Some(parent) = std::path::Path::new(&p).parent() {
                let _ = fs::create_dir_all(parent);
            }
            let mut f = fs::File::create(&p).expect("create out");
            f.write_all(buf.as_bytes()).expect("write");
            eprintln!("dump_tasks: wrote {} bytes to {}", buf.len(), p);
        }
        None => {
            print!("{}", buf);
        }
    }
}

fn axis_names() -> &'static [&'static str] {
    &[
        "correctness",
        "provenance",
        "bitemporal_recall",
        "contradiction",
        "math_science",
        "english_discourse_coreference",
        "privacy_redaction",
        "procedural_skill",
        "feedback_adaptation",
        "determinism_rebuild",
    ]
}

fn emit_prompt_tasks(out: &mut String, population: usize) {
    let _ = AxisScores::WEIGHTS; // keep linker happy
    let mut id: u64 = 0;
    for axis in axis_names() {
        for worker in 0..population {
            id += 1;
            let mut o = BTreeMap::new();
            o.insert("id".to_string(), Json::Int(id as i64));
            o.insert("axis".to_string(), Json::Str(axis.to_string()));
            o.insert("worker".to_string(), Json::Int(worker as i64));
            o.insert(
                "instruction".to_string(),
                Json::Str(format!(
                    "Score the memory benchmark contract on the {axis} axis (0.0–1.0). \
                    Emit one MEMORY_BENCH_SCORE line: \
                    MEMORY_BENCH_SCORE|spec=<reference_context_pack|reference_evidence_ledger|reference_claim_skeptic>|axis={axis}|raw=<float>|cap=<float>|evidence=<file:line>|deduction=<text>"
                )),
            );
            o.insert(
                "seed_files".to_string(),
                json::arr_str([
                    "crates/memory-benchmark/README.md",
                    "crates/memory-benchmark/src/fixture/data.rs",
                    "docs/ADVANCED_MEMORY_CHALLENGE.md",
                ]),
            );
            out.push_str(&Json::Object(o).to_string());
            out.push('\n');
        }
    }
}

fn emit_exec_tasks(out: &mut String, _population: usize) {
    let fixtures = fixture::all();
    for f in fixtures {
        let mut o = BTreeMap::new();
        o.insert("id".to_string(), Json::Int(f.id as i64));
        o.insert("block".to_string(), Json::Str(f.block.name().to_string()));
        o.insert("domain".to_string(), Json::Str(f.domain.name().to_string()));
        o.insert(
            "pathologies".to_string(),
            json::arr_str(f.pathologies.iter().map(|p| p.name().to_string())),
        );
        o.insert(
            "public_bench".to_string(),
            json::arr_str(f.public_bench.iter().map(|p| p.name().to_string())),
        );
        out.push_str(&Json::Object(o).to_string());
        out.push('\n');
    }
}
