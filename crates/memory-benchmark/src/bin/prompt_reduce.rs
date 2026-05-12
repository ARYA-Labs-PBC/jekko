//! `prompt_reduce` — aggregate MEMORY_BENCH_SCORE lines from the prompt-scoring fan_out.
//!
//! Reads a JSONL/text file of worker outputs (one MEMORY_BENCH_SCORE line per worker)
//! and emits a deterministic JSON + Markdown summary:
//!   * per-spec, per-axis median (drop high/low when ≥ 5 votes)
//!   * cap_without_evidence: votes lacking file:line evidence are clipped to cap
//!   * cross-spec ranking
//!
//! Input line format:
//!     MEMORY_BENCH_SCORE|spec=<NAME>|axis=<id>|raw=<float>|cap=<float>|evidence=<file:line>|deduction=<text>

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::Write;

use memory_benchmark::json::{self, Json};

#[derive(Clone)]
struct Vote {
    raw: f64,
    cap: f64,
    has_evidence: bool,
}

fn main() {
    let mut votes_path: Option<String> = None;
    let mut out_path: Option<String> = None;
    let mut markdown_path: Option<String> = None;
    let mut ranking_path: Option<String> = None;
    let mut disagreement_path: Option<String> = None;
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--votes" => {
                votes_path = args.get(i + 1).cloned();
                i += 2;
            }
            "--out" => {
                out_path = args.get(i + 1).cloned();
                i += 2;
            }
            "--markdown" => {
                markdown_path = args.get(i + 1).cloned();
                i += 2;
            }
            "--ranking" => {
                ranking_path = args.get(i + 1).cloned();
                i += 2;
            }
            "--disagreement" => {
                disagreement_path = args.get(i + 1).cloned();
                i += 2;
            }
            _ => i += 1,
        }
    }

    let votes_text: String = match &votes_path {
        Some(p) => match fs::read_to_string(p) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("prompt_reduce: cannot read votes file {:?}: {}", p, err);
                std::process::exit(2);
            }
        },
        None => String::new(),
    };

    let mut buckets: BTreeMap<String, BTreeMap<String, Vec<Vote>>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut record_count = 0usize;
    for (line_no, line) in votes_text.lines().enumerate() {
        let line = line.trim();
        if !line.starts_with("MEMORY_BENCH_SCORE|") {
            continue;
        }
        if has_unredacted_canary(line) {
            fatal(&format!(
                "unredacted canary in vote record at line {}",
                line_no + 1
            ));
        }
        if !seen.insert(line.to_string()) {
            fatal(&format!("duplicate vote record at line {}", line_no + 1));
        }
        let (spec, axis, vote) = parse_vote(line)
            .unwrap_or_else(|error| fatal(&format!("line {}: {}", line_no + 1, error)));
        buckets
            .entry(spec)
            .or_default()
            .entry(axis)
            .or_default()
            .push(vote);
        record_count += 1;
    }

    // Reduce.
    let mut per_spec = BTreeMap::new();
    let mut ranking: Vec<(String, f64)> = Vec::new();
    let mut disagreement = BTreeMap::new();
    for (spec, axes) in &buckets {
        let mut spec_obj = BTreeMap::new();
        let mut spec_disagreement = BTreeMap::new();
        let mut spec_total = 0.0_f64;
        let mut spec_axes_used = 0u32;
        for (axis, votes) in axes {
            let mut values: Vec<f64> = votes
                .iter()
                .map(|vote| {
                    if vote.has_evidence {
                        vote.raw
                    } else {
                        vote.raw.min(vote.cap)
                    }
                })
                .collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let original_values = values.clone();
            if values.len() >= 5 {
                values.remove(values.len() - 1); // drop high
                values.remove(0); // drop low
            }
            if values.is_empty() {
                continue;
            }
            let median = values[values.len() / 2];
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let mut a_obj = BTreeMap::new();
            a_obj.insert("median".to_string(), Json::Float(median));
            a_obj.insert("mean".to_string(), Json::Float(mean));
            a_obj.insert(
                "votes_after_drop".to_string(),
                Json::Int(values.len() as i64),
            );
            spec_obj.insert(axis.clone(), Json::Object(a_obj));
            let min = original_values.first().copied().unwrap_or(0.0);
            let max = original_values.last().copied().unwrap_or(0.0);
            spec_disagreement.insert(
                axis.clone(),
                json::obj(&[
                    ("votes", Json::Int(original_values.len() as i64)),
                    ("min", Json::Float(min)),
                    ("max", Json::Float(max)),
                    ("spread", Json::Float(max - min)),
                ]),
            );
            spec_total += median;
            spec_axes_used += 1;
        }
        if spec_axes_used > 0 {
            ranking.push((spec.clone(), spec_total / spec_axes_used as f64));
        }
        per_spec.insert(spec.clone(), Json::Object(spec_obj));
        disagreement.insert(spec.clone(), Json::Object(spec_disagreement));
    }

    ranking.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut top = BTreeMap::new();
    top.insert("kind".to_string(), Json::Str("prompt-score".to_string()));
    top.insert("per_spec".to_string(), Json::Object(per_spec));
    top.insert(
        "ranking".to_string(),
        Json::Array(
            ranking
                .iter()
                .map(|(s, v)| {
                    json::obj(&[("spec", Json::Str(s.clone())), ("score", Json::Float(*v))])
                })
                .collect(),
        ),
    );
    let json_str = Json::Object(top).to_string();

    write_file(&out_path, &json_str);

    if let Some(p) = &disagreement_path {
        let json_disagreement = json::obj(&[
            ("kind", Json::Str("population-disagreement".to_string())),
            ("records", Json::Int(record_count as i64)),
            ("per_spec", Json::Object(disagreement)),
        ])
        .to_string();
        write_file(&Some(p.clone()), &json_disagreement);
    }

    if let Some(p) = &ranking_path {
        let json_rank = Json::Array(
            ranking
                .iter()
                .map(|(s, v)| {
                    json::obj(&[("spec", Json::Str(s.clone())), ("score", Json::Float(*v))])
                })
                .collect(),
        )
        .to_string();
        write_file(&Some(p.clone()), &json_rank);
    }

    if let Some(p) = &markdown_path {
        let mut md = String::from("# Memory Benchmark Prompt-Scoring Report\n\n## Ranking\n\n");
        md.push_str("| Spec | Median axis score |\n|---|---:|\n");
        for (s, v) in &ranking {
            md.push_str(&format!("| {} | {:.2} |\n", s, v));
        }
        md.push_str("\n## Notes\n\n");
        md.push_str(
            "- Drop high/low applied when ≥ 5 votes per axis.\n\
             - Votes lacking `evidence=<file:line>` are capped at `cap`.\n\
             - Aggregation is purely deterministic over the input file.\n",
        );
        write_file(&Some(p.clone()), &md);
    }

    eprintln!(
        "prompt_reduce: {} specs, {} ranking rows",
        buckets.len(),
        ranking.len()
    );
}

fn parse_vote(line: &str) -> Result<(String, String, Vote), String> {
    let mut fields = BTreeMap::new();
    for kv in line.split('|').skip(1) {
        let mut it = kv.splitn(2, '=');
        let key = it.next().unwrap_or("");
        let value = it
            .next()
            .ok_or_else(|| format!("missing '=' in field {:?}", kv))?;
        if key.is_empty() {
            return Err("empty field key".to_string());
        }
        fields.insert(key, value);
    }

    let spec = required_field(&fields, "spec")?.to_string();
    let axis = required_field(&fields, "axis")?.to_string();
    let raw = parse_score(required_field(&fields, "raw")?, "raw")?;
    let cap = parse_score(required_field(&fields, "cap")?, "cap")?;
    let evidence = required_field(&fields, "evidence")?;
    let has_evidence = evidence != "none" && evidence.contains(':');
    let _deduction = required_field(&fields, "deduction")?;

    Ok((
        spec,
        axis,
        Vote {
            raw,
            cap,
            has_evidence,
        },
    ))
}

fn required_field<'a>(fields: &'a BTreeMap<&str, &str>, key: &str) -> Result<&'a str, String> {
    fields
        .get(key)
        .copied()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing required field `{}`", key))
}

fn parse_score(value: &str, field: &str) -> Result<f64, String> {
    let score = value
        .parse::<f64>()
        .map_err(|error| format!("invalid {} score {:?}: {}", field, value, error))?;
    if !score.is_finite() || !(0.0..=100.0).contains(&score) {
        return Err(format!("{} score {:?} outside 0..100", field, value));
    }
    Ok(score)
}

fn has_unredacted_canary(line: &str) -> bool {
    let patterns: [String; 4] = [
        format!("{}{}{}", "sk-memory-", "bench-", "CANARY"),
        format!("{}{}{}", "AKIA", "MEMORY", "BENCHCANARY"),
        format!("{}{}{}", "MEMORY", "_BENCH", "_CANARY"),
        format!("{}-{}-{}", "memory", "benchmark", "test-corp"),
    ];
    patterns.iter().any(|p| line.contains(p))
}

fn fatal(message: &str) -> ! {
    eprintln!("prompt_reduce: {}", message);
    std::process::exit(2);
}

fn write_file(path: &Option<String>, content: &str) {
    if let Some(p) = path {
        if let Some(parent) = std::path::Path::new(p).parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut f = match fs::File::create(p) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("prompt_reduce: write {}: {}", p, e);
                return;
            }
        };
        let _ = f.write_all(content.as_bytes());
    }
}
