# 06 — Implementation Roadmap (5 Phases)

5 phases over ~5 weeks. Each phase has a sharp end condition. No phase ships unless its verification gate is green.

**Implementation status (snapshot date 2026-05-13):** Phases 1–5 landed, then the Track A audit hardening pass corrected benchmark semantics and AutoResearch safety gates. See `refs/snapshot.md` for measured scores and shipped file list. Honest post-Track-A cogcore northstar = 77.63; references calibrated 82.88–83.13; 88+30+3 = 121 tests green across the three crates. QBank remains fixture-backed `dev_only`, so `chase-daemon` is not armable.

## Phase 1 — cogcore skeleton + benchmark wiring (week 1) — ✅ DONE

Goal: a stub cogcore that compiles and runs against the existing benchmark without breaking calibration.

### Steps

1. `cargo new --lib crates/cogcore` from inside the workspace.
2. Add to workspace `Cargo.toml`.
3. Set up `Cargo.toml` for cogcore: zero default deps, `experimental_blake3` + `experimental_hnsw` feature flags off by default.
4. Create the module skeleton (`lib.rs`, `adapter.rs`, `core.rs`, stubs for `ledger.rs`, `cell.rs`, etc.).
5. Implement `MemorySystem` trait minimally in `adapter.rs`:
   - `observe`: store events in `Vec<Event>` (same as `reference_evidence_ledger`)
   - `recall`: substring match on subject/body
   - `recall_at` / `recall_as_of`: bitemporal filter (mirror `reference_context_pack`)
   - `feedback` / `forget` / `rebuild` / `export_state_hash`: stub implementations
6. Wire one new match arm into `crates/memory-benchmark/src/runner.rs::boxed_adapter`:
   ```rust
   "cogcore" => Ok(Box::new(cogcore::Adapter::default())),
   ```
7. Add `cogcore = { path = "../cogcore" }` to `crates/memory-benchmark/Cargo.toml` `[dev-dependencies]`.
8. Add `tests/trait_smoke.rs` — round-trip via `MemorySystem` API.
9. Add `tests/benchmark_smoke.rs` — invoke `memory_benchmark::runner::run_candidate("cogcore")` and assert score > 0.

### Phase 1 verification gate (must all pass)

```bash
# Existing tests still pass
cargo test --manifest-path crates/memory-benchmark/Cargo.toml --locked

# Existing fast lane still passes
just memory-benchmark-fast

# cogcore-specific tests pass
cargo test --manifest-path crates/cogcore/Cargo.toml --locked

# Benchmark recognizes cogcore as a candidate
cargo run --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench -- \
    --candidate cogcore --suite public --out /tmp/cogcore-smoke.json

# Existing calibration band still holds for the 4 references
# (the no-branded-identifiers test must also still pass)
```

Phase 1 ends with cogcore scoring something in the [30, 70] range — comparable to a slightly-better baseline. Not yet competitive with references.

## Phase 2 — cogcore real (week 2-3) — ✅ DONE

Goal: cogcore beats `reference_evidence_ledger` on T0 + T1. Achieves real compounding and hardening behavior.

### Step 2.1 — WAL + ledger + replay

- Implement `ledger.rs` with the on-disk record format from `02-cogcore-design.md` §4.
- In-memory backend (`StorageBackend::Memory`) for benchmark use.
- `rebuild()` replays from seq 0 and produces byte-identical `export_state_hash()`.
- Add `tests/ledger_replay.rs` — ingest 50 events, recall 10, then `rebuild()`, assert `export_state_hash` is the same.

### Step 2.2 — MemoryCell + projections

- BM25 inverted index (stdlib-only, sorted insert)
- Subject map: `subject → Vec<CellId>`
- Equation lane: `lhs → Vec<EqAtom>`
- Token bigram index
- Token interner with `canonicalize_after_replay`

### Step 2.3 — Recall pipeline (no Hebbian/FSRS yet)

- Implement `retrieval.rs` with the fusion score from `05-formulas.md` §6.
- Bitemporal filter (mirror `reference_context_pack`).
- Privacy scan (fragment-built canaries, Vault short-circuit).
- Pack greedy fill.
- Target: ≥ 75 on T0.

### Step 2.4 — Add Hebbian + FSRS + RecallTouch

- Implement `hebb.rs` with the update rules from `05-formulas.md` §4.
- Implement `fsrs.rs` with the half-life formulas from `05-formulas.md` §3.
- Add `WalOp::RecallTouch` to ledger.
- Test in `tests/ledger_replay.rs` — after observe stream + recall stream, `export_state_hash` is stable across `rebuild()`.

### Step 2.5 — Concept emergence + topic strength

- Implement `concept.rs::promote_concepts` with MinHash + Jaccard from `05-formulas.md` §5.
- Implement `topic.rs` strength formula from `05-formulas.md` §1.
- Add `tests/topic_hardens.rs` — ingest 50 synthetic neutrino abstracts, assert `topic_strength_of("neutrino-physics") ≥ 0.8`.

### Step 2.6 — Paper ingestion

- Implement `ingest/paper.rs` (section split), `ingest/equation.rs` (SI table), `ingest/theorem.rs` (header regex).
- Test on `data/real-paper-bank/` synthetic samples.
- `ExtractorBackend::RuleBackend` is default; `MaybeLlmBackend` wires later (Phase 5+).

### Step 2.7 — Calibration to ≥85

- Run `cargo run --bin bench -- --candidate cogcore --suite public`.
- If < 85, profile per-axis using `axes_to_json` output.
- Most likely gaps: `contradiction` axis (need `SkeptikSurfaced` on more cases) and `feedback_adaptation` (confidence band).

### Phase 2 verification gate

```bash
just memory-benchmark-fast                           # everything still compiles + determinism

cargo test --manifest-path crates/cogcore/Cargo.toml --locked
# trait_smoke, ledger_replay, topic_hardens, benchmark_smoke all green

cargo run --bin bench -- --candidate cogcore --suite public      # ≥ 85
cargo run --bin bench -- --candidate cogcore --suite generated   # ≥ 85
cargo run --bin verify_determinism -- --candidate cogcore --suite public   # exit 0
```

## Phase 3 — Benchmark 12-axis extension (week 4) — ✅ DONE

Goal: north-star benchmark exists and is calibrated.

### Step 3.1 — Extend AxisScores

- Add `compounding: f32` and `topic_hardening: f32` fields to `AxisScores`.
- Update `WEIGHTS`, `weighted()`, `merge_max`, `from_single`, `ScoringAxis` enum.
- Update `axis_weights_sum_to_100` test.

### Step 3.2 — Add scoring functions

- Add `scorer::compounding()` and `scorer::topic_hardening()` returning `Option<f32>`.
- Update `grade_all_axes` to fill them.
- Legacy T0 fixtures return `None` from these → calibration preserved.

### Step 3.3 — New case kinds + types

- Add `OracleKind::Compounding` and `OracleKind::Hardening` to `case.rs`.
- Add `CompoundCase`, `HardeningCase`, `CompoundQuery`, `HardeningStep` types.
- Add `Split::PublicCompounding` and `Split::PublicHardening` to `case.rs` enum.

### Step 3.4 — New generators

- Create `crates/memory-benchmark/src/generated/compounding.rs` with 6 fixture-kind generators.
- Create `crates/memory-benchmark/src/generated/hardening.rs` with 5-timestep case generator.
- Wire into `generated/suite.rs` matching arms.
- Add `runner_generated::run_compounding_case` and `run_hardening_case`.

### Step 3.5 — Hard gate extensions

- Extend `GateFindings` with 3 new fields.
- Extend `apply_hard_gates` with 3 new caps.
- Add unit tests for each new cap.

### Step 3.6 — Justfile targets

- Add `memory-benchmark-northstar` (per `03-benchmark-12axis.md` §5).
- Add `memory-benchmark-northstar-determinism` (run twice, `cmp`).

### Step 3.7 — Calibration check

- Run `just memory-benchmark-northstar candidate=baseline` → ∈ [25, 75]
- Run for each reference adapter → ∈ [70, 90]
- Run for cogcore → ≥ 85

If a reference exits its band, adjust trim weights (see `03-benchmark-12axis.md` §7).

### Phase 3 verification gate

```bash
cargo test --manifest-path crates/memory-benchmark/Cargo.toml --locked
# axis_weights_sum_to_100 now expects 12 axes
# new gate unit tests pass
# all references still in [70, 90] on T0 (legacy 10-axis behavior preserved)

just memory-benchmark-northstar candidate=baseline
# total in [25, 75]

just memory-benchmark-northstar candidate=reference_evidence_ledger
# total in [70, 90]

just memory-benchmark-northstar candidate=cogcore
# total ≥ 85

just memory-benchmark-northstar-determinism candidate=baseline
# exit 0 (cmp succeeds)

# Wall clock under 5 min on commodity hardware
time just memory-benchmark-northstar candidate=cogcore
# < 5m00s
```

## Phase 4 — AutoResearch orchestrator (week 5-6) — ✅ SKELETON (T1 only); T2-T4 deferred

Goal: 1 cycle of 20-worker chase runs end-to-end. T1+T2+T3+T4 all wired. Shadow check, calibration anti-tamper, receipt chain operational.

### Step 4.1 — Skeleton orchestrator

- `cargo new --bin tools/autoresearch` from workspace.
- Subcommands: `tick`, `daemon`, `seed`, `forensics`.
- `seed` initializes `autoresearch/chase-best` branch.

### Step 4.2 — Worktree management

- `tick` does `git worktree add --detach .jekko/daemon/memory-benchmark-chase/worktrees/$CYCLE_ID/$WORKER_ID autoresearch/chase-best` for each worker.
- Per-worker `CARGO_TARGET_DIR=.jekko/daemon/memory-benchmark-chase/target/$CYCLE_ID/$WORKER_ID`.
- After cycle, prune worktrees older than last 5 cycles.

### Step 4.3 — T1 proposer (deterministic GA)

- `tools/autoresearch/src/proposer/genetic.rs`.
- Seeded by `(cycle_id, worker_id, best_state_hash, $MEMORY_BENCHMARK_SEED)`.
- Uniform crossover + Gaussian mutation `normal_from_seed(...)`.
- Initial target: tune `cogcore/config.toml` weights (the α_r, α_c, etc.).

### Step 4.4 — End-to-end first cycle (T1 only)

- Run `chase-tick`.
- Verify all 20 workers emit `<id>.json` + `<id>.patch`.
- Verify reducer produces `best-state.json`, `promotion-decision.json`, `negative-memory.jsonl`, `scoreboard.tsv`, and `receipts/<cycle>.json`.

### Step 4.5 — T2 + T3 template library

- Templates at `tools/autoresearch/templates/t2/`, `t3/`.
- 5+ T2 templates: `swap_scoring_function`, `swap_redaction_strategy`, `swap_topic_decay_curve`, `add_cache_layer`, …
- 5+ T3 templates: `add_secondary_index`, `add_bloom_filter`, `add_temporal_skip_list`, `add_hierarchical_concept_layer`, `add_lru_cache`.

### Step 4.6 — chase_reduce binary

- `crates/memory-benchmark/src/bin/chase_reduce.rs`.
- Wraps `chase_report::build_chase_outputs` with:
  - Shadow check (run private-generated suite on promotion candidate only)
  - Calibration anti-tamper (rerun 4 references in worker tree)
  - Receipt emission
- Extend `chase_report::render_negative_memory` schema (per `04-autoresearch-loop.md` §7).

### Step 4.7 — T4 LLM proposer

- `tools/autoresearch/src/proposer/llm.rs`.
- Prompt template at `tools/autoresearch/prompts/mutate_t4.md`.
- Inputs to prompt: cogcore source + last 20 negative-memory entries (deduped) + axis weakness signal.
- Forbidden-token static check before applying patch.
- Per-cycle T4 budget: $5; aggregate run budget: $25.

### Step 4.8 — Abort conditions

- Orchestrator monitors after each cycle:
  - Shadow regression < -3.0 → pause
  - Gate spike > 5× trailing-10 median → pause
  - Worker crash rate > 20% (2 cycles) → pause
  - Reference drift > 1.0 → pause
  - Negative memory growth > 50/cycle (5 cycles) → pause
- On abort: drop `aborted.flag` + reason file + `forensics-bundle.tar`.

### Step 4.9 — ZYAL update

- Update `docs/ZYAL/examples/memory-benchmark/autoresearch-chase.zyal`:
  - Replace `fan_out.split` shell with `autoresearch tick --emit-tasks`
  - Replace `fan_out.reduce` with `chase_reduce`
  - Add `shadow_divergence` + `reference_drift` gates
  - Bump `scoring.weights` to 12-axis

### Phase 4 verification gate

```bash
# One cycle end-to-end
just chase-tick
# .jekko/daemon/memory-benchmark-chase/best-state.json exists
# .jekko/daemon/memory-benchmark-chase/promotion-decision.json exists
# .jekko/daemon/memory-benchmark-chase/receipts/<cycle>.json exists
# scoreboard.tsv has 20 lines
# negative-memory.jsonl has ≤ 19 entries (some workers may have been promoted)

# Anti-tamper smoke: manually edit a reference adapter in a worktree, run reducer, verify it rejects
# (one-time forensic check, not a regular test)

# Force regression smoke: T1 mutation that lowers compounding by 4pts; verify reducer rejects with GateRegression
# (forensic check)

# Cycle wall clock
time just chase-tick
# < 12 min cold, < 4 min warm

# Shadow check
MEMORY_BENCHMARK_SHADOW_SEED=test-shadow-0001 just chase-tick
# Receipt shows shadow_score field populated; abs(public - shadow) reported
```

### Post-Track-A correction — ✅ DONE

The initial Phase 4 skeleton was not production safe. Track A hardening landed in commit `2617e2a1b` and changed the trust boundary:

- hardening now scores repeated timesteps with reinforcements injected between recalls;
- QBank production mode fails when redistributable paper JSON is missing;
- fixture QBank requires `memory_benchmark_dev_qbank=1` and emits `dev_only:true`;
- AutoResearch runs fresh references per cycle;
- reducer reference drift is measured in absolute score points;
- reducer rejects dev-only lanes and trusted-core patch violations;
- dirty-source AutoResearch runs are explicitly non-promotable.

The chase daemon stays disarmed until QBank is non-dev and a clean-source, non-dev AutoResearch cycle passes shadow/reference/trusted-core gates.

## Phase 5 — smartmemory/ documentation + final polish (week 6) — ✅ DONE

Goal: documentation matches code. Initial AutoResearch run produces a trustworthy promotion decision; promotion is allowed only when non-dev gates pass.

### Steps

1. Update `smartmemory/00-audit.md` if anything in the audit changed during implementation.
2. Update `smartmemory/01-gaps.md` to reflect what was actually closed.
3. Update `smartmemory/02-cogcore-design.md` with actual line numbers from the implemented code.
4. Update `smartmemory/refs/critical-files.md` with final paths.
5. Run a clean north-star (cogcore on baseline) and capture the JSON in `smartmemory/refs/baseline-northstar-snapshot.json`.
6. Run a chase smoke (1 cycle), capture `receipts/<cycle>.json` in `smartmemory/refs/`.

### Phase 5 verification gate

- All 11 files in `smartmemory/` reference actual file paths and line numbers (no stale references).
- One AutoResearch cycle has run cleanly without aborts.
- One promotion has landed only if non-dev gates pass; otherwise the promotion decision and negative-memory log explain the rejection.

## Phase 6 — Track B capability levelup (current)

Goal: recover capability after honest scoring by improving cogcore rather than weakening benchmark gates.

### Active work split

- Claude owns `crates/cogcore/**` Track B work: audit cleanup, compounding diagnostic, 10K scale test, hardening convergence test, and ingest scaffold.
- Codex owns non-overlapping B8/docs work: `autoresearch-chase.zyal` contract update and stale snapshot refresh.

### Track B work packets

| ID | Item | Status |
|---|---|---|
| B1 | cogcore ingest pipeline | claimed by Claude |
| B2 | consolidation backend + budget trait | open after B1 |
| B3 | cogcore stream papers ZYAL + bench binary | open after B1/B7 |
| B4 | real-paper-chain compounding fixture kind | open after B1 |
| B5 | 10K scale validation | claimed by Claude |
| B6 | hardening_converges cogcore test | claimed by Claude |
| B7 | qbank-builder `--emit-cogcore` mode | open after B1 type contract |
| B8 | autoresearch chase ZYAL gate update | in progress by Codex |

### Phase 6 verification gate

```bash
cargo test --manifest-path crates/cogcore/Cargo.toml --locked --no-fail-fast
cargo test --manifest-path crates/memory-benchmark/Cargo.toml --locked --no-fail-fast
cargo test --manifest-path tools/autoresearch/Cargo.toml --locked --no-fail-fast
just memory-benchmark-fast
just memory-benchmark-new-suite-determinism cogcore
just memory-benchmark-northstar cogcore
```

Phase 6 is not done until cogcore has meaningful hardening convergence evidence and QBank trust provenance is either real/non-dev or explicitly blocked from promotion.

## Phase 7+ (post-roadmap, optional)

Future work, deliberately deferred:

- **Multimodal grounding** — image/audio/screenshot events. Hook is the existing `EventKind::Resource`.
- **Federated memory** — multi-agent sharing, consensus, privacy boundaries across nodes.
- **Skill execution sandbox** — defining threat model + AST validation + capability boundaries for `EventKind::Skill` invocation. Currently skills are stored, not executed.
- **Neural embedding lane** — `experimental_hnsw` feature wires a real ANN index. The fusion score adds a `+ 0.6 · embedding_similarity` term.
- **Persistent on-disk** — currently in-memory backend is the default; production deployment needs the disk WAL backend exercised at scale.
- **Multi-language paper ingestion** — currently English-only; arxiv has Chinese / Japanese / Spanish papers worth ingesting.

These are tracked but not gated.

## Critical path summary

```
Phase 1 ──┐
Phase 2 ──┤── must complete before Phase 4
Phase 3 ──┘
Phase 4 ────── ended with skeleton only
Phase 5 ────── docs and initial receipts
Track A ────── corrected safety gates; chase stays disarmed while QBank is dev-only
Track B ────── cogcore capability recovery and real-paper trust path
```

Phases 1-2 are sequential (cogcore must exist before AutoResearch can mutate it). Phase 3 (benchmark extension) can run in parallel with Phase 2 (cogcore real) — they touch different files. Phase 4 (AutoResearch) requires both 2 and 3 complete. Phase 5 (docs) shadows Phase 4.

Estimated total: 5-6 weeks of focused work for a 1-2 person team. Solo, 8-10 weeks.

## Risk-watch (before starting each phase)

| Phase | Highest risk | Pre-phase check |
|---|---|---|
| 1 | Trait shape changes during Phase 2 → rework | Confirm `MemorySystem` API is frozen for v1 |
| 2 | Determinism bug from `WalOp::RecallTouch` | Property test `tests/ledger_replay.rs` from day 1 |
| 3 | Calibration drift on references after trimming weights | Run baseline through full T0 before changing axes |
| 4 | Worktree creation fails on macOS for sparse-checkout | Test worktree+sparse on dev machine before scripting |
| 5 | docs go stale faster than they update | Run `just memory-benchmark-northstar` after every commit |
| Track A/B | dev-only QBank accidentally treated as trusted | Require `dev_only:false`, 50 real accepted papers, fresh references, and reducer rejection evidence |

Phase-end commits should land on a branch off main, not directly on main, until the full chase smoke is green.
