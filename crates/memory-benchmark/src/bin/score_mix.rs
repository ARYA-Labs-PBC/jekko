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
    dev_only: bool,
    qbank_trusted: Option<bool>,
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

    let payload = build_payload(name, &inputs, total, fixtures_run, fixtures_passed).to_string();
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

fn build_payload(
    name: String,
    inputs: &[InputScore],
    total: f64,
    fixtures_run: i64,
    fixtures_passed: i64,
) -> Json {
    let mut parts = Vec::new();
    let mut dev_only_inputs = Vec::new();
    let mut qbank_seen = false;
    let mut qbank_trusted = true;
    for input in inputs {
        let mut entry = match json::obj(&[
            ("name", Json::Str(input.name.clone())),
            ("weight", Json::Float(input.weight)),
            ("path", Json::Str(input.path.clone())),
            ("total", Json::Float(input.total)),
            ("fixtures_run", Json::Int(input.fixtures_run)),
            ("fixtures_passed", Json::Int(input.fixtures_passed)),
            ("dev_only", Json::Bool(input.dev_only)),
        ]) {
            Json::Object(entry) => entry,
            _ => unreachable!("json::obj returns an object"),
        };
        if input.dev_only {
            dev_only_inputs.push(Json::Str(input.name.clone()));
        }
        if let Some(trusted) = input.qbank_trusted {
            entry.insert("qbank_trusted".to_string(), Json::Bool(trusted));
        }
        if input.name == "qbank" || input.qbank_trusted.is_some() {
            qbank_seen = true;
            qbank_trusted &= input.qbank_trusted.unwrap_or(!input.dev_only);
        }
        parts.push(Json::Object(entry));
    }
    let mut top = BTreeMap::new();
    top.insert("name".to_string(), Json::Str(name));
    top.insert("suite".to_string(), Json::Str("mixed".to_string()));
    top.insert("total".to_string(), Json::Float(total));
    top.insert("fixtures_run".to_string(), Json::Int(fixtures_run));
    top.insert("fixtures_passed".to_string(), Json::Int(fixtures_passed));
    top.insert(
        "dev_only".to_string(),
        Json::Bool(inputs.iter().any(|input| input.dev_only)),
    );
    top.insert("dev_only_inputs".to_string(), Json::Array(dev_only_inputs));
    if qbank_seen {
        top.insert("qbank_trusted".to_string(), Json::Bool(qbank_trusted));
    }
    top.insert("inputs".to_string(), Json::Array(parts));
    Json::Object(top)
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
        dev_only: bool_value(&obj, "dev_only").unwrap_or(false),
        qbank_trusted: bool_value(&obj, "qbank_trusted"),
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

fn bool_value(obj: &BTreeMap<String, Json>, key: &str) -> Option<bool> {
    match obj.get(key) {
        Some(Json::Bool(value)) => Some(*value),
        _ => None,
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
        assert!(!input.dev_only);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn mixed_report_propagates_dev_only_qbank_status() {
        let inputs = vec![
            InputScore {
                name: "generated".to_string(),
                weight: 0.60,
                path: "generated.json".to_string(),
                total: 90.0,
                fixtures_run: 10,
                fixtures_passed: 9,
                dev_only: false,
                qbank_trusted: None,
            },
            InputScore {
                name: "qbank".to_string(),
                weight: 0.40,
                path: "qbank.json".to_string(),
                total: 80.0,
                fixtures_run: 50,
                fixtures_passed: 40,
                dev_only: true,
                qbank_trusted: Some(false),
            },
        ];

        let payload = build_payload("northstar".to_string(), &inputs, 86.0, 60, 49);
        let Json::Object(obj) = payload else {
            panic!("score_mix payload must be an object");
        };
        assert_eq!(obj.get("dev_only"), Some(&Json::Bool(true)));
        assert_eq!(obj.get("qbank_trusted"), Some(&Json::Bool(false)));
        assert_eq!(
            obj.get("dev_only_inputs"),
            Some(&Json::Array(vec![Json::Str("qbank".to_string())]))
        );
    }
}
