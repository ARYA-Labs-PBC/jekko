#!/usr/bin/env bash
# Shared helpers for ops/ci/*.sh. Keep local CI and workflow entrypoints
# aligned by sourcing the same tool pins and small helper functions.

set -euo pipefail

CI_ROOT="${CI_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
JANKURAI_ARTIFACT_ROOT="${JANKURAI_ARTIFACT_ROOT:-${CI_ROOT}/.jankurai}"
ARTIFACT_ROOT="${ARTIFACT_ROOT:-${JANKURAI_ARTIFACT_ROOT}}"
export JANKURAI_ARTIFACT_ROOT ARTIFACT_ROOT

RUST_TOOLCHAIN="${RUST_TOOLCHAIN:-1.95.0}"
GITLEAKS_VERSION="${GITLEAKS_VERSION:-8.30.0}"
CARGO_AUDIT_VERSION="${CARGO_AUDIT_VERSION:-0.22.1}"
ZIZMOR_VERSION="${ZIZMOR_VERSION:-1.12.0}"
CARGO_LLVM_COV_VERSION="${CARGO_LLVM_COV_VERSION:-0.6.16}"

step() { printf '\n\033[1;36m==> %s\033[0m\n' "$1"; }
note() { printf '\033[0;90m... %s\033[0m\n' "$1"; }
fail() { printf '\n\033[1;31m!! %s\033[0m\n' "$1" >&2; exit 1; }

require_cmd() {
  local cmd="$1"
  local hint="$2"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    fail "missing required command: $cmd (install: $hint)"
  fi
}

assert_path() {
  local p="$1"
  [[ -e "$p" ]] || fail "expected artifact not produced: $p"
  note "artifact present: $p"
}

assert_nonempty() {
  local p="$1"
  assert_path "$p"
  [[ -s "$p" ]] || fail "expected artifact is empty: $p"
}

require_pr_policy_contract() {
  local gh_token="${GH_TOKEN:-}"
  local github_token="${GITHUB_TOKEN:-}"
  local gh_repo="${GH_REPO:-}"
  local github_repository="${GITHUB_REPOSITORY:-}"
  local base_ref="${GITHUB_BASE_REF:-}"
  local head_ref="${GITHUB_HEAD_REF:-}"
  local event_path="${GITHUB_EVENT_PATH:-}"

  [ -n "$gh_token" ] || fail "missing required env GH_TOKEN"
  [ -n "$github_token" ] || fail "missing required env GITHUB_TOKEN"
  [ "$gh_token" = "$github_token" ] || fail "GH_TOKEN and GITHUB_TOKEN must match"
  [ -n "$gh_repo" ] || fail "missing required env GH_REPO"
  [ -n "$github_repository" ] || fail "missing required env GITHUB_REPOSITORY"
  [ "$gh_repo" = "$github_repository" ] || fail "GH_REPO and GITHUB_REPOSITORY must match"
  [ -n "$base_ref" ] || fail "missing required env GITHUB_BASE_REF"
  [ -n "$head_ref" ] || fail "missing required env GITHUB_HEAD_REF"
  [ -n "$event_path" ] || fail "missing required env GITHUB_EVENT_PATH"
  [[ -s "$event_path" ]] || fail "expected GITHUB_EVENT_PATH to point to a non-empty file: $event_path"

  note "pr-policy contract: GH_REPO=$gh_repo GITHUB_REPOSITORY=$github_repository GITHUB_BASE_REF=$base_ref GITHUB_HEAD_REF=$head_ref GITHUB_EVENT_PATH=$event_path GH_TOKEN=[redacted] GITHUB_TOKEN=[redacted]"
}

pull_request_target_json() {
  local repo="${1:-}"
  local base_ref="${2:-}"
  local head_ref="${3:-}"
  local pr_json
  local pr_number
  local author_association
  local event_path

  require_cmd gh "https://cli.github.com/manual/gh"
  require_cmd jq "brew install jq"

  [ -n "$repo" ] || fail "pull_request_target_json requires a repo"
  [ -n "$base_ref" ] || fail "pull_request_target_json requires a base ref"
  [ -n "$head_ref" ] || fail "pull_request_target_json requires a head ref"

  if ! pr_json="$(gh pr view --repo "$repo" "$head_ref" --json number,title,body,author,createdAt,headRefName,baseRefName)"; then
    fail "gh pr view failed for '${head_ref}' in '${repo}'"
  fi

  pr_number="$(printf '%s' "$pr_json" | jq -r '.number')"
  if [ -z "$pr_number" ] || [ "$pr_number" = "null" ]; then
    fail "could not resolve pull request number from gh pr view payload"
  fi

  if [ "$(printf '%s' "$pr_json" | jq -r '.baseRefName')" != "$base_ref" ] || \
    [ "$(printf '%s' "$pr_json" | jq -r '.headRefName')" != "$head_ref" ]; then
    fail "gh pr view returned mismatched refs for '${head_ref}' in '${repo}'"
  fi

  if ! author_association="$(gh api "/repos/${repo}/pulls/${pr_number}" --jq '.author_association')"; then
    fail "gh api failed while reading pull request author association for #${pr_number} in '${repo}'"
  fi

  if [ -z "${author_association}" ] || [ "$author_association" = "null" ]; then
    fail "pull request #${pr_number} is missing author_association"
  fi

  event_path="$(mktemp -t jekko-pr-target-event.XXXXXX)"
  jq -n \
    --argjson pr "$pr_json" \
    --arg author_association "$author_association" \
    --arg base_ref "$base_ref" \
    --arg head_ref "$head_ref" \
    '{
      "pull_request": {
        "number": $pr.number,
        "title": $pr.title,
        "body": $pr.body,
        "user": { "login": $pr.author.login },
        "author_association": $author_association,
        "created_at": $pr.createdAt,
        "head": { "ref": $head_ref },
        "base": { "ref": $base_ref }
      },
      "repository": {
        "default_branch": $base_ref
      }
    }' >"$event_path"

  echo "$event_path"
}
