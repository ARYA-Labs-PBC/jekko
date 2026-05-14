# 04 — AutoResearch Loop (All 4 Tiers Active)

User chose "All 4 tiers from day 0." This file specifies the loop, the tier ladder, the safety rails, the receipt chain, and the abort conditions.

## 1. Trust zones (three concentric layers)

### Trusted core (read-only to AutoResearch)

Workers cannot edit:
- `crates/memory-benchmark/src/{runner.rs, case.rs, fixture/, oracle/, generated/, corpus/, scoring/, runner_support.rs, runner_generated.rs}`
- `crates/memory-benchmark/tests/`
- `crates/memory-benchmark/src/lib.rs` (calibration test lives here)
- `crates/memory-benchmark/src/scorer.rs`
- `crates/memory-benchmark/src/adapters/reference_*.rs` (4 calibration anchors)
- Anything under `crates/memory-benchmark/src/bin/` except `chase_reduce.rs` (orchestrator-only edit point)
- `docs/ZYAL/SPEC.md`

Only human-reviewed PRs change these. The orchestrator enforces the boundary by rejecting patches that touch any of these paths.

### Mutable surface (worker sandbox)

Workers CAN edit:
- All of `crates/cogcore/src/`
- These 5 non-reference candidate files:
  - `crates/memory-benchmark/src/candidates/ledger_first.rs`
  - `crates/memory-benchmark/src/candidates/hybrid_index.rs`
  - `crates/memory-benchmark/src/candidates/temporal_graph.rs`
  - `crates/memory-benchmark/src/candidates/compression_first.rs`
  - `crates/memory-benchmark/src/candidates/skeptic_dataset.rs`

The 4 reference adapters (`reference_context_pack`, `reference_evidence_ledger`, `reference_claim_skeptic`, plus `baseline`) are frozen calibration anchors. AutoResearch must NEVER edit them; the reducer will rerun them in the worker tree and reject promotions that cause drift > 0.5 points.

### Orchestrator (host process)

New `tools/autoresearch/` Rust binary. Owns:
- Cycle scheduling
- Worktree creation / cleanup
- Mutation proposers (T1-T4)
- Reducer invocation
- Receipt chain commits
- Abort detection

## 2. Seeding

A long-lived branch `autoresearch/chase-best`. Day 0: `autoresearch/chase-best == main` (i.e., baseline as the implicit best, or whatever you pick as the seed).

Best-state hash:
```
best_state_hash = fnv1a(
    git_rev_parse("autoresearch/chase-best:crates/cogcore")
    || ⨁ git_hash_object(file) for file in mutable_candidate_files
)
```

### Worker checkout

```bash
git worktree add --detach \
    .jekko/daemon/memory-benchmark-chase/worktrees/$CYCLE_ID/$WORKER_ID \
    autoresearch/chase-best
```

Each worker gets:
- An isolated worktree (no shared mutable state)
- `CARGO_TARGET_DIR=.jekko/daemon/memory-benchmark-chase/target/$CYCLE_ID/$WORKER_ID` for incremental builds
- Environment: `$MEMORY_BENCHMARK_SEED`, `$BENCH_NOW` inherited from orchestrator

### Worker mutation flow

1. Worker enters worktree at `autoresearch/chase-best`.
2. Mutation oracle proposes a patch (see §3).
3. Worker applies patch directly to working tree (NO commit).
4. Worker runs `memory-benchmark-northstar` (§4).
5. Worker writes `<id>.json` (lane report) + `<id>.patch` (`git diff`) to `reports/lanes/`.
6. No commit — the patch is the artifact.

### Promotion → commit

When the reducer promotes worker `W`:
- Orchestrator: `git -C <main-repo> apply --3way <W's patch>` against `autoresearch/chase-best`
- Commit message: `chase: promote $CYCLE_ID/$WORKER_ID +<delta> pts`
- Co-Authored-By: `AutoResearch <noreply@anthropic.com>`
- Receipt JSON committed in same commit (single audit path)
- `main` only updates on human-reviewed merge of `autoresearch/chase-best`

This is the trust boundary. `main` never directly receives AutoResearch commits.

## 3. Mutation strategy ladder

All 4 tiers active day 0. Distribution per cycle: **T1=50%, T2=30%, T3=15%, T4=5%**. Shifts toward higher tiers after K=5 stagnant cycles (no promotion).

### T1 — Config / hyperparameter sweep

**Surface**: `cogcore/config.toml` (cogcore's tunable constants) OR hyperparameter consts in `crates/memory-benchmark/src/candidates/shared.rs`.

**Cap**: 30 lines changed total.

**Proposer** (`tools/autoresearch/src/proposer/genetic.rs`):
- Deterministic GA over numeric values
- Population = current best + top-3 from population ledger
- Uniform crossover, Gaussian mutation `normal_from_seed(cycle_id * worker_id, sigma=0.15)`
- Seeded by `$MEMORY_BENCHMARK_SEED` + `cycle_id` — fully reproducible

**Reject if**:
- Patch touches paths outside the allowlist
- Line count > 30

### T2 — Single-algorithm swap

**Surface**: ONE file under `crates/cogcore/src/` OR ONE of the 5 non-reference candidate files.

**Cap**: 150 lines, 1 file.

**Proposer**: template expansion + GA. Templates live at `tools/autoresearch/templates/t2/`:
- `swap_scoring_function.rs.tmpl`
- `swap_redaction_strategy.rs.tmpl`
- `swap_topic_decay_curve.rs.tmpl`
- `add_cache_layer.rs.tmpl`

**Reject if**:
- More than 1 file touched
- Line count > 150

### T3 — New data structure

**Surface**: up to 5 files under `crates/cogcore/src/`. May add new modules within that crate.

**Cap**: 600 lines, 5 files.

**Proposer**: rule-based template library at `tools/autoresearch/templates/t3/`:
- `add_secondary_index.rs.tmpl` — `HashMap<topic, Vec<event_id>>` for fast topic lookups
- `add_bloom_filter.rs.tmpl` — deterministic Bloom filter for tombstones
- `add_temporal_skip_list.rs.tmpl` — skip list for time-windowed queries
- `add_hierarchical_concept_layer.rs.tmpl` — multi-level concept clustering
- `add_lru_cache.rs.tmpl` — deterministic LRU on context-pack hashes

Templates have placeholder slots that GA fills.

**Reject if**:
- Touches `crates/memory-benchmark/` at all
- Adds a `[dependencies]` line to `Cargo.toml` (zero-dep invariant)
- Build fails (`cargo build --manifest-path crates/cogcore/Cargo.toml --locked`)

### T4 — LLM-suggested edit

**Surface**: full mutable surface (`crates/cogcore/src/` + 5 non-reference candidates).

**Cap**: 1500 lines, 8 files.

**Proposer**: LLM call via `models.profiles.builder` (the jnoccio-fusion profile from current ZYAL). Prompt template at `tools/autoresearch/prompts/mutate_t4.md`. Inputs to the prompt:
1. Full source of current-best `crates/cogcore/`
2. Last 20 negative-memory entries (deduped by `mutation_diff_hash`)
3. Benchmark axis weights from `scoring/axes.rs`
4. One axis-weakness signal — the axis with lowest current score

Output: unified diff.

**Reject if**:
- Diff doesn't apply cleanly
- Imports a crate not already in workspace
- Contains forbidden tokens (regex-detected):
  - `SystemTime::now`, `Instant::now`
  - `rand::`, `thread_rng`
  - `chrono::`
  - `env::var(`
  - `process::Command`
  - `unsafe ` (the keyword, in any context — Rust hot path stays safe-Rust)

### Universal rejection rules (all tiers)

- Any change in trusted core (any path under §1's trusted-core list) → `reason=tier_violation`
- `cargo check` failure on `memory_benchmark` crate after applying patch → `reason=core_compile_break`
- Static grep detects clock/random source in patch → `reason=nondeterminism_token`
- Patch contains symlink that resolves outside the mutable surface → `reason=path_escape`

Determinism contract for proposers: every proposer must be deterministic given `(cycle_id, worker_id, best_state_hash, $MEMORY_BENCHMARK_SEED)`. Reproducing a specific worker is a single command.

## 4. Worker harness

The new `memory-benchmark-northstar` Justfile target (see `03-benchmark-12axis.md`).

Per worker, in worktree:
1. `cargo build --manifest-path crates/cogcore/Cargo.toml --locked` (~30s warm, ~120s cold)
2. `cargo build --manifest-path crates/memory-benchmark/Cargo.toml --locked --bin bench --bin verify_determinism --bin score_mix` (~30s warm, ~90s cold)
3. `bench --candidate <chosen> --suite generated --seed $MEMORY_BENCHMARK_SEED --fixtures 120`
4. `bench --suite compounding`, `bench --suite hardening`, `bench --suite real-papers`
5. `verify_determinism` — exit 0 or worker is dead
6. `score_mix` north-star composite (5 inputs)
7. `git diff > <worker_id>.patch`

**Per-worker wall clock**: 10min cold, 90s warm typical. 20 workers in parallel → ~12min cold cycle, ~3min warm cycle.

## 5. Promotion contract (`crates/memory-benchmark/src/bin/chase_reduce.rs`)

```rust
fn promote_or_reject(
    c: &LaneReport,
    best: &BestState,
    shadow: &ShadowReport,
) -> Decision {
    if !c.gates.is_clean() { return Reject::HardGate; }
    if c.gates.has_new_failures_against(&best.gates) {
        return Reject::GateRegression;
    }
    if !c.determinism_verified { return Reject::Nondeterminism; }
    if c.patch.is_empty() { return Reject::NoPatch; }
    if c.diff_touches_trusted_core() { return Reject::TierViolation; }

    let public_delta = c.score - best.score;
    if public_delta < 0.75 { return Reject::InsufficientMargin; }

    // Calibration anti-tamper: rerun the 4 references in the worker's tree.
    // If baseline exits [25, 75] OR any reference exits [70, 90] → reject.
    if c.reference_drift().any(|d| d.abs() > 0.5) {
        return Reject::ReferenceDrift;
    }

    // Shadow check: candidate must hold the gain on the private split.
    let shadow_delta = shadow.score - best.shadow_score;
    if (public_delta - shadow_delta).abs() > 5.0 {
        return Reject::ShadowDivergence;
    }
    if shadow_delta < 0.0 { return Reject::ShadowRegression; }

    Promote
}
```

The reducer writes the same artifact set as today PLUS the new `cycle-receipt.json`:
- `best-state.json` — current best score + state hash + axis breakdown
- `promotion-decision.json` — accepted | rejected + reason + diff hash
- `negative-memory.jsonl` — append-only ledger of failed candidates
- `best.patch` — diff from `autoresearch/chase-best` to the new best (empty if no promotion)
- `scoreboard.tsv` — all 20 workers' scores
- `curriculum-proposals.json` — proposed new fixtures (advisory)
- **`cycle-receipt.json`** — see §9

## 6. Shadow suite (anti-overfit)

`Split::PrivateGenerated` already exists in `case.rs` but never run in the chase.

Wire-up:
- Justfile target `memory-benchmark-shadow`:
  ```just
  memory-benchmark-shadow candidate="baseline":
    cargo run ... --suite private-generated --seed $MEMORY_BENCHMARK_PRIVATE_SEED ...
  ```
- Private seed at `.jekko/daemon/memory-benchmark-chase/private-seed.env`
- Sparse-checkout config in worker worktrees excludes this file (so AutoResearch can't read it)
- Reducer runs shadow only on the promotion candidate (cost discipline)
- Wall clock budget: 60s

### Pause condition

If for K=3 consecutive cycles `|public_score - shadow_score| > 5.0`, the orchestrator sets `.jekko/daemon/memory-benchmark-chase/paused.flag` and the ZYAL daemon's `pause_on` fires.

### Seed rotation

Every 50 cycles the private seed rotates via:
```
new_private_seed = blake3_hex($MEMORY_BENCHMARK_SEED || cycle_id_str)[:32]
```

Rotation receipt is committed to `autoresearch/chase-best` so reviewers can verify the schedule. The new seed value is never committed — only the commitment hash.

## 7. Negative memory schema

One JSONL line per rejected candidate per cycle:

```json
{
  "cycle_id": "0000123",
  "worker_id": "lane_07",
  "candidate_id": "fnv1a:8c7a...",
  "parent_id": "fnv1a:a3b1...",
  "mutation_kind": "T2_algorithm_swap",
  "mutation_template": "swap_scoring_function",
  "mutation_diff_hash": "fnv1a:9e2f...",
  "mutation_diff_size": 84,
  "score_delta": -2.4,
  "shadow_delta": null,
  "gates": { "privacy_leaks": 0, "nondeterminism": 1, "total": 1 },
  "axis_regressions": ["determinism_rebuild"],
  "reason": "nondeterminism",
  "cost_usd": 0.12,
  "observed_at_run": "0000123"
}
```

Extend `render_negative_memory` in `chase_report.rs` with the new fields. Append-only via `OpenOptions::append(true)`. Dedup happens at T4 prompt construction time (by `mutation_diff_hash`), not at write time — the ledger preserves all history.

## 8. Curriculum proposals (advisory)

Triggered by the reducer when score distribution is pathological:

- **Saturated** (all candidates ≥0.99 weighted on a fixture): `{kind: harder_variant, fixture_id, pathology, suggested_change}`.
- **Universal failure** (all ≤0.10): `{kind: clarify_oracle, fixture_id, current_must_include, divergence}`.
- **Bimodal** (>30% pass, <70% pass, no middle): `{kind: ambiguous_pathology, fixture_id, passing_candidates, failing_candidates}`.

Proposals are appended to `curriculum-proposals.json`. They are **strictly advisory** — they never touch `crates/memory-benchmark/src/fixture/`. A human reviewer drafts a PR adding the new fixture; CI's `candidate_score_bands_stay_calibrated` test protects against fixtures that would push any reference out of the [70, 90] band.

## 9. Per-cycle receipt

Committed alongside the promotion (or as a solo commit if no promotion):

```json
{
  "cycle_id": "0000123",
  "started_at_run": "0000123",
  "best_state_hash": "fnv1a:a3b1...",
  "best_state_parent": "fnv1a:5d2c...",
  "candidate_count": 20,
  "tier_distribution": { "T1": 10, "T2": 6, "T3": 3, "T4": 1 },
  "build_failures": 1,
  "determinism_failures": 0,
  "gates_failed": { "privacy_leaks": 0, "nondeterminism": 0, "future_leaks": 0 },
  "promotions": 1,
  "promoted_worker": "lane_07",
  "promoted_diff_hash": "fnv1a:9e2f...",
  "shadow_drift": 1.2,
  "public_score": 87.4,
  "shadow_score": 86.2,
  "cost_usd": 3.18,
  "reducer_wall_clock_ms": 4500,
  "scoreboard_sha": "fnv1a:f019..."
}
```

Receipts form a continuous chain on `autoresearch/chase-best`. Aggregate receipts produce the audit trail.

## 10. Abort conditions

The orchestrator (not the ZYAL daemon) enforces. On abort: stop spawning workers, leave the last 3 cycles' worktrees in place, drop `aborted.flag` + a reason file + `forensics-bundle.tar` (last 3 receipts + current worktree diffs).

Conditions:
1. **Shadow regression**: any cycle where `shadow_delta < -3.0` even if not promoted → pause for human review.
2. **Gate spike**: total gate count across all 20 workers > 5× trailing-10-cycle median.
3. **Worker crash rate** > 20% (>4 of 20 fail before emitting a report) for 2 consecutive cycles.
4. **Reference drift** > 1.0 point on ANY frozen reference adapter (strongest single signal that the benchmark was illegally touched).
5. **Negative memory growth** > 50 entries/cycle for 5 cycles (proposer is degenerating).

## 11. Files to touch

### New

- `tools/autoresearch/Cargo.toml`, `tools/autoresearch/src/main.rs` — orchestrator binary. Subcommands: `tick` (run one cycle), `daemon` (loop), `seed` (initialize `autoresearch/chase-best`), `forensics` (bundle on abort).
- `tools/autoresearch/src/proposer/{genetic.rs, template.rs, llm.rs, mod.rs}` — proposer modules.
- `tools/autoresearch/templates/t2/*.rs.tmpl`, `t3/*.rs.tmpl` — template library.
- `tools/autoresearch/prompts/mutate_t4.md` — LLM prompt template.
- `crates/memory-benchmark/src/bin/chase_reduce.rs` — new reducer binary, wraps `chase_report::build_chase_outputs` with shadow + reference-drift + receipt emission. `population_report` stays untouched (existing preflight target undisturbed).
- New Justfile targets: `memory-benchmark-northstar`, `memory-benchmark-shadow`, `chase-reduce`, `chase-tick`, `chase-daemon`.

### Modified

- `docs/ZYAL/examples/memory-benchmark/autoresearch-chase.zyal`:
  - Replace `fan_out.split` shell with `autoresearch tick --emit-tasks`
  - Replace `fan_out.reduce` with `chase_reduce` invocation
  - Add `gates` block listing `shadow_divergence`, `reference_drift`
  - Add `worktree_root` field to fleet
  - Add `private-seed.env` to sandbox `paths` with `read` access
  - Bump `scoring.weights` to 12-axis (matches updated `AxisScores::WEIGHTS`)
- `crates/memory-benchmark/src/chase_report.rs`:
  - Extend `GateVector` with `shadow_divergence`, `reference_drift`
  - Extend `CandidateSnapshot` with `cycle_id`, `parent_id`, `mutation_kind`, `mutation_template`, `mutation_diff_hash`, `mutation_diff_size`
  - Extend `render_negative_memory` with new schema
  - Export `build_cycle_receipt(...)` helper
- `crates/memory-benchmark/Cargo.toml` — register the new `chase_reduce` binary. Zero-dep invariant preserved.
- `.gitignore` — add `.jekko/daemon/memory-benchmark-chase/worktrees/`, `.jekko/daemon/memory-benchmark-chase/target/`.

## 12. Directory layout

```
.jekko/daemon/memory-benchmark-chase/
├── worktrees/<cycle_id>/<worker_id>/    (gitignored)
├── target/<cycle_id>/<worker_id>/       (gitignored)
├── reports/lanes/<worker_id>.{json,patch,generated.json,qbank.json,compounding.json,hardening.json,shadow.json}
├── receipts/<cycle_id>.json
├── memory/population-ledger.jsonl       (existing)
├── negative-memory.jsonl                (extended schema)
├── curriculum-proposals.json            (existing)
├── best-state.json                      (existing)
├── promotion-decision.json              (existing)
├── scoreboard.tsv                       (existing)
├── best.patch                           (existing)
├── private-seed.env                     (sparse-excluded from worker checkouts)
├── paused.flag                          (control plane)
└── aborted.flag                         (control plane)
```

## 13. Risks (see `07-risks.md`)

- Proposer infinite-loops on local optimum → anti-stall (10 cycles → tier shift; 20 cycles → T4 structural-change-needed prompt).
- Overfit to public seed → shadow suite required for promotion.
- AutoResearch finds deterministic exploit of `weighted_fraction` → reducer reruns references in worker tree, rejects on reference drift.
- Worktree disk exhaustion → orchestrator prunes after last 5 cycles.
- LLM proposer leaks secrets in diff → pre-apply scan with `memory.redaction.patterns`.
- Tier bypass via symlinks → `fs::canonicalize` all patch paths; reject anything outside allowlist.
- Cost runaway → per-cycle T4 budget $5, aggregate $25; orchestrator halts T4 calls when spend > $4.
