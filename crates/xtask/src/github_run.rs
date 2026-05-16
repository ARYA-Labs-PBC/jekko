use anyhow::{bail, Context, Result};
use std::process::Command as ProcessCommand;

use jekko_core::provider::ModelRef;

use crate::current_github_event_context;

const DEFAULT_MODEL: &str = "big-pickle";

pub fn run() -> Result<()> {
    let context = current_github_event_context()?;
    let model = match std::env::var("MODEL") {
        Ok(value) => value,
        Err(_) => DEFAULT_MODEL.to_string(),
    };
    let model_ref = match ModelRef::parse(&model) {
        Ok(parsed) => parsed,
        Err(_) => ModelRef::parse(&format!("jekko/{model}")).map_err(|err| anyhow::anyhow!(err))?,
    };
    let prompt = build_prompt(&context);

    let mut cmd = ProcessCommand::new("cargo");
    cmd.args([
        "run",
        "--quiet",
        "-p",
        "jekko-cli",
        "--",
        "run",
        "--provider",
        model_ref.provider_id.as_str(),
        "--model",
        model_ref.id.as_str(),
        "--agent",
        "github",
        &prompt,
    ]);

    if let Ok(cwd) = std::env::current_dir() {
        cmd.current_dir(cwd);
    }

    let status = cmd
        .status()
        .context("run `cargo run -p jekko-cli -- run` for GitHub automation")?;
    if !status.success() {
        bail!("`cargo run -p jekko-cli -- run` for GitHub automation failed with {status}");
    }
    Ok(())
}

fn build_prompt(context: &jekko_core::github::GitHubEventContext) -> String {
    let mut sections = Vec::new();
    sections.push(format!("GitHub event: {}", context.event_name));
    sections.push(format!("Repository: {}/{}", context.owner, context.repo));
    if let Some(actor) = context.actor.as_deref() {
        sections.push(format!("Actor: {actor}"));
    }
    if let Some(number) = context.field("target.number") {
        sections.push(format!("Target number: {number}"));
    }
    if let Some(title) = context.field("target.title") {
        sections.push(format!("Title: {title}"));
    }
    if let Some(body) = context.field("target.body") {
        sections.push(format!("Body:\n{body}"));
    }
    if let Some(comment) = context.field("comment.body") {
        sections.push(format!("Comment:\n{comment}"));
    }
    if let Some(path) = context.field("comment.path") {
        sections.push(format!("Comment path: {path}"));
    }
    if let Some(line) = context.field("comment.line") {
        sections.push(format!("Comment line: {line}"));
    }
    let release_heading = match context.field("release.name") {
        Some(value) => Some(value),
        None => context.field("release.tag_name"),
    };
    if let Some(heading) = release_heading {
        sections.push(format!("Release: {heading}"));
    }

    sections.push(
        "You are Jekko's GitHub automation agent. Use the GitHub CLI and the local repository tools to perform the requested action. For comment-triggered events, inspect the target code before replying. Keep the response concise and only make changes or comments that are justified by the repository state.".to_string(),
    );

    sections.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use jekko_core::github::parse_github_event_context;
    use serde_json::json;

    #[test]
    fn build_prompt_includes_comment_context() {
        let payload = json!({
            "repository": { "default_branch": "main" },
            "issue": { "number": 7, "title": "Bug", "body": "Please fix" },
            "comment": { "body": "/jekko summarize", "path": "src/lib.rs", "line": 42 }
        });
        let ctx =
            parse_github_event_context("issue_comment", "owner/repo", Some("alice"), &payload)
                .unwrap();
        let prompt = build_prompt(&ctx);
        assert!(prompt.contains("GitHub event: issue_comment"));
        assert!(prompt.contains("Repository: owner/repo"));
        assert!(prompt.contains("Actor: alice"));
        assert!(prompt.contains("Target number: 7"));
        assert!(prompt.contains("Comment path: src/lib.rs"));
        assert!(prompt.contains("Comment line: 42"));
    }
}
