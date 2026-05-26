#!/usr/bin/env bash
# Lane 1/3 of the security:scan split. Runs ONLY trufflehog + gitleaks
# (the secret-scanning tools). Fast (~60s) and doesn't need the Rust
# toolchain. Pair with security-syft-grype.sh + security-rust-tools.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
source ops/ci/lib.sh

install_gitleaks() {
  if command -v gitleaks >/dev/null 2>&1; then return 0; fi
  if command -v go >/dev/null 2>&1; then
    go install github.com/zricethezav/gitleaks/v8@v8.30.1
    export PATH="$(go env GOPATH)/bin:$PATH"
  else
    local GL_VER="8.30.1" tmp
    tmp="$(mktemp -d)"
    curl -fsSL \
      "https://github.com/gitleaks/gitleaks/releases/download/v${GL_VER}/gitleaks_${GL_VER}_linux_x64.tar.gz" \
      -o "$tmp/gitleaks.tgz"
    tar -xzf "$tmp/gitleaks.tgz" -C "$tmp" gitleaks
    mkdir -p "${HOME}/.local/bin"
    install -m 0755 "$tmp/gitleaks" "${HOME}/.local/bin/gitleaks"
    export PATH="${HOME}/.local/bin:${PATH}"
  fi
}

install_gitleaks

if ! command -v trufflehog >/dev/null 2>&1; then
  curl -sSfL https://raw.githubusercontent.com/trufflesecurity/trufflehog/main/scripts/install.sh \
    | sh -s -- -b /usr/local/bin
fi

mkdir -p "${JANKURAI_ARTIFACT_ROOT}/security"
trufflehog filesystem . --json --no-update | tee "${JANKURAI_ARTIFACT_ROOT}/security/trufflehog-report.json"
gitleaks detect --no-git --report-path "${JANKURAI_ARTIFACT_ROOT}/security/gitleaks-report.json" --report-format json || true
