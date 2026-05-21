#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

git config --global user.email "bot@jekko.ai"
git config --global user.name "jekko"
cargo test --workspace --locked --no-fail-fast
