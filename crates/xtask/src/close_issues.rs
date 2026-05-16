use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::process::Command as ProcessCommand;

const DEFAULT_REPOSITORY: &str = "neverhuman/jekko";

#[derive(Debug, Clone, PartialEq, Eq)]
struct IssueRecord {
    number: u64,
    updated_at: DateTime<Utc>,
}

pub fn run() -> Result<()> {
    let repo = match std::env::var("GITHUB_REPOSITORY") {
        Ok(value) => value,
        Err(_) => DEFAULT_REPOSITORY.to_string(),
    };
    let cutoff = sixty_days_ago();
    let issues = fetch_open_issues(&repo)?;

    let mut closed = 0usize;
    for issue in issues {
        if issue.updated_at >= cutoff {
            println!("Found fresh issue #{}, stopping", issue.number);
            break;
        }

        close_issue(&repo, issue.number)?;
        closed += 1;
    }

    println!("Closed {} issues total", closed);
    Ok(())
}

fn sixty_days_ago() -> DateTime<Utc> {
    Utc::now() - Duration::days(60)
}

fn fetch_open_issues(repo: &str) -> Result<Vec<IssueRecord>> {
    let mut results = Vec::new();
    let mut page = 1usize;

    loop {
        let output = ProcessCommand::new("gh")
            .args([
                "api",
                &format!("/repos/{repo}/issues?state=open&sort=updated&direction=asc&per_page=100"),
                "-F",
                &format!("page={page}"),
            ])
            .output()
            .context("running gh api issues")?;
        if !output.status.success() {
            bail!("gh api issues failed with status {}", output.status);
        }

        let issues: Vec<Value> =
            serde_json::from_slice(&output.stdout).context("parse issues JSON")?;
        if issues.is_empty() {
            break;
        }

        let page_len = issues.len();
        for issue in issues {
            if issue.get("pull_request").is_some() {
                continue;
            }
            let number = issue
                .get("number")
                .and_then(Value::as_u64)
                .context("missing issue number")?;
            let updated_at = issue
                .get("updated_at")
                .and_then(Value::as_str)
                .context("missing issue updated_at")?;
            results.push(IssueRecord {
                number,
                updated_at: parse_timestamp(updated_at)?,
            });
        }

        if page_len < 100 {
            break;
        }
        page += 1;
    }

    Ok(results)
}

fn close_issue(repo: &str, number: u64) -> Result<()> {
    let msg = "To stay organized issues are automatically closed after 60 days of no activity. If the issue is still relevant please open a new one.";
    gh_issue([
        "comment",
        &number.to_string(),
        "--repo",
        repo,
        "--body",
        msg,
    ])?;
    gh_issue([
        "close",
        &number.to_string(),
        "--repo",
        repo,
        "--reason",
        "not planned",
        "--yes",
    ])?;
    println!("Closed https://github.com/{repo}/issues/{number}");
    Ok(())
}

fn gh_issue<const N: usize>(args: [&str; N]) -> Result<()> {
    let output = ProcessCommand::new("gh")
        .args(args)
        .output()
        .context("running gh issue")?;
    if !output.status.success() {
        bail!("gh issue {:?} failed with status {}", args, output.status);
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
