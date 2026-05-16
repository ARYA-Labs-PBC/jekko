use anyhow::{bail, Context, Result};
use std::io::Write;
use std::process::{Command as ProcessCommand, Stdio};

use jekko_core::provider::ModelRef;

use crate::current_github_event_context;
use crate::pr_info::pull_request_field;

const DEFAULT_MODEL: &str = "big-pickle";

pub fn run() -> Result<()> {
    let context = current_github_event_context()?;
    let login = context
        .field("target.author.login")
        .context("missing target.author.login")?;

    if login == "jekko-agent[bot]" || is_team_member(&login)? {
        println!("Skipping: {login} is a team member or bot");
        return Ok(());
    }

    let number = pr_number(&context)?;
    let title = pull_request_field(number, "title")?;
    let body = pull_request_field(number, "body")?;
    let prompt = build_prompt(number, &title, &body);

    let model = match std::env::var("MODEL") {
        Ok(value) => value,
        Err(_) => DEFAULT_MODEL.to_string(),
    };
    let model_ref = match ModelRef::parse(&model) {
        Ok(parsed) => parsed,
        Err(_) => ModelRef::parse(&format!("jekko/{model}")).map_err(|err| anyhow::anyhow!(err))?,
    };

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
        "duplicate-pr",
    ]);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());

    let mut child = cmd
        .spawn()
        .context("run `cargo run -p jekko-cli -- run` for PR duplicate checks")?;
    {
        let stdin = child.stdin.as_mut().context("open child stdin")?;
        stdin.write_all(prompt.as_bytes()).context("write prompt")?;
    }

    let output = child
        .wait_with_output()
        .context("wait for PR duplicate checks")?;
    if !output.status.success() {
        bail!(
            "`cargo run -p jekko-cli -- run` for PR duplicate checks failed with {}",
            output.status
        );
    }

    let comment = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if comment.is_empty() || comment == "No duplicate PRs found" {
        return Ok(());
    }

    let repo = std::env::var("GITHUB_REPOSITORY").context("reading GITHUB_REPOSITORY")?;
    gh_pr_comment(
        &repo,
        number,
        &format!("_The following comment was made by an LLM, it may be inaccurate:_\n\n{comment}"),
    )?;
    Ok(())
}

fn pr_number(context: &jekko_core::github::GitHubEventContext) -> Result<u64> {
    context
        .field("target.number")
        .context("missing target.number")?
        .parse()
        .context("parse target.number")
}

fn is_team_member(login: &str) -> Result<bool> {
    let team_members =
        std::fs::read_to_string(".github/TEAM_MEMBERS").context("read .github/TEAM_MEMBERS")?;
    Ok(team_members.lines().any(|line| line == login))
}

fn build_prompt(number: u64, title: &str, body: &str) -> String {
    format!(
        "Check for duplicate pull requests related to this new PR:

CURRENT_PR_NUMBER: {number}

Title: {title}

Description:
{body}

Return exactly `No duplicate PRs found` if there are none. Otherwise, return a concise comment body that explains the likely duplicates and why they matter."
    )
}

fn gh_pr_comment(repo: &str, number: u64, body: &str) -> Result<()> {
    let output = ProcessCommand::new("gh")
        .args([
            "pr",
            "comment",
            &number.to_string(),
            "--repo",
            repo,
            "--body",
            body,
        ])
        .output()
        .context("running gh pr comment")?;
    if !output.status.success() {
        bail!("gh pr comment failed with status {}", output.status);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_mentions_pr_details_and_no_duplicate_contract() {
        let prompt = build_prompt(42, "Bug fix", "Please review");
        assert!(prompt.contains("CURRENT_PR_NUMBER: 42"));
        assert!(prompt.contains("Title: Bug fix"));
        assert!(prompt.contains("No duplicate PRs found"));
    }
}
