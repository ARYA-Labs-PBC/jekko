#!/usr/bin/env bash
# LOCAL ONLY. Orchestrates the attended TUI battle matrix through the built
# jekko binary and tuiwright PTY tests.

set -u -o pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

ARTIFACT_ROOT="${JEKKO_TUI_BATTLE_ARTIFACT_DIR:-target/tuiwright-jekko/battle}"
RECEIPT_DIR="$ARTIFACT_ROOT/receipts"
COMMAND_LOG_DIR="$ARTIFACT_ROOT/command-logs"
mkdir -p "$RECEIPT_DIR" "$COMMAND_LOG_DIR"

fail() {
  printf 'tui-battle-local: %s\n' "$*" >&2
  exit 1
}

require_preflight() {
  [[ "${CI:-}" != "true" ]] || fail "refusing to run in CI"
  [[ "${JEKKO_TUI_BATTLE:-}" = "1" ]] || fail "set JEKKO_TUI_BATTLE=1"
  command -v rtk >/dev/null 2>&1 || fail "missing rtk"
  command -v jq >/dev/null 2>&1 || fail "missing jq"
  command -v cargo >/dev/null 2>&1 || fail "missing cargo"

  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/codex-plan}"
  export JEKKO_TUI_ARTIFACT_DIR="$ARTIFACT_ROOT"
  export JEKKO_LIVE_MODEL="${JEKKO_LIVE_MODEL:-jekko/gpt-5-nano}"
  export JEKKO_CHAT_MODEL="${JEKKO_CHAT_MODEL:-$JEKKO_LIVE_MODEL}"

  if [[ -z "${JEKKO_BIN:-}" ]]; then
    printf '==> building host jekko binary\n' >&2
    rtk cargo build -p jekko-cli --locked
    JEKKO_BIN="$(rtk cargo run -p xtask -- host-binary-path)"
    export JEKKO_BIN
  fi
  [[ -x "$JEKKO_BIN" ]] || fail "JEKKO_BIN missing or non-executable: $JEKKO_BIN"
}

write_receipt() {
  local id="$1"
  local label="$2"
  local command="$3"
  local status="$4"
  local exit_code="$5"
  local started_at="$6"
  local finished_at="$7"
  local duration_s="$8"
  local log_path="$9"
  jq -n \
    --arg schema "jekko.tui_battle.receipt.v1" \
    --arg id "$id" \
    --arg label "$label" \
    --arg command "$command" \
    --arg status "$status" \
    --arg started_at "$started_at" \
    --arg finished_at "$finished_at" \
    --arg log_path "$log_path" \
    --arg artifact_root "$ARTIFACT_ROOT" \
    --arg jekko_bin "$JEKKO_BIN" \
    --arg model "$JEKKO_CHAT_MODEL" \
    --argjson exit_code "$exit_code" \
    --argjson duration_s "$duration_s" \
    '{schema:$schema,id:$id,label:$label,command:$command,status:$status,exit_code:$exit_code,started_at:$started_at,finished_at:$finished_at,duration_s:$duration_s,log_path:$log_path,artifact_root:$artifact_root,jekko_bin:$jekko_bin,model:$model}' \
    > "$RECEIPT_DIR/${id}.json"
}

run_step() {
  local id="$1"
  local label="$2"
  local command="$3"
  local log_path="$COMMAND_LOG_DIR/${id}.log"
  local started_at finished_at start_epoch finish_epoch duration_s exit_code status

  printf '\n==> [%s] %s\n' "$id" "$label" >&2
  printf 'cmd: %s\n' "$command" >&2
  started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  start_epoch="$(date -u +%s)"

  bash -lc "$command" >"$log_path" 2>&1
  exit_code=$?

  finished_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  finish_epoch="$(date -u +%s)"
  duration_s=$((finish_epoch - start_epoch))
  if [[ "$exit_code" -eq 0 ]]; then
    status="passed"
  else
    status="failed"
  fi
  write_receipt "$id" "$label" "$command" "$status" "$exit_code" "$started_at" "$finished_at" "$duration_s" "$log_path"

  if [[ "$exit_code" -ne 0 ]]; then
    printf '\nFAILED [%s] %s (exit %s)\n' "$id" "$label" "$exit_code" >&2
    printf 'receipt: %s\n' "$RECEIPT_DIR/${id}.json" >&2
    printf 'log: %s\n\n' "$log_path" >&2
    tail -80 "$log_path" >&2 || true
    exit "$exit_code"
  fi
}

require_live_env() {
  [[ "${JEKKO_TUI_BATTLE_LIVE:-}" = "1" ]] || return 1
  [[ -n "${JEKKO_API_KEY:-}" ]] || fail "JEKKO_TUI_BATTLE_LIVE=1 requires JEKKO_API_KEY"
  return 0
}

require_preflight

printf 'artifact root: %s\n' "$ARTIFACT_ROOT" >&2
printf 'JEKKO_BIN: %s\n' "$JEKKO_BIN" >&2
printf 'JEKKO_CHAT_MODEL: %s\n' "$JEKKO_CHAT_MODEL" >&2

run_step "01-startup-smoke" \
  "Boot first-frame smoke" \
  "rtk just tui-startup-smoke"

run_step "02-tui-ci" \
  "Full CI-safe TUI lane" \
  "rtk just tui-ci"

run_step "03-tuiwright-local-full" \
  "Full local tuiwright lane" \
  "rtk just tuiwright-local-full"

run_step "04-render-matrix" \
  "Render matrix capture and diff" \
  "JEKKO_TUI_CAPTURE=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test baseline_matrix -- --test-threads=1 --nocapture && JEKKO_RUST_MATRIX=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test rust_baseline_matrix -- --test-threads=1 --nocapture && rtk cargo run -p xtask -- baseline-diff --baseline \"\$JEKKO_TUI_ARTIFACT_DIR/baseline\" --rust \"\$JEKKO_TUI_ARTIFACT_DIR/rust\" --threshold \"\${JEKKO_TUI_BATTLE_DIFF_THRESHOLD:-80}\""

run_step "05-command-palette" \
  "Command palette UX" \
  "JEKKO_RUST_MATRIX=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test rust_dialog_keys rust_command_palette_filter_then_close -- --exact --nocapture"

run_step "06-model-provider-theme" \
  "Model/provider/theme dialogs" \
  "JEKKO_RUST_MATRIX=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test rust_dialog_keys -- --nocapture"

run_step "07-slash-popup" \
  "Slash popup basics" \
  "JEKKO_RUST_MATRIX=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test rust_slash_popup -- --nocapture"

run_step "08-slash-actions" \
  "Slash command action notices" \
  "JEKKO_TUI_BATTLE=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test tui_battle_local slash_command_action_notices_render -- --exact --nocapture"

run_step "09-background-commands" \
  "Background command UX" \
  "JEKKO_TUI_BATTLE=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test tui_battle_local background_command_lifecycle_is_visible -- --exact --nocapture"

run_step "10-prompt-editing" \
  "Prompt editing" \
  "JEKKO_TUI_BATTLE=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test tui_battle_local prompt_editing_keystrokes_are_not_swallowed -- --exact --nocapture"

run_step "11-13-chat-enter" \
  "Enter no-provider, configured path, and mock assistant path" \
  "TUI_CHAT_TEST=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test tui_chat_enter_mock -- --ignored --nocapture"

run_step "14-scroll-resize" \
  "Scroll and resize" \
  "JEKKO_TUI_BATTLE=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test tui_battle_local scroll_and_resize_long_transcript_stays_healthy -- --exact --nocapture"

run_step "15-paste-handling" \
  "Paste handling" \
  "JEKKO_TUI_BATTLE=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test tui_battle_local paste_handling_collapses_and_expands_large_paste -- --exact --nocapture"

run_step "16-zyal-home-paste" \
  "ZYAL home paste" \
  "JEKKO_TUI_BATTLE=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test tui_battle_local zyal_home_paste_shows_indicator -- --exact --nocapture"

run_step "17-zyal-session-paste" \
  "ZYAL session paste" \
  "JEKKO_TUIWRIGHT_PTY=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test zyal_session_paste pasting_zyal_inside_a_session_lights_up_the_right_sidebar -- --exact --nocapture"

run_step "18-19-jnoccio-local" \
  "Jnoccio offline and local mock server UX" \
  "JNOCCIO_TUI_TEST=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test jnoccio_tui_dashboard -- --ignored --nocapture"

if require_live_env; then
  run_step "20-live-roundtrip" \
    "Live prompt round-trip" \
    "JEKKO_TUI_BATTLE=1 JEKKO_TUI_BATTLE_LIVE=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test live_programming_challenges live_battle_prompt_roundtrip -- --ignored --exact --nocapture"

  run_step "21-live-challenge-a" \
    "Live programming challenge A" \
    "JEKKO_TUI_BATTLE=1 JEKKO_TUI_BATTLE_LIVE=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test live_programming_challenges live_programming_challenge_adds_rust_function_and_test -- --ignored --exact --nocapture"

  run_step "22-live-challenge-b" \
    "Live programming challenge B" \
    "JEKKO_TUI_BATTLE=1 JEKKO_TUI_BATTLE_LIVE=1 rtk cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test live_programming_challenges live_programming_challenge_debugs_failing_test -- --ignored --exact --nocapture"
else
  printf '\n==> skipping live steps 20-22 (set JEKKO_TUI_BATTLE_LIVE=1 and JEKKO_API_KEY to run attended)\n' >&2
fi

printf '\nTUI battle completed. Receipts: %s\n' "$RECEIPT_DIR" >&2
