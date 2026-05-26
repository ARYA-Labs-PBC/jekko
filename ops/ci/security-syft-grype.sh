#!/usr/bin/env bash
# Lane 2/3 of the security:scan split. Runs ONLY syft (SBOM) + grype
# (CVE scan). Medium (~100-120s). Pair with security-trufflehog.sh +
# security-rust-tools.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
source ops/ci/lib.sh

install_syft() {
  if command -v syft >/dev/null 2>&1; then return 0; fi
  local tmp
  tmp="$(mktemp -d)"
  curl -sSfL https://raw.githubusercontent.com/anchore/syft/main/install.sh -o "$tmp/install-syft.sh"
  mkdir -p "${HOME}/.local/bin"
  sh "$tmp/install-syft.sh" -b "${HOME}/.local/bin"
  export PATH="${HOME}/.local/bin:${PATH}"
}

install_grype() {
  if command -v grype >/dev/null 2>&1; then return 0; fi
  local tmp
  tmp="$(mktemp -d)"
  curl -sSfL https://raw.githubusercontent.com/anchore/grype/main/install.sh -o "$tmp/install-grype.sh"
  mkdir -p "${HOME}/.local/bin"
  sh "$tmp/install-grype.sh" -b "${HOME}/.local/bin"
  export PATH="${HOME}/.local/bin:${PATH}"
}

install_syft
install_grype

mkdir -p "${JANKURAI_ARTIFACT_ROOT}/security"
syft . -o spdx-json="${JANKURAI_ARTIFACT_ROOT}/security/sbom.spdx.json" || true
grype sbom:"${JANKURAI_ARTIFACT_ROOT}/security/sbom.spdx.json" --fail-on high || true
