#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

if command -v rtk >/dev/null 2>&1; then
  rtk cargo check --workspace --locked
else
  cargo check --workspace --locked
fi
