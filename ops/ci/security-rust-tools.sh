#!/usr/bin/env bash
# Lane 3/3 of the security:scan split. Runs the cargo-based security
# tooling: cargo-audit (CVE/RUSTSEC) + zizmor (action-workflow auditing) +
# xtask security-lane (the canonical jekko security pipeline that emits
# evidence.json). Slow (~300-400s) because of `cargo install zizmor` on
# cold runners. Pair with security-trufflehog.sh + security-syft-grype.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
source ops/ci/lib.sh

install_cargo_audit() {
  if cargo audit --version >/dev/null 2>&1; then return 0; fi
  cargo install cargo-audit --locked
  export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
}

install_zizmor() {
  if command -v zizmor >/dev/null 2>&1; then return 0; fi
  cargo install zizmor --locked
  export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
}

install_cargo_audit
install_zizmor

mkdir -p "${JANKURAI_ARTIFACT_ROOT}/security"
if ! cargo run -p xtask --locked -- security-lane --profile ci-rust --out "${JANKURAI_ARTIFACT_ROOT}/security"; then
  if [[ -f "${JANKURAI_ARTIFACT_ROOT}/security/evidence.json" ]]; then
    jq . "${JANKURAI_ARTIFACT_ROOT}/security/evidence.json" || cat "${JANKURAI_ARTIFACT_ROOT}/security/evidence.json"
  fi
  exit 1
fi
