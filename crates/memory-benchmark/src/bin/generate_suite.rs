use std::env;
use std::fs;

use memory_benchmark::generated::{generate_suite, GeneratedSuiteConfig};
use memory_benchmark::json::{self, Json};
use memory_benchmark::Split;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut split = Split::PublicGenerated;
    let mut seed = "public-dev-0001".to_string();
    let mut fixtures = 500usize;
    let mut difficulty = 2u8;
    let mut out: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--split" => {
                if let Some(value) = args.get(i + 1) {
                    split = match value.as_str() {
                        "public-dev" => Split::PublicGenerated,
                        "private" => Split::PrivateGenerated,
                        "stress" => Split::Stress,
                        "public" => Split::PublicSmoke,
                        _ => split,
                    };
                }
                i += 2;
            }
            "--seed" => {
                if let Some(value) = args.get(i + 1) {
                    seed = value.clone();
                }
                i += 2;
            }
            "--fixtures" => {
                if let Some(value) = args.get(i + 1).and_then(|v| v.parse::<usize>().ok()) {
                    fixtures = value;
                }
                i += 2;
            }
            "--difficulty" => {
                if let Some(value) = args.get(i + 1).and_then(|v| v.parse::<u8>().ok()) {
                    difficulty = value.clamp(1, 5);
                }
                i += 2;
            }
            "--out" => {
                out = args.get(i + 1).cloned();
                i += 2;
            }
            _ => i += 1,
        }
    }

    let config = GeneratedSuiteConfig {
        benchmark_version: "memory-benchmark-v2",
        split,
        seed_label: seed.clone(),
        fixture_count: fixtures,
        difficulty,
    };
    let cases = generate_suite(&config);
    let body = json::obj(&[
        (
            "benchmark_version",
            Json::Str(config.benchmark_version.to_string()),
        ),
        ("split", Json::Str(split.name().to_string())),
        ("seed_label", Json::Str(seed)),
        ("fixture_count", Json::Int(cases.len() as i64)),
        (
            "cases",
            Json::Array(
                cases
                    .iter()
                    .map(|case| {
                        json::obj(&[
                            ("id", Json::Str(case.id.clone())),
                            ("domain", Json::Str(case.domain.name().to_string())),
                            ("block", Json::Str(case.block.name().to_string())),
                            ("oracle", Json::Str(format!("{:?}", case.oracle.kind))),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
    .to_string();
    if let Some(path) = out {
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&path, body).expect("write generated suite");
        eprintln!("generate_suite: fixtures={} -> {}", fixtures, path);
    } else {
        println!("{}", body);
    }
}
