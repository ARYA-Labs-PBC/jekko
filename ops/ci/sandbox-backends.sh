#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

if [ -n "${INSTALL_CMD:-}" ] && [ "${INSTALL_CMD}" != "true" ]; then
  bash -lc "$INSTALL_CMD"
fi

cargo build --manifest-path crates/sandboxctl/Cargo.toml --locked
cargo run --manifest-path crates/sandboxctl/Cargo.toml --locked --quiet -- validate
cargo test --manifest-path crates/sandboxctl/Cargo.toml --locked --tests --no-fail-fast
