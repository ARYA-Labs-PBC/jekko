#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

git config --global user.email "bot@jekko.ai"
git config --global user.name "jekko"
rtk cargo build -p jekko-cli --locked
rtk cargo run -p jekko-cli -- --version
rtk cargo test -p jekko-tui --locked --no-fail-fast
JEKKO_BIN="$(rtk cargo run -p xtask -- host-binary-path)" cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml default_tui_paints_first_frame -- --nocapture
JEKKO_BIN="$(rtk cargo run -p xtask -- host-binary-path)" cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --no-run
