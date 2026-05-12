#!/usr/bin/env bash
# Local CI entrypoint. Keep the lane order in one place so the local
# equivalent and GitHub workflows stay aligned.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

LANE="${1:-all}"

case "$LANE" in
  quick) just fast ;;
  audit) just ci-local-audit ;;
  all) just ci-local ;;
  doctor) bash scripts/ci-doctor.sh ;;
  *)
    printf 'usage: %s {quick|audit|all|doctor}\n' "$0" >&2
    exit 2
    ;;
esac
