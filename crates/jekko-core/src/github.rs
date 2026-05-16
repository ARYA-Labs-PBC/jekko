#![allow(missing_docs)]
//! GitHub-event payload parser. Pre-existing module retained verbatim.
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GitHubEventContext {
    pub event_name: String,
    pub owner: String,
    pub repo: String,
    pub actor: Option<String>,
    pub issue: GitHubIssue,
    pub pull_request: GitHubPullRequest,
    pub comment: GitHubComment,
    pub repository: GitHubRepository,
    pub release: GitHubRelease,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GitHubIssue {
    pub number: Option<u64>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub author_login: Option<String>,
    pub author_association: Option<String>,
    pub state: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GitHubPullRequest {
    pub number: Option<u64>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub author_login: Option<String>,
    pub author_association: Option<String>,
    pub state: Option<String>,
    pub draft: Option<bool>,
    pub merged: Option<bool>,
    pub head_ref: Option<String>,
    pub base_ref: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GitHubComment {
    pub body: Option<String>,
    pub id: Option<String>,
    pub path: Option<String>,
    pub line: Option<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GitHubRepository {
    pub default_branch: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GitHubRelease {
    pub name: Option<String>,
    pub tag_name: Option<String>,
    pub html_url: Option<String>,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum GitHubEventError {
    #[error("repository must be in the form owner/repo")]
    InvalidRepository,
    #[error("payload must be a JSON object")]
    InvalidPayload,
}

pub fn parse_github_event_context(
    event_name: &str,
    repository: &str,
    actor: Option<&str>,
    payload: &Value,
) -> Result<GitHubEventContext, GitHubEventError> {
    let (owner, repo) = parse_repository(repository)?;
    let payload = payload
        .as_object()
        .ok_or(GitHubEventError::InvalidPayload)?;

    Ok(GitHubEventContext {
        event_name: event_name.to_string(),
        owner,
        repo,
        actor: actor.map(str::to_string),
        issue: parse_issue(payload),
        pull_request: parse_pull_request(payload),
        comment: parse_comment(payload),
        repository: parse_repository_meta(payload),
        release: parse_release(payload),
    })
}

impl GitHubEventContext {
    pub fn field(&self, path: &str) -> Option<String> {
        match path {
            "event.name" => Some(self.event_name.clone()),
            "repo.owner" => Some(self.owner.clone()),
            "repo.name" => Some(self.repo.clone()),
            "actor" => self.actor.clone(),
            "repository.default_branch" => self.repository.default_branch.clone(),
            "issue.number" => self.issue.number.map(|value| value.to_string()),
            "issue.title" => self.issue.title.clone(),
            "issue.body" => self.issue.body.clone(),
            "issue.author.login" => self.issue.author_login.clone(),
            "issue.author.association" => self.issue.author_association.clone(),
            "issue.state" => self.issue.state.clone(),
            "issue.created_at" => self.issue.created_at.clone(),
            "pull_request.number" => self.pull_request.number.map(|value| value.to_string()),
            "pull_request.title" => self.pull_request.title.clone(),
            "pull_request.body" => self.pull_request.body.clone(),
            "pull_request.author.login" => self.pull_request.author_login.clone(),
            "pull_request.author.association" => self.pull_request.author_association.clone(),
            "pull_request.state" => self.pull_request.state.clone(),
            "pull_request.draft" => self.pull_request.draft.map(|value| value.to_string()),
            "pull_request.merged" => self.pull_request.merged.map(|value| value.to_string()),
            "pull_request.head_ref" => self.pull_request.head_ref.clone(),
            "pull_request.base_ref" => self.pull_request.base_ref.clone(),
            "pull_request.created_at" => self.pull_request.created_at.clone(),
            "target.number" => self
                .issue
                .number
                .or(self.pull_request.number)
                .map(|value| value.to_string()),
            "target.title" => self.issue.title.clone().or(self.pull_request.title.clone()),
            "target.body" => self.issue.body.clone().or(self.pull_request.body.clone()),
            "target.author.login" => self
                .issue
                .author_login
                .clone()
                .or(self.pull_request.author_login.clone()),
            "target.author.association" => self
                .issue
                .author_association
                .clone()
                .or(self.pull_request.author_association.clone()),
            "target.created_at" => self
                .issue
                .created_at
                .clone()
                .or(self.pull_request.created_at.clone()),
            "comment.body" => self.comment.body.clone(),
            "comment.id" => self.comment.id.clone(),
            "comment.path" => self.comment.path.clone(),
            "comment.line" => self.comment.line.map(|value| value.to_string()),
            "release.name" => self.release.name.clone(),
            "release.tag_name" => self.release.tag_name.clone(),
            "release.html_url" => self.release.html_url.clone(),
            _ => None,
        }
    }
}

fn parse_repository(repository: &str) -> Result<(String, String), GitHubEventError> {
    let (owner, repo) = repository
        .split_once('/')
        .ok_or(GitHubEventError::InvalidRepository)?;
    if owner.is_empty() || repo.is_empty() {
        return Err(GitHubEventError::InvalidRepository);
    }
    Ok((owner.to_string(), repo.to_string()))
}

fn parse_issue(payload: &serde_json::Map<String, Value>) -> GitHubIssue {
    let Some(issue) = payload.get("issue").and_then(Value::as_object) else {
        return GitHubIssue::default();
    };
    GitHubIssue {
        number: number(issue.get("number")),
        title: string(issue.get("title")),
        body: string(issue.get("body")),
        author_login: string_at(issue, &["user", "login"]),
        author_association: string(issue.get("author_association")),
        state: string(issue.get("state")),
        created_at: string(issue.get("created_at")),
    }
}

fn parse_pull_request(payload: &serde_json::Map<String, Value>) -> GitHubPullRequest {
    let Some(pr) = payload.get("pull_request").and_then(Value::as_object) else {
        return GitHubPullRequest::default();
    };
    GitHubPullRequest {
        number: number(pr.get("number")),
        title: string(pr.get("title")),
        body: string(pr.get("body")),
        author_login: string_at(pr, &["user", "login"]),
        author_association: string(pr.get("author_association")),
        state: string(pr.get("state")),
        draft: boolean(pr.get("draft")),
        merged: boolean(pr.get("merged")),
        head_ref: string_at(pr, &["head", "ref"]),
        base_ref: string_at(pr, &["base", "ref"]),
        created_at: string(pr.get("created_at")),
    }
}

fn parse_comment(payload: &serde_json::Map<String, Value>) -> GitHubComment {
    let Some(comment) = payload.get("comment").and_then(Value::as_object) else {
        return GitHubComment::default();
    };
    GitHubComment {
        body: string(comment.get("body")),
        id: string(comment.get("id")),
        path: string(comment.get("path")),
        line: number(comment.get("line")),
    }
}

fn parse_repository_meta(payload: &serde_json::Map<String, Value>) -> GitHubRepository {
    let Some(repo) = payload.get("repository").and_then(Value::as_object) else {
        return GitHubRepository::default();
    };
    GitHubRepository {
        default_branch: string(repo.get("default_branch")),
    }
}

fn parse_release(payload: &serde_json::Map<String, Value>) -> GitHubRelease {
    let Some(release) = payload.get("release").and_then(Value::as_object) else {
        return GitHubRelease::default();
    };
    GitHubRelease {
        name: string(release.get("name")),
        tag_name: string(release.get("tag_name")),
        html_url: string(release.get("html_url")),
    }
}

fn string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(str::to_string)
}

fn number(value: Option<&Value>) -> Option<u64> {
    value.and_then(Value::as_u64)
}

fn boolean(value: Option<&Value>) -> Option<bool> {
    value.and_then(Value::as_bool)
}

fn string_at(map: &serde_json::Map<String, Value>, path: &[&str]) -> Option<String> {
    let mut current: &Value = map.get(path.first().copied()?)?;
    for key in path.iter().skip(1) {
        current = current.as_object()?.get(*key)?;
    }
    current.as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_issue_comment_payload() {
        let payload = json!({
            "issue": {
                "number": 42,
                "title": "Fix docs",
                "body": "Please update docs",
                "author_association": "CONTRIBUTOR",
                "user": { "login": "alice" },
                "state": "open"
            },
            "comment": {
                "body": "/triage",
                "id": "c1",
                "path": "README.md",
                "line": 17
            },
            "repository": { "default_branch": "main" }
        });

        let ctx = parse_github_event_context("issues", "neverhuman/jekko", Some("alice"), &payload)
            .unwrap();
        assert_eq!(ctx.issue.number, Some(42));
        assert_eq!(ctx.field("target.number").as_deref(), Some("42"));
        assert_eq!(ctx.field("target.title").as_deref(), Some("Fix docs"));
        assert_eq!(ctx.field("comment.body").as_deref(), Some("/triage"));
        assert_eq!(
            ctx.field("repository.default_branch").as_deref(),
            Some("main")
        );
        assert_eq!(ctx.field("issue.created_at"), None);
    }

    #[test]
    fn parses_pull_request_payload() {
        let payload = json!({
            "pull_request": {
                "number": 7,
                "title": "Add feature",
                "body": "Body",
                "author_association": "MEMBER",
                "user": { "login": "bob" },
                "state": "open",
                "draft": false,
                "merged": false,
                "created_at": "2026-01-01T00:00:00Z",
                "head": { "ref": "feature" },
                "base": { "ref": "main" }
            },
            "repository": { "default_branch": "main" }
        });

        let ctx =
            parse_github_event_context("pull_request", "neverhuman/jekko", Some("bob"), &payload)
                .unwrap();
        assert_eq!(ctx.pull_request.number, Some(7));
        assert_eq!(ctx.field("issue.number"), None);
        assert_eq!(ctx.field("target.number").as_deref(), Some("7"));
        assert_eq!(
            ctx.field("pull_request.author.login").as_deref(),
            Some("bob")
        );
        assert_eq!(
            ctx.field("pull_request.head_ref").as_deref(),
            Some("feature")
        );
        assert_eq!(
            ctx.field("pull_request.created_at").as_deref(),
            Some("2026-01-01T00:00:00Z")
        );
    }

    #[test]
    fn rejects_invalid_repository() {
        let payload = json!({});
        let err = parse_github_event_context("issues", "invalid", None, &payload).unwrap_err();
        assert_eq!(err, GitHubEventError::InvalidRepository);
    }
}
