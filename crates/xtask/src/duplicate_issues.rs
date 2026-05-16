use anyhow::{bail, Context, Result};
use std::io::Write;
use std::process::{Command as ProcessCommand, Stdio};

use crate::current_github_event_context;

pub fn run() -> Result<()> {
    let context = current_github_event_context()?;
    let issue_number = context
        .field("target.number")
        .context("missing target.number")?;
    const MISSING_FIELD_DEFAULT: String = String::new();
    let title = match context.field("target.title") {
        Some(value) => value,
        None => MISSING_FIELD_DEFAULT,
    };
    let body = match context.field("target.body") {
        Some(value) => value,
        None => MISSING_FIELD_DEFAULT,
    };
    let prompt = build_prompt(&issue_number, &title, &body);

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
        "duplicate-issues",
    ]);
    cmd.stdin(Stdio::piped());

    let mut child = cmd
        .spawn()
        .context("run `cargo run -p jekko-cli -- run` for duplicate issue checks")?;
    {
        let stdin = child.stdin.as_mut().context("open child stdin")?;
        stdin.write_all(prompt.as_bytes()).context("write prompt")?;
    }
    let status = child.wait().context("wait for duplicate issue checks")?;
    if !status.success() {
        bail!("`cargo run -p jekko-cli -- run` for duplicate issue checks failed with {status}");
    }
    Ok(())
}

fn build_prompt(number: &str, title: &str, body: &str) -> String {
    format!(
        r#"A new issue has been created:

Issue number: {number}
Title: {title}

Body:
{body}

You have TWO tasks. Perform both, then post a SINGLE comment if needed.

TASK 1: CONTRIBUTING GUIDELINES COMPLIANCE CHECK

Check whether the issue follows our contributing guidelines and issue templates.

This project has three issue templates that every issue MUST use one of:

1. Bug Report - requires a Description field with real content
2. Feature Request - requires a verification checkbox and description, title should start with [FEATURE]:
3. Question - requires the Question field with real content

Additionally check:
- No AI-generated walls of text (long, AI-generated descriptions are not acceptable)
- The issue has real content, not just template sample text left unchanged
- Bug reports should include some context about how to reproduce
- Feature requests should explain the problem or need
- We want to push for having the user provide system description & information

Do NOT be nitpicky about optional fields. Only flag real problems like: no template used, required fields empty or sample text only, obviously AI-generated walls of text, or completely empty/nonsensical content.

TASK 2: DUPLICATE CHECK

Search through existing issues (excluding #{number}) to find potential duplicates.
Consider:
1. Similar titles or descriptions
2. Same error messages or symptoms
3. Related functionality or components
4. Similar feature requests

Additionally, if the issue mentions keybinds, keyboard shortcuts, or key bindings, note the pinned keybinds issue #4997.

POSTING YOUR COMMENT:

Based on your findings, post a SINGLE comment on issue #{number}.

If the issue is NOT compliant, start the comment with:
<!-- issue-compliance -->
Then explain what needs to be fixed and that they have 2 hours to edit the issue before it is automatically closed. Also add the label needs:compliance to the issue using: gh issue edit {number} --add-label needs:compliance

If duplicates were found, include a section about potential duplicates with links.

If the issue mentions keybinds/keyboard shortcuts, include a note about #4997.

If the issue IS compliant AND no duplicates were found, do not comment.
"#,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_mentions_compliance_and_duplicates() {
        let prompt = build_prompt("42", "Bug", "Need help");
        assert!(prompt.contains("Issue number: 42"));
        assert!(prompt.contains("TASK 1: CONTRIBUTING GUIDELINES COMPLIANCE CHECK"));
        assert!(prompt.contains("TASK 2: DUPLICATE CHECK"));
        assert!(prompt.contains("needs:compliance"));
        assert!(prompt.contains("#4997"));
    }
}
