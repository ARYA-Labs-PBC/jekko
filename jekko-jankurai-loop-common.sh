fatal() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 1
}

resolve_path() {
  local path="$1"
  if [[ -d "$path" ]]; then
    (cd "$path" && pwd -P)
    return
  fi

  local dir="${path%/*}"
  local base="${path##*/}"
  if [[ "$dir" == "$path" ]]; then
    dir="."
  fi

  (cd "$dir" && printf '%s/%s\n' "$(pwd -P)" "$base")
}

expand_home() {
  local path="$1"
  if [[ "$path" == "~" ]]; then
    printf '%s\n' "$HOME"
    return
  fi
  if [[ "$path" == "~/"* ]]; then
    printf '%s/%s\n' "$HOME" "${path#~/}"
    return
  fi
  printf '%s\n' "$path"
}

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\r/\\r/g'
}

write_command_file() {
  local file="$1"
  shift

  printf '%q ' "$@" > "$file"
  printf '\n' >> "$file"
}

paint() {
  local color="$1"
  shift
  if [[ "$color_enabled" == "1" ]]; then
    printf '\033[38;5;%sm%s\033[0m' "$color" "$*"
  else
    printf '%s' "$*"
  fi
}

separator() {
  local char="${1:-=}"
  printf '%*s' 60 '' | tr ' ' "$char"
}

run_color() {
  local run="$1"
  local index=$(( (palette_offset + run - 1) % palette_len ))
  printf '%s\n' "${palette[$index]}"
}

write_env_file() {
  local file="$1"
  local unlock_secret_path_set=0
  local unlock_secret_file_present=0

  if [[ -n "${JNOCCIO_UNLOCK_SECRET_PATH:-}" ]]; then
    unlock_secret_path_set=1
    if [[ -f "$(expand_home "$JNOCCIO_UNLOCK_SECRET_PATH")" ]]; then
      unlock_secret_file_present=1
    fi
  fi

  {
    echo "PROMPT_FILE=$PROMPT_FILE"
    echo "REPO_DIR=$REPO_DIR"
    echo "LOG_DIR=$LOG_DIR"
    echo "SLEEP_SECONDS=$SLEEP_SECONDS"
    echo "MAX_RUNS=$MAX_RUNS"
    echo "MODEL=$MODEL"
    echo "VARIANT=$VARIANT"
    echo "AGENT=$AGENT"
    echo "JEKKO_BIN=$JEKKO_BIN"
    echo "ZYAL_RUN=$ZYAL_RUN"
    echo "JEKKO_AUTO_ALLOW_READS=$JEKKO_AUTO_ALLOW_READS"
    echo "JSON_EVENTS=$JSON_EVENTS"
    echo "AUTO_UNLOCK=$AUTO_UNLOCK"
    echo "REQUIRE_UNLOCK=$REQUIRE_UNLOCK"
    echo "STOP_ON_FAILURE=$STOP_ON_FAILURE"
    echo "SKIP_PERMISSIONS=$SKIP_PERMISSIONS"
    echo "LOOP_COLOR=$LOOP_COLOR"
    echo "NO_COLOR=${NO_COLOR:-}"
    echo "JNOCCIO_UNLOCK_SECRET_PATH_SET=$unlock_secret_path_set"
    echo "JNOCCIO_UNLOCK_SECRET_FILE_PRESENT=$unlock_secret_file_present"
  } > "$file"
}

write_metadata_file() {
  local file="$1"
  local run_number="$2"
  local stamp="$3"
  local unlock_ran="$4"
  local unlock_status="${5:-null}"
  local run_started="$6"
  local run_status="${7:-null}"

  cat > "$file" <<EOF
{
  "run_number": $run_number,
  "stamp": "$(json_escape "$stamp")",
  "prompt_file": "$(json_escape "$PROMPT_FILE")",
  "repo_dir": "$(json_escape "$REPO_DIR")",
  "log_dir": "$(json_escape "$LOG_DIR")",
  "model": "$(json_escape "$MODEL")",
  "variant": "$(json_escape "$VARIANT")",
  "agent": "$(json_escape "$AGENT")",
  "jekko_bin": "$(json_escape "$JEKKO_BIN")",
  "json_events": $([[ "$JSON_EVENTS" == "1" ]] && echo true || echo false),
  "auto_unlock": $([[ "$AUTO_UNLOCK" == "1" ]] && echo true || echo false),
  "require_unlock": $([[ "$REQUIRE_UNLOCK" == "1" ]] && echo true || echo false),
  "stop_on_failure": $([[ "$STOP_ON_FAILURE" == "1" ]] && echo true || echo false),
  "skip_permissions": $([[ "$SKIP_PERMISSIONS" == "1" ]] && echo true || echo false),
  "loop_color": "$(json_escape "$LOOP_COLOR")",
  "no_color": $([[ -n "${NO_COLOR:-}" ]] && echo true || echo false),
  "unlock_ran": $unlock_ran,
  "unlock_status": $unlock_status,
  "run_started": $run_started,
  "run_status": $run_status
}
EOF
}
