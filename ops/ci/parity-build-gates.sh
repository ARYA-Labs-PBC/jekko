#!/usr/bin/env bash
# Lane 3/3 of the parity:rust-parity-gates split. Runs the release build
# of jekko-cli + the 7 xtask parity gates. Medium duration (~180s) since
# baseline-diff exercises the built binary. Pair with parity-fmt-clippy.sh
# + parity-test.sh for full parity coverage.
set -euo pipefail

export CARGO_TERM_COLOR=always
export RUSTFLAGS='-D warnings'

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

cargo build -p jekko-cli --release --locked
cargo run -p xtask --locked -- db-migration-smoke
cargo run -p xtask --locked -- cli-help-parity --strict
cargo run -p xtask --locked -- tool-schema-parity --strict
cargo run -p xtask --locked -- session-fixture-parity --strict
cargo run -p xtask --locked -- openapi-check --strict
cargo run -p xtask --locked -- httpapi-parity
cargo run -p xtask --locked -- baseline-diff --threshold 80
