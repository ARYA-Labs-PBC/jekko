#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

EVENT="$(jq -r '.pull_request.number' "$GITHUB_EVENT_PATH")"
LOGIN="$(jq -r '.pull_request.user.login' "$GITHUB_EVENT_PATH")"
TITLE="$(jq -r '.pull_request.title' "$GITHUB_EVENT_PATH")"
BODY="$(jq -r '.pull_request.body // ""' "$GITHUB_EVENT_PATH")"
CREATED="$(jq -r '.pull_request.created_at' "$GITHUB_EVENT_PATH")"
DEFAULT_BRANCH="$(jq -r '.repository.default_branch // "main"' "$GITHUB_EVENT_PATH")"

cutoff="2026-02-19T00:00:00Z"
if [[ "$CREATED" < "$cutoff" ]]; then
  exit 0
fi
if [ "$LOGIN" = "jekko-agent[bot]" ]; then
  exit 0
fi
if grep -qxF "$LOGIN" .github/TEAM_MEMBERS; then
  exit 0
fi

title_pattern='^(feat|fix|docs|chore|refactor|test)\s*(\([a-zA-Z0-9-]+\))?\s*:'
if ! [[ "$TITLE" =~ $title_pattern ]]; then
  gh api --method POST "/repos/${GITHUB_REPOSITORY}/issues/${EVENT}/labels" -f labels='["needs:title"]' >/dev/null
  gh api --method POST "/repos/${GITHUB_REPOSITORY}/issues/${EVENT}/comments" -f body="Hey! Your PR title \`${TITLE}\` doesn't follow conventional commit format.

Please update it to start with one of:
- \`feat:\` or \`feat(scope):\` new feature
- \`fix:\` or \`fix(scope):\` bug fix
- \`docs:\` or \`docs(scope):\` documentation changes
- \`chore:\` or \`chore(scope):\` maintenance tasks
- \`refactor:\` or \`refactor(scope):\` code refactoring
- \`test:\` or \`test(scope):\` adding or updating tests

Where \`scope\` is the package name (e.g., \`app\`, \`jekko\`).

See [CONTRIBUTING.md](../blob/${DEFAULT_BRANCH}/CONTRIBUTING.md#pr-titles) for details." >/dev/null
  exit 0
fi

gh api --method DELETE "/repos/${GITHUB_REPOSITORY}/issues/${EVENT}/labels/needs:title" >/dev/null 2>&1 || true

if [[ "$TITLE" =~ ^(docs|refactor|feat)\s*(\([a-zA-Z0-9-]+\))?\s*: ]]; then
  gh api --method DELETE "/repos/${GITHUB_REPOSITORY}/issues/${EVENT}/labels/needs:issue" >/dev/null 2>&1 || true
  exit 0
fi

ISSUE_COUNT="$(gh api graphql -f owner="${GITHUB_REPOSITORY%/*}" -f repo="${GITHUB_REPOSITORY#*/}" -F number="$EVENT" -f query='query($owner:String!, $repo:String!, $number:Int!) { repository(owner:$owner, name:$repo) { pullRequest(number:$number) { closingIssuesReferences(first: 1) { totalCount } } } }' --jq '.data.repository.pullRequest.closingIssuesReferences.totalCount')"
if [ "$ISSUE_COUNT" = "0" ]; then
  gh api --method POST "/repos/${GITHUB_REPOSITORY}/issues/${EVENT}/labels" -f labels='["needs:issue"]' >/dev/null
  gh api --method POST "/repos/${GITHUB_REPOSITORY}/issues/${EVENT}/comments" -f body="Thanks for your contribution!

This PR doesn't have a linked issue. All PRs must reference an existing issue.

Please:
1. Open an issue describing the bug/feature (if one doesn't exist)
2. Add \`Fixes #<number>\` or \`Closes #<number>\` to this PR description

See [CONTRIBUTING.md](../blob/${DEFAULT_BRANCH}/CONTRIBUTING.md#issue-first-policy) for details." >/dev/null
  exit 0
fi

gh api --method DELETE "/repos/${GITHUB_REPOSITORY}/issues/${EVENT}/labels/needs:issue" >/dev/null 2>&1 || true
