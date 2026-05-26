#!/usr/bin/env bash
# Lane 1/3 of the parity:rust-parity-gates split. Runs ONLY the formatter +
# clippy checks. Fast (~70-90s). Pair with parity-test.sh + parity-build-gates.sh
# for full parity coverage.
set -euo pipefail

export CARGO_TERM_COLOR=always
export RUSTFLAGS='-D warnings'

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
