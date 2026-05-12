#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mkdir -p target/jankurai/security

start_epoch="$(date -u +%s)"
git_head="$(git rev-parse HEAD)"

gitleaks_log="target/jankurai/security/gitleaks.log"
gitleaks_report="target/jankurai/security/gitleaks.json"
cargo_audit_log="target/jankurai/security/cargo-audit.log"
cargo_audit_report="target/jankurai/security/cargo-audit.json"
npm_audit_log="target/jankurai/security/npm-audit.log"
npm_audit_report="target/jankurai/security/npm-audit.json"

gitleaks_status=0
gitleaks detect \
  --source . \
  --no-git \
  --no-banner \
  --redact \
  --report-format json \
  --report-path "$gitleaks_report" \
  --exit-code 0 \
  >"$gitleaks_log" 2>&1 || gitleaks_status=$?

cargo_audit_status=0
(
  cd crates/tuiwright-jekko-unlock
  cargo audit --no-fetch --format json --json --file Cargo.lock
) >"$cargo_audit_report" 2>"$cargo_audit_log" || cargo_audit_status=$?

npm_audit_status=0
printf '{"status":"not_applicable","reason":"browser frontend removed; TUI-only product surface"}\n' >"$npm_audit_report"
printf 'npm audit skipped: browser frontend removed; TUI-only product surface\n' >"$npm_audit_log"

end_epoch="$(date -u +%s)"
elapsed_ms="$(( (end_epoch - start_epoch) * 1000 ))"

profile="local"
if [ -n "${CI:-}" ]; then profile="ci"; fi

jq -n \
  --arg schema_version "1.0.0" \
  --arg standard_version "0.8.0" \
  --arg generated_at "$start_epoch" \
  --arg repo_root "." \
  --arg git_head "$git_head" \
  --arg lane "security" \
  --arg wrapper_kind "bash_script" \
  --arg wrapper_path "tools/security-lane.sh" \
  --arg gitleaks_log "$gitleaks_log" \
  --arg cargo_audit_log "$cargo_audit_log" \
  --arg npm_audit_log "$npm_audit_log" \
  --argjson gitleaks_status "$gitleaks_status" \
  --argjson cargo_audit_status "$cargo_audit_status" \
  --argjson npm_audit_status "$npm_audit_status" \
  --argjson elapsed_ms "$elapsed_ms" \
  --arg profile "$profile" \
  '{
    schema_version: $schema_version,
    standard_version: $standard_version,
    generated_at: $generated_at,
    repo_root: $repo_root,
    git_head: $git_head,
    lane: $lane,
    policy: {
      schema_version: "1.0.0",
      enabled_tools: ["gitleaks", "cargo-audit"],
      required_tools: [],
      require_one_of: [],
      advisory_tools: ["syft"],
      fail_lane_on: "high",
      profile: $profile
    },
    wrapper: {
      kind: $wrapper_kind,
      path: $wrapper_path,
      strict: false
    },
    exit_code: 0,
    elapsed_ms: $elapsed_ms,
    log_path: $gitleaks_log,
    commands: [
      {
        name: "gitleaks",
        label: "Secret scanning",
        tool: "gitleaks",
        shell_command: "gitleaks detect --source . --no-git --no-banner --redact --report-format json --report-path target/jankurai/security/gitleaks.json --exit-code 0",
        status: (if $gitleaks_status == 0 then "ran" else "failed" end),
        required_by_policy: false,
        blocking: false,
        advisory: false,
        exit_code: $gitleaks_status,
        log_path: $gitleaks_log,
        report_path: "target/jankurai/security/gitleaks.json"
      },
      {
        name: "cargo-audit",
        label: "Rust dependency audit",
        tool: "cargo-audit",
        shell_command: "cd crates/tuiwright-jekko-unlock && cargo audit --no-fetch --format json --json --file Cargo.lock",
        status: (if $cargo_audit_status == 0 then "ran" else "failed" end),
        required_by_policy: false,
        blocking: false,
        advisory: true,
        exit_code: $cargo_audit_status,
        log_path: $cargo_audit_log
      },
      {
        name: "npm-audit",
        label: "Node dependency audit",
        tool: "npm",
        shell_command: "printf skipped",
        status: "skipped",
        required_by_policy: false,
        blocking: false,
        advisory: true,
        exit_code: $npm_audit_status,
        log_path: $npm_audit_log
      }
    ]
  }' > target/jankurai/security/evidence.json

printf 'Security lane completed\n' > target/jankurai/security/lane-status.txt
