#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ISSUE_NUMBER="$(jq -r 'if .pull_request then .pull_request.number else .issue.number end' "$GITHUB_EVENT_PATH")"
A_ASSOC="$(jq -r 'if .pull_request then .pull_request.author_association else .issue.author_association end' "$GITHUB_EVENT_PATH")"
if [ "$A_ASSOC" = "CONTRIBUTOR" ]; then
  gh api --method POST "/repos/${GITHUB_REPOSITORY}/issues/${ISSUE_NUMBER}/labels" -f labels='["contributor"]' >/dev/null
fi
