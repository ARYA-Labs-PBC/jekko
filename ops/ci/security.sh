#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
source ops/ci/lib.sh

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

install_syft() {
  if command -v syft >/dev/null 2>&1; then
    return 0
  fi

  local tmp
  tmp="$(mktemp -d)"
  curl -sSfL https://raw.githubusercontent.com/anchore/syft/main/install.sh \
    -o "$tmp/install-syft.sh"
  mkdir -p "${HOME}/.local/bin"
  sh "$tmp/install-syft.sh" -b "${HOME}/.local/bin"
  export PATH="${HOME}/.local/bin:${PATH}"
}

install_zizmor() {
  if ! command -v zizmor >/dev/null 2>&1; then
    cargo install zizmor --locked
  fi
  export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
}

install_gitleaks
install_cargo_audit
install_zizmor
install_syft

mkdir -p "${JANKURAI_ARTIFACT_ROOT}/security"
if ! cargo run -p xtask --locked -- security-lane --profile ci --out "${JANKURAI_ARTIFACT_ROOT}/security"; then
  if [[ -f "${JANKURAI_ARTIFACT_ROOT}/security/evidence.json" ]]; then
    jq . "${JANKURAI_ARTIFACT_ROOT}/security/evidence.json" || cat "${JANKURAI_ARTIFACT_ROOT}/security/evidence.json"
  fi
  exit 1
fi
