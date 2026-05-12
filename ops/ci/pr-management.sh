#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

LOGIN="$(jq -r '.pull_request.user.login' "$GITHUB_EVENT_PATH")"
if [ "$LOGIN" = "jekko-agent[bot]" ] || grep -qxF "$LOGIN" .github/TEAM_MEMBERS; then
  printf '%s\n' "Skipping: $LOGIN is a team member or bot"
  exit 0
fi

bun install
bun i -g jekko-ai

PR_NUMBER="$(jq -r '.pull_request.number' "$GITHUB_EVENT_PATH")"
{
  echo "Check for duplicate PRs related to this new PR:"
  echo
  echo "CURRENT_PR_NUMBER: $PR_NUMBER"
  echo
  echo "Title: $(gh pr view "$PR_NUMBER" --json title --jq .title)"
  echo
  echo "Description:"
  gh pr view "$PR_NUMBER" --json body --jq .body
} > pr_info.txt

COMMENT="$(bun script/duplicate-pr.ts -f pr_info.txt "Check the attached file for PR details and search for duplicates")"
if [ "$COMMENT" != "No duplicate PRs found" ]; then
  gh pr comment "$PR_NUMBER" --body "_The following comment was made by an LLM, it may be inaccurate:_

$COMMENT"
fi
