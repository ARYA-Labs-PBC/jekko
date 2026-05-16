use anyhow::{bail, Context, Result};
use std::process::Command as ProcessCommand;

use crate::current_github_event_context;
use crate::pr_info::pull_request_field;

pub fn run() -> Result<()> {
    let context = current_github_event_context()?;
    let number = pr_number(&context)?;
    let title = pull_request_field(number, "title")?;
    let body = pull_request_field(number, "body")?;
    let sha = pull_request_field(number, "head.sha")?;

    let prompt = review_prompt(number, &title, &body);

    let mut cmd = ProcessCommand::new("cargo");
    cmd.args([
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
        "review",
        &prompt,
    ]);
    cmd.env("PR_NUMBER", number.to_string());
    cmd.env("PR_TITLE", &title);
    cmd.env("PR_BODY", &body);
    cmd.env("PR_SHA", &sha);
    let status = cmd
        .status()
        .context("run `cargo run -p jekko-cli -- run` for PR review")?;
    if !status.success() {
        bail!("`cargo run -p jekko-cli -- run` for PR review failed with {status}");
    }
    Ok(())
}

fn pr_number(context: &jekko_core::github::GitHubEventContext) -> Result<u64> {
    Ok(context
        .field("pull_request.number")
        .context("missing pull_request.number")?
        .parse()
        .context("parse pull_request.number")?)
}

fn review_prompt(number: u64, title: &str, body: &str) -> String {
    format!(
        "A new pull request has been created: '{title}'

<pr-number>
{number}
</pr-number>

<pr-description>
{body}
</pr-description>

Please check all the code changes in this pull request against the style guide, also look for any bugs if they exist. Diffs are important but make sure you read the entire file to get proper context. Make it clear the suggestions are merely suggestions and the human can decide what to do

When critiquing code against the style guide, be sure that the code is ACTUALLY in violation, don't complain about else statements if they already use early returns there. You may complain about excessive nesting though, regardless of else statement usage.
When critiquing code style don't be a zealot, we don't like \"let\" statements but sometimes they are the simplest option, if someone does a bunch of nesting with let, they should consider using iife (see packages/jekko/src/util.iife.ts)

Use the gh cli to create comments on the files for the violations. Try to leave the comment on the exact line number. If you have a suggested fix include it in a suggestion code block.
If you are writing suggested fixes, BE SURE THAT the change you are recommending is actually valid typescript, often I have seen missing closing \"}}\" or other syntax errors.
Generally, write a comment instead of writing suggested change if you can help it.

Command MUST be like this.
gh api \\
  --method POST \\
  -H \"Accept: application/vnd.github+json\" \\
  -H \"X-GitHub-Api-Version: 2022-11-28\" \\
  /repos/${{GITHUB_REPOSITORY}}/pulls/{number}/comments \\
  -f 'body=[summary of issue]' -f 'commit_id=${{PR_SHA}}' -f 'path=[path-to-file]' -F \"line=[line]\" -f 'side=RIGHT'

Only create comments for actual violations. If the code follows all guidelines, post a structured review receipt comment to the issue using a reproducible command.
Replay command:
gh api --method POST -H \"Accept: application/vnd.github+json\" -H \"X-GitHub-Api-Version: 2022-11-28\" /repos/${{GITHUB_REPOSITORY}}/issues/{number}/comments -f \"body={{\\\"review\\\":\\\"pass\\\",\\\"lane\\\":\\\"audit\\\",\\\"timestamp_utc\\\":\\\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\\\",\\\"proof\\\":\\\"no_violations_found\\\",\\\"replay\\\":\\\"gh api /repos/${{GITHUB_REPOSITORY}}/pulls/{number}/files\\\"}}\""
    )
}
