# refs/snapshot.md — Implementation Snapshot

Captured after Phase 1-5 implementation and refreshed after the 2026-05-13 Track A hardening pass in commit `2617e2a1b`. Numbers below come from running each lane on a development machine with the toolchain pinned at Rust 1.95.0.

Correction note: the earlier 90.65 cogcore northstar and 100.00 hardening score were pre-Track-A values. The current benchmark uses reinforce-between-query hardening semantics, production QBank missing-paper failure, fresh AutoResearch references, absolute reference drift, and dev-only promotion rejection.

## Test counts

| Suite | Count |
|---|---:|
| `cargo test --manifest-path crates/memory-benchmark/Cargo.toml --locked --no-fail-fast` | 88 |
| `cargo test --manifest-path crates/cogcore/Cargo.toml --locked --no-fail-fast` | 30 |
| `cargo test --manifest-path tools/autoresearch/Cargo.toml --locked --no-fail-fast` | 3 |

## Northstar composite (T0 0.10 + T1 0.30 + Compounding 0.20 + Hardening 0.15 + QBank 0.20)

| Candidate | Northstar | Within band? |
|---|---:|---|
| baseline | 73.31 | yes — [25, 75] |
| reference_context_pack | 83.13 | yes — [70, 90] |
| reference_evidence_ledger | 83.00 | yes — [70, 90] |
| reference_claim_skeptic | 82.88 | yes — [70, 90] |
| **cogcore** | **77.63** | below references; honest Track B target |

## Per-suite scores

cogcore:
- T0 (PublicSmoke, 100 fixtures): 91.21
- T1 (PublicGenerated, 120 fixtures): 100.00
- Compounding (24 fixtures, 6 fixture-kinds): 80.00
- Hardening (20 fixtures): 10.00
- QBank real-papers (50 fixture challenges): 85.64, `dev_only:true`

## Determinism

- `just memory-benchmark-fast`: OK; all four reference adapters verify cleanly.
- `just memory-benchmark-new-suite-determinism cogcore`: OK for compounding, hardening, private-generated, and real-papers dev mode.
- `just memory-benchmark-northstar-determinism cogcore`: OK; two northstar runs byte-compare equal.

## AutoResearch tick

```bash
rtk cargo run --manifest-path tools/autoresearch/Cargo.toml --bin autoresearch -- seed --state-dir .jekko/daemon/memory-benchmark-chase-review
rtk cargo run --manifest-path tools/autoresearch/Cargo.toml --bin autoresearch -- tick --workers 1 --candidate cogcore --state-dir .jekko/daemon/memory-benchmark-chase-review --use-dirty-source-dev-only
```

Cycle outputs:
- `receipts/0000000.json` — receipt with `attempted`, `best_total`, `median_total`, `candidate`, `dev_only:true`, and `reference_report_count:3`
- `best-state.json` — unchanged current baseline state
- `scoreboard.tsv` — appended one line per cycle
- `reports/lanes/lane_NN/{northstar.json, proposal.json}` — per-worker output
- `promotion-decision.json` — `decision:"reject"`, raw top `dev_only:true`, eligible lanes 0
- `reports/shadow.json` — dev-only shadow report
- `reports/references/0000000/{reference_context_pack,reference_evidence_ledger,reference_claim_skeptic}.json` — fresh per-cycle references

## Files shipped (Phase 1-4)

### Phase 1 — cogcore skeleton

```
crates/cogcore/
├── Cargo.toml
├── rust-toolchain.toml
├── src/
│   ├── lib.rs
│   ├── core.rs           # Phase 2 rewrite (real engine)
│   ├── hash.rs
│   ├── time.rs
│   └── canary.rs
└── tests/
    ├── trait_smoke.rs
    └── benchmark_smoke.rs
```

### Phase 2 — cogcore real

```
crates/cogcore/src/
├── ledger.rs    # WAL append-only with Observe/Tombstone/Feedback/RecallTouch
├── index.rs     # BM25-lite inverted index + MinHash sketches
├── hebb.rs      # Sparse co-activation matrix
├── fsrs.rs      # Per-cell / per-topic half-life
├── concept.rs   # Concept + Topic types + attachment threshold
└── topic.rs     # Topic strength formula
```

`crates/memory-benchmark/src/adapters/cogcore_adapter.rs` translates `MemorySystem` ↔ `cogcore::Core`.

### Phase 3 — 12-axis benchmark

```
crates/memory-benchmark/src/
├── scoring/axes.rs        # +compounding (10), +topic_hardening (8); sum = 100
├── scoring/gates.rs       # +compounding_regression, +hardening_regression,
│                          # +knowledge_non_degradation gates
├── scorer.rs              # scorer::compounding + scorer::topic_hardening
├── case.rs                # +Split::PublicCompounding, +Split::PublicHardening,
│                          # +OracleKind::Compounding, +OracleKind::Hardening
├── runner.rs              # 12-axis accumulator
├── runner_generated.rs    # Dispatch new suites
├── runner_support.rs      # --suite compounding|hardening|private-generated
├── memory_api.rs          # axes_to_json includes 2 new fields
├── corpus/real_papers/score.rs  # AxisScores struct update
├── generated/mod.rs       # Re-exports
├── generated/compounding.rs  # 6 fixture-kinds; seed: compound-public-0001
└── generated/hardening.rs    # 5-event reinforcement; seed: harden-public-0001
```

Justfile targets:
- `memory-benchmark-northstar candidate=baseline` — full composite
- `memory-benchmark-northstar-determinism` — runs twice and byte-compares
- `memory-benchmark-shadow` — private-seed suite

### Phase 4 — AutoResearch orchestrator

```
tools/autoresearch/
├── Cargo.toml
├── rust-toolchain.toml
└── src/
    ├── main.rs              # Subcommands: seed, tick, daemon, forensics
    └── proposer/
        ├── mod.rs
        └── genetic.rs       # Deterministic Gaussian perturbation proposer
```

Justfile targets:
- `chase-seed` — initialize chase state directory
- `chase-tick workers=N candidate=NAME` — one cycle
- `chase-daemon workers=N candidate=NAME` — loop until pause/abort flag

State directory: `.jekko/daemon/memory-benchmark-chase/`
- `best-state.json`
- `negative-memory.jsonl`
- `scoreboard.tsv`
- `receipts/<cycle_id>.json`
- `reports/lanes/lane_NN/northstar.json` + `proposal.json`

## Calibration check (Phase 3 axis trim)

Before: 10 axes summing to 100, references in [70, 90].
After: 12 axes (correctness 14, provenance 10, math_science 12, bitemporal_recall 10, contradiction 8, english_discourse_coreference 6, privacy_redaction 8, procedural_skill 4, feedback_adaptation 4, determinism_rebuild 6, compounding 10, topic_hardening 8 = 100). All four references stay in [70, 90] on northstar.

## Performance (development machine, warm cache)

Single `memory-benchmark-northstar candidate=cogcore` run: ~1 second (cargo cache warm). Cold compile adds ~30-60 seconds. Total well under the 5-minute wall-clock budget.

## What is NOT yet implemented (deferred to follow-up phases)

- T2/T3/T4 mutation proposers (only T1 hyperparameter sweep ships).
- Real-paper QBank trust: checked-in bank has 50 fixture challenges but no redistributable paper JSON, so production validation fails and dev fixture mode is required.
- Non-dev AutoResearch promotion: reducer rejects dev-only lanes; latest dry run correctly rejected promotion.
- Track B cogcore capability recovery: hardening is 10.00 and compounding is 80.00.
- LLM-based T4 proposer with negative-memory prompt construction.
- Disk-backed WAL (in-memory only).
- Concept emergence is invoked offline via `consolidate()` but never called by the
  benchmark hot path; topic strength formula is implemented but topics are not
  yet auto-created from concept communities.
- Paper ingestion equation/theorem parsers (deferred; current ingestion stores raw
  Event bodies and lets BM25 + concept attachment cluster naturally).

These are tracked in `06-roadmap.md` "Phase 6+".
