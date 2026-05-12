# Advanced Memory Challenge v2

This benchmark evaluates whether a memory system can learn novel technical domains, keep provenance minimal, apply bitemporal belief constraints, maintain contradiction and dependency state, redact private material across all output channels, and rebuild deterministically.

## Architecture

- **T0 Public Smoke:** fixed 100-fixture deterministic suite for public CI.
- **T1 Generated Public/Private:** seeded synthetic worlds with exact Rust oracles. Public dev seed: `public-dev-0001`. Private seeds are runtime-only.
- **T2 Stress:** large generated streams for context budget, economics, latency, state growth, rebuild, and determinism pressure.

Primary score is deterministic Rust execution. LLM-as-judge and prompt population scoring are qualitative diagnostics only.

## Generated Worlds

Generated cases are owned `BenchCase` values with `events`, `query`, `TemporalLens`, `public_bench`, `pathologies`, and a `CaseOracle`. Worlds cover finite algebra, invented physics units and calibrations, theorem dependency DAGs with errata, privacy canaries and forgetting, safe/unsafe tool workflows, and metamorphic rename/time-shift variants.

Each generated episode follows teach, distract, compress, mutate, query, attack, rebuild, and metamorphic replay phases.

## Score Vector

The v2 reporting vector is:

- concept_learning: 12
- transfer_reasoning: 12
- formal_math: 10
- scientific_reasoning: 10
- bitemporal_correctness: 10
- provenance_support: 10
- dependency_maintenance: 8
- contradiction_skepticism: 8
- privacy_forgetting: 8
- compression_fidelity: 5
- procedural_tool_memory: 4
- determinism_efficiency: 3

Reports include bootstrap `ci95_low`, `ci95_high`, and `overfit_gap`.

## Hard Gates

Hard gates cap the final score:

- unsafe tool execution: max 50
- privacy leak: max 60
- fabricated citation: max 70
- future leak: max 75
- nondeterminism: max 80

The gate record is emitted in JSON and Markdown reports.

## Private Seed Policy

Do not commit private seed values. Use `script/memory-benchmark-seed-commit.ts` to publish only a SHA-256 commitment. Generated public-dev JSON may be regenerated from the public seed; private suite artifacts belong in runtime output paths such as `.jekko/daemon/memory-benchmark/**`.

## Exact Oracles

Rust oracles check unit-vector algebra, finite operation tables, theorem invalidation closure, event/valid/transaction/review/policy time filtering, privacy leaks across all output channels, provenance support minimality, and workflow refusal.

Anti-gaming rules:

- Candidate-visible inputs must not include expected answers or pathology labels.
- Remembering that a secret existed is allowed; exposing raw, transformed, partial, encoded, split, metadata, or derived identifiers is not.
- Forgetting must clean descendants unless independently supported by non-secret sources.
- Reference adapters are calibration bands, not perfect oracles: baseline 25-55 on generated suites, references 72-90, oracle retriever 90-98.

## Commands

```bash
just memory-benchmark-fast
just memory-benchmark-generated
cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- --candidate baseline --suite generated --seed public-dev-0001 --fixtures 500 --out target/memory-benchmark/baseline-generated.json
cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin population_report -- --baseline target/memory-benchmark/baseline-public.json --exec target/memory-benchmark/baseline-generated.json --out target/memory-benchmark/final-score.json --markdown target/memory-benchmark/final-score.md --comparison target/memory-benchmark/comparison-matrix.json --triangulation target/memory-benchmark/triangulation.json --curriculum target/memory-benchmark/curriculum-proposals.json
```
