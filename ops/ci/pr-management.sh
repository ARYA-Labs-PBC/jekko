#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

LOGIN="$(rtk cargo run -p xtask -- github-event target.author.login)"
if [ "$LOGIN" = "jekko-agent[bot]" ] || grep -qxF "$LOGIN" .github/TEAM_MEMBERS; then
  printf '%s\n' "Skipping: $LOGIN is a team member or bot"
  exit 0
fi

rtk cargo run -p xtask -- pr-management
