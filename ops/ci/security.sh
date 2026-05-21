#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

install_gitleaks() {
  if command -v gitleaks >/dev/null 2>&1; then
    return 0
  fi
  if command -v go >/dev/null 2>&1; then
    go install github.com/zricethezav/gitleaks/v8@v8.30.1
    export PATH="$(go env GOPATH)/bin:$PATH"
  else
    cargo install gitleaks --locked
    export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
  fi
}

install_cargo_audit() {
  if ! cargo audit --version >/dev/null 2>&1; then
    cargo install cargo-audit --locked
  fi
  export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
}

install_gitleaks
install_cargo_audit

if ! cargo run -p xtask --locked -- security-lane --out target/jankurai/security; then
  if [[ -f target/jankurai/security/evidence.json ]]; then
    jq . target/jankurai/security/evidence.json || cat target/jankurai/security/evidence.json
  fi
  exit 1
fi
