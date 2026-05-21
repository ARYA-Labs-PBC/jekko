use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde_json::Value;

pub fn run(score_path: &Path) -> Result<()> {
    let text =
        fs::read_to_string(score_path).with_context(|| format!("read {}", score_path.display()))?;
    let json: Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", score_path.display()))?;

    let score = json.get("score").and_then(Value::as_i64).unwrap_or(0);
    let minimum = json
        .get("minimum_score")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let hard_findings = hard_findings(&json);

    println!("jankurai-gate: score={score} minimum={minimum} hard_findings={hard_findings}");

    if hard_findings > 0 {
        bail!("jankurai gate failed: {hard_findings} hard finding(s)");
    }
    if minimum > 0 && score < minimum {
        bail!("jankurai gate failed: score {score} below minimum {minimum}");
    }
    Ok(())
}

fn hard_findings(json: &Value) -> i64 {
    json.get("hard_findings")
        .and_then(Value::as_i64)
        .or_else(|| {
            json.get("decision")
                .and_then(|decision| decision.get("hard_findings"))
                .and_then(Value::as_i64)
        })
        .unwrap_or(0)
}
