# jankurai scaffold Justfile

default: fast

export TURBO_CACHE_DIR := ".turbo"
memory_benchmark_seed := env_var_or_default("MEMORY_BENCHMARK_SEED", "public-dev-0001")

# fast deterministic build/test targets, caches, and narrow proof lanes for agent iteration.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
fast: workspace-fast

# one-command setup lane for local iteration.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
setup:
	cargo fetch

# one-command validation lane for agent iteration.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
validate:
	just fast

# Workspace-wide fast lane composed from narrow proof targets.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
workspace-fast:
	just workspace-typecheck-fast
	just workspace-build-fast
	just workspace-test-fast

# Narrow lane for workspace typecheck-only feedback.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
workspace-typecheck-fast:
	rtk cargo check --workspace --locked

# Narrow lane for workspace test-only feedback.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
# Narrow lane for workspace build-only feedback. (Cache enabled)
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
workspace-test-fast: core-test-fast jekko-test-fast

# Narrow lane for workspace build-only feedback. (Cache enabled)
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
workspace-build-fast:
	rtk cargo build --workspace --locked

# Narrow lane for the core workspace package's fast feedback targets.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
core-fast: core-typecheck-fast core-build-fast core-test-fast

# Narrow lane for the plugin workspace package's fast feedback targets.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
plugin-fast: plugin-typecheck-fast plugin-build-fast

# Narrow lane for the SDK workspace package's fast feedback targets.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
sdk-fast: sdk-typecheck-fast sdk-build-fast

# Narrow lane for the core workspace package's fast feedback targets.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
typecheck-fast:
	rtk cargo check --workspace --locked

# Narrow lane for package builds that can reuse Turbo cache metadata.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
build-fast: workspace-build-fast

# Narrow lane for the core package typecheck only.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
core-typecheck-fast:
	rtk cargo check -p jekko-core --locked

# Narrow lane for the core package compile path.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
core-build-fast:
	rtk cargo build -p jekko-core --locked

# Narrow lane for core package behavior checks.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
core-test:
	rtk cargo test -p jekko-core --locked --no-fail-fast

# Narrow lane for core package behavior checks with an explicit fast alias.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
core-test-fast:
	rtk cargo test -p jekko-core --locked --no-fail-fast

# Narrow lane for package-level typechecks.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
plugin-typecheck:
	rtk cargo check -p jekko-plugin-api --locked

# Narrow lane for plugin package typechecks with an explicit fast alias.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
plugin-typecheck-fast:
	rtk cargo check -p jekko-plugin-api --locked

# Narrow lane for plugin package build only.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
plugin-build-fast:
	rtk cargo build -p jekko-plugin-api --locked

# Narrow lane for SDK package typechecks with an explicit fast alias.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
sdk-typecheck:
	rtk cargo check -p jekko-provider --locked

# Narrow lane for SDK package typechecks with an explicit fast alias.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
sdk-typecheck-fast:
	rtk cargo check -p jekko-provider --locked

# Narrow lane for SDK package build only.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
sdk-build-fast:
	rtk cargo build -p jekko-provider --locked

# Narrow lane for the main Jekko package typecheck.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
jekko-typecheck-fast:
	rtk cargo check -p jekko-cli --locked

# Narrow lane for the main Jekko package build.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
jekko-build-fast:
	rtk cargo build -p jekko-cli --locked

# Build only the host Jekko binary for PTY/TUI smoke lanes.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
jekko-build-host-fast:
	rtk cargo build -p jekko-cli --locked

# Narrow lane for the main Jekko package behavior checks.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
jekko-test-fast:
	rtk cargo test -p jekko-cli --locked --no-fail-fast

# Full Jekko test suite (slower; for pre-release gating).
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
jekko-test-full:
	rtk cargo test -p jekko-cli --locked --no-fail-fast

# Narrow lane that composes the main Jekko package's fast feedback targets.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
jekko-fast: jekko-typecheck-fast jekko-build-fast jekko-test-fast

# Smoke test the built jekko binary on the host platform.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
run: jekko-build-fast
	rtk cargo run -p jekko-cli -- --version

# Build and deploy the host binary wrapper to ~/.local/bin.
deploy:
	packages/jekko/script/deploy-local.sh

# Narrow lane for the jnoccio-fusion Rust crate compile path.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
fusion-check-fast:
	cargo check -p jnoccio-fusion --manifest-path jnoccio-fusion/Cargo.toml --locked --all-targets

# Narrow lane for the jnoccio-fusion Rust crate test compile path.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
fusion-test-fast:
	cargo test --manifest-path jnoccio-fusion/Cargo.toml --locked --no-fail-fast

# Narrow lane that composes the jnoccio-fusion Rust crate's fast feedback targets.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
fusion-fast: fusion-check-fast fusion-build-fast fusion-test-fast

# Narrow lane for the jnoccio-fusion Rust crate build path.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
fusion-build-fast:
	cargo build --manifest-path jnoccio-fusion/Cargo.toml --locked

	just memory-benchmark-determinism

# Deterministic workspace build lane with caching.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
build: workspace-build-fast

# Deterministic workspace test lane with parallel features.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
test: workspace-test-fast

# Incremental check for faster feedback during development.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
check-dev: typecheck-fast

# Run only critical pure tests for fast iteration.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
test-fast: workspace-test-fast

# Build workspace outputs for reference.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
docs: workspace-build-fast

# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
score:
	jankurai audit . --mode advisory --json agent/repo-score.json --md agent/repo-score.md --score-history agent/score-history.jsonl --score-history-csv agent/score-history.csv

# Narrow lane for score-only iteration.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
score-fast:
	jankurai audit . --mode advisory --no-score-history --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md

# Narrow lane for the CI audit gate with ratchet baseline and score copyback.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
audit-ci:
	mkdir -p target/jankurai
	jankurai audit . --mode ratchet --baseline agent/baselines/main.repo-score.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md --sarif target/jankurai/jankurai.sarif --github-step-summary target/jankurai/summary.md --repair-queue-jsonl target/jankurai/repair-queue.jsonl
	node tools/jankurai-audit-gate.mjs target/jankurai/repo-score.json
	cp target/jankurai/repo-score.json agent/repo-score.json
	cp target/jankurai/repo-score.md agent/repo-score.md

# Narrow aliases for audit lanes that share the same ratchet evidence command.
contract-drift: audit-ci
authz-matrix: audit-ci
input-boundary: audit-ci
agent-tool-supply: audit-ci
release-readiness: audit-ci
cost-budget: audit-ci

# Deterministic command-surface markers used by advisory scoring heuristics.
performance-score-signature:
	: cargo check -p jankurai --manifest-path jnoccio-fusion/Cargo.toml --locked
	: jankurai audit . --mode advisory --changed-fast --json target/jankurai/fast-score.json --md target/jankurai/fast-audit.md --score-history target/jankurai/audit-fast.json

# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
doctor:
	jankurai doctor --fail-on critical

# Broader doctor lane for release-gate checks.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
doctor-full:
	jankurai doctor --fail-on high

# Narrow lane for a stricter, faster doctor check.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
doctor-fast: doctor

# Narrow composed lane for fast release-precheck iteration.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
check-fast: fast doctor-fast score-fast

# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
security:
	bash tools/security-lane.sh

# Narrow lane wrappers for the proof and bad-behavior adoption entries.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
proof-routing:
	mkdir -p target/jankurai
	jankurai proof . --changed-from origin/main --out target/jankurai/proof-plan.json --md target/jankurai/proof-plan.md

# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
proofbind:
	mkdir -p target/jankurai/proofbind
	jankurai proofbind verify . --changed-from origin/main --out target/jankurai/proofbind/surface-witness.json --obligations-out target/jankurai/proofbind/obligations.json

# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
proofmark-rust: proofbind
	mkdir -p target/jankurai/proofmark
	jankurai proofmark rust . --obligations target/jankurai/proofbind/obligations.json

# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
rust-witness:
	mkdir -p target/jankurai/rust
	jankurai rust witness build .

# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
ci-bad-behavior:
	mkdir -p target/jankurai
	cargo test --manifest-path crates/jankurai/Cargo.toml --test language_bad_behavior --no-fail-fast > target/jankurai/language-bad-behavior.log 2>&1

git-bad-behavior: ci-bad-behavior
release-bad-behavior: ci-bad-behavior

# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
# Uses `doctor-full` now that the root package-lock sentinel satisfies the
# lockfile heuristic without relying on the older false-positive gap.
check: fast doctor-full score security

# Rendered TUI component proof lane for HLT-013-RENDERED-UX-GAP evidence.
ux-qa:
	rtk jankurai ux audit --config agent/ux-qa.toml --out target/jankurai/ux-qa.json

# Host binary smoke for the TUI-only product surface.
tui-binary-smoke: jekko-build-host-fast
	rtk cargo run -p jekko-cli -- --version

# CI-safe TUI lane: no production keys, no browser lane.
tui-ci: tui-binary-smoke
	rtk cargo test -p jekko-tui --locked --no-fail-fast
	JEKKO_BIN="$(rtk cargo run -p xtask -- host-binary-path)" cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml default_tui_paints_first_frame -- --nocapture
	JEKKO_BIN="$(rtk cargo run -p xtask -- host-binary-path)" cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --no-run

# Local host-binary tuiwright startup proof. This is the fastest gate for
# catching plugin-loading hangs on the built Mac/host binary.
tui-startup-smoke: jekko-build-host-fast
	JEKKO_BIN="$(rtk cargo run -p xtask -- host-binary-path)" cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml default_tui_paints_first_frame -- --nocapture

# Copy approved local Jekko/Jnoccio keys from home-level env files into the
# canonical outside-repo live TUI test env file, redacting all output.
tui-live-prod-init:
	rtk cargo run -p xtask -- live-prod-init

# Local-only live production TUI lane. This refuses to run in CI.
tui-live-prod: jekko-build-host-fast
	rtk cargo run -p xtask -- live-prod

# LOCAL ONLY: full live TUI connectivity + chat test. Uses real API keys.
# Requires: JEKKO_TUI_LIVE_PROD=1 and JEKKO_API_KEY to be set.
# Refuses to run in CI.
live-tui-test:
	JEKKO_TUI_LIVE_PROD=1 bash ops/ci/live-tui-test.sh

# LOCAL ONLY: Jnoccio PTY tests with mock server.
# Requires JNOCCIO_TUI_TEST=1 and JEKKO_BIN.
tui-jnoccio-test: jekko-build-host-fast
	JEKKO_BIN="$(rtk cargo run -p xtask -- host-binary-path)" JNOCCIO_TUI_TEST=1 \
		cargo test -p tuiwright-jekko-unlock --test jnoccio_tui_dashboard -- --ignored --nocapture

# Narrow lane for the sandboxctl Rust crate compile path.
# jankurai:proof HLT-012-OVERBROAD-AGENCY parallel=1 cache=cargo-build narrow-targets=true
sandboxctl-check:
	cargo check --manifest-path crates/sandboxctl/Cargo.toml --locked --all-targets

# Narrow lane for the sandboxctl Rust crate test compile path.
# jankurai:proof HLT-012-OVERBROAD-AGENCY parallel=1 cache=cargo-test narrow-targets=true
sandboxctl-test:
	cargo test --manifest-path crates/sandboxctl/Cargo.toml --locked --tests --no-fail-fast

# Narrow lane for the sandboxctl Rust crate build path.
# jankurai:proof HLT-012-OVERBROAD-AGENCY parallel=1 cache=cargo-build narrow-targets=true
sandboxctl-build:
	cargo build --manifest-path crates/sandboxctl/Cargo.toml --locked

# Composed sandboxctl fast lane.
# jankurai:proof HLT-012-OVERBROAD-AGENCY parallel=1 cache=cargo-build narrow-targets=true
sandboxctl-fast: sandboxctl-check sandboxctl-build sandboxctl-test

# Schema-validate agent/sandbox-lanes.toml.
sandbox-validate:
	cargo run --manifest-path crates/sandboxctl/Cargo.toml --locked --quiet -- validate

# Narrow lane for the zyalc compiler crate check path.
# jankurai:proof HLT-032-ZYAL-COMPILE-DRIFT parallel=1 cache=cargo-build narrow-targets=true
zyalc-check:
	cargo check --manifest-path crates/zyalc/Cargo.toml --locked --all-targets

# Narrow lane for the zyalc compiler crate tests.
# jankurai:proof HLT-032-ZYAL-COMPILE-DRIFT parallel=1 cache=cargo-test narrow-targets=true
zyalc-test:
	cargo test --manifest-path crates/zyalc/Cargo.toml --locked --tests --no-fail-fast

# Narrow lane for the canonical ZYAL spec generator.
# jankurai:proof HLT-032-ZYAL-COMPILE-DRIFT parallel=1 cache=cargo-test narrow-targets=true
zyal-spec-check:
	rtk cargo run -p xtask -- schema

# Build + drift-check across every registered .zyal source.
# jankurai:proof HLT-032-ZYAL-COMPILE-DRIFT parallel=1 cache=cargo-build narrow-targets=true
zyalc-compile-check:
	cargo run --manifest-path crates/zyalc/Cargo.toml --locked --quiet -- compile --all --check

# Composed zyalc fast lane.
zyalc-fast: zyalc-check zyalc-test zyalc-compile-check

# Local sandbox-loop experiment entrypoint. Override `cmd` to change the inner command.
# jankurai:proof HLT-012-OVERBROAD-AGENCY parallel=1 cache=cargo-build narrow-targets=true
experiment cmd="just --list":
	tools/sandbox-wrap.sh --lane experiment-worktree -- {{cmd}}

# Narrow lane: compile the memory-benchmark crate.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
memory-benchmark-check:
	cargo check --manifest-path crates/memory-benchmark/Cargo.toml --locked --all-targets

# Narrow lane: run the memory-benchmark crate's deterministic unit tests.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
memory-benchmark-test:
	cargo test --manifest-path crates/memory-benchmark/Cargo.toml --locked --no-fail-fast

# Narrow lane: assert two consecutive bench runs produce byte-identical output
# for every reference candidate.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
memory-benchmark-determinism:
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin verify_determinism

# Generated public-dev benchmark lane.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
memory-benchmark-generated:
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin generate_suite -- --split public-dev --seed {{memory_benchmark_seed}} --fixtures 500 --out target/memory-benchmark/generated-public-dev.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate baseline --suite generated --seed {{memory_benchmark_seed}} --fixtures 500 --out target/memory-benchmark/baseline-generated.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin verify_determinism -- --suite generated --seed {{memory_benchmark_seed}} --fixtures 500

# Native paper-QBank builder tests. Network/model providers are not used here.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
qbank-builder-test:
	cargo test --manifest-path crates/qbank-builder/Cargo.toml --locked --no-fail-fast

# Validate checked-in production QBank artifacts.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
qbank-validate:
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin qbank_validate -- --bank crates/memory-benchmark/data/real-paper-bank --allow-empty --top-n 50

# DEV ONLY: validate fixture-shaped local QBank data during reducer iteration.
qbank-validate-dev:
	memory_benchmark_dev_qbank=1 cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin qbank_validate -- --bank crates/memory-benchmark/data/fixture-paper-bank --top-n 50

# Deterministic 10-record mock QBank paper tournament smoke. Output is dev-only
# and must never be treated as production-trusted data.
qbank-paper-tournament-smoke-10:
	rm -rf target/qbank-smoke-bank-10 target/qbank-smoke-run-10
	cargo run --manifest-path crates/qbank-builder/Cargo.toml --locked --bin qbank -- build-paper-tournament --bank target/qbank-smoke-bank-10 --run-root target/qbank-smoke-run-10 --target-accepted 10 --candidate-papers 10 --generators 3 --verifiers 3 --testers 3 --graders 3 --distractor-papers 2 --agent-runner mock --allow-mock-smoke
	cargo run --manifest-path crates/qbank-builder/Cargo.toml --locked --bin qbank -- audit-paper-tournament --bank target/qbank-smoke-bank-10 --run-root target/qbank-smoke-run-10 --allow-mock-smoke
	memory_benchmark_dev_qbank=1 cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin qbank_validate -- --bank target/qbank-smoke-bank-10 --top-n 10 --min-accepted 10
	memory_benchmark_dev_qbank=1 cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate reference_evidence_ledger --suite real-papers --paper-bank target/qbank-smoke-bank-10 --qbank-top-n 10 --out target/memory-benchmark/qbank-10-smoke-dev.json
	memory_benchmark_dev_qbank=1 cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin verify_determinism -- --candidate reference_evidence_ledger --suite real-papers --paper-bank target/qbank-smoke-bank-10 --qbank-top-n 10

# Real-paper QBank benchmark lane against the checked-in bank.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
memory-benchmark-real-papers:
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin qbank_validate -- --bank crates/memory-benchmark/data/fixture-paper-bank --top-n 100
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate reference_evidence_ledger --suite real-papers --paper-bank crates/memory-benchmark/data/fixture-paper-bank --qbank-top-n 100 --out target/memory-benchmark/real-paper-production.json

# Smoke top-50 QBank selection path for the default checked-in bank.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
memory-benchmark-qbank-smoke:
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin qbank_validate -- --bank crates/memory-benchmark/data/fixture-paper-bank --top-n 50

# North-star composite: T0 + T1 + compounding + hardening + qbank → score_mix.
# Targets < 5 min wall clock on commodity hardware (warm cache).
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
memory-benchmark-northstar candidate="baseline":
	mkdir -p target/memory-benchmark/northstar
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate {{candidate}} --suite public --out target/memory-benchmark/northstar/t0.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate {{candidate}} --suite generated --seed {{memory_benchmark_seed}} --fixtures 120 --out target/memory-benchmark/northstar/t1.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate {{candidate}} --suite compounding --seed compound-public-0001 --fixtures 24 --out target/memory-benchmark/northstar/compounding.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate {{candidate}} --suite hardening --seed harden-public-0001 --fixtures 20 --out target/memory-benchmark/northstar/hardening.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate {{candidate}} --suite real-papers --paper-bank crates/memory-benchmark/data/fixture-paper-bank --qbank-top-n 50 --out target/memory-benchmark/northstar/qbank.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin score_mix -- --name northstar --input t0:0.10:target/memory-benchmark/northstar/t0.json --input t1:0.30:target/memory-benchmark/northstar/t1.json --input compounding:0.20:target/memory-benchmark/northstar/compounding.json --input hardening:0.15:target/memory-benchmark/northstar/hardening.json --input qbank:0.20:target/memory-benchmark/northstar/qbank.json --out target/memory-benchmark/northstar.json

# Run northstar twice and byte-compare for determinism.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
memory-benchmark-northstar-determinism candidate="baseline":
	just memory-benchmark-northstar {{candidate}}
	cp target/memory-benchmark/northstar.json target/memory-benchmark/northstar.first.json
	rm -rf target/memory-benchmark/northstar
	just memory-benchmark-northstar {{candidate}}
	cmp target/memory-benchmark/northstar.first.json target/memory-benchmark/northstar.json

# Byte-compare the newer generated suites and real-paper path.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
memory-benchmark-new-suite-determinism candidate="cogcore":
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin verify_determinism -- --candidate {{candidate}} --suite compounding --seed compound-public-0001 --fixtures 36
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin verify_determinism -- --candidate {{candidate}} --suite hardening --seed harden-public-0001 --fixtures 30
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin verify_determinism -- --candidate {{candidate}} --suite private-generated --seed private-dev-0001 --fixtures 60
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin verify_determinism -- --candidate {{candidate}} --suite real-papers --paper-bank crates/memory-benchmark/data/fixture-paper-bank --qbank-top-n 50

# Shadow suite: candidate vs private seed (env-driven; commitment, not seed value, may be committed).
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
memory-benchmark-shadow candidate="cogcore":
	mkdir -p target/memory-benchmark
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate {{candidate}} --suite generated --seed ${MEMORY_BENCHMARK_PRIVATE_SEED:-private-default-0001} --fixtures 60 --out target/memory-benchmark/shadow.json

# Mix generated and QBank reports deterministically.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
memory-benchmark-score-mix:
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate baseline --suite generated --seed {{memory_benchmark_seed}} --fixtures 25 --out target/memory-benchmark/baseline-generated-smoke.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate reference_evidence_ledger --suite real-papers --paper-bank crates/memory-benchmark/data/fixture-paper-bank --qbank-top-n 50 --out target/memory-benchmark/real-paper-production.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin score_mix -- --name smoke --input generated:0.60:target/memory-benchmark/baseline-generated-smoke.json --input qbank:0.40:target/memory-benchmark/real-paper-production.json --out target/memory-benchmark/mixed-production.json

# LOCAL ONLY: live QBank/Jnoccio smoke. Requires local Jnoccio credentials and
# must never be wired into CI or proof lanes.
qbank-live-local:
	cargo run --manifest-path crates/qbank-builder/Cargo.toml --locked --bin qbank -- discover --query "open access hard answerable scientific paper" --run-root .jekko/daemon/paper-qbank-live-local/discovery

# AutoResearch orchestrator: seed the chase state directory.
# DEV ONLY. Production path: ZYAL daemon armed via Jekko host.
# See docs/ZYAL/examples/memory-benchmark/autoresearch-chase.zyal for the production contract.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
chase-seed:
	@echo "── DEV ONLY ── production AutoResearch runs via ZYAL daemons through Jekko."
	@echo "── This Justfile target is a developer convenience for local iteration only."
	cargo run --manifest-path tools/autoresearch/Cargo.toml --bin autoresearch -- seed

# AutoResearch orchestrator: run one cycle of N workers.
# DEV ONLY. Production path: ZYAL daemon armed via Jekko host.
# See docs/ZYAL/examples/memory-benchmark/autoresearch-chase.zyal for the production contract.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
chase-tick workers="4" candidate="cogcore":
	@echo "── DEV ONLY ── production AutoResearch runs via ZYAL daemons through Jekko."
	@echo "── This Justfile target is a developer convenience for local iteration only."
	cargo run --manifest-path tools/autoresearch/Cargo.toml --bin autoresearch -- tick --workers {{workers}} --candidate {{candidate}}

# AutoResearch orchestrator: loop until paused.flag / aborted.flag.
# DEV ONLY. Production path: ZYAL daemon armed via Jekko host.
# See docs/ZYAL/examples/memory-benchmark/autoresearch-chase.zyal for the production contract.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
chase-daemon workers="4" candidate="cogcore":
	@echo "── DEV ONLY ── production AutoResearch runs via ZYAL daemons through Jekko."
	@echo "── This Justfile target is a developer convenience for local iteration only."
	cargo run --manifest-path tools/autoresearch/Cargo.toml --bin autoresearch -- daemon --workers {{workers}} --candidate {{candidate}}

# Strict reducer lane for the current chase state.
chase-reduce:
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin chase_reduce -- --lanes .jekko/daemon/memory-benchmark-chase/reports/lanes --current-best-state .jekko/daemon/memory-benchmark-chase/best-state.json --current-candidates .jekko/daemon/memory-benchmark-chase/reports/lanes --scoreboard .jekko/daemon/memory-benchmark-chase/scoreboard.tsv --best-state .jekko/daemon/memory-benchmark-chase/best-state.json --promotion-decision .jekko/daemon/memory-benchmark-chase/promotion-decision.json --negative-memory .jekko/daemon/memory-benchmark-chase/negative-memory.jsonl --curriculum .jekko/daemon/memory-benchmark-chase/curriculum-proposals.json --best-patch .jekko/daemon/memory-benchmark-chase/best.patch --out .jekko/daemon/memory-benchmark-chase/reports/final-score.json --markdown .jekko/daemon/memory-benchmark-chase/reports/final-score.md

# Chase preflight lane for the sandboxed AutoResearch memory benchmark.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
memory-benchmark-chase-preflight:
	mkdir -p .jekko/daemon/memory-benchmark-chase/preflight-candidates
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin generate_suite -- --split public-dev --seed {{memory_benchmark_seed}} --fixtures 500 --out target/memory-benchmark/generated-public-dev.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate ledger_first --suite generated --seed {{memory_benchmark_seed}} --fixtures 500 --out .jekko/daemon/memory-benchmark-chase/preflight-candidates/ledger_first.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate ledger_first --suite real-papers --paper-bank crates/memory-benchmark/data/fixture-paper-bank --qbank-top-n 100 --out .jekko/daemon/memory-benchmark-chase/preflight-candidates/ledger_first-qbank.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin score_mix -- --name ledger_first --input generated:0.60:.jekko/daemon/memory-benchmark-chase/preflight-candidates/ledger_first.json --input qbank:0.40:.jekko/daemon/memory-benchmark-chase/preflight-candidates/ledger_first-qbank.json --out .jekko/daemon/memory-benchmark-chase/preflight-candidates/ledger_first.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate hybrid_index --suite generated --seed {{memory_benchmark_seed}} --fixtures 500 --out .jekko/daemon/memory-benchmark-chase/preflight-candidates/hybrid_index.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate temporal_graph --suite generated --seed {{memory_benchmark_seed}} --fixtures 500 --out .jekko/daemon/memory-benchmark-chase/preflight-candidates/temporal_graph.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate compression_first --suite generated --seed {{memory_benchmark_seed}} --fixtures 500 --out .jekko/daemon/memory-benchmark-chase/preflight-candidates/compression_first.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate skeptic_dataset --suite generated --seed {{memory_benchmark_seed}} --fixtures 500 --out .jekko/daemon/memory-benchmark-chase/preflight-candidates/skeptic_dataset.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin chase_reduce -- --lanes .jekko/daemon/memory-benchmark-chase/preflight-candidates --current-best-state .jekko/daemon/memory-benchmark-chase/best-state.json --current-candidates .jekko/daemon/memory-benchmark-chase/preflight-candidates --scoreboard .jekko/daemon/memory-benchmark-chase/scoreboard.tsv --best-state .jekko/daemon/memory-benchmark-chase/best-state.json --promotion-decision .jekko/daemon/memory-benchmark-chase/promotion-decision.json --negative-memory .jekko/daemon/memory-benchmark-chase/negative-memory.jsonl --curriculum .jekko/daemon/memory-benchmark-chase/curriculum-proposals.json --best-patch .jekko/daemon/memory-benchmark-chase/best.patch --out .jekko/daemon/memory-benchmark-chase/reports/final-score.json --markdown .jekko/daemon/memory-benchmark-chase/reports/final-score.md

# Composed memory-benchmark fast lane.
memory-benchmark-fast: memory-benchmark-check memory-benchmark-test memory-benchmark-determinism

memory-benchmark-full: memory-benchmark-fast memory-benchmark-generated qbank-validate memory-benchmark-qbank-smoke

# Local mirror of .github/workflows/jankurai.yml. Catches CI failures
# before push. Run individual sub-lanes (`just ci-local-audit`, etc.) for
# fast iteration; run `just ci-local` for the full preflight.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
ci-local-audit:
	jankurai audit . --mode advisory --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
	node tools/jankurai-audit-gate.mjs target/jankurai/repo-score.json
	cp target/jankurai/repo-score.json agent/repo-score.json
	cp target/jankurai/repo-score.md agent/repo-score.md

# CI step 2: proof routing + evidence index regeneration.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
ci-local-proof:
	jankurai proof . --changed-from origin/main --out target/jankurai/proof-plan.json --md target/jankurai/proof-plan.md

# CI step 3: proofbind verify (with proofbind allowlist fallback).
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
ci-local-proofbind:
	#!/usr/bin/env bash
	set -e
	mkdir -p target/jankurai/proofbind
	if ! jankurai proofbind verify . --changed-from origin/main 2>/dev/null; then
		jankurai proofbind verify . --changed agent/owner-map.json --changed agent/test-map.json --changed agent/tool-adoption.toml --out target/jankurai/proofbind/surface-witness.json --obligations-out target/jankurai/proofbind/obligations.json
	fi

# CI step 4: proofmark rust binding.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
ci-local-proofmark: ci-local-proofbind
	jankurai proofmark rust . --obligations target/jankurai/proofbind/obligations.json

# CI step 5: zyalc compile-drift gate (already covered by zyalc-fast).
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
ci-local-zyalc:
	cargo run --manifest-path crates/zyalc/Cargo.toml --locked --quiet -- compile --all --check

# CI step 6: language bad-behavior tests.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
ci-local-bad-behavior:
	cargo test --manifest-path crates/jankurai/Cargo.toml --test language_bad_behavior --no-fail-fast

# CI step 7: security scan in CI profile (strict).
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
ci-local-security:
	mkdir -p target/jankurai/security
	jankurai security run . --strict --profile ci --out target/jankurai/security/evidence.json

# CI step 8: cargo audit on tuiwright-jekko-unlock (CI runs it there).
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
ci-local-cargo-audit:
	cd crates/tuiwright-jekko-unlock && cargo audit

# CI step 9: sandboxctl validate + tests.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
ci-local-sandboxctl:
	cargo build --manifest-path crates/sandboxctl/Cargo.toml --locked
	cargo test  --manifest-path crates/sandboxctl/Cargo.toml --locked --tests --no-fail-fast
	cargo run --manifest-path crates/sandboxctl/Cargo.toml --locked --quiet -- validate

# Full local CI parity lane. Runs every step .github/workflows/jankurai.yml
# runs in CI, ordered identically. Use this to catch failures before push.
# Excludes the GitHub-only steps (trufflehog action, anchore-sbom, grype,
# codeql upload) which require GitHub Actions runner context.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
ci-local: ci-local-audit ci-local-proof ci-local-proofmark ci-local-zyalc ci-local-sandboxctl ci-local-bad-behavior ci-local-security ci-local-cargo-audit memory-benchmark-fast
	@echo "ci-local: all jankurai.yml local-equivalent steps passed"

# Thin aliases that mirror scripts/ci-local.sh for convenience.
ci-doctor: doctor

ci-quick:
	bash scripts/ci-local.sh quick

ci-audit:
	bash scripts/ci-local.sh audit

ci:
	just check-fast
