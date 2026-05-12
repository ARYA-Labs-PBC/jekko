#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

git config --global user.email "bot@jekko.ai"
git config --global user.name "jekko"
bun --cwd packages/jekko ./script/build.ts --single --skip-install
bun --cwd packages/jekko ./script/tui-binary-smoke.ts
bun --cwd packages/jekko test test/cli/tui/ test/cli/cmd/tui/
JEKKO_BIN="$(bun --cwd packages/jekko ./script/host-binary-path.ts)" cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --no-run
JEKKO_BIN="$(bun --cwd packages/jekko ./script/host-binary-path.ts)" cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --no-fail-fast
