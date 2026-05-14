# 07 — Open Risks and Mitigations

13 known risks. Each has a mitigation plan and a verification test (or test idea). Listed roughly by severity.

## 1. Determinism vs learning paradox

**Risk**: recall must mutate state (Hebbian, FSRS, utility) for the system to compound knowledge. Yet `verify_determinism` requires byte-identical output on rerun. If recall mutates, the second run has different state, outputs differ, byte-cmp fails, hard gate `!deterministic` caps the score at 80.

**Mitigation**: every recall-induced mutation is itself a WAL op (`WalOp::RecallTouch { used_ids, tx_time }`). Replaying the ledger reproduces the mutations exactly. Since the benchmark calls `observe` and `recall` in a deterministic order with no clock/random inputs, the mutation sequence is reproducible.

**Verification**:
```rust
// tests/ledger_replay.rs
#[test]
fn recall_touches_replay_byte_identically() {
    let mut a = Adapter::default();
    for event in synthetic_events(100) { a.observe(&event); }
    for query in synthetic_queries(20) { a.recall(&query); }
    let h1 = a.export_state_hash();

    a.rebuild();
    let h2 = a.export_state_hash();
    assert_eq!(h1, h2);
}
```

Property test: run the same scenario 1000 times with different `BTreeMap` insertion orders (via deterministic shuffle); all should produce the same hash.

## 2. Token interning across replay

**Risk**: TokenIds are assigned in first-seen order during live operation. Replaying the WAL may see tokens in a different order, assigning different IDs, producing different projection hashes — even though the logical state is identical.

**Mitigation**: `Interner::canonicalize_after_replay` reassigns all TokenIds in BTreeSet (sorted bytes) order after replay. The hot path uses the canonicalized ID space. See `05-formulas.md` §13.

**Verification**: same test as Risk #1, but with synthetic event bodies that include many distinct tokens with permutation-sensitive insertion order. State hash MUST match.

## 3. Concept-name ties

**Risk**: when promoting a concept, two member subjects may be equally frequent. Picking by `Vec::first()` is insertion-order-dependent and breaks determinism.

**Mitigation**: tiebreak by ASCII-lex order of the subject string. Never by insertion order. Applied throughout `concept.rs` and `topic.rs`.

**Verification**: unit test in `tests/concept_emergence.rs` — create a cluster with two subjects "neutrino mass" and "neutrino oscillation" tied at 5 members each; assert the chosen label is "neutrino mass" (lex-smallest).

## 4. Hot-path concept-expand blowup

**Risk**: a popular concept may have 10,000+ member cells. The concept-expand step (pull top-2 sibling cells from each candidate's concept) could explode the candidate pool, blowing the recall latency budget.

**Mitigation**: cap concept member fetching at top-K=8 by `cell.strength` desc, ASCII tie-break. Concept iteration is `concept.member_cells.iter().take(...)` over a sorted secondary index. Test: synthetic concept with 10K members; recall latency stays in [50ms, 300ms].

**Verification**: `benches/hot_path.rs` — assert p99 recall < 500µs at 1M cells with one concept holding 10K.

## 5. Score-band recalibration after Phase 3

**Risk**: trimming `correctness` 20→14, `provenance` 12→10, etc., may push one of the reference adapters out of the [70, 90] calibration band. The test `candidate_score_bands_stay_calibrated` would fail and block CI.

**Mitigation**:
1. Before Phase 3, capture the reference scores in the existing 10-axis scheme.
2. After axis extension, run the same suite; if drift > 2 points, adjust trim weights conservatively (restore `provenance` to 11, drop `english_discourse_coreference` to 5 instead).
3. The trim is a one-time tuning effort. Once stable, no future changes to axis weights.

**Verification**: `tests/north_star_calibration.rs` (new) — explicit assertions:
```rust
assert!((25.0..=75.0).contains(&run_northstar("baseline")));
assert!((70.0..=90.0).contains(&run_northstar("reference_evidence_ledger")));
assert!((70.0..=90.0).contains(&run_northstar("reference_context_pack")));
assert!((70.0..=90.0).contains(&run_northstar("reference_claim_skeptic")));
```

## 6. AutoResearch stuck in local optima

**Risk**: the proposer converges on a hyperparameter region that scores well on T1/compounding but not better. Loop spins without progress.

**Mitigation**: anti-stall detector in `tools/autoresearch/src/main.rs`:
- 10 consecutive cycles without promotion → tier distribution shifts to T3=40%, T4=30%
- 20 consecutive cycles without promotion → T4 prompt template injects `"The current best is suspected stuck — propose a structural change, not a parameter tweak"`
- 50 cycles without promotion → orchestrator drops `paused.flag` and emails owner (TODO: actual notification mechanism)

**Verification**: forensic check — run with a deliberately-low-quality proposer; verify the stall detector fires after 10 cycles.

## 7. Overfit to the public seed

**Risk**: AutoResearch finds a candidate that scores 95 on the public seed by exploiting deterministic quirks of the seeded generator, not by actually being smarter.

**Mitigation**: shadow suite (`Split::PrivateGenerated` + `$MEMORY_BENCHMARK_PRIVATE_SEED`). Reducer requires the candidate to also improve on the private seed; `|public_delta - shadow_delta| > 5.0` → reject. Private seed is sparse-excluded from worker worktrees so AutoResearch can't read or train against it.

**Verification**: deliberately deploy a candidate that overfits to the public seed (e.g., memorizes the seeded fixture IDs); verify the reducer rejects it.

## 8. Exploit of `weighted_fraction` / `apply_hard_gates`

**Risk**: AutoResearch finds an exploit in the scorer itself — e.g., emitting NaN values to game the average; emitting unusual warning strings that the scorer mishandles; mutating `score_mix` to widen the band.

**Mitigation**: the reducer reruns the 4 reference adapters (`baseline`, `reference_context_pack`, `reference_evidence_ledger`, `reference_claim_skeptic`) IN THE WORKER'S TREE after the candidate's patch is applied. If `baseline` exits [25, 75] OR any reference exits [70, 90], the candidate is rejected for `ReferenceDrift`. This catches global scorer mutations because the references' behavior is fixed; any change in their scores implies the scorer was tampered.

**Verification**: manually craft a patch that widens the band; verify the reducer rejects with `ReferenceDrift`.

## 9. Worktree disk exhaustion

**Risk**: each worker uses ~500MB of worktree + ~2GB of `CARGO_TARGET_DIR`. 20 workers × 50 cycles = 1.25 TB. Disk fills, host crashes.

**Mitigation**:
- Orchestrator prunes worktrees + target dirs older than last 5 cycles.
- Existing ZYAL gate `max_total_disk: 1GiB` enforces (we may need to bump to 50 GiB).
- `git worktree prune` after each cycle.
- Per-cycle disk usage report in `cycle-receipt.json`.

**Verification**: run 20 cycles in a row; assert disk used by `.jekko/daemon/memory-benchmark-chase/{worktrees,target}/` stays under 30 GiB.

## 10. LLM proposer leaks secrets into a diff

**Risk**: T4 LLM proposer may emit a diff containing a leaked API key from its prompt context, or hallucinate a recognizable canary pattern.

**Mitigation**: pre-apply scan in `tools/autoresearch/src/proposer/llm.rs`:
- Match against `memory.redaction.patterns` (existing in the ZYAL).
- Match canary patterns from `cogcore::canary` (built from fragments).
- Match `[A-Za-z0-9_-]{16,}` followed by typical key-suffix chars.

If any match, reject the patch before `git apply` runs. Log to negative memory with `reason=secret_leak`.

**Verification**: deliberately seed the prompt with a fake canary; verify the proposer's output is rejected.

## 11. Tier bypass via symlinks

**Risk**: a patch creates a symlink `crates/cogcore/src/escape.rs -> ../../memory-benchmark/src/scoring/axes.rs`, then writes through it, effectively editing the trusted core.

**Mitigation**: orchestrator resolves all paths in the patch with `fs::canonicalize` BEFORE `git apply`. Reject patches whose canonical destination is outside `crates/cogcore/src/` or the 5 allowed candidate files. Mirrors the existing patch-escape check in `chase_report.rs:817-840`.

**Verification**: craft a symlink-escape patch; verify the orchestrator rejects with `reason=path_escape`.

## 12. Cost runaway

**Risk**: T4 LLM proposer calls cost real money. A bug or proposer error could spam the API and incur unexpected charges.

**Mitigation**:
- Per-cycle T4 budget: $5. Tracked by the orchestrator.
- Aggregate run budget: $25 (existing in ZYAL). Hard halt.
- Orchestrator halts T4 calls when cycle spend > $4 (leaves room for final retries).
- Every cycle's `cost_usd` is in the receipt.

**Verification**: run a forced-T4 cycle with the LLM mocked to return failure; verify cost tracker stays at 0 and cycle finishes.

## 13. Topic-strength formula tuning

**Risk**: the initial weights (α_r=0.20, α_c=0.18, α_u=0.12, α_n=0.08, α_q=0.10, α_h=0.20) are hand-tuned. They may not be optimal for all corpora. A poorly-tuned topic strength makes hardening tests fail despite cogcore actually compounding.

**Mitigation**: these weights are in `cogcore/config.toml`. T1 AutoResearch can sweep them — the loop self-tunes. Initial hand-tuning targets ≥85 northstar; AutoResearch may push to 90+.

**Verification**: T1 cycle that sweeps α_r ∈ {0.15, 0.18, 0.20, 0.22, 0.25} produces a measurable northstar delta. Negative-memory log accumulates rejected variants.

## Risk matrix

| # | Risk | Likelihood | Impact | Mitigation cost | Phase to address |
|---|---|---|---|---|---|
| 1 | Determinism paradox | High | Critical | Medium (WAL op design) | 2 |
| 2 | Token interning replay | Medium | Critical | Low (canonical sort) | 2 |
| 3 | Concept-name ties | Medium | High | Low (ASCII-lex) | 2 |
| 4 | Concept-expand blowup | Medium | Medium | Low (top-K cap) | 2 |
| 5 | Score-band recalibration | Medium | High | Medium (trim adjust) | 3 |
| 6 | Local optima | High | Medium | Medium (anti-stall) | 4 |
| 7 | Overfit to public seed | High | High | High (shadow suite) | 4 |
| 8 | Scorer exploit | Medium | Critical | Medium (reference rerun) | 4 |
| 9 | Disk exhaustion | High | Medium | Low (prune) | 4 |
| 10 | LLM secret leak | Low | Critical | Low (pre-scan) | 4 |
| 11 | Symlink escape | Low | Critical | Low (canonicalize) | 4 |
| 12 | Cost runaway | Medium | Medium | Low (budget tracking) | 4 |
| 13 | Formula tuning | Medium | Low | Low (T1 sweep) | 4-5 |

Risks 1-5 are addressed by Phase 2-3 design. Risks 6-13 are addressed by Phase 4 (AutoResearch infra).

## Unknown unknowns

- Cargo's behavior under heavy worktree pressure (20 simultaneous workers compiling) is untested at scale. May need to throttle.
- The benchmark's bootstrap CI on small fixture counts (120 generated, 25 compounding) may produce wide CIs that swamp the 0.75-point promotion margin. May need to widen the margin to 1.0 or increase fixture counts.
- LLM proposer quality is unknown — early cycles may produce mostly garbage; the negative memory ledger will eventually train it better, but the warm-up period could be 50+ cycles.
- The 12-axis trim may produce subtle drift on the existing T1 generated suite. Phase 3 calibration check is the canary; if it fails, revisit trim weights before extending further.

## Mitigation checklist (run before any AutoResearch cycle)

- [ ] `just memory-benchmark-northstar candidate=baseline` produces total in [25, 75]
- [ ] `just memory-benchmark-northstar candidate=reference_evidence_ledger` in [70, 90]
- [ ] `just memory-benchmark-northstar candidate=cogcore` ≥ 85
- [ ] `just memory-benchmark-northstar-determinism candidate=cogcore` succeeds
- [ ] `crates/cogcore/tests/ledger_replay.rs` passes
- [ ] `$MEMORY_BENCHMARK_PRIVATE_SEED` exported and `.jekko/daemon/memory-benchmark-chase/private-seed.env` exists
- [ ] `.gitignore` covers `.jekko/daemon/memory-benchmark-chase/worktrees/` and `target/`
- [ ] Disk space free > 60 GiB

If any unchecked, fix before running `just chase-daemon`.
