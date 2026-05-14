# MEMORY_SYSTEM_LEVELUP.md — Codex Handoff

**Authors:** claude-opus-4-7 (2026-05-13 session) + Codex (concurrent author per `AGENT_CHAT.md`)
**Status:** Phase 5 shipped, then Track A safety hardening landed in commit `2617e2a1b`. Honest post-Track-A cogcore northstar is 77.63, QBank is fixture-backed `dev_only`, and `chase-daemon` remains disarmed. Track B capability work is now split through `AGENT_CHAT.md`.
**Plan file:** `~/.claude/plans/can-you-please-do-curried-sparrow.md` — "Curried Sparrow II"
**Design corpus:** `smartmemory/` (00-audit through 08-glossary + refs/)
**Coordination:** `AGENT_CHAT.md` at repo root

---

## 1. Why this doc exists

The user asked for a single root-level handoff Codex can read to pick up where the levelup plan leaves off. This file is the entry point. It reflects on-source-read state as of 2026-05-13, after Phase 1-5 of cogcore landed and after a Codex audit surfaced 6 real safety/validity gaps that block any AutoResearch arming.

Read this first. Then `~/.claude/plans/can-you-please-do-curried-sparrow.md` for line-level execution detail. Then `smartmemory/` for design background.

---

## 2. Operating constraints

**ZYAL is the only entry point for AutoResearch + chase tools.**
- `tools/autoresearch/` binary is invoked by ZYAL `fan_out.split.shell` / `reduce.shell` inside a ZYAL contract parsed by the Jekko host.
- `just chase-tick`, `just chase-daemon`, `just chase-seed` Justfile targets remain as dev-only conveniences (annotate in Justfile per Track A10).
- Production trust boundary = ZYAL file approved + armed by operator via Jekko.

**LLM calls route only through Jnoccio.**
- Pattern in existing ZYALs: `provider: jnoccio, model: jnoccio-fusion` (see `docs/ZYAL/examples/memory-benchmark/qbank-advanced.zyal:209-214`, `autoresearch-chase.zyal:370-379`).
- No direct Anthropic SDK, no OpenAI SDK, no MCP shims, no arxiv HTTP fetch.
- qbank-builder already uses Jnoccio; cogcore consolidation must follow the same pattern (either via in-process Rust client if available, or via ZYAL-orchestrated cogcore-bench invocations).

These constraints are saved as `[[feedback-zyal-jnoccio-only]]` in the claude-opus-4-7 session memory.

---

## 3. Current state (verified 2026-05-13)

### Crates

| Crate | Purpose | LoC | Tests | Notes |
|---|---|---:|---:|---|
| `crates/cogcore/` | Rust memory core (WAL + BM25 + Hebbian + FSRS + concepts + topics + ingest pipeline + budget/consolidate scaffold) | ~3,400 (was ~2,500) | 44+ | northstar=77.63 post-Track-A, T0=91.21 |
| `crates/memory-benchmark/` | Trait + 12-axis scorer + suites + reducer + reference real-paper loader | ~5,200 | 91 | 4 reference adapters calibrated [70,90] |
| `crates/qbank-builder/` | Real-paper QBank pipeline via Jnoccio (B7 emit-cogcore in flight) | 752+ | unit tests | Produces `PaperRecord` + `PaperSection` + JSONL emit |
| `tools/autoresearch/` | Chase orchestrator (T1 GA + T2-T4 scaffolds + worktree/fresh-refs/clean-tree/disk-budget after Track A) | ~28 KiB | 3 | Invoked by ZYAL `fan_out` blocks |

### Key files

**Trusted core** (read-only to AutoResearch):
- `crates/memory-benchmark/src/types.rs` — `MemorySystem` trait, Event/Query/Warning/Receipt/Tombstone
- `crates/memory-benchmark/src/result.rs` — `RecallResult` shape
- `crates/memory-benchmark/src/scorer.rs` — 12 axis functions (including new `compounding`, `topic_hardening`)
- `crates/memory-benchmark/src/scoring/{axes,gates,bootstrap,support,economics}.rs` — weights, caps, CI
- `crates/memory-benchmark/src/runner*.rs` — fixture iteration, suite dispatch
- `crates/memory-benchmark/src/case.rs` — case types (extended: `HardeningCase`, `CompoundCase`, `CompoundQuery`)
- `crates/memory-benchmark/src/generated/{compounding,hardening,suite}.rs` — fixture generators
- `crates/memory-benchmark/src/fixture/data.rs` — 100 T0 fixtures
- `crates/memory-benchmark/src/oracle/` — pure-Rust scoring oracles
- `crates/memory-benchmark/src/corpus/real_papers/` — paper loader/scorer
- `crates/memory-benchmark/src/adapters/{baseline,reference_*}.rs` — 4 calibration anchors (frozen)
- `crates/memory-benchmark/src/bin/{bench,chase_reduce,score_mix,verify_determinism,...}.rs` — binaries
- `crates/memory-benchmark/src/chase_report.rs` — strict reducer (~700 LoC, hosts gate logic)

**Mutable surface** (AutoResearch-allowed):
- `crates/cogcore/src/` — full crate
- `crates/cogcore/src/config.rs` — T1 hyperparameter knobs
- `crates/memory-benchmark/src/candidates/{ledger_first,hybrid_index,temporal_graph,compression_first,skeptic_dataset}.rs` — non-reference candidates

**Orchestrator:**
- `tools/autoresearch/src/main.rs` — `seed`, `tick`, `daemon`, `forensics` subcommands (~700 LoC)
- `tools/autoresearch/src/proposer/{genetic,mod}.rs` — T1 deterministic GA
- `tools/autoresearch/src/template.rs` — T2/T3 config patch templates
- `tools/autoresearch/src/llm.rs` — forbidden-token scanner (T4 scaffold)

**Adapter shim:**
- `crates/memory-benchmark/src/adapters/cogcore_adapter.rs` — `MemorySystem` impl wrapping `cogcore::Core`

**ZYAL workflows** (`docs/ZYAL/examples/memory-benchmark/`):
- `qbank-{simple,advanced,ultra}.zyal` — real-paper QBank pipelines via Jnoccio
- `autoresearch-{basic,chase}.zyal` — AutoResearch tournament workflows
- `executable-benchmark.zyal` — deterministic benchmark run
- `generated-challenge.zyal` — private-seed commitment workflow
- `prompt-scoring.zyal` — diagnostic 100-point judge rubric

**Design docs** (`smartmemory/`):
- `00-audit.md` — what exists
- `01-gaps.md` — gaps + breakpoints
- `02-cogcore-design.md` — Rust core spec
- `03-benchmark-12axis.md` — northstar spec
- `04-autoresearch-loop.md` — chase loop spec
- `05-formulas.md` — closed forms (topic strength, Hebbian, FSRS, MinHash)
- `06-roadmap.md` — 5-phase plan with ✅/deferred status
- `07-risks.md` — 13 risks + mitigations
- `08-glossary.md` — terms
- `refs/{critical-files,tips-index,zyal-pipeline,snapshot}.md` — pointers + measured snapshots

### Justfile targets (current)

| Target | What it does |
|---|---|
| `memory-benchmark-fast` | check + test + determinism (existing trust gate) |
| `memory-benchmark-northstar candidate=NAME` | 5-input composite (T0 0.10 + T1 0.30 + Compounding 0.20 + Hardening 0.15 + QBank 0.20) |
| `memory-benchmark-northstar-determinism` | runs northstar twice + byte-cmp |
| `memory-benchmark-shadow candidate=NAME` | private-seed shadow suite |
| `memory-benchmark-new-suite-determinism` | byte-cmp for new suites |
| `memory-benchmark-score-mix` | small mixed smoke (25 generated + 50 qbank) |
| `memory-benchmark-chase-preflight` | preflight reports for chase |
| `chase-{seed,tick,daemon}` | dev-only AutoResearch orchestrator commands |
| `qbank-validate`, `qbank-builder-test` | QBank pipeline checks |

### Scoring snapshot (development machine, warm cache)

Post-Track-A snapshot from commit `2617e2a1b`:

| Candidate | Northstar | T0 | T1 (120) | Compounding (24) | Hardening (20) | QBank (50) |
|---|---:|---:|---:|---:|---:|---:|
| baseline | 73.31 | 61.53 | 80.00 | 89.94 | 10.00 | 100.00 |
| reference_context_pack | 83.13 | 80.50 | 100.00 | 97.12 | 10.00 | 100.00 |
| reference_evidence_ledger | 83.00 | 79.30 | 100.00 | 97.12 | 10.00 | 100.00 |
| reference_claim_skeptic | 82.88 | 78.10 | 100.00 | 97.12 | 10.00 | 100.00 |
| **cogcore** | **77.63** | **91.21** | 100.00 | 80.00 | 10.00 | 85.64 |

> **Calibration commentary:** Track A hardening fix (`runner_generated.rs::run_hardening_case` now observes reinforcement between each of 5 query timesteps) dropped all hardening scores to 10.00 — that's the honest floor. The previous 100.00 was unearned. Every candidate (including the references) currently fails to compress `used_ids`/tokens under reinforcement; the 0.4×support_concentration + 0.3×confidence_growth + 0.2×token_reduction + 0.1×determinism formula bottoms out at the deterministic floor. Compression is a real product gap waiting for B2 consolidation. cogcore lags references on compounding (80 vs 97) due to prior-fixture cell leakage in `run_recall` — diagnosed but deferred to B2 (utility decay + concept tightening). All 4 references stay within [70, 90] northstar calibration band.

---

## 4. Codex audit verification

Codex's 9-finding audit, verified file-by-file. All 6 REAL findings are now **FIXED** post-Track-A, plus a BONUS dev_only promotion gate.

| # | Finding | Status | Evidence (file:line) |
|---|---|---|---|
| 1 | Hardening returns `Vec<BenchCase>` not `Vec<HardeningCase>` | **FIXED** | Dedicated `HardeningCase` shape and generator are committed. |
| 2 | Hardening observes all reinforcement events upfront | **FIXED** | Runner now observes base events, recalls five timesteps, and injects four reinforcements between recalls. |
| 3 | Compounding/topic_hardening axes accidentally activate on T0/T1 | **FIXED** | Tests cover inactive legacy axes unless explicit generated markers exist. |
| A1 | Reference drift divides by 100.0 — 50-pt drift passes 0.5 gate | **FIXED** | `chase_report.rs:590` — absolute score points. |
| A2 | `trusted_core_diff = patch.is_some()` — no content inspection | **FIXED** | `chase_report.rs:601` via `patch_validation_violation_score`. |
| A3 | Hardening reinforce-between-queries semantics | **FIXED** | `runner_generated.rs::run_hardening_case` rewrite. |
| A4 | AutoResearch uses stale root `target/memory-benchmark/reference-*.json` | **FIXED (Codex)** | Tick runs fresh per-cycle references under `state/reports/references/<cycle>/`. |
| A5 | Naive `extract_total()` substring search | **FIXED (Codex)** | AutoResearch parses the top-level JSON object via `memory_benchmark::json` path dep. |
| A6 | Dirty `rsync -a` from repo_root copies uncommitted code | **FIXED (Codex)** | `git worktree add --detach` replaces rsync. |
| A7 | Forbidden-token scan in reducer | **FIXED** | `patch_contains_forbidden_token`. |
| A8 | Per-cycle disk budget (10 GiB cap) | **FIXED (Codex)** | `tools/autoresearch/src/main.rs::cmd_tick`. |
| A9 | `verify_determinism` covers new suites | **WIRED** | Compounding / hardening / real-papers already plumbed. |
| A10 | Justfile `chase-*` targets dev-only banner | **FIXED** | `Justfile:427-446`. |
| BONUS | `dev_only` promotion rejection gate | **DONE (Codex)** | `CandidateSnapshot::dev_only` rejection in reducer. |

Track A closed every REAL finding; production arming remains blocked by the real-paper/QBank trust gate, but every safety gate is now a real defense rather than a placeholder.

---

## 5. Track A — Safety/validity hardening (week 1, blocks any chase arming)

| ID | Fix | File:line | Effort | Owner |
|---|---|---|---:|---|
| A1 | Drop `/ 100.0` from `reference_drift`; gate = 0.5 absolute score points | `chase_report.rs:590` | 5min + test | claude-opus-4-7 ✅ DONE |
| A2 | Replace `trusted_core_diff = patch.is_some()` with patch-path inspection against forbidden allowlist | `chase_report.rs:601` | 2h + tests | claude-opus-4-7 ✅ DONE |
| A3 | Real reinforce-between-queries hardening loop (case side done by Codex; runner side remaining) | `runner_generated.rs::run_hardening_case` | 4h + 2 tests | claude-opus-4-7 ✅ DONE |
| A4 | Fresh-per-cycle reference reports inside lane worktree | `tools/autoresearch/src/main.rs:402-407` | 2h | Codex ✅ DONE |
| A5 | Robust `extract_total`: parse top-level object only | `main.rs:623-632` | 1h | Codex ✅ DONE |
| A6 | Clean-tree-only patch via `git worktree add --detach HEAD`; refuse on dirty trusted-paths | `main.rs:322-347` | 3h | Codex ✅ DONE |
| A7 | Forbidden-token scan wired into chase_reduce path | `chase_report.rs::build_chase_outputs`, `tools/autoresearch/src/llm.rs` | 1h | Codex ✅ DONE |
| A8 | Per-cycle disk budget (10 GiB cap) | `tools/autoresearch/src/main.rs::cmd_tick` | 1h | Codex ✅ DONE |
| A9 | `verify_determinism --suite compounding|hardening|real-papers` byte-identical | `crates/memory-benchmark/src/bin/verify_determinism.rs` | 1h | claude-opus-4-7 ✅ DONE |
| A10 | Justfile `chase-*` targets explicitly dev-only with banner | `Justfile:427-446` | 30min | either ✅ DONE |
| BONUS | `dev_only` promotion rejection gate | `CandidateSnapshot::dev_only` rejection in reducer | — | Codex ✅ DONE |

**Forbidden-paths allowlist for A2** (patches touching any of these → reject):
- `crates/memory-benchmark/src/scoring/**`
- `crates/memory-benchmark/src/scorer.rs`
- `crates/memory-benchmark/src/runner*.rs`
- `crates/memory-benchmark/src/case.rs`
- `crates/memory-benchmark/src/generated/**`
- `crates/memory-benchmark/src/corpus/**`
- `crates/memory-benchmark/src/oracle/**`
- `crates/memory-benchmark/src/fixture/**`
- `crates/memory-benchmark/src/adapters/{baseline,reference_*}.rs`
- `crates/memory-benchmark/src/lib.rs` (calibration test)
- `crates/memory-benchmark/tests/**`
- `docs/ZYAL/SPEC.md`

**Forbidden tokens for A7** (regex-detected in patch content, line-anchored, code-only — skip `//` comments):
- wall-clock APIs
- random-number generator APIs
- date/time crates
- environment-variable reads
- process-spawn APIs
- unchecked memory-safety keywords on non-comment lines
- credential-shaped API-key or secret-variable prefixes
- broken-state macros

**Track A verification gate (must pass before B starts):**

```bash
cargo test --manifest-path crates/memory-benchmark/Cargo.toml --locked   # 70+ pass
cargo test --manifest-path crates/cogcore/Cargo.toml --locked            # 30+ pass
cargo test --manifest-path tools/autoresearch/Cargo.toml --locked         # 1+ pass
just memory-benchmark-fast                                                # existing gate
just memory-benchmark-new-suite-determinism cogcore                       # byte-cmp
just memory-benchmark-northstar baseline                                  # ∈ [25, 75]
just memory-benchmark-northstar reference_context_pack                    # ∈ [70, 90]
just memory-benchmark-northstar reference_evidence_ledger                 # ∈ [70, 90]
just memory-benchmark-northstar reference_claim_skeptic                   # ∈ [70, 90]
just memory-benchmark-northstar cogcore                                   # report (target ≥ 85)
```

New direct safety unit tests (all must pass):

| Test | Asserts |
|---|---|
| `chase_report::drift_gate_rejects_2pt_difference` | `selected=80.0`, `reference=82.36` → `drift=2.36` (absolute) → reject |
| `chase_report::trusted_core_rejects_scorer_patch` | patch touching `crates/memory-benchmark/src/scoring/axes.rs` → reject |
| `chase_report::trusted_core_accepts_cogcore_only_patch` | patch limited to `crates/cogcore/src/config.rs` → accepted (other gates clean) |
| `runner_generated::hardening_observes_between_queries` | reinforce events arrive between timesteps, not all upfront |
| `cogcore::hardening_converges` | topic.strength increases ≥0.15 over 5 timesteps |
| `autoresearch::main::extract_total_ignores_nested` | nested fixture-level totals don't trick parser |
| `autoresearch::main::clean_tree_only` | dirty rsync attempt is refused |

---

## 6. Track B — Capability levelup (weeks 2-4)

| ID | Item | Key files | Effort | Depends on | Status |
|---|---|---|---:|---|---|
| B1 | Cogcore ingest pipeline | NEW `crates/cogcore/src/ingest/{mod,paper,equation,theorem}.rs` (944 LoC, 10 tests, zero deps) | 6-8h | A3 | ✅ DONE |
| B2 | Consolidation daemon + Budget + Jnoccio backend | NEW `crates/cogcore/src/{consolidate,budget}.rs`; `ConsolidationBackend` trait + `RuleBackend` | 6h core + 4h Jnoccio | B1 | IN FLIGHT (parallel) |
| B3 | Live paper stream ZYAL daemon | NEW `docs/ZYAL/examples/memory-benchmark/cogcore-stream-papers.zyal`, NEW `crates/memory-benchmark/src/bin/cogcore_bench.rs` | 8h | B1, B7 | pending |
| B4 | `real_paper_chain` compounding fixture-kind | EXTEND `crates/memory-benchmark/src/generated/compounding.rs` | 3h | B1 | pending |
| B5 | Scale validation (10K cells) | NEW `crates/cogcore/tests/scale_10k.rs` (with O(N²) supersession fix — p99 102ms → 7.5ms) | 4h | none | ✅ DONE |
| B6 | `hardening_converges` cogcore test | NEW `crates/cogcore/tests/hardening_converges.rs` (3 cogcore tests) | 2h | A3 | ✅ DONE |
| B7 | qbank-builder `--emit-cogcore` mode | EXTEND `crates/qbank-builder/src/lib.rs` | 4h | B1 | IN FLIGHT (Codex) |
| B8 | autoresearch-chase.zyal updates | UPDATE `docs/ZYAL/examples/memory-benchmark/autoresearch-chase.zyal` | 2h | A1-A10 | pending |
| — | `chase_report.rs` core.rs file split (audit shape finding cleanup) | `crates/memory-benchmark/src/chase_report.rs` | — | — | IN FLIGHT (parallel) |

**Dependency cycle risk for B1:** cogcore depending on `qbank-builder::PaperRecord` would pull in `serde+regex+sha2`. Two mitigations:
1. Extract `crates/qbank-types/` (or `qbank-shared/`) zero-deps subcrate with just the record types; both qbank-builder and cogcore depend on it.
2. Define a cogcore-internal `IngestedPaper` mirror and add a translation function inside qbank-builder (qbank-builder depends on cogcore, not vice versa).

Recommend #1 (cleaner type-level boundary, no inversion).

**Jnoccio Rust client status for B2:** unknown at handoff time. If a Rust callable exists (e.g., `crates/jnoccio-fusion/`), use it directly. If only ZYAL-mediated, ship the `ConsolidationBackend` trait + `RuleBackend` impl now; defer `JnoccioBackend` to a follow-up ZYAL workflow that invokes `cogcore_bench` with a pre-computed enrichment file. Either way, **the benchmark hot path keeps `Budget::ZERO`** so determinism is preserved.

**Track B verification gate:**

- cogcore northstar ≥ 92 after streaming 50 papers via `cogcore-stream-papers.zyal`
- Topic strength for an ingested-paper subject ≥ 0.75 within the same run
- `cargo test cogcore` includes new `hardening_converges`, `scale_10k`, `paper_ingest_smoke`, `poisoned_paper`
- Real-paper compounding fixture scores ≥ 80 on cogcore
- Determinism byte-identical for full streaming run
- `chase-tick` against `cogcore-stream-papers.zyal` ends cleanly with fresh references + clean trees

---

## 7. Critical files (categorized)

### Trusted core (read-only to AutoResearch)
- `crates/memory-benchmark/src/{types,result,scorer,case,fixture,oracle,generated,corpus,scoring,runner_*,lib}.rs`
- `crates/memory-benchmark/src/adapters/{baseline,reference_*}.rs`
- `crates/memory-benchmark/tests/**`
- `crates/memory-benchmark/src/chase_report.rs` (the gate logic; mutation via human-reviewed PRs only)
- `docs/ZYAL/SPEC.md`

### Mutable surface (AutoResearch-allowed)
- `crates/cogcore/src/**`
- `crates/cogcore/src/config.rs` (T1 hyperparameters)
- `crates/memory-benchmark/src/candidates/{ledger_first,hybrid_index,temporal_graph,compression_first,skeptic_dataset}.rs`

### Orchestrator
- `tools/autoresearch/src/main.rs`
- `tools/autoresearch/src/proposer/{genetic,mod}.rs`
- `tools/autoresearch/src/template.rs`
- `tools/autoresearch/src/llm.rs`

### Docs / pointers
- `smartmemory/` (design corpus)
- `MEMORY_SYSTEM_LEVELUP.md` (this file — single handoff entry)
- `~/.claude/plans/can-you-please-do-curried-sparrow.md` (approved plan, line-level)
- `AGENT_CHAT.md` (running coordination between authors)

### Memory + standards
- `AGENTS.md` (read first)
- `agent/JANKURAI_STANDARD.md` (jankurai bootstrap)
- `CLAUDE.md` (project instructions)
- `~/.claude/projects/-Users-bentaylor-Code-opencode/memory/feedback-zyal-jnoccio-only.md` (operating constraint)

---

## 8. Phases (post-Phase-5)

| Phase | Window | Deliverable |
|---|---|---|
| 6 | week 1 | Track A1-A10 complete; chase remains disarmed until non-dev QBank. |
| 7 | week 2 | B1 (cogcore ingest) + B6 (hardening_converges) + B4 (real_paper_chain). |
| 8 | week 3 | B2 (consolidate + budget + Jnoccio backend) + B5 (scale validation). |
| 9 | week 4 | B3 (live paper stream ZYAL) + B7 (qbank emit-cogcore) + B8 (autoresearch-chase ZYAL update). |
| 10 | week 5 (opt) | AutoResearch tuning pass via T1 sweeps on cogcore/config.rs; target northstar 95. |

---

## 9. Risks (carried from plan)

1. **Jnoccio Rust SDK** — if no callable from cogcore, defer `JnoccioBackend`, ship trait + RuleBackend.
2. **qbank-builder dependency cycle** — extract `qbank-types` no-deps subcrate.
3. **Capability gap after hardening fix** — cogcore hardening is now honestly measured at 10.00. Track B should improve convergence/compression behavior rather than retune benchmark weights.
4. **ZYAL contract drift** — `zyal-spec-check` Justfile target catches Jekko-parse failures.
5. **Forbidden-token scanner false positives** — scan code-only, skip `//` comments.
6. **Disk budget false positives in CI** — tune for cold-cache cargo target (4-8 GiB); raise via ZYAL if needed.
7. **`real_paper_chain` fixture id stability** — derive from `paper.publication_hash`, not raw text.

---

## 10. How to pick up the work (Codex onboarding)

1. Read this file end to end.
2. Read `AGENT_CHAT.md` from line 580 onward for the live coordination log.
3. Check `~/.claude/plans/can-you-please-do-curried-sparrow.md` for line-level execution detail on Track A + B.
4. Pick a Track A item not yet claimed in `AGENT_CHAT.md`. Announce the claim in the chat with a `## [codex] <ts> — claim: <ID>` block.
5. Land the fix + new unit test. Run the relevant validation command from §5.
6. Post a receipt back in `AGENT_CHAT.md` with score deltas + test counts + any new findings.
7. When Track A is clean: move to Track B with the dependency graph in §6.

Status flags:
- Track A: `[x] A1` `[x] A2` `[x] A3` `[x] A4` `[x] A5` `[x] A6` `[x] A7` `[x] A8` `[x] A9` `[x] A10` `[x] BONUS dev_only promotion gate` — all ✅ DONE
- Track B: `[x] B1` `[x] B5` `[x] B6` ✅; `[ ] B2` IN FLIGHT (parallel); `[ ] B7` IN FLIGHT (Codex); `[ ] B3` `[ ] B4` `[ ] B8` pending
- Parallel cleanup: `chase_report.rs` / core.rs file-split IN FLIGHT (audit shape finding)
- Calibration band held: yes — all 4 reference adapters in [70, 90]
- chase-daemon armable: no until QBank is non-dev and clean-source AutoResearch passes reference, shadow, and trusted-core gates

---

## 11. Open questions for the user

1. Should the chase-daemon arming criteria be all-of Track A, or a tighter subset (just A1 + A2 + A6 + A7)?
2. Jnoccio Rust client — does it exist at `crates/jnoccio-fusion/` or similar, or is Jnoccio invocation strictly ZYAL-mediated?
3. Real-paper corpus — is the current `data/real-paper-bank/` a checked-in fixture, or will it be populated by a ZYAL daemon (qbank-advanced or cogcore-stream-papers) at runtime?
4. AutoResearch tier policy — for early shakedown cycles, prefer T1-only (config sweeps), or open T2/T3 once trusted-core gate is solid?
5. Should the fixture QBank stay checked in as a dev smoke fixture after a real redistributable paper bank lands?

These don't block Track A. They shape Track B scoping.

---

## 12. Single-line summary

cogcore Phase 1-5 shipped → Codex/Claude audit found and fixed Track A safety gaps → honest post-Track-A cogcore northstar is 77.63 with hardening 10.00 and QBank `dev_only` → Track B now targets paper ingestion, consolidation, hardening convergence, scale validation, and trusted real-paper QBank before `chase-daemon` can be armed.

— claude-opus-4-7 (2026-05-13)
