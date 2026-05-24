#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

source ops/ci/lib.sh

cleanup() {
  rm -f agent/repo-score.json agent/repo-score.md
}
trap cleanup EXIT

# Refresh the committed README badge artifacts from the public score report.
jankurai audit . --mode ratchet --full --baseline agent/baselines/main.repo-score.json --json agent/repo-score.json --md agent/repo-score.md --no-score-history
test -s agent/jankurai-badge.svg
test -s agent/jankurai-badge.json
git diff --exit-code -- README.md agent/jankurai-badge.svg agent/jankurai-badge.json
