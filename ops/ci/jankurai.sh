#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

cargo install --git https://github.com/neverhuman/jankurai --tag v1.3.0 --locked jankurai
if command -v go >/dev/null 2>&1; then
  go install github.com/gitleaks/gitleaks/v8@v8.24.2
  export PATH="$(go env GOPATH)/bin:$PATH"
elif ! command -v gitleaks >/dev/null 2>&1; then
  cargo install gitleaks --locked
fi
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"

jankurai --version
jankurai audit . --mode ratchet --baseline agent/baselines/main.repo-score.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md --sarif target/jankurai/jankurai.sarif --github-step-summary target/jankurai/summary.md --repair-queue-jsonl target/jankurai/repair-queue.jsonl
node tools/jankurai-audit-gate.mjs target/jankurai/repo-score.json
jankurai proof . --changed-from origin/main --out target/jankurai/proof-plan.json --md target/jankurai/proof-plan.md
if ! jankurai proofbind verify . --changed-from origin/main --out target/jankurai/proofbind/surface-witness.json --obligations-out target/jankurai/proofbind/obligations.json 2>/dev/null; then
  jankurai proofbind verify . --changed agent/owner-map.json --changed agent/test-map.json --changed agent/tool-adoption.toml --out target/jankurai/proofbind/surface-witness.json --obligations-out target/jankurai/proofbind/obligations.json
fi
jankurai proofmark rust . --obligations target/jankurai/proofbind/obligations.json
mkdir -p target/jankurai
rtk jankurai ux audit --config agent/ux-qa.toml --out target/jankurai/ux-qa.json
cd crates/tuiwright-jekko-unlock && jankurai rust witness build .
cd "$ROOT"
cargo run --manifest-path crates/zyalc/Cargo.toml --locked --quiet -- compile --all --check
cargo build --manifest-path crates/sandboxctl/Cargo.toml --locked
cargo test --manifest-path crates/sandboxctl/Cargo.toml --locked --tests --no-fail-fast
cargo test --manifest-path crates/jankurai/Cargo.toml --test language_bad_behavior --no-fail-fast
bash tools/security-lane.sh
cargo install cargo-audit --locked
cd crates/tuiwright-jekko-unlock && cargo audit
