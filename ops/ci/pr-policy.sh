#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
source ops/ci/lib.sh

if [ "$#" -ne 1 ]; then
  fail "usage: $0 {standards|compliance}"
fi

policy="$1"
case "$policy" in
  standards|compliance)
    ;;
  *)
    fail "usage: $0 {standards|compliance}"
    ;;
esac

require_pr_policy_contract
note "pr-policy wrapper: dispatching $policy"
exec bash "$ROOT/ops/ci/pr-${policy}.sh"
