#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
source ops/ci/lib.sh

resolve_github_repository

if [ -z "${GITHUB_EVENT_PATH:-}" ]; then
  if [ "${JEKKO_PR_DRY_RUN:-}" != "1" ]; then
    echo "pr-compliance: no GITHUB_EVENT_PATH; local dry-run skipped"
    exit 0
  fi
  GITHUB_EVENT_PATH="$(pull_request_target_json)"
  export GITHUB_EVENT_PATH
  export GITHUB_EVENT_NAME="pull_request_target"
  trap 'rm -f "$GITHUB_EVENT_PATH"' EXIT
fi

cargo run -p xtask -- pr-compliance
