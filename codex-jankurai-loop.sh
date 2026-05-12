#!/usr/bin/env bash
set -euo pipefail

PROMPT_FILE="${PROMPT_FILE:-./prompt.md}"
REPO_DIR="${REPO_DIR:-$PWD}"
LOG_DIR="${LOG_DIR:-$HOME/.codex-loop-runs}"
SLEEP_SECONDS="${SLEEP_SECONDS:-5}"
MAX_RUNS="${MAX_RUNS:-0}" # 0 = forever
SANDBOX="${SANDBOX:-workspace-write}"
APPROVAL="${APPROVAL:-never}"
MODEL="${MODEL:-gpt-5.3-codex-spark}"
REASONING_EFFORT="${REASONING_EFFORT:-high}"
CODEX_BIN="${CODEX_BIN:-codex}"
COLOR="${COLOR:-never}"
JSON_EVENTS="${JSON_EVENTS:-0}"
STOP_ON_FAILURE="${STOP_ON_FAILURE:-0}"

die() {
  echo "ERROR: $*" >&2
  exit 1
}

abs_path() {
  local path="$1"
  if [[ -d "$path" ]]; then
    (cd "$path" && pwd -P)
  else
    local dir
    local base
    dir="$(dirname "$path")"
    base="$(basename "$path")"
    (cd "$dir" && printf '%s/%s\n' "$(pwd -P)" "$base")
  fi
}

command -v "$CODEX_BIN" >/dev/null 2>&1 || die "codex CLI not found on PATH. Install with: npm i -g @openai/codex"
[[ -f "$PROMPT_FILE" ]] || die "prompt file not found: $PROMPT_FILE"
[[ -d "$REPO_DIR" ]] || die "repo dir not found: $REPO_DIR"

case "$MAX_RUNS" in
  ''|*[!0-9]*) die "MAX_RUNS must be a non-negative integer" ;;
esac

PROMPT_FILE="$(abs_path "$PROMPT_FILE")"
REPO_DIR="$(abs_path "$REPO_DIR")"
mkdir -p "$LOG_DIR"
LOG_DIR="$(abs_path "$LOG_DIR")"

run_number=0

trap 'echo; echo "Stopping codex loop."; exit 130' INT TERM

while true; do
  run_number=$((run_number + 1))

  if [[ "$MAX_RUNS" != "0" && "$run_number" -gt "$MAX_RUNS" ]]; then
    echo "Reached MAX_RUNS=$MAX_RUNS. Exiting."
    exit 0
  fi

  stamp="$(date -u +"%Y%m%dT%H%M%SZ")"
  run_dir="$LOG_DIR/run-${run_number}-${stamp}"
  mkdir -p "$run_dir"

  stdout_file="$run_dir/stdout.txt"
  stderr_file="$run_dir/stderr.txt"
  final_file="$run_dir/final-message.md"
  status_file="$run_dir/status.txt"
  command_file="$run_dir/command.txt"
  env_file="$run_dir/env.txt"

  cp "$PROMPT_FILE" "$run_dir/prompt.md"

  codex_args=(
    exec
    --cd "$REPO_DIR"
    --sandbox "$SANDBOX"
    --ephemeral
    --color "$COLOR"
    --model "$MODEL"
    -c "approval_policy=\"$APPROVAL\""
    -c "model_reasoning_effort=\"$REASONING_EFFORT\""
    -c 'sandbox_permissions=["disk-full-read-access"]'
    --output-last-message "$final_file"
  )

  if [[ "$JSON_EVENTS" == "1" ]]; then
    codex_args+=(--json)
  fi

  codex_args+=(-)

  {
    echo "PROMPT_FILE=$PROMPT_FILE"
    echo "REPO_DIR=$REPO_DIR"
    echo "LOG_DIR=$LOG_DIR"
    echo "SANDBOX=$SANDBOX"
    echo 'SANDBOX_PERMISSIONS=["disk-full-read-access"]'
    echo "APPROVAL=$APPROVAL"
    echo "MODEL=$MODEL"
    echo "REASONING_EFFORT=$REASONING_EFFORT"
    echo "CODEX_BIN=$CODEX_BIN"
  } > "$env_file"

  printf '%q ' "$CODEX_BIN" "${codex_args[@]}" > "$command_file"
  printf '\n' >> "$command_file"

  echo
  echo "============================================================"
  echo "Codex run #$run_number"
  echo "Prompt: $PROMPT_FILE"
  echo "Repo:   $REPO_DIR"
  echo "Model:  $MODEL ($REASONING_EFFORT)"
  echo "Logs:   $run_dir"
  echo "============================================================"

  set +e
  "$CODEX_BIN" "${codex_args[@]}" \
    < "$PROMPT_FILE" \
    > "$stdout_file" \
    2> "$stderr_file"
  status=$?
  set -e

  echo "$status" > "$status_file"
  echo "Codex exit code: $status"

  if [[ -s "$final_file" ]]; then
    echo
    echo "Final message:"
    echo "------------------------------------------------------------"
    cat "$final_file"
    echo
    echo "------------------------------------------------------------"
  elif [[ -s "$stdout_file" ]]; then
    echo
    echo "Stdout:"
    echo "------------------------------------------------------------"
    cat "$stdout_file"
    echo
    echo "------------------------------------------------------------"
  fi

  if [[ "$status" -ne 0 ]]; then
    echo "Codex failed. Last stderr:"
    tail -n 80 "$stderr_file" || true

    if [[ "$STOP_ON_FAILURE" == "1" ]]; then
      echo "STOP_ON_FAILURE=1; exiting."
      exit "$status"
    fi
  fi

  echo "Sleeping ${SLEEP_SECONDS}s before next fresh Codex session..."
  sleep "$SLEEP_SECONDS"
done
