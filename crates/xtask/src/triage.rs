use anyhow::{bail, Context, Result};
use std::process::Command as ProcessCommand;

use crate::current_github_event_context;

const MISSING_FIELD_DEFAULT: &str = "";

pub fn run() -> Result<()> {
    let context = current_github_event_context()?;
    let number = context
        .field("target.number")
        .context("missing target.number")?;
    let title = match context.field("target.title") {
        Some(value) => value,
        None => MISSING_FIELD_DEFAULT.to_string(),
    };
    let body = match context.field("target.body") {
        Some(value) => value,
        None => MISSING_FIELD_DEFAULT.to_string(),
    };
    let prompt = build_prompt(&number, &title, &body);

    let status = ProcessCommand::new("cargo")
        .args([
            "run",
            "--quiet",
            "-p",
            "jekko-cli",
            "--",
            "run",
            "--provider",
            "jekko",
            "--model",
            "big-pickle",
            "--agent",
            "triage",
            &prompt,
        ])
        .status()
        .context("run `cargo run -p jekko-cli -- run` for issue triage")?;
    if !status.success() {
        bail!("`cargo run -p jekko-cli -- run` for issue triage failed with {status}");
    }
    Ok(())
}

fn build_prompt(number: &str, title: &str, body: &str) -> String {
    format!(
        "The following issue was just opened, triage it:

Issue number: {number}

Title: {title}

{body}
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_includes_issue_metadata() {
        let prompt = build_prompt("42", "Bug", "Need help");
        assert!(prompt.contains("Issue number: 42"));
        assert!(prompt.contains("Title: Bug"));
        assert!(prompt.contains("Need help"));
    }
}
