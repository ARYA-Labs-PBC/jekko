use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use std::process::Command as ProcessCommand;

use crate::current_github_event_context;

const DEFAULT_BRANCH: &str = "main";

pub fn run() -> Result<()> {
    let context = current_github_event_context()?;
    let repo = std::env::var("GITHUB_REPOSITORY").context("reading GITHUB_REPOSITORY")?;
    let number = pr_number(&context)?;
    let login = pull_request_field(&context, "pull_request.author.login")?;
    let title = pull_request_field(&context, "pull_request.title")?;
    let created_at = parse_timestamp(&pull_request_field(&context, "pull_request.created_at")?)?;
    let default_branch = match context
        .field("repository.default_branch")
        .filter(|value| !value.is_empty())
    {
        Some(value) => value,
        None => DEFAULT_BRANCH.to_string(),
    };

    if created_at < cutoff() {
        return Ok(());
    }
    if login == "jekko-agent[bot]" || is_team_member(&login)? {
        return Ok(());
    }

    if !is_valid_title(&title) {
        gh_api([
            "--method",
            "POST",
            &format!("/repos/{repo}/issues/{number}/labels"),
            "-f",
            "labels=[\"needs:title\"]",
        ])?;
        gh_api([
            "--method",
            "POST",
            &format!("/repos/{repo}/issues/{number}/comments"),
            "-f",
            &format!(
                "body=Hey! Your PR title `{title}` doesn't follow conventional commit format.\n\nPlease update it to start with one of:\n- `feat:` or `feat(scope):` new feature\n- `fix:` or `fix(scope):` bug fix\n- `docs:` or `docs(scope):` documentation changes\n- `chore:` or `chore(scope):` maintenance tasks\n- `refactor:` or `refactor(scope):` code refactoring\n- `test:` or `test(scope):` adding or updating tests\n\nWhere `scope` is the package name (e.g., `app`, `jekko`).\n\nSee [CONTRIBUTING.md](../blob/{default_branch}/CONTRIBUTING.md#pr-titles) for details."
            ),
        ])?;
        return Ok(());
    }

    let _ = gh_api([
        "--method",
        "DELETE",
        &format!("/repos/{repo}/issues/{number}/labels/needs:title"),
    ]);

    if title_is_issue_exempt(&title) {
        let _ = gh_api([
            "--method",
            "DELETE",
            &format!("/repos/{repo}/issues/{number}/labels/needs:issue"),
        ]);
        return Ok(());
    }

    Ok(())
}

fn cutoff() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-02-19T00:00:00Z")
        .expect("valid cutoff")
        .with_timezone(&Utc)
}

fn pr_number(context: &jekko_core::github::GitHubEventContext) -> Result<u64> {
    context
        .field("pull_request.number")
        .context("missing pull_request.number")?
        .parse()
        .context("parse pull_request.number")
}

fn pull_request_field(
    context: &jekko_core::github::GitHubEventContext,
    field: &str,
) -> Result<String> {
    context
        .field(field)
        .with_context(|| format!("missing GitHub event field: {field}"))
}

fn is_team_member(login: &str) -> Result<bool> {
    let team_members =
        std::fs::read_to_string(".github/TEAM_MEMBERS").context("read .github/TEAM_MEMBERS")?;
    Ok(team_members.lines().any(|line| line == login))
}

fn is_valid_title(title: &str) -> bool {
    matches_title_prefix(title, &["feat", "fix", "docs", "chore", "refactor", "test"])
}

fn title_is_issue_exempt(title: &str) -> bool {
    matches_title_prefix(title, &["docs", "refactor", "feat"])
}

fn matches_title_prefix(title: &str, allowed: &[&str]) -> bool {
    let trimmed = title.trim_start();
    allowed.iter().any(|prefix| {
        let base = format!("{prefix}:");
        if trimmed.starts_with(&base) {
            return true;
        }
        if trimmed.starts_with(prefix) {
            let Some(rest) = trimmed.strip_prefix(prefix) else {
                return false;
            };
            if let Some(rest) = rest.strip_prefix('(') {
                if let Some(end) = rest.find(')') {
                    let scope = &rest[..end];
                    return !scope.is_empty()
                        && scope.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                        && rest[end + 1..].trim_start().starts_with(':');
                }
            }
        }
        false
    })
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
    fn validates_committer_prefixes() {
        assert!(is_valid_title("feat: add thing"));
        assert!(is_valid_title("fix(ui): patch thing"));
        assert!(!is_valid_title("build: nope"));
    }

    #[test]
    fn detects_issue_exempt_titles() {
        assert!(title_is_issue_exempt("docs: update"));
        assert!(title_is_issue_exempt("refactor(core): update"));
        assert!(title_is_issue_exempt("feat: add"));
        assert!(!title_is_issue_exempt("fix: patch"));
    }

    #[test]
    fn parses_rfc3339_timestamps() {
        let parsed = parse_timestamp("2026-01-01T12:34:56Z").unwrap();
        assert_eq!(parsed.to_rfc3339(), "2026-01-01T12:34:56+00:00");
    }
}
