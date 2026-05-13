# Codex Postcompletion Review — 2026-05-13

## Verdict

The prior "100% finished" claim is not valid. The benchmark and chase loop had several validity gaps that could make inflated scores or unsafe AutoResearch promotion look acceptable.

## Starting State

Initial dirty worktree had memory-benchmark, cogcore, AutoResearch, Justfile, and smartmemory changes already present. Codex first cleaned and committed that state as:

- `2d256b050 Add memory benchmark chase infrastructure`

After the user demanded completion of the original audit plan, work resumed with Claude as an equal collaborator through `AGENT_CHAT.md`.

## Confirmed Gaps

- Generated hardening was not a true timestep suite: reinforcements were effectively pre-observed and repeated recalls hit the same adapter state.
- Compounding cases needed explicit multi-query/control semantics.
- The checked-in QBank is fixture-only and lacks redistributable paper JSON files.
- Production QBank could fabricate paper content from answer keys.
- Reducer reference drift divided score-point deltas by `100.0`.
- Reducer trusted-core protection was based on patch presence rather than forbidden-path inspection.
- AutoResearch orchestration used stale root reference reports, naive top-level score parsing, string-spliced JSON wrapping, and unsafe dirty-source worktree sync.
- `chase-daemon` remains unsafe to arm until the full reducer/orchestrator/shadow/QBank gates pass.

## Work Split

Claude claimed and completed the reducer and generated-runner safety items in:

- `crates/memory-benchmark/src/chase_report.rs`
- `crates/memory-benchmark/src/runner_generated.rs`
- `crates/memory-benchmark/src/bin/verify_determinism.rs`
- `crates/memory-benchmark/tests/hardening_timesteps.rs`
- `MEMORY_SYSTEM_LEVELUP.md`

Codex claimed QBank, AutoResearch orchestration, and this receipt trail:

- `crates/memory-benchmark/src/corpus/real_papers/*`
- `crates/memory-benchmark/tests/real_papers.rs`
- `crates/memory-benchmark/src/bin/qbank_validate.rs`
- `tools/autoresearch/**`
- `smartmemory/10-codex-postcompletion-review.md`
- `smartmemory/refs/codex-review-2026-05-13.md`

## QBank Status

QBank is explicitly `dev_only` until real redistributable papers and support section hashes exist.

Implemented:

- Production missing-paper fallback removed from `observe_paper`.
- Fixture-paper fallback requires `memory_benchmark_dev_qbank=1`.
- Real-paper reports include `dev_only` and `qbank_trusted`.
- `qbank_validate` emits `dev_only`.
- Validation requires redistributable paper JSON and support hashes unless dev mode is explicit.

Validation receipts:

- `rtk cargo test --manifest-path crates/memory-benchmark/Cargo.toml --locked real_papers --no-fail-fast -- --test-threads=1` passed: 9 tests, 70 filtered.
- `memory_benchmark_dev_qbank=1 rtk cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin qbank_validate -- --bank crates/memory-benchmark/data/real-paper-bank --top-n 50` passed with `"dev_only":true`.
- `rtk cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin qbank_validate -- --bank crates/memory-benchmark/data/real-paper-bank --top-n 50` failed as intended with 50 missing paper JSON errors.

## Stop Conditions

Stop conditions still active:

- Do not arm `chase-daemon`.
- Do not treat the checked-in QBank as trusted.
- Do not promote AutoResearch candidates from `dev_only` runs.
- Do not tune cogcore for score until hardening, compounding, QBank, reducer, and AutoResearch gates are green.

## Next Required Proof

Completed before handoff:

- `rtk cargo test --manifest-path crates/memory-benchmark/Cargo.toml --locked --no-fail-fast` passed.
- `rtk cargo test --manifest-path crates/cogcore/Cargo.toml --locked --no-fail-fast` passed.
- `rtk cargo test --manifest-path tools/autoresearch/Cargo.toml --locked --no-fail-fast` passed.
- `rtk just memory-benchmark-fast` passed.
- `rtk just memory-benchmark-new-suite-determinism cogcore` passed.
- Baseline/reference/cogcore north-star totals were collected.
- AutoResearch seed plus one dirty-source dev-only tick completed and rejected promotion.

North-star totals:

- Baseline: 73.3055.
- `reference_context_pack`: 83.1294.
- `reference_evidence_ledger`: 83.0029.
- `reference_claim_skeptic`: 82.8767.
- `cogcore`: 77.6303.

## Handoff Note

This receipt is intentionally conservative. Any current `dev_only` QBank or dirty-source AutoResearch receipt is non-promotable evidence only. `chase-daemon` is not safe to arm.
