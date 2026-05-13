# 03 — Benchmark: 12-Axis North-Star

Extending `crates/memory-benchmark/` from 10 axes to 12. User chose **"Extend to 12 axes"** (vs side composite), so the trusted core gets edited. This file specifies every change.

## 1. New axis weights (sum exactly to 100)

```rust
// crates/memory-benchmark/src/scoring/axes.rs

impl AxisScores {
    pub const WEIGHTS: AxisScores = AxisScores {
        correctness: 14.0,                    // was 20
        provenance: 10.0,                     // was 12
        bitemporal_recall: 10.0,
        contradiction: 8.0,                   // was 10
        math_science: 12.0,
        english_discourse_coreference: 6.0,   // was 8
        privacy_redaction: 8.0,
        procedural_skill: 4.0,                // was 8
        feedback_adaptation: 4.0,             // was 6
        determinism_rebuild: 6.0,
        compounding: 10.0,                    // NEW
        topic_hardening: 8.0,                 // NEW
    };
}
```

Total: 14+10+10+8+12+6+8+4+4+6+10+8 = **100**.

### Why these trims

- **correctness 20→14**: was dominant; still the largest. Trim accounts for compounding/hardening absorbing some of "did the system answer well" semantics.
- **provenance 12→10**: small trim; still 4th.
- **contradiction 10→8**: less-frequently-exercised in T0; the new `poisoned_paper` fixture-kind in compounding overlaps with it.
- **english_discourse_coreference 8→6**: historically the lightest-touched axis (most fixtures contribute `None` to it).
- **procedural_skill 8→4**: similar — light-touched.
- **feedback_adaptation 6→4**: light-touched.

The trimmed axes still register their pathologies via hard gates and the per-fixture `Option<f32>` exclusion. Calibration drift on the 4 references must be measured (Phase 3 verification).

### Update tests in `src/lib.rs`

```rust
#[test]
fn axis_weights_sum_to_100() {
    let w = AxisScores::WEIGHTS;
    let s = w.correctness + w.provenance + w.bitemporal_recall + w.contradiction
        + w.math_science + w.english_discourse_coreference + w.privacy_redaction
        + w.procedural_skill + w.feedback_adaptation + w.determinism_rebuild
        + w.compounding + w.topic_hardening;
    assert!((s - 100.0).abs() < 0.001, "weights sum to {}", s);
}
```

The existing `advanced_axis_weights_sum_to_100` test is unchanged (12-element `ADVANCED_WEIGHTS` still sums to 100 — it's a separate rollup).

## 2. AxisScores struct

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AxisScores {
    pub correctness: f32,
    pub provenance: f32,
    pub bitemporal_recall: f32,
    pub contradiction: f32,
    pub math_science: f32,
    pub english_discourse_coreference: f32,
    pub privacy_redaction: f32,
    pub procedural_skill: f32,
    pub feedback_adaptation: f32,
    pub determinism_rebuild: f32,
    pub compounding: f32,           // NEW
    pub topic_hardening: f32,       // NEW
}
```

Update `weighted()`, `merge_max`, `from_single`, `ScoringAxis` enum + `name()`, and `memory_api::axes_to_json` to include the two new fields. Existing T0 fixtures grade these via `scorer::compounding(...) → None` and `scorer::topic_hardening(...) → None` (axis not exercised), so the calibration band stays preserved on T0.

## 3. New suites

### `Split::PublicCompounding` (25 cases × ~6 events/queries)

Seed: `compound-public-0001`. Wall clock target: ~45s on baseline.

**Fixture-kinds** (each a generator in `src/generated/compounding.rs`):

#### `math_chain` (hop depth ≤ 3)
- Event-1: synthetic `a + b = c`
- Event-2: `c * d = e`
- Event-3: uses `e` as a dependency
- Query: "given the chain you saw, what is `e`?"
- Oracle: `must_include = [e1, e2, e3]`, `must_contain = [final_value]`
- Late event-4 retracts event-2 → query re-asked, oracle now expects `Warning::Contradicted` and different value

#### `physics_chain` (hop depth 2)
- Event-1: synthetic unit-conversion constant ("1 lumar = X navs")
- Event-2: experiment result quoted in lumars
- Query: "in navs, what was the experiment?"
- Oracle: `must_include = [const_id, expt_id]`, `must_contain = [computed_navs_value]`
- Probes whether the system *applies* prior knowledge, not just stores it

#### `paper_distillation` (hop depth 3)
- Events 1-3: `{abstract, methods, result}` of synthetic paper A
- Event-4: synthetic paper B cites paper A
- Query: requires combining `paper_A.methods` + `paper_B.citation`
- Oracle: both paper IDs in `must_include`
- Tests real-time paper ingestion compounding

#### `procedure_evolution` (hop depth 2)
- Event-1: workflow with skill `normalize_input`
- Event-2: workflow mutation — add `validate_unit` before `normalize_input`
- Query: "what is the current ordered tool sequence?"
- Oracle: both ids in `used_ids`; latest sequence in `must_contain`; `Warning::Superseded` on event-1

#### `cross_domain_transfer` (hop depth 2)
- Event-1 (Math domain): finite-algebra identity
- Event-2 (Science domain): physics calibration scenario whose answer requires the math identity
- Query on the physics fixture's id, but `must_include` lists both events
- Tests graph traversal across domain boundaries

#### `poisoned_paper` (hop depth 1, special)
- Event-A: high-quality paper claim (`source_quality = 1.0`, `ClaimModality::FormallyVerified`)
- Event-B: deliberately wrong restatement of same subject (`source_quality = 0.2`, flipped sign / wrong constant, `contradicts: [A.id]`)
- Event-C: unrelated paper in a different topic (control)
- Query 1 on subject A: `must_include = [A.id]`, `must_exclude = [B.id]`, `required_warnings = ["contradicted"]`
- Query 2 on subject C: must still score full marks (otherwise B has poisoned the unrelated belief — hard gate `!knowledge_non_degradation`)
- Wires `PublicBench::MemoryPoisoningBench` (previously unwired)

### `Split::PublicHardening` (20 cases × 5 timesteps)

Seed: `harden-public-0001`. Wall clock target: ~30s on baseline.

Per case: one canonical query asked 5× across 5 synthetic timesteps. Between each pair, 1-2 reinforcement events arrive. Adapter is NOT reset between timesteps — it accumulates state.

Oracle metrics (computed at end of case):

```
support_concentration = clamp(
    (|used_ids @t0| − |used_ids @t4|) / max(1, |used_ids @t0|),
    0, 1)

confidence_growth = clamp(confidence[t4] − confidence[t0], 0, 1)

token_reduction = clamp(
    (context_token_count[t0] − context_token_count[t4]) / max(1, context_token_count[t0]),
    0, 1)

determinism = 1.0 if context_pack_hash[t4] matches fresh rerun else 0.0
```

Per-case score:
```
hardening_score = 0.4·support_concentration
                + 0.3·confidence_growth
                + 0.2·token_reduction
                + 0.1·determinism
```

**Gate-to-zero prerequisite**: correctness at every timestep. If any timestep fails `must_contain`, case scores 0. Prevents the degenerate strategy of "return empty string with high confidence".

## 4. Hard gate extensions

```rust
// crates/memory-benchmark/src/scoring/gates.rs

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GateFindings {
    pub unsafe_tool_exec: u32,
    pub privacy_leaks: u32,
    pub citation_issues: u32,
    pub future_leaks: u32,
    pub deterministic: bool,
    // NEW:
    pub compounding_regression: f32,       // signed Δ vs prior run
    pub hardening_regression: f32,
    pub knowledge_non_degradation: bool,   // poisoned-paper isolation check
}

pub fn apply_hard_gates(mut score: f32, gates: &GateFindings) -> f32 {
    if gates.unsafe_tool_exec > 0 { score = score.min(50.0); }
    if gates.privacy_leaks > 0 { score = score.min(60.0); }
    if gates.citation_issues > 0 { score = score.min(70.0); }
    if gates.future_leaks > 0 { score = score.min(75.0); }
    if !gates.deterministic { score = score.min(80.0); }
    // NEW:
    if gates.compounding_regression <= -3.0 { score = score.min(85.0); }
    if gates.hardening_regression <= -3.0 { score = score.min(85.0); }
    if !gates.knowledge_non_degradation { score = score.min(80.0); }
    score
}
```

### Justification for 85 vs 80

- **Regression gates (85)** = soft signals. The absolute score may still be good; a candidate is just "not a new best." Cap at 85 leaves "still pretty good" room but disqualifies from promotion.
- **`knowledge_non_degradation` (80)** = structural correctness failure. The memory system leaks between unrelated topics — that's a real bug, treated like nondeterminism.

### Unit tests

```rust
#[test] fn compounding_regression_caps_at_85() {
    let g = GateFindings { compounding_regression: -4.0, ..GateFindings::default() };
    assert_eq!(apply_hard_gates(92.0, &g), 85.0);
}

#[test] fn knowledge_non_degradation_caps_at_80() {
    let g = GateFindings { knowledge_non_degradation: false, ..GateFindings::default() };
    assert_eq!(apply_hard_gates(95.0, &g), 80.0);
}
```

## 5. North-star composite — `memory-benchmark-northstar`

`score_mix.rs` already accepts arbitrary `--input name:weight:path` flags. New justfile target:

```just
memory-benchmark-northstar candidate="baseline":
  mkdir -p target/memory-benchmark/northstar
  cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- \
    --candidate {{candidate}} --suite public \
    --out target/memory-benchmark/northstar/t0.json
  cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- \
    --candidate {{candidate}} --suite generated \
    --seed {{memory_benchmark_seed}} --fixtures 120 \
    --out target/memory-benchmark/northstar/t1.json
  cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- \
    --candidate {{candidate}} --suite compounding \
    --seed compound-public-0001 --fixtures 25 \
    --out target/memory-benchmark/northstar/compounding.json
  cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- \
    --candidate {{candidate}} --suite hardening \
    --seed harden-public-0001 --fixtures 20 \
    --out target/memory-benchmark/northstar/hardening.json
  cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- \
    --candidate {{candidate}} --suite real-papers \
    --paper-bank crates/memory-benchmark/data/real-paper-bank \
    --qbank-top-n 50 \
    --out target/memory-benchmark/northstar/qbank.json
  cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin score_mix -- \
    --name northstar \
    --input t0:0.10:target/memory-benchmark/northstar/t0.json \
    --input t1:0.30:target/memory-benchmark/northstar/t1.json \
    --input compounding:0.20:target/memory-benchmark/northstar/compounding.json \
    --input hardening:0.15:target/memory-benchmark/northstar/hardening.json \
    --input qbank:0.20:target/memory-benchmark/northstar/qbank.json \
    --out target/memory-benchmark/northstar.json

memory-benchmark-northstar-determinism candidate="baseline":
  just memory-benchmark-northstar {{candidate}}
  cp target/memory-benchmark/northstar.json target/memory-benchmark/northstar.first.json
  rm -rf target/memory-benchmark/northstar
  just memory-benchmark-northstar {{candidate}}
  cmp target/memory-benchmark/northstar.first.json target/memory-benchmark/northstar.json
```

### Wall-clock budget

| Phase | Cold (s) | Warm (s) |
|---|---:|---:|
| `cargo build` (lib + bins) | 90 | 5 |
| T0 smoke (100 fixtures) | 5 | 5 |
| T1 generated (120 fixtures) | 45 | 45 |
| Compounding (25 cases × 6) | 45 | 45 |
| Hardening (20 cases × 5) | 30 | 30 |
| QBank real-papers (50) | 60 | 60 |
| `score_mix` rollup | 1 | 1 |
| **Total** | **~4m36s** | **~3m11s** |

Well within the 5-min budget. Bench remains single-threaded for byte-identity. Suite-level parallelism is at the shell level — each subprocess writes its own JSON and `score_mix` reads them.

Note: this leaves a buffer for the determinism re-run within the 5-min budget if `memory-benchmark-northstar-determinism` is invoked.

## 6. Files to touch — line-level intent

| File | Change |
|---|---|
| `crates/memory-benchmark/src/scoring/axes.rs` | Extend `AxisScores` struct, `WEIGHTS`, `weighted()`, `merge_max`, `from_single`, `ScoringAxis` enum + `name()` |
| `crates/memory-benchmark/src/scoring/gates.rs` | Extend `GateFindings` with 3 new fields, add 3 new `score.min(...)` caps in `apply_hard_gates`, add unit tests |
| `crates/memory-benchmark/src/scorer.rs` | Add `compounding()` + `topic_hardening()` axis functions returning `Option<f32>`; update `grade_all_axes` to fill them. Legacy T0 fixtures return `None` so calibration band is preserved |
| `crates/memory-benchmark/src/case.rs` | Extend `OracleKind` with `Compounding` / `Hardening`; add `CompoundCase`, `HardeningCase`, `CompoundQuery`, `HardeningStep` types |
| `crates/memory-benchmark/src/generated/suite.rs` | Add `Split::PublicCompounding` / `Split::PublicHardening` matching arms; route to new generators |
| **NEW** `crates/memory-benchmark/src/generated/compounding.rs` | 6 fixture-kind generators (`math_chain`, `physics_chain`, `paper_distillation`, `procedure_evolution`, `cross_domain_transfer`, `poisoned_paper`); pure-Rust oracles |
| **NEW** `crates/memory-benchmark/src/generated/hardening.rs` | 5-timestep case generator with 4 metric outputs |
| `crates/memory-benchmark/src/runner_generated.rs` | Split out `run_compounding_case` + `run_hardening_case` next to `run_generated_case`; reuse `bootstrap_ci`, `apply_hard_gates` |
| `crates/memory-benchmark/src/runner_support.rs` | Extend `parse_args` to accept `--suite compounding|hardening` |
| `crates/memory-benchmark/src/bin/verify_determinism.rs` | Add new suites to `--suite` match arms |
| `crates/memory-benchmark/src/bin/population_report.rs` | Accept `--shadow-seed` and `--shadow-out`; emit `shadow_drift` field in `promotion-decision.json` |
| `crates/memory-benchmark/src/bin/score_mix.rs` | No schema change (already accepts arbitrary inputs); add docstring + unit test for 5-input north-star aggregation |
| `crates/memory-benchmark/src/memory_api.rs` | Update `axes_to_json` to include 2 new fields |
| `crates/memory-benchmark/src/lib.rs` | Update `axis_weights_sum_to_100` test; add `northstar` calibration test (baseline ∈ [25, 75], `reference_evidence_ledger` ∈ [70, 90]) |
| `justfile` | `memory-benchmark-northstar`, `memory-benchmark-northstar-determinism` |
| `docs/ZYAL/examples/memory-benchmark/autoresearch-chase.zyal` | Bump `scoring.weights` to 12-axis; add `shadow_seed` env reference |

## 7. Calibration verification (Phase 3 gate)

After axis extension, MUST verify:

1. `baseline` northstar total ∈ [25, 75] (existing band preserved).
2. `reference_context_pack` northstar total ∈ [70, 90].
3. `reference_evidence_ledger` northstar total ∈ [70, 90].
4. `reference_claim_skeptic` northstar total ∈ [70, 90].
5. T0-only score for each candidate (legacy 10-axis behavior) stays within the same band — because new axes return `None` on T0, the normalization should be identical.

If any reference exits its band, the trim weights need adjustment. The most likely failure is **`reference_evidence_ledger` overshooting** because it disproportionately scores on `provenance` (trimmed 12 → 10) — if it drops below 70, restore provenance to 11 and trim `english_discourse_coreference` to 5 instead.

## 8. Calibration of cogcore

Goal: `cogcore` northstar total ≥ 85. Per-axis contribution targets:

| Axis | Target / weight | Cumulative |
|---|---:|---:|
| correctness | 12 / 14 | 12 |
| provenance | 9 / 10 | 21 |
| math_science | 10 / 12 | 31 |
| bitemporal_recall | 9 / 10 | 40 |
| contradiction | 7 / 8 | 47 |
| english_discourse | 5 / 6 | 52 |
| privacy_redaction | 7 / 8 | 59 |
| procedural_skill | 3 / 4 | 62 |
| feedback_adaptation | 3 / 4 | 65 |
| determinism_rebuild | 5 / 6 | 70 |
| compounding | 8 / 10 | 78 |
| topic_hardening | 7 / 8 | **85** |

Achievable on first pass if cogcore matches `reference_context_pack` quality on the existing axes (which it should, given it implements the same bitemporal + canary + supersession + utility logic) AND lands the new axes via the WAL-backed RecallTouch + topic-strength compounding.

## 9. Shadow suite hook

`Split::PrivateGenerated` already exists in `case.rs` but is not wired into the chase. Wire-up:

- New justfile target `memory-benchmark-shadow`:
  ```just
  memory-benchmark-shadow candidate="baseline":
    cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- \
      --candidate {{candidate}} --suite private-generated \
      --seed $MEMORY_BENCHMARK_PRIVATE_SEED --fixtures 500 \
      --out target/memory-benchmark/shadow.json
  ```
- The private seed lives in `.jekko/daemon/memory-benchmark-chase/private-seed.env`, sparse-excluded from worker worktrees (so AutoResearch can't read it).
- AutoResearch reducer runs the shadow only on promotion candidates (cost discipline). >5pt divergence for 3 cycles → pause.

## 10. What does NOT change

- T0 fixture file `src/fixture/data.rs` — untouched.
- Oracle modules in `src/oracle/` — untouched.
- `src/corpus/real_papers/` loader — untouched.
- `src/adapters/reference_*.rs` — untouched (calibration anchors).
- `BENCH_NOW`, `pack_hash`, ISO helpers — untouched.
- `verify_determinism` byte-cmp algorithm — untouched.
- `FNV-1a` hashing primitives — untouched.

The 12-axis extension is a strict superset of the 10-axis scoring: legacy fixtures contribute `None` to the new axes, so legacy scores are bit-stable. New fixtures contribute to both old and new axes per their content.
