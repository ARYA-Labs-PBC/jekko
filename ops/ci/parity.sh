#!/usr/bin/env bash
set -euo pipefail

export CARGO_TERM_COLOR=always
export RUSTFLAGS='-D warnings'

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked --no-fail-fast
cargo build -p jekko-cli --release --locked
cargo run -p xtask --locked -- db-migration-smoke
cargo run -p xtask --locked -- cli-help-parity --strict
cargo run -p xtask --locked -- tool-schema-parity --strict
cargo run -p xtask --locked -- session-fixture-parity --strict
cargo run -p xtask --locked -- openapi-check --strict
cargo run -p xtask --locked -- httpapi-parity
cargo run -p xtask --locked -- baseline-diff --threshold 80
