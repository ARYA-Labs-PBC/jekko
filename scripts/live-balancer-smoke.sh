#!/usr/bin/env bash
# LOCAL ONLY. Live smoke for the multi-user key balancer.
#
# Builds + installs jekko, then runs the gated tuiwright test which spawns
# `jekko run` repeatedly and proves every provider-specific candidate key in
# the active `~/.jekko/users/<user>/llm.env` tree is selected at least once.
#
# Refuses to run in CI.
#
# Usage:
#   scripts/live-balancer-smoke.sh                       # default 6 runs, openrouter
#   JEKKO_LIVE_BALANCER_COUNT=20 scripts/live-balancer-smoke.sh
#   JEKKO_LIVE_BALANCER_PROVIDER=groq \
#     JEKKO_LIVE_BALANCER_MODEL=groq-qwen3-32b \
#     scripts/live-balancer-smoke.sh

set -euo pipefail

if [[ "${CI:-}" == "true" ]]; then
  echo "scripts/live-balancer-smoke.sh: refusing to run in CI" >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

JEKKO_BIN="${JEKKO_BIN:-/opt/homebrew/bin/jekko}"

echo "==> build release jekko"
rtk cargo build --release -p jekko-cli

echo "==> install to ${JEKKO_BIN}"
install -m 0755 target/release/jekko "$JEKKO_BIN"

echo "==> ad-hoc sign (macOS)"
if command -v codesign >/dev/null 2>&1; then
  codesign --force --sign - "$JEKKO_BIN" || true
fi

echo "==> version check"
"$JEKKO_BIN" --version

echo "==> run live balancer test"
JEKKO_LIVE_BALANCER=1 \
JEKKO_BIN="$JEKKO_BIN" \
rtk cargo test \
  -p tuiwright-jekko-unlock \
  --test live_balancer \
  balancer_proves_all_provider_candidates \
  -- --ignored --exact --nocapture
