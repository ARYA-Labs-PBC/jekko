#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

EVENT="$(jq -r '.pull_request.number' "$GITHUB_EVENT_PATH")"
LOGIN="$(jq -r '.pull_request.user.login' "$GITHUB_EVENT_PATH")"
BODY="$(jq -r '.pull_request.body // ""' "$GITHUB_EVENT_PATH")"
TITLE="$(jq -r '.pull_request.title' "$GITHUB_EVENT_PATH")"
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

issues=()
has_what() { [[ "$BODY" =~ "### What does this PR do?" ]]; }
has_type() { [[ "$BODY" =~ "### Type of change" ]]; }
has_verify() { [[ "$BODY" =~ "### How did you verify your code works?" ]]; }
has_checklist() { [[ "$BODY" =~ "### Checklist" ]]; }
has_issue() { [[ "$BODY" =~ "### Issue for this PR" ]]; }

if ! has_what || ! has_type || ! has_verify || ! has_checklist || ! has_issue; then
  issues+=("PR description is missing required template sections. Please use the [PR template](../blob/${DEFAULT_BRANCH}/.github/pull_request_template.md).")
fi

if has_what; then
  what_block="$(printf '%s\n' "$BODY" | awk '/### What does this PR do\?/{flag=1; next} /^### /{flag=0} flag')"
  placeholder='Please provide a description of the issue'
  if [[ -z "${what_block//[[:space:]]/}" || ( "$what_block" == *"$placeholder"* && "${what_block//$placeholder/}" =~ ^[[:space:]\*]*$ ) ]]; then
    issues+=('"What does this PR do?" section is empty or only contains placeholder text. Please describe your changes.')
  fi
fi

if has_type; then
  type_block="$(printf '%s\n' "$BODY" | awk '/### Type of change/{flag=1; next} /^### /{flag=0} flag')"
  if ! printf '%s' "$type_block" | grep -qi '\- \[x\]'; then
    issues+=('No "Type of change" checkbox is checked. Please select at least one.')
  fi
fi

if ! [[ "$TITLE" =~ ^(docs|refactor|feat)\s*(\([a-zA-Z0-9-]+\))?\s*: ]]; then
  if has_issue; then
    issue_block="$(printf '%s\n' "$BODY" | awk '/### Issue for this PR/{flag=1; next} /^### /{flag=0} flag')"
    if ! printf '%s' "$issue_block" | grep -Eq '(closes|fixes|resolves)\s+#\d+|#\d+'; then
      issues+=('No issue referenced. Please add `Closes #<number>` linking to the relevant issue.')
    fi
  fi
fi

if has_verify; then
  verify_block="$(printf '%s\n' "$BODY" | awk '/### How did you verify your code works\?/{flag=1; next} /^### /{flag=0} flag')"
  if [ -z "${verify_block//[[:space:]]/}" ]; then
    issues+=('"How did you verify your code works?" section is empty. Please explain how you tested.')
  fi
fi

if has_checklist; then
  checklist_block="$(printf '%s\n' "$BODY" | awk '/### Checklist/{flag=1; next} /^### /{flag=0} flag')"
  if ! printf '%s' "$checklist_block" | grep -qi '\- \[x\]'; then
    issues+=('At least one checklist item must be checked.')
  fi
fi

if [ "${#issues[@]}" -gt 0 ]; then
  body="PR review found the following issues:\n\n"
  for issue in "${issues[@]}"; do body+="- ${issue}\n"; done
  body+="\nSee [CONTRIBUTING.md](../blob/${DEFAULT_BRANCH}/CONTRIBUTING.md#pr-titles) and [.github/pull_request_template.md](../blob/${DEFAULT_BRANCH}/.github/pull_request_template.md)."
  gh api --method POST "/repos/${GITHUB_REPOSITORY}/issues/${EVENT}/comments" -f body="$body" >/dev/null
  gh api --method POST "/repos/${GITHUB_REPOSITORY}/issues/${EVENT}/labels" -f labels='["needs:issue"]' >/dev/null 2>&1 || true
else
  gh api --method DELETE "/repos/${GITHUB_REPOSITORY}/issues/${EVENT}/labels/needs:issue" >/dev/null 2>&1 || true
fi
