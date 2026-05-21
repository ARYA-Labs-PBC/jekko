#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

if [ -z "${GITHUB_EVENT_PATH:-}" ]; then
  echo "pr-standards: no GITHUB_EVENT_PATH; local dry-run skipped"
  exit 0
fi

cargo run -p xtask -- pr-standards
