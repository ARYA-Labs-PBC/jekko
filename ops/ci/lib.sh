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

resolve_github_repository() {
  local repo_json

  require_cmd gh "https://cli.github.com/manual/gh"

  if [ -z "${GITHUB_REPOSITORY:-}" ]; then
    if ! repo_json="$(gh repo view --json nameWithOwner --jq '.nameWithOwner')"; then
      fail "gh repo view failed; run in a repo checkout or set GITHUB_REPOSITORY"
    fi
  else
    if ! repo_json="$(gh repo view "$GITHUB_REPOSITORY" --json nameWithOwner --jq '.nameWithOwner')"; then
      fail "gh repo view failed; run in a repo checkout or set GITHUB_REPOSITORY"
    fi
  fi

  GITHUB_REPOSITORY="$repo_json"
  export GITHUB_REPOSITORY
}

pull_request_target_json() {
  local pr_reference="${1:-}"
  local current_branch
  local pr_json
  local pr_number
  local author_association
  local default_branch
  local event_path

  require_cmd gh "https://cli.github.com/manual/gh"
  require_cmd jq "brew install jq"
  resolve_github_repository

  if [ -z "${pr_reference}" ]; then
    if ! current_branch="$(git rev-parse --abbrev-ref HEAD)"; then
      fail "could not resolve current branch; pass an explicit PR reference: pull_request_target_json <pr_ref>"
    fi
    if [ -z "$current_branch" ] || [ "$current_branch" = "HEAD" ]; then
      fail "detached HEAD; pass an explicit PR reference: pull_request_target_json <pr_ref>"
    fi
    pr_reference="$current_branch"
  fi

  if ! pr_json="$(gh pr view --repo "$GITHUB_REPOSITORY" "$pr_reference" --json number,title,body,author,createdAt,headRefName,baseRefName --jq '.')"; then
    fail "gh pr view failed for '${pr_reference}' in '${GITHUB_REPOSITORY}'"
  fi

  pr_number="$(printf '%s' "$pr_json" | jq -r '.number')"
  if [ -z "$pr_number" ] || [ "$pr_number" = "null" ]; then
    fail "could not resolve pull request number from gh pr view payload"
  fi

  if ! author_association="$(gh api "/repos/${GITHUB_REPOSITORY}/pulls/${pr_number}" --jq '.author_association')"; then
    fail "gh api failed while reading pull request author association for #${pr_number}"
  fi

  if [ -z "${author_association}" ] || [ "$author_association" = "null" ]; then
    fail "pull request #${pr_number} is missing author_association"
  fi

  if ! repo_json="$(gh repo view "$GITHUB_REPOSITORY" --json defaultBranchRef --jq '.')"; then
    fail "gh repo view failed while reading default branch for '${GITHUB_REPOSITORY}'"
  fi
  default_branch="$(printf '%s' "$repo_json" | jq -r '.defaultBranchRef.name // empty')"
  if [ -z "$default_branch" ]; then
    fail "could not determine repository default branch for '${GITHUB_REPOSITORY}'"
  fi

  event_path="$(mktemp -t jekko-pr-target-event.XXXXXX)"
  jq -n \
    --argjson pr "$pr_json" \
    --arg author_association "$author_association" \
    --arg default_branch "$default_branch" \
    '{
      "pull_request": {
        "number": $pr.number,
        "title": $pr.title,
        "body": $pr.body,
        "user": { "login": $pr.author.login },
        "author_association": $author_association,
        "created_at": $pr.createdAt,
        "head": { "ref": $pr.headRefName },
        "base": { "ref": $pr.baseRefName }
      },
      "repository": {
        "default_branch": $default_branch
      }
    }' >"$event_path"

  echo "$event_path"
}
