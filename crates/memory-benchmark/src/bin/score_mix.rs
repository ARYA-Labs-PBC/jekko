//! Deterministically mix generated-suite and QBank-suite reports.

use memory_benchmark::json::{self, Json};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

#[derive(Debug)]
struct InputScore {
    name: String,
    weight: f64,
    path: String,
    total: f64,
    fixtures_run: i64,
    fixtures_passed: i64,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("score_mix: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let name = value(&args, "--name").unwrap_or_else(|| "mixed".to_string());
    let out = value(&args, "--out");
    let mut inputs = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--input" {
            let spec = args.get(i + 1).ok_or("--input requires name:weight:path")?;
            inputs.push(read_input(spec)?);
            i += 2;
        } else {
            i += 1;
        }
    }
    if inputs.is_empty() {
        return Err("at least one --input name:weight:path is required".to_string());
    }
    let weight_sum: f64 = inputs.iter().map(|input| input.weight).sum();
    if weight_sum <= 0.0 {
        return Err("input weights must sum above zero".to_string());
    }
    inputs.sort_by(|a, b| a.name.cmp(&b.name));
    let total = inputs
        .iter()
        .map(|input| input.total * input.weight)
        .sum::<f64>()
        / weight_sum;
    let fixtures_run = inputs.iter().map(|input| input.fixtures_run).sum::<i64>();
    let fixtures_passed = inputs
        .iter()
        .map(|input| input.fixtures_passed)
        .sum::<i64>();

    let mut parts = Vec::new();
    for input in &inputs {
        parts.push(json::obj(&[
            ("name", Json::Str(input.name.clone())),
            ("weight", Json::Float(input.weight)),
            ("path", Json::Str(input.path.clone())),
            ("total", Json::Float(input.total)),
            ("fixtures_run", Json::Int(input.fixtures_run)),
            ("fixtures_passed", Json::Int(input.fixtures_passed)),
        ]));
    }
    let mut top = BTreeMap::new();
    top.insert("name".to_string(), Json::Str(name));
    top.insert("suite".to_string(), Json::Str("mixed".to_string()));
    top.insert("total".to_string(), Json::Float(total));
    top.insert("fixtures_run".to_string(), Json::Int(fixtures_run));
    top.insert("fixtures_passed".to_string(), Json::Int(fixtures_passed));
    top.insert("inputs".to_string(), Json::Array(parts));
    let payload = Json::Object(top).to_string();
    if let Some(path) = out {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create {}: {err}", parent.display()))?;
        }
        fs::write(&path, format!("{payload}\n")).map_err(|err| format!("write {path}: {err}"))?;
    } else {
        println!("{payload}");
    }
    Ok(())
}

fn read_input(spec: &str) -> Result<InputScore, String> {
    let mut parts = spec.splitn(3, ':');
    let name = parts.next().unwrap_or_default().to_string();
    let weight = parts
        .next()
        .ok_or("missing weight")?
        .parse::<f64>()
        .map_err(|err| format!("bad weight: {err}"))?;
    let path = parts.next().ok_or("missing path")?.to_string();
    let text = fs::read_to_string(&path).map_err(|err| format!("read {path}: {err}"))?;
    let parsed = json::parse(&text).map_err(|err| format!("parse {path}: {err}"))?;
    let obj = match parsed {
        Json::Object(obj) => obj,
        _ => return Err(format!("{path}: report must be a JSON object")),
    };
    Ok(InputScore {
        name,
        weight,
        path,
        total: number(&obj, "total")?,
        fixtures_run: integer(&obj, "fixtures_run")?,
        fixtures_passed: integer(&obj, "fixtures_passed")?,
    })
}

fn value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}

fn number(obj: &BTreeMap<String, Json>, key: &str) -> Result<f64, String> {
    match obj.get(key) {
        Some(Json::Float(value)) => Ok(*value),
        Some(Json::Int(value)) => Ok(*value as f64),
        _ => Err(format!("missing numeric {key}")),
    }
}

fn integer(obj: &BTreeMap<String, Json>, key: &str) -> Result<i64, String> {
    match obj.get(key) {
        Some(Json::Int(value)) => Ok(*value),
        Some(Json::Float(value)) => Ok(*value as i64),
        _ => Err(format!("missing integer {key}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_input_spec() {
        let dir = std::env::temp_dir().join(format!("score-mix-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let report = dir.join("generated.json");
        fs::write(
            &report,
            r#"{"name":"generated","total":50.0,"fixtures_run":10,"fixtures_passed":5}"#,
        )
        .expect("write report");
        let input = read_input(&format!("generated:0.60:{}", report.display())).expect("input");
        assert_eq!(input.name, "generated");
        assert_eq!(input.weight, 0.60);
        assert_eq!(input.total, 50.0);
        let _ = fs::remove_dir_all(dir);
    }
}
