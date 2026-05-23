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

if [ -z "${GH_TOKEN:-}" ] && [ -n "${GITHUB_TOKEN:-}" ]; then
  export GH_TOKEN="$GITHUB_TOKEN"
fi

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

git_remote_repository() {
  local origin_url
  origin_url="$(git remote get-url origin 2>/dev/null)" || return 1

  if [[ "$origin_url" =~ ^(ssh://)?([^@/]+@)?github\.com[:/]+([^/]+)/([^/.]+)(\.git)?$ ]]; then
    printf '%s/%s\n' "${BASH_REMATCH[3]}" "${BASH_REMATCH[4]}"
    return 0
  fi

  if [[ "$origin_url" =~ ^https?://([^@/]+@)?github\.com[:/]+([^/]+)/([^/.]+)(\.git)?$ ]]; then
    printf '%s/%s\n' "${BASH_REMATCH[2]}" "${BASH_REMATCH[3]}"
    return 0
  fi

  if [[ "$origin_url" =~ github\.com[:/]+([^/]+)/([^/.]+)(\.git)?$ ]]; then
    printf '%s/%s\n' "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}"
    return 0
  fi

  return 1
}

git_remote_default_branch() {
  local default_branch

  if default_branch="$(git symbolic-ref --quiet --short refs/remotes/origin/HEAD 2>/dev/null)"; then
    default_branch="${default_branch#origin/}"
    if [ -n "$default_branch" ]; then
      printf '%s\n' "$default_branch"
      return 0
    fi
  fi

  if default_branch="$(git remote show origin 2>/dev/null | awk -F': ' '/HEAD branch/ { print $2; exit }')"; then
    if [ -n "$default_branch" ]; then
      printf '%s\n' "$default_branch"
      return 0
    fi
  fi

  return 1
}

resolve_github_repository() {
  local repo_json

  require_cmd gh "https://cli.github.com/manual/gh"

  if [ -n "${GITHUB_REPOSITORY:-}" ]; then
    repo_json="$GITHUB_REPOSITORY"
  elif ! repo_json="$(git_remote_repository)"; then
    if [ -n "${GITHUB_EVENT_PATH:-}" ] && [ -f "$GITHUB_EVENT_PATH" ]; then
      require_cmd jq "brew install jq"
      repo_json="$(jq -r '.repository.full_name // empty' "$GITHUB_EVENT_PATH" 2>/dev/null)"
    fi
    if [ -z "${repo_json:-}" ]; then
      if ! repo_json="$(gh repo view --json nameWithOwner --jq '.nameWithOwner' 2>/dev/null)"; then
        fail "could not resolve GITHUB_REPOSITORY from the checkout; set GITHUB_REPOSITORY or use a GitHub remote"
      fi
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
    if [ -n "${GITHUB_HEAD_REF:-}" ]; then
      pr_reference="$GITHUB_HEAD_REF"
    fi
  fi

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

  default_branch="${GITHUB_BASE_REF:-}"
  if [ -z "$default_branch" ]; then
    default_branch="$(git_remote_default_branch || true)"
  fi
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
