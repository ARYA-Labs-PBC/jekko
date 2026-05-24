use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::process::Command as ProcessCommand;

pub fn run() -> Result<()> {
    let repo = std::env::var("GITHUB_REPOSITORY").context("reading GITHUB_REPOSITORY")?;
    let cutoff = two_hours_ago();
    let items = fetch_compliance_items(&repo)?;

    if items.is_empty() {
        println!("No open issues/PRs with needs:compliance label");
        return Ok(());
    }

    let mut closed = 0usize;
    for item in items {
        let issue_number = item
            .get("number")
            .and_then(Value::as_u64)
            .context("missing issue number")?;
        let is_pr = item.get("pull_request").is_some();
        let comments = fetch_issue_comments(&repo, issue_number)?;
        let Some(comment) = comments.iter().find(|comment| {
            comment
                .get("body")
                .and_then(Value::as_str)
                .is_some_and(|body| body.contains("<!-- issue-compliance -->"))
        }) else {
            continue;
        };
        let created_at = comment
            .get("created_at")
            .and_then(Value::as_str)
            .context("missing compliance comment created_at")?;
        let created_at = parse_timestamp(created_at)?;
        if created_at > cutoff {
            continue;
        }

        let close_message = if is_pr {
            "This pull request has been automatically closed because it was not updated to meet our [contributing guidelines](../blob/dev/CONTRIBUTING.md) within the 2-hour window.\n\nFeel free to open a new pull request that follows our guidelines."
        } else {
            "This issue has been automatically closed because it was not updated to meet our [contributing guidelines](../blob/dev/CONTRIBUTING.md) within the 2-hour window.\n\nFeel free to open a new issue that follows our issue templates."
        };

        gh_api([
            "--method",
            "POST",
            &format!("/repos/{repo}/issues/{issue_number}/comments"),
            "-f",
            &format!("body={close_message}"),
        ])?;
        let _ = gh_api([
            "--method",
            "DELETE",
            &format!("/repos/{repo}/issues/{issue_number}/labels/needs%3Acompliance"),
        ]);
        if is_pr {
            gh_api([
                "--method",
                "PATCH",
                &format!("/repos/{repo}/pulls/{issue_number}"),
                "-f",
                "state=closed",
            ])?;
        } else {
            gh_api([
                "--method",
                "PATCH",
                &format!("/repos/{repo}/issues/{issue_number}"),
                "-f",
                "state=closed",
                "-f",
                "state_reason=not_planned",
            ])?;
        }
        println!(
            "Closed non-compliant {} #{} after 2-hour window",
            if is_pr { "PR" } else { "issue" },
            issue_number
        );
        closed += 1;
    }

    if closed == 0 {
        println!("No compliant issues/PRs were ready for auto-close.");
    }
    Ok(())
}

fn fetch_compliance_items(repo: &str) -> Result<Vec<Value>> {
    let mut page = 1usize;
    let mut results = Vec::new();

    loop {
        let output = ProcessCommand::new("gh")
            .args([
                "api",
                &format!("/repos/{repo}/issues"),
                "-f",
                "labels=needs:compliance",
                "-f",
                "state=open",
                "-f",
                "per_page=100",
                "-F",
                &format!("page={page}"),
            ])
            .output()
            .context("running gh api issues")?;
        if !output.status.success() {
            bail!("gh api issues failed with status {}", output.status);
        }
        let items: Vec<Value> =
            serde_json::from_slice(&output.stdout).context("parse issues JSON")?;
        if items.is_empty() {
            break;
        }
        let count = items.len();
        results.extend(items);
        if count < 100 {
            break;
        }
        page += 1;
    }

    Ok(results)
}

fn fetch_issue_comments(repo: &str, issue_number: u64) -> Result<Vec<Value>> {
    let mut results = Vec::new();
    let mut page = 1usize;

    loop {
        let output = ProcessCommand::new("gh")
            .args([
                "api",
                &format!("/repos/{repo}/issues/{issue_number}/comments"),
                "-f",
                "per_page=100",
                "-F",
                &format!("page={page}"),
            ])
            .output()
            .context("running gh api issue comments")?;
        if !output.status.success() {
            bail!("gh api issue comments failed with status {}", output.status);
        }

        let comments: Vec<Value> =
            serde_json::from_slice(&output.stdout).context("parse issue comments JSON")?;
        if comments.is_empty() {
            break;
        }
        let count = comments.len();
        results.extend(comments);
        if count < 100 {
            break;
        }
        page += 1;
    }

    Ok(results)
}

fn two_hours_ago() -> DateTime<Utc> {
    Utc::now() - Duration::hours(2)
}

fn gh_api<const N: usize>(args: [&str; N]) -> Result<()> {
    let output = ProcessCommand::new("gh")
        .args(["api"])
        .args(args)
        .output()
        .context("running gh api")?;
    if !output.status.success() {
        bail!("gh api {:?} failed with status {}", args, output.status);
    }
    Ok(())
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("parse RFC3339 timestamp: {value}"))?
        .with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rfc3339_timestamps() {
        let parsed = parse_timestamp("2026-01-01T12:34:56Z").unwrap();
        assert_eq!(parsed.to_rfc3339(), "2026-01-01T12:34:56+00:00");
    }
}
