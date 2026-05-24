#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

source ops/ci/lib.sh

test -s "${JANKURAI_ARTIFACT_ROOT}/repo-score.json"
test -s "${JANKURAI_ARTIFACT_ROOT}/repo-score.md"
test -s "agent/jankurai-badge.svg"
test -s "agent/jankurai-badge.json"
