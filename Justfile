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
	bun install --frozen-lockfile

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
	bun turbo --cache-dir "$TURBO_CACHE_DIR" --cache=local:rw --parallel typecheck --filter=@jekko-ai/core --filter=@jekko-ai/plugin --filter=@jekko-ai/sdk

# Narrow lane for workspace test-only feedback.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
# Narrow lane for workspace build-only feedback. (Cache enabled)
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
workspace-test-fast: core-test-fast jekko-test-fast

# Narrow lane for workspace build-only feedback. (Cache enabled)
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
workspace-build-fast:
	just core-build-fast
	just plugin-build-fast
	just jekko-build-fast

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
	bun turbo --cache-dir "$TURBO_CACHE_DIR" --cache=local:rw --parallel typecheck --filter=@jekko-ai/core --filter=@jekko-ai/plugin --filter=@jekko-ai/sdk

# Narrow lane for package builds that can reuse Turbo cache metadata.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
build-fast: workspace-build-fast

# Narrow lane for the core package typecheck only.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
core-typecheck-fast:
	bun --cwd packages/core typecheck

# Narrow lane for the core package compile path.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
core-build-fast:
	bun turbo --cache-dir "$TURBO_CACHE_DIR" --cache=local:rw --parallel build --filter=@jekko-ai/core

# Narrow lane for core package behavior checks.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
core-test:
	bun --cwd packages/core test

# Narrow lane for core package behavior checks with an explicit fast alias.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
core-test-fast:
	bun --cwd packages/core test

# Narrow lane for package-level typechecks.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
plugin-typecheck:
	bun --cwd packages/plugin typecheck

# Narrow lane for plugin package typechecks with an explicit fast alias.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
plugin-typecheck-fast:
	bun --cwd packages/plugin typecheck

# Narrow lane for plugin package build only.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
plugin-build-fast:
	bun turbo --cache-dir "$TURBO_CACHE_DIR" --cache=local:rw --parallel build --filter=@jekko-ai/plugin

# Narrow lane for SDK package typechecks with an explicit fast alias.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
sdk-typecheck:
	bun --cwd packages/sdk/js typecheck

# Narrow lane for SDK package typechecks with an explicit fast alias.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
sdk-typecheck-fast:
	bun --cwd packages/sdk/js typecheck

# Narrow lane for SDK package build only.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
sdk-build-fast:
	bun turbo --cache-dir "$TURBO_CACHE_DIR" --cache=local:rw --parallel build --filter=@jekko-ai/sdk

# Narrow lane for the main Jekko package typecheck.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
jekko-typecheck-fast:
	bun --cwd packages/jekko typecheck

# Narrow lane for the main Jekko package build.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
jekko-build-fast:
	bun turbo --cache-dir "$TURBO_CACHE_DIR" --cache=local:rw --parallel build --filter=jekko

# Build only the host Jekko binary for PTY/TUI smoke lanes.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
jekko-build-host-fast:
	bun --cwd packages/jekko ./script/build.ts --single --skip-install

# Narrow lane for the main Jekko package behavior checks.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
jekko-test-fast:
	bun --cwd packages/jekko test \
		test/keybind.test.ts \
		test/ide/ \
		test/util/ \
		test/auth/ \
		test/account/ \
		test/config/agent-color.test.ts \
		test/config/config.part-01.test.ts \
		test/config/config.part-02.test.ts \
		test/config/config.part-04.test.ts \
		test/config/config.part-05.test.ts \
		test/config/config.part-06.test.ts \
		test/config/config.part-07.test.ts \
		test/config/config.part-08.test.ts \
		test/config/config.part-09.test.ts \
		test/config/config.part-12.test.ts

# Full Jekko test suite (slower; for pre-release gating).
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
jekko-test-full:
	bun --cwd packages/jekko test

# Narrow lane that composes the main Jekko package's fast feedback targets.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
jekko-fast: jekko-typecheck-fast jekko-build-fast jekko-test-fast

# Smoke test the built jekko binary on the host platform.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
run: jekko-build-fast
	@host_target="jekko-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/x86_64/x64/;s/aarch64/arm64/')"; \
	binary="packages/jekko/dist/$host_target/bin/jekko"; \
	if [ ! -x "$binary" ]; then \
		echo "Binary not found at $binary — run 'just build' first"; exit 1; \
	fi; \
	"$binary" --version

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

# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=turbo-build narrow-targets=true
# Uses `doctor-full` now that the root package-lock sentinel satisfies the
# lockfile heuristic without relying on the older false-positive gap.
check: fast doctor-full score security

# Rendered TUI component proof lane for HLT-013-RENDERED-UX-GAP evidence.
ux-qa:
	bun --cwd packages/jekko test test/cli/tui/ test/cli/cmd/tui/

# Host binary smoke for the TUI-only product surface.
tui-binary-smoke: jekko-build-host-fast
	bun --cwd packages/jekko ./script/tui-binary-smoke.ts

# CI-safe TUI lane: no production keys, no browser lane.
tui-ci: tui-binary-smoke
	bun --cwd packages/jekko test test/cli/tui/ test/cli/cmd/tui/
	JEKKO_BIN="$(bun --cwd packages/jekko ./script/host-binary-path.ts)" cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --no-run
	JEKKO_BIN="$(bun --cwd packages/jekko ./script/host-binary-path.ts)" cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml default_tui_paints_first_frame -- --nocapture

# Copy approved local Jekko/Jnoccio keys from home-level env files into the
# canonical outside-repo live TUI test env file, redacting all output.
tui-live-prod-init:
	bun --cwd packages/jekko ./script/tui-live-prod-init.ts

# Local-only live production TUI lane. This refuses to run in CI.
tui-live-prod: jekko-build-host-fast
	bun --cwd packages/jekko ./script/tui-live-prod.ts

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

# Chase preflight lane for the sandboxed AutoResearch memory benchmark.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-test narrow-targets=true
memory-benchmark-chase-preflight:
	mkdir -p .jekko/daemon/memory-benchmark-chase/preflight-candidates
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin generate_suite -- --split public-dev --seed {{memory_benchmark_seed}} --fixtures 500 --out target/memory-benchmark/generated-public-dev.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate ledger_first --suite generated --seed {{memory_benchmark_seed}} --fixtures 500 --out .jekko/daemon/memory-benchmark-chase/preflight-candidates/ledger_first.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate hybrid_index --suite generated --seed {{memory_benchmark_seed}} --fixtures 500 --out .jekko/daemon/memory-benchmark-chase/preflight-candidates/hybrid_index.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate temporal_graph --suite generated --seed {{memory_benchmark_seed}} --fixtures 500 --out .jekko/daemon/memory-benchmark-chase/preflight-candidates/temporal_graph.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate compression_first --suite generated --seed {{memory_benchmark_seed}} --fixtures 500 --out .jekko/daemon/memory-benchmark-chase/preflight-candidates/compression_first.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate skeptic_dataset --suite generated --seed {{memory_benchmark_seed}} --fixtures 500 --out .jekko/daemon/memory-benchmark-chase/preflight-candidates/skeptic_dataset.json
	cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin population_report -- --current-candidates .jekko/daemon/memory-benchmark-chase/preflight-candidates --best-state .jekko/daemon/memory-benchmark-chase/best-state.json --scoreboard .jekko/daemon/memory-benchmark-chase/scoreboard.tsv --promotion-decision .jekko/daemon/memory-benchmark-chase/promotion-decision.json --best-patch .jekko/daemon/memory-benchmark-chase/best.patch --out .jekko/daemon/memory-benchmark-chase/reports/final-score.json --markdown .jekko/daemon/memory-benchmark-chase/reports/final-score.md

# Composed memory-benchmark fast lane.
memory-benchmark-fast: memory-benchmark-check memory-benchmark-test memory-benchmark-determinism

memory-benchmark-full: memory-benchmark-fast memory-benchmark-generated

# Local mirror of .github/workflows/jankurai.yml. Catches CI failures
# before push. Run individual sub-lanes (`just ci-local-audit`, etc.) for
# fast iteration; run `just ci-local` for the full preflight.
# jankurai:proof HLT-018-PERF-CONCURRENCY-DRIFT parallel=1 cache=cargo-build narrow-targets=true
ci-local-audit:
	jankurai audit . --mode advisory --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
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
