# refs/snapshot.md — Implementation Snapshot

Captured after Phase 1-5 implementation and refreshed after the 2026-05-13/14 Track A hardening pass in commit `2617e2a1b` plus the B1 ingest scaffold and B5/B6 follow-on tests. Numbers below come from running each lane on a development machine with the toolchain pinned at Rust 1.95.0.

Correction note: the earlier 90.65 cogcore northstar and 100.00 hardening score were pre-Track-A values. The current benchmark uses reinforce-between-query hardening semantics, production QBank missing-paper failure, fresh AutoResearch references, absolute reference drift, and dev-only promotion rejection.

## Scoring snapshot (post-Track-A)

| Candidate | Northstar | T0 | T1 (120) | Compounding (24) | Hardening (20) | QBank (50) |
|---|---:|---:|---:|---:|---:|---:|
| baseline | 73.31 | 61.53 | 80.00 | 89.94 | 10.00 | 100.00 |
| reference_context_pack | 83.13 | 80.50 | 100.00 | 97.12 | 10.00 | 100.00 |
| reference_evidence_ledger | 83.00 | 79.30 | 100.00 | 97.12 | 10.00 | 100.00 |
| reference_claim_skeptic | 82.88 | 78.10 | 100.00 | 97.12 | 10.00 | 100.00 |
| **cogcore** | **77.63** | **91.21** | 100.00 | 80.00 | 10.00 | 85.64 |

All 4 reference adapters remain within the [70, 90] northstar calibration band. cogcore lags references on compounding (80 vs 97) and QBank (85.64 vs 100.00); honest Track B targets.

## Track A snapshot (2026-05-13/14 audit hardening)

All 6 Codex-flagged safety findings closed. Catastrophic gates (drift /100, trusted_core_diff = patch.is_some()) now real defenses.

| Item | Status | File:line |
|---|---|---|
| A1 drift /100 removed | ✅ | chase_report.rs:590 |
| A2 trusted_core path inspection | ✅ | chase_report.rs:601 + helpers (~1328-1532) |
| A3 hardening reinforce-between-queries | ✅ | runner_generated.rs::run_hardening_case |
| A4 fresh-per-cycle ref reports | ✅ | tools/autoresearch/src/main.rs |
| A5 robust JSON parse | ✅ | autoresearch via memory_benchmark::json path dep |
| A6 clean-tree-only patch | ✅ | git worktree add replaces rsync |
| A7 forbidden-token scan in reducer | ✅ | patch_contains_forbidden_token |
| A8 per-cycle disk budget | ✅ | tools/autoresearch/src/main.rs::cmd_tick |
| A9 verify_determinism new suites | ✅ | already wired (compounding/hardening/real-papers) |
| A10 Justfile chase-* dev-only banner | ✅ | Justfile:427-446 |
| BONUS: dev_only promotion gate | ✅ | Codex — CandidateSnapshot::dev_only rejection |

## Test counts post-Track-A + B1

- memory-benchmark: **91** (was 70 pre-Track-A, +21 new gate/timestep/QBank-dev_only/path-inspection/etc. tests)
- cogcore: **44** (was 30, +3 hardening_converges, +1 scale_10k, +10 ingest tests)
- autoresearch: **3** (was 1, +2 Codex)
- Total: **138 tests green**

| Suite | Count |
|---|---:|
| `cargo test --manifest-path crates/memory-benchmark/Cargo.toml --locked --no-fail-fast` | 91 |
| `cargo test --manifest-path crates/cogcore/Cargo.toml --locked --no-fail-fast` | 44 |
| `cargo test --manifest-path tools/autoresearch/Cargo.toml --locked --no-fail-fast` | 3 |

## Bonus performance fix

`Core::has_supersession_partner` was O(N²) — supersession check iterated all cells per candidate. Fixed to use existing `subject_index` (BTreeMap). p99 recall @ 10K cells release: **102ms → 7.5ms**. Zero API change, determinism preserved.

## B1 ingest scaffold shipped

`crates/cogcore/src/ingest/` (944 LoC, 10 tests, zero new deps):

- `mod.rs` — `IngestBackend` trait
- `paper.rs` — `IngestedPaper` / `PaperSection` / `SourceSpec` (cogcore-internal mirror types to avoid qbank-builder dep cycle), `RuleBackend`, `parse_jsonl_event` (consumer side of B7's emit contract)
- `equation.rs` — LaTeX-ish extractor + SI unit normalization
- `theorem.rs` — Theorem header regex + dependency-DAG scaffold

Handoff contract for B7: JSONL of StoredEvent shape (documented at `AGENT_CHAT.md` ~2026-05-13T23:30Z post). `parse_jsonl_event` round-trip tested.

## Known gaps

- cogcore compounding 80 vs references 97 — root cause diagnosed: prior-fixture cells leak via BM25 token overlap. Quick literal-substring gate caught it (compounding 80→97) but cost 1.90 T0 points (revert). Real fix: B2 utility decay or concept tightening. Deferred.
- All adapters score 10.00 on hardening — none compress `used_ids` or tokens under reinforcement. Real product gap, awaits B2 consolidation.
- cogcore QBank 85.64 vs refs 100.00 — likely BM25 tokenization missing surface-form variants. Diagnostic open.
- Audit score 84 (1 point shy of 85) — file-split refactor in flight to clear soft `:shape` finding.

## Compounding & hardening — what they actually measure now

- **Compounding** suite (24 fixtures, 6 fixture-kinds: math_chain, physics_chain, paper_distillation, procedure_evolution, cross_domain_transfer, poisoned_paper) — multi-event ingest + multi-query reasoning. Score: depth-weighted mean (depth_weights = [1.0, 1.5, 2.25, 3.4]).
- **Hardening** suite (20 fixtures, 5 timesteps each) — canonical event + 4 reinforcement events observed between each of 5 queries. Score: 0.4*support_concentration + 0.3*confidence_growth + 0.2*token_reduction + 0.1*determinism, gated-to-zero on correctness.

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
