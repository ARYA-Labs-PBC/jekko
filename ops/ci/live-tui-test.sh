#!/usr/bin/env bash
# ops/ci/live-tui-test.sh
#
# LOCAL-ONLY live TUI connectivity + chat test.
# Uses REAL API keys to prove:
#   1. Jnoccio server connectivity (model count > 0)
#   2. TUIwright chat round-trip (prompt → streaming response visible in TUI)
#
# NEVER run in CI — guarded by JEKKO_TUI_LIVE_PROD=1 opt-in.
#
# Usage:
#   chmod +x ops/ci/live-tui-test.sh
#   ./ops/ci/live-tui-test.sh
#
# Or via Justfile:
#   just live-tui-test

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# ── Opt-in guard ───────────────────────────────────────────────────────
if [[ "${JEKKO_TUI_LIVE_PROD:-0}" != "1" ]]; then
  echo "ERROR: set JEKKO_TUI_LIVE_PROD=1 to run live TUI tests." >&2
  echo "       These tests use real API keys and make real network calls." >&2
  exit 1
fi

if [[ "${CI:-false}" == "true" ]]; then
  echo "ERROR: live TUI tests must NOT run in CI." >&2
  exit 1
fi

# ── Resolve the jekko binary ───────────────────────────────────────────
JEKKO_BIN="${JEKKO_BIN:-/opt/homebrew/bin/jekko}"
if [[ ! -f "$JEKKO_BIN" ]]; then
  echo "ERROR: JEKKO_BIN=$JEKKO_BIN does not exist." >&2
  echo "       Build it first: cd packages/jekko && bun run build" >&2
  exit 1
fi

# ── Resolve API key ────────────────────────────────────────────────────
if [[ -z "${JEKKO_API_KEY:-}" ]]; then
  ENV_FILE="$REPO_ROOT/.env.jnoccio"
  if [[ -f "$ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    export JEKKO_API_KEY="$(grep -E '^JEKKO_API_KEY=' "$ENV_FILE" | cut -d= -f2- | tr -d '"'"'")"
  fi
fi

if [[ -z "${JEKKO_API_KEY:-}" ]]; then
  echo "ERROR: JEKKO_API_KEY is not set and .env.jnoccio has no JEKKO_API_KEY entry." >&2
  exit 1
fi

# ── Model selection ────────────────────────────────────────────────────
JEKKO_LIVE_MODEL="${JEKKO_LIVE_MODEL:-anthropic/claude-3-5-haiku}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Jekko LIVE TUI Test"
echo "  Binary : $JEKKO_BIN"
echo "  Model  : $JEKKO_LIVE_MODEL"
echo "  API key: ${JEKKO_API_KEY:0:8}…"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

cd "$REPO_ROOT"

# ── Run tests ──────────────────────────────────────────────────────────
# 1. Live prompt round-trip through TUI
JEKKO_TUI_LIVE_PROD=1 \
JEKKO_BIN="$JEKKO_BIN" \
JEKKO_API_KEY="$JEKKO_API_KEY" \
JEKKO_LIVE_MODEL="$JEKKO_LIVE_MODEL" \
  cargo test \
    -p tuiwright-jekko-unlock \
    --test live_prod_tui \
    -- \
    --ignored \
    --nocapture \
    2>&1

# 2. Jnoccio connectivity tests (with mock server + real binary)
JEKKO_BIN="$JEKKO_BIN" \
JNOCCIO_TUI_TEST=1 \
  cargo test \
    -p tuiwright-jekko-unlock \
    --test jnoccio_tui_dashboard \
    -- \
    --ignored \
    --nocapture \
    2>&1

# 3. Chat enter tests
JEKKO_BIN="$JEKKO_BIN" \
TUI_CHAT_TEST=1 \
  cargo test \
    -p tuiwright-jekko-unlock \
    --test tui_chat_enter_mock \
    -- \
    --ignored \
    --nocapture \
    2>&1

echo ""
echo "✅ All live TUI tests passed."
echo "   Screenshots saved to: target/tuiwright-jekko/"
