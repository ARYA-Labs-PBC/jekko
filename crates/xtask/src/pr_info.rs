use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::process::Command as ProcessCommand;

pub fn run(number: u64, field: String) -> Result<()> {
    let value = pull_request_field(number, &field)?;
    println!("{value}");
    Ok(())
}

pub fn pull_request_field(number: u64, field: &str) -> Result<String> {
    let repo =
        std::env::var("GITHUB_REPOSITORY").context("reading GITHUB_REPOSITORY for pull request")?;
    let output = ProcessCommand::new("gh")
        .args(["api", &format!("/repos/{repo}/pulls/{number}")])
        .output()
        .context("running gh api pull request")?;
    if !output.status.success() {
        bail!("gh api pull request failed with status {}", output.status);
    }

    let value: Value =
        serde_json::from_slice(&output.stdout).context("parse pull request JSON response")?;
    let Some(found) = crate::json_lookup(&value, field) else {
        bail!("missing pull request field: {field}");
    };
    Ok(match found {
        Value::String(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn reads_nested_field_from_pull_request_json() {
        let value = json!({
            "title": "hello",
            "body": "world",
            "head": { "sha": "abc123" }
        });
        let found = crate::json_lookup(&value, "head.sha").unwrap();
        assert_eq!(found.as_str(), Some("abc123"));
    }
}
