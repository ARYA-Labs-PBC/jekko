#!/usr/bin/env bash
# Lane 2/3 of the parity:rust-parity-gates split. Runs ONLY cargo test
# --workspace. Long-tail (~250s). Pair with parity-fmt-clippy.sh +
# parity-build-gates.sh for full parity coverage.
set -euo pipefail

export CARGO_TERM_COLOR=always
export RUSTFLAGS='-D warnings'

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

cargo test --workspace --locked --no-fail-fast
