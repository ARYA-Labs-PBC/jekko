#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

SYSTEM="${SYSTEM:?SYSTEM must be set}"
set -euo pipefail
BUILD_LOG=$(mktemp)
trap 'rm -f "$BUILD_LOG"' EXIT
nix build ".#packages.${SYSTEM}.node_modules_updater" --no-link 2>&1 | tee "$BUILD_LOG" || true
HASH="$(nix run --inputs-from . nixpkgs#gnugrep -- -oP 'got:\s*\Ksha256-[A-Za-z0-9+/=]+' "$BUILD_LOG" | tail -n1 || true)"
if [ -z "$HASH" ]; then
  echo "::error::Failed to compute hash for ${SYSTEM}"
  cat "$BUILD_LOG"
  exit 1
fi
echo "$HASH" > hash.txt
echo "Computed hash for ${SYSTEM}: $HASH"
