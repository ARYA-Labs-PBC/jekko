#!/usr/bin/env bash
# LOCAL ONLY. Live smoke for the multi-user key balancer.
#
# Builds + installs jekko, then runs the gated tuiwright test which spawns
# `jekko run` N times and asserts both user_1 and user_2 keys were picked
# by jekko_runtime::key_balancer::KeyBalancer.
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
USER_1_ENV="${HOME}/.jekko/users/user_1/llm.env"
USER_2_ENV="${HOME}/.jekko/users/user_2/llm.env"

for path in "$USER_1_ENV" "$USER_2_ENV"; do
  if [[ ! -f "$path" ]]; then
    echo "missing $path — populate both user_1 and user_2 llm.env first" >&2
    exit 1
  fi
done

echo "==> build release jekko"
cargo build --release -p jekko-cli

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
cargo test \
  -p tuiwright-jekko-unlock \
  --test live_balancer \
  balancer_distributes_across_user_1_and_user_2 \
  -- --ignored --nocapture
