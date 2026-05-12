#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

: "${MODEL:?MODEL must be set to provider/model}"
bun --cwd packages/jekko src/index.ts github run
