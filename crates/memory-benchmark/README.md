# memory-benchmark

Deterministic Rust harness for advanced memory systems. Primary scoring is executable Rust oracle scoring; LLM or population judging is diagnostic only.

## Tiers

- **T0 public smoke:** fixed 100-fixture suite in `src/fixture/data.rs`.
- **T1 generated:** seeded synthetic math, science, theorem, privacy, and workflow cases with machine-checkable oracles.
- **T2 stress:** same generator path at larger event/query counts for latency, context, rebuild, and state growth checks.

## Quick Start

```bash
cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- \
  --candidate baseline --suite public --out target/memory-benchmark/baseline-public.json

cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin generate_suite -- \
  --split public-dev --seed public-dev-0001 --fixtures 500 \
  --out target/memory-benchmark/generated-public-dev.json

cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- \
  --candidate baseline --suite generated --seed public-dev-0001 --fixtures 500 \
  --out target/memory-benchmark/baseline-generated.json

cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin verify_determinism -- \
  --suite generated --seed public-dev-0001 --fixtures 500
```

Repo routes:

```bash
just memory-benchmark-fast
just memory-benchmark-generated
just memory-benchmark-full
```

## Candidates

- `baseline`: deliberately weak vector scan.
- `reference_context_pack`: budgeted context pack with bitemporal filtering and redaction.
- `reference_evidence_ledger`: ledger-oriented reference with modality lifecycle changes.
- `reference_claim_skeptic`: contradiction-first reference.
- `ledger_first`, `hybrid_index`, `temporal_graph`, `compression_first`, `skeptic_dataset`: candidate lanes with different tradeoffs.

Implement `MemorySystem`, register the adapter in `runner.rs`, and verify with `bench --candidate <name>`.

## Generated Suites

Generated cases use `GeneratedSuiteConfig { split, seed_label, fixture_count, difficulty }`. Public development seed labels may be committed. Private seed values must not be committed; commit only a SHA-256 commitment from:

```bash
bun script/memory-benchmark-seed-commit.ts "$PRIVATE_SEED"
```

## Reports

`bench` emits canonical JSON with fixture scores plus hard-gate and bootstrap CI data for generated suites. `population_report` writes `final-score.json`, `final-score.md`, `axis-breakdown.json`, `gate-findings.json`, `support-minimality.json`, `privacy-audit.json`, `economics.json`, `bootstrap-ci.json`, `comparison-matrix.json`, `triangulation.json`, and `curriculum-proposals.json` when requested. In chase mode, it also emits `scoreboard.tsv`, `best-state.json`, `promotion-decision.json`, `negative-memory.jsonl`, and `best.patch` from lane reports.

## Determinism

No benchmark hot path uses wall-clock time, network, process randomness, or external dependencies. JSON keys are sorted, FNV-1a hashes are stable, and `verify_determinism` byte-compares repeated runs.
