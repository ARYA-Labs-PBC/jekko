#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

cp target/jankurai/repo-score.json agent/repo-score.json
cp target/jankurai/repo-score.md agent/repo-score.md
