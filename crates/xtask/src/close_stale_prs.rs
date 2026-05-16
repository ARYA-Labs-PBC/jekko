use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::process::Command as ProcessCommand;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PullRequestRecord {
    number: u64,
    title: String,
    author_login: Option<String>,
    created_at: DateTime<Utc>,
    last_commit_at: Option<DateTime<Utc>>,
    last_comment_at: Option<DateTime<Utc>>,
    last_review_at: Option<DateTime<Utc>>,
}

pub fn run(explicit_dry_run: Option<bool>) -> Result<()> {
    let repo = std::env::var("GITHUB_REPOSITORY").context("reading GITHUB_REPOSITORY")?;
    let (owner, name) = repo
        .split_once('/')
        .context("GITHUB_REPOSITORY must be in owner/repo form")?;

    let event_path = std::env::var("GITHUB_EVENT_PATH").context("reading GITHUB_EVENT_PATH")?;
    let event_text = std::fs::read_to_string(&event_path)
        .with_context(|| format!("read GitHub event payload {event_path}"))?;
    let event: Value =
        serde_json::from_str(&event_text).context("parse GitHub event payload JSON")?;
    let dry_run = match explicit_dry_run {
        Some(value) => value,
        None => event_dry_run(&event),
    };

    let cutoff = sixty_days_ago();
    let prs = fetch_open_pull_requests(owner, name)?;
    let inactive: Vec<_> = prs
        .into_iter()
        .filter(|pr| last_activity(pr) <= cutoff)
        .collect();

    if inactive.is_empty() {
        println!("No inactive pull requests found.");
        return Ok(());
    }

    for pr in &inactive {
        if dry_run {
            println!(
                "[dry-run] Would close PR #{} from {}: {}",
                pr.number,
                pr.author_login.as_deref().unwrap_or("unknown"),
                pr.title
            );
        } else {
            gh_pr(&["comment", &pr.number.to_string(), "--body", close_message()])?;
            gh_pr(&["close", &pr.number.to_string()])?;
        }
    }

    println!("Processed {} inactive pull requests.", inactive.len());
    Ok(())
}

fn close_message() -> &'static str {
    "Closing this pull request because it has had no updates for more than 60 days. If you plan to continue working on it, feel free to reopen or open a new PR."
}

const DEFAULT_DRY_RUN: bool = false;

fn event_dry_run(event: &Value) -> bool {
    let Some(value) = event.get("inputs").and_then(|inputs| inputs.get("dryRun")) else {
        return DEFAULT_DRY_RUN;
    };
    if let Some(text) = value.as_str() {
        return text == "true";
    }
    match value.as_bool() {
        Some(flag) => flag,
        None => DEFAULT_DRY_RUN,
    }
}

fn sixty_days_ago() -> DateTime<Utc> {
    Utc::now() - Duration::days(60)
}

fn fetch_open_pull_requests(owner: &str, name: &str) -> Result<Vec<PullRequestRecord>> {
    let mut results = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut cmd = ProcessCommand::new("gh");
        cmd.args([
            "api",
            "graphql",
            "-f",
            &format!("owner={owner}"),
            "-f",
            &format!("repo={name}"),
        ]);
        if let Some(cursor) = &cursor {
            cmd.args(["-F", &format!("cursor={cursor}")]);
        }
        cmd.args(["-f", &format!("query={}", pull_request_query())]);
        let output = cmd.output().context("running gh api graphql")?;
        if !output.status.success() {
            bail!("gh api graphql failed with status {}", output.status);
        }

        let json: Value =
            serde_json::from_slice(&output.stdout).context("parse GraphQL response JSON")?;
        let pull_requests = json
            .get("data")
            .and_then(|data| data.get("repository"))
            .and_then(|repo| repo.get("pullRequests"))
            .context("missing repository.pullRequests in GraphQL response")?;

        let page_info = pull_requests
            .get("pageInfo")
            .and_then(Value::as_object)
            .context("missing pullRequests.pageInfo")?;
        let nodes = pull_requests
            .get("nodes")
            .and_then(Value::as_array)
            .context("missing pullRequests.nodes")?;

        for node in nodes {
            results.push(parse_pull_request(node)?);
        }

        let has_next = page_info
            .get("hasNextPage")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !has_next {
            break;
        }
        cursor = page_info
            .get("endCursor")
            .and_then(Value::as_str)
            .map(str::to_string);
    }

    Ok(results)
}

fn pull_request_query() -> &'static str {
    r#"
query($owner: String!, $repo: String!, $cursor: String = null) {
  repository(owner: $owner, name: $repo) {
    pullRequests(first: 100, states: OPEN, after: $cursor) {
      pageInfo { hasNextPage endCursor }
      nodes {
        number
        title
        author { login }
        createdAt
        commits(last: 1) { nodes { commit { committedDate } } }
        comments(last: 1) { nodes { createdAt } }
        reviews(last: 1) { nodes { createdAt } }
      }
    }
  }
}
"#
}

fn parse_pull_request(node: &Value) -> Result<PullRequestRecord> {
    Ok(PullRequestRecord {
        number: node
            .get("number")
            .and_then(Value::as_u64)
            .context("missing pull request number")?,
        title: node
            .get("title")
            .and_then(Value::as_str)
            .context("missing pull request title")?
            .to_string(),
        author_login: node
            .get("author")
            .and_then(Value::as_object)
            .and_then(|author| author.get("login"))
            .and_then(Value::as_str)
            .map(str::to_string),
        created_at: node
            .get("createdAt")
            .and_then(Value::as_str)
            .context("missing pull request createdAt")?
            .parse()
            .context("parse pull request createdAt")?,
        last_commit_at: node
            .get("commits")
            .and_then(Value::as_object)
            .and_then(|commits| commits.get("nodes"))
            .and_then(Value::as_array)
            .and_then(|nodes| nodes.first())
            .and_then(|commit| commit.get("commit"))
            .and_then(Value::as_object)
            .and_then(|commit| commit.get("committedDate"))
            .and_then(Value::as_str)
            .map(parse_timestamp)
            .transpose()?,
        last_comment_at: node
            .get("comments")
            .and_then(Value::as_object)
            .and_then(|comments| comments.get("nodes"))
            .and_then(Value::as_array)
            .and_then(|nodes| nodes.first())
            .and_then(|comment| comment.get("createdAt"))
            .and_then(Value::as_str)
            .map(parse_timestamp)
            .transpose()?,
        last_review_at: node
            .get("reviews")
            .and_then(Value::as_object)
            .and_then(|reviews| reviews.get("nodes"))
            .and_then(Value::as_array)
            .and_then(|nodes| nodes.first())
            .and_then(|review| review.get("createdAt"))
            .and_then(Value::as_str)
            .map(parse_timestamp)
            .transpose()?,
    })
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("parse RFC3339 timestamp: {value}"))?
        .with_timezone(&Utc))
}

fn last_activity(pr: &PullRequestRecord) -> DateTime<Utc> {
    [
        Some(pr.created_at),
        pr.last_commit_at,
        pr.last_comment_at,
        pr.last_review_at,
    ]
    .into_iter()
    .flatten()
    .max()
    .unwrap_or(pr.created_at)
}

fn gh_pr(args: &[&str]) -> Result<()> {
    let output = ProcessCommand::new("gh")
        .args(["pr"])
        .args(args)
        .output()
        .context("running gh pr")?;
    if !output.status.success() {
        bail!("gh pr {:?} failed with status {}", args, output.status);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_latest_activity() {
        let pr = PullRequestRecord {
            number: 1,
            title: "t".to_string(),
            author_login: None,
            created_at: parse_timestamp("2026-01-01T00:00:00Z").unwrap(),
            last_commit_at: Some(parse_timestamp("2026-01-02T00:00:00Z").unwrap()),
            last_comment_at: Some(parse_timestamp("2025-12-31T00:00:00Z").unwrap()),
            last_review_at: Some(parse_timestamp("2026-01-03T00:00:00Z").unwrap()),
        };
        assert_eq!(
            last_activity(&pr),
            parse_timestamp("2026-01-03T00:00:00Z").unwrap()
        );
    }

    #[test]
    fn parses_rfc3339_timestamps() {
        let parsed = parse_timestamp("2026-01-01T12:34:56Z").unwrap();
        assert_eq!(parsed.to_rfc3339(), "2026-01-01T12:34:56+00:00");
    }
}
