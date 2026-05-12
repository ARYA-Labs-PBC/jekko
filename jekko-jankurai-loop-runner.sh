run_loop() {
  run_number=0

  while true; do
    run_number=$((run_number + 1))

    if [[ "$MAX_RUNS" != "0" && "$run_number" -gt "$MAX_RUNS" ]]; then
      echo "Reached MAX_RUNS=$MAX_RUNS. Exiting."
      exit 0
    fi

    stamp="$(date -u +"%Y%m%dT%H%M%SZ")"
    run_dir="$LOG_DIR/run-${run_number}-${stamp}"
    mkdir -p "$run_dir"

    prompt_copy="$run_dir/prompt.md"
    env_file="$run_dir/env.txt"
    command_file="$run_dir/command.txt"
    status_file="$run_dir/status.txt"
    stdout_file="$run_dir/stdout.txt"
    stderr_file="$run_dir/stderr.txt"
    final_file="$run_dir/final-message.md"
    metadata_file="$run_dir/metadata.json"

    cp "$PROMPT_FILE" "$prompt_copy"
    : > "$final_file"
    write_env_file "$env_file"

    run_cmd=(
      "$JEKKO_BIN"
      run
      --dir "$REPO_DIR"
      --model "$MODEL"
      --title "Jankurai loop run #$run_number"
      --output-last-message "$final_file"
    )

    if [[ "$JSON_EVENTS" == "1" ]]; then
      run_cmd+=(--format json)
    fi
    if [[ -n "$VARIANT" ]]; then
      run_cmd+=(--variant "$VARIANT")
    fi
    if [[ -n "$AGENT" ]]; then
      run_cmd+=(--agent "$AGENT")
    fi
    if [[ "$SKIP_PERMISSIONS" == "1" ]]; then
      run_cmd+=(--dangerously-skip-permissions)
    fi

    write_command_file "$command_file" "${run_cmd[@]}"

    unlock_ran=0
    unlock_status=null
    if [[ "$AUTO_UNLOCK" == "1" && "$MODEL" == jnoccio/* ]]; then
      unlock_ran=1
      unlock_stdout="$run_dir/unlock-stdout.json"
      unlock_stderr="$run_dir/unlock-stderr.txt"
      unlock_status_file="$run_dir/unlock-status.txt"
      unlock_command_file="$run_dir/unlock-command.txt"
      unlock_cmd=(
        "$JEKKO_BIN"
        providers
        unlock
        jnoccio
        --repo "$REPO_DIR"
        --json
      )

      write_command_file "$unlock_command_file" "${unlock_cmd[@]}"

      set +e
      "${unlock_cmd[@]}" > "$unlock_stdout" 2> "$unlock_stderr"
      unlock_status=$?
      set -e
      printf '%s\n' "$unlock_status" > "$unlock_status_file"

      if [[ "$unlock_status" -ne 0 ]]; then
        echo "Jnoccio unlock failed with exit code $unlock_status."
        write_metadata_file "$metadata_file" "$run_number" "$stamp" true "$unlock_status" false null
        printf '%s\n' "$unlock_status" > "$status_file"
        if [[ "$REQUIRE_UNLOCK" == "1" ]]; then
          exit "$unlock_status"
        fi
      fi
    fi

    echo
    echo "$(paint "$(run_color "$run_number")" "$(separator "=")")"
    echo "$(paint "$(run_color "$run_number")" "Jekko run #$run_number")"
    echo "$(paint "$(run_color "$run_number")" "Prompt: $PROMPT_FILE")"
    echo "$(paint "$(run_color "$run_number")" "Repo:   $REPO_DIR")"
    echo "$(paint "$(run_color "$run_number")" "Model:  $MODEL${VARIANT:+ ($VARIANT)}")"
    echo "$(paint "$(run_color "$run_number")" "Logs:   $run_dir")"
    echo "$(paint "$(run_color "$run_number")" "$(separator "=")")"

    set +e
    "${run_cmd[@]}" < "$prompt_copy" > "$stdout_file" 2> "$stderr_file"
    status=$?
    set -e

    printf '%s\n' "$status" > "$status_file"
    write_metadata_file "$metadata_file" "$run_number" "$stamp" "$([[ "$unlock_ran" == "1" ]] && echo true || echo false)" "$unlock_status" true "$status"
    echo "Jekko exit code: $status"

    if [[ -s "$final_file" ]]; then
      echo
      echo "Final message:"
      echo "$(separator "-")"
      cat "$final_file"
      echo
      echo "$(separator "-")"
    elif [[ -s "$stdout_file" ]]; then
      echo
      echo "Stdout:"
      echo "$(separator "-")"
      cat "$stdout_file"
      echo
      echo "$(separator "-")"
    fi

    if [[ "$status" -ne 0 ]]; then
      echo "Jekko failed. Last stderr:"
      tail -n 80 "$stderr_file" || true
      if [[ "$STOP_ON_FAILURE" == "1" ]]; then
        echo "STOP_ON_FAILURE=1; exiting."
        exit "$status"
      fi
    fi

    if [[ "$MAX_RUNS" == "0" || "$run_number" -lt "$MAX_RUNS" ]]; then
      echo "Sleeping ${SLEEP_SECONDS}s before next fresh Jekko session..."
      sleep "$SLEEP_SECONDS"
    fi
  done
}
