#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
export ZYAL_RUN="${ZYAL_RUN:-1}"
export JEKKO_AUTO_ALLOW_READS="${JEKKO_AUTO_ALLOW_READS:-1}"

source "$SCRIPT_DIR/jekko-jankurai-loop-common.sh"
source "$SCRIPT_DIR/jekko-jankurai-loop-runner.sh"

PROMPT_FILE="${PROMPT_FILE:-./prompt.md}"
REPO_DIR="${REPO_DIR:-$PWD}"
LOG_DIR="${LOG_DIR:-$HOME/.jekko-loop-runs}"
SLEEP_SECONDS="${SLEEP_SECONDS:-5}"
MAX_RUNS="${MAX_RUNS:-0}"
MODEL="${MODEL:-jnoccio/jnoccio-fusion}"
VARIANT="${VARIANT:-}"
AGENT="${AGENT:-}"
JEKKO_BIN="${JEKKO_BIN:-jekko}"
JSON_EVENTS="${JSON_EVENTS:-0}"
AUTO_UNLOCK="${AUTO_UNLOCK:-1}"
REQUIRE_UNLOCK="${REQUIRE_UNLOCK:-1}"
STOP_ON_FAILURE="${STOP_ON_FAILURE:-0}"
SKIP_PERMISSIONS="${SKIP_PERMISSIONS:-0}"
LOOP_COLOR="${LOOP_COLOR:-auto}"

command -v "$JEKKO_BIN" >/dev/null 2>&1 || fatal "jekko CLI not found on PATH. Install or build it first."
[[ -f "$PROMPT_FILE" ]] || fatal "prompt file not found: $PROMPT_FILE"
[[ -d "$REPO_DIR" ]] || fatal "repo dir not found: $REPO_DIR"

case "$MAX_RUNS" in
  ''|*[!0-9]*) fatal "MAX_RUNS must be a non-negative integer" ;;
esac

case "$SLEEP_SECONDS" in
  ''|*[!0-9]*) fatal "SLEEP_SECONDS must be a non-negative integer" ;;
esac

case "$JSON_EVENTS" in
  0|1) ;;
  *) fatal "JSON_EVENTS must be 0 or 1" ;;
esac

case "$AUTO_UNLOCK" in
  0|1) ;;
  *) fatal "AUTO_UNLOCK must be 0 or 1" ;;
esac

case "$REQUIRE_UNLOCK" in
  0|1) ;;
  *) fatal "REQUIRE_UNLOCK must be 0 or 1" ;;
esac

case "$STOP_ON_FAILURE" in
  0|1) ;;
  *) fatal "STOP_ON_FAILURE must be 0 or 1" ;;
esac

case "$SKIP_PERMISSIONS" in
  0|1) ;;
  *) fatal "SKIP_PERMISSIONS must be 0 or 1" ;;
esac

case "$LOOP_COLOR" in
  auto|always|never) ;;
  *) fatal "LOOP_COLOR must be auto, always, or never" ;;
esac

PROMPT_FILE="$(resolve_path "$PROMPT_FILE")"
REPO_DIR="$(resolve_path "$REPO_DIR")"
mkdir -p "$LOG_DIR"
LOG_DIR="$(resolve_path "$LOG_DIR")"

palette=(196 202 208 214 201 45)
palette_len="${#palette[@]}"
palette_offset=$((RANDOM % palette_len))

if [[ -n "${NO_COLOR:-}" || "$LOOP_COLOR" == "never" ]]; then
  color_enabled=0
elif [[ "$LOOP_COLOR" == "always" || -t 1 ]]; then
  color_enabled=1
else
  color_enabled=0
fi

trap 'echo; echo "Stopping jekko loop."; exit 130' INT TERM

run_loop
