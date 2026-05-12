#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

bun i -g jekko-ai
issue_number="$(jq -r '.issue.number' "$GITHUB_EVENT_PATH")"
issue_title="$(jq -r '.issue.title' "$GITHUB_EVENT_PATH")"
issue_body="$(jq -r '.issue.body // ""' "$GITHUB_EVENT_PATH")"

jekko run --agent triage "The following issue was just opened, triage it:

Title: ${issue_title}

${issue_body}"
