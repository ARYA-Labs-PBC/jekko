#!/usr/bin/env bash
# Shared helpers for ops/ci/*.sh. Keep local CI and workflow entrypoints
# aligned by sourcing the same tool pins and small helper functions.

set -euo pipefail

CI_ROOT="${CI_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
ARTIFACT_ROOT="${ARTIFACT_ROOT:-${CI_ROOT}/target/jankurai}"

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
