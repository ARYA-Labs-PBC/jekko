# 08 — Glossary

Project-specific terms used throughout `smartmemory/`, `crates/memory-benchmark/`, and `crates/cogcore/`. Alphabetic.

## A

**A-MEM** — agentic memory pattern (Xu et al., 2024). Zettelkasten-style memory where new cells dynamically link to existing concept nodes; the link graph and summaries evolve as the network grows. cogcore's `concept.rs::promote_concepts` implements an A-MEM-style algorithm via MinHash clustering.

**Adapter** — concrete implementation of the `MemorySystem` trait. cogcore exposes one (`cogcore::Adapter`). The benchmark has 28 (4 references, 5 candidates, 20 arena lanes).

**ADVANCED_WEIGHTS** — alternative 12-axis weights in `crates/memory-benchmark/src/scoring/axes.rs:29` (concept_learning, transfer_reasoning, formal_math, …). Used by `population_report` for an alternate rollup. Independent of the new 12-axis `WEIGHTS` for the north-star.

**AutoResearch** — the chase loop. Aggressive parameter+algorithm search over memory designs, gated by the benchmark, run in worktree-isolated workers with shadow suite checks.

**Axis** — one dimension of scoring. cogcore's 12 axes (after Phase 3): correctness, provenance, math_science, bitemporal_recall, contradiction, english_discourse_coreference, privacy_redaction, procedural_skill, feedback_adaptation, determinism_rebuild, **compounding**, **topic_hardening**. Each weighted to sum to 100.

## B

**BENCH_NOW** — `"2026-05-12T00:00:00Z"` (`crates/memory-benchmark/src/memory_api.rs:14`). The canonical "now" used by deterministic bench runs. Hot paths must NEVER call `std::time::SystemTime::now()`.

**Bitemporal** — having two time axes: `valid_from`/`valid_to` (world time, when the fact is true) and `tx_time` (transaction time, when we recorded it). cogcore preserves both; `recall_at` filters by world time, `recall_as_of` by transaction time.

**BLAKE3** — cryptographic hash used in `experimental_blake3` feature. Default is FNV-1a (faster, stdlib-only, sufficient for byte-identity in a non-adversarial deterministic context).

**BM25** — Best Match 25 ranking algorithm. cogcore implements a BM25-lite without an external `tantivy` dep — inverted index in sorted `Vec`s, standard formula with `k1=1.5`, `b=0.75`.

**Bootstrap CI** — 95% confidence interval via bootstrap resampling (`crates/memory-benchmark/src/scoring/bootstrap.rs`). Computed per-suite, reported in north-star JSON.

## C

**Calibration band** — score range a reference adapter must hit. `baseline ∈ [25, 75]`, `reference_* ∈ [70, 90]`. Enforced by `candidate_score_bands_stay_calibrated` test in `crates/memory-benchmark/src/lib.rs:130`.

**Candidate** — any registered memory implementation. Includes baseline, references, 5 non-reference candidates, 20 arena lanes, and (post-Phase 1) cogcore. Listed in `runner.rs::boxed_adapter`.

**Causal mask** — when `recall_as_of(query, tx)` is called, cells with `tx_time > tx` are dropped. The recall result must carry `Warning::CausalMaskApplied` to signal this.

**ClaimModality** — enum from `crates/memory-benchmark/src/types.rs:60`:
- `Observed` (we directly saw it)
- `AssertedBySource` (some external source asserted it)
- `InferredByAgent` (we derived it)
- `HumanApproved` (a human confirmed it)
- `FormallyVerified` (Lean/Coq/dimensional check passed)

Prevents conflation of "paper says X" with "I believe X."

**chase_reduce** — new binary `crates/memory-benchmark/src/bin/chase_reduce.rs`. The AutoResearch reducer. Wraps `chase_report::build_chase_outputs` with shadow check, calibration anti-tamper, and receipt emission.

**Compounding** — the property that knowledge gained from earlier events makes later queries more answerable. Measured by the new `compounding` axis (weight 10) on the `Split::PublicCompounding` suite. Fixture-kinds: math_chain, physics_chain, paper_distillation, procedure_evolution, cross_domain_transfer, poisoned_paper.

**Concept** — A-MEM/Zettelkasten unit. ≥3 cells sharing a token-bigram Jaccard ≥ 0.55. Has `kernel_tokens` (top-15 TF-IDF intersection), `member_cells` set, MinHash sketch.

**Co-activation (coact)** — Hebbian matrix `BTreeMap<(CellId, CellId), f32>`. Increments on shared recall, increments more on feedback success, decrements on falsified feedback. Offline decay every 30d half-life; prune < 0.02.

**ContextPack** — the output of `recall`. The MEMSPEC corpus calls it this; in code it's `RecallResult` (`crates/memory-benchmark/src/result.rs`). Token-budget-bounded, deterministically packed via greedy fill.

**Curriculum proposal** — advisory output from the reducer when score distribution is pathological (saturated, universal failure, bimodal). Appended to `curriculum-proposals.json`. Never auto-applied — a human reviewer drafts a fixture PR.

## D

**Determinism rebuild** — axis 10 (weight 6). Tests whether `context_pack_hash` is non-empty AND stable across `rebuild()`. cogcore achieves this via `WalOp::RecallTouch`.

**`dump_tasks`** — binary `crates/memory-benchmark/src/bin/dump_tasks.rs`. Used by `fan_out.split` in chase ZYAL to scatter work items across workers.

## E

**`experimental_*` features** — opt-in Cargo features for cogcore: `experimental_blake3` (faster hash, 1 dep) and `experimental_hnsw` (real ANN, 1 dep). Off by default to preserve zero-dep guarantee.

**EpisodeStep** — `crates/memory-benchmark/src/case.rs`. One unit of activity in a fixture's input sequence — either an `Event` to observe or a `Query` to recall.

**EventKind** — 16-variant enum (`crates/memory-benchmark/src/types.rs:28`). cogcore stores cells with these kinds; some get special projections (Equation → equation lane, Theorem → DAG, Skill → procedural lane, Counterexample → never decay).

## F

**Feedback adaptation** — axis 9 (weight 4). The candidate's `confidence` on recall must fall in the fixture-declared `(lo, hi)` band. cogcore uses `confidence = 0.6 · cell.utility + 0.4 · cell.source_quality`.

**Fixture** — one unit of T0 scoring. Either a hand-crafted fixture in `src/fixture/data.rs` or a generator-produced one. Has `EpisodeStep`s (input), `Expected` (oracle), `block`, `domain`, `grade` function.

**Fleet** — ZYAL term for the set of workers running a daemon. Chase ZYAL uses `fleet.max_workers: 20, isolation: git_worktree`.

**FNV-1a** — Fowler-Noll-Vo hash, 64-bit. Used everywhere in `crates/memory-benchmark/src/hash.rs`. Fast, stable, no deps, sufficient for byte-identity in a non-adversarial context.

**FSRS** — Free Spaced Repetition Scheduler (Anki's variant). cogcore uses an adapted version on both cells (per-cell `strength` + `half_life_hours`) and topics. Half-life doubles with each successful recall at high success rates.

## G

**Gate** — score-cap rule. `apply_hard_gates(score, gates)` (`crates/memory-benchmark/src/scoring/gates.rs:10`) takes the minimum of multiple `score.min(cap)` checks. Strictest cap wins.

**GateFindings** — struct in `gates.rs`. Tracks `unsafe_tool_exec`, `privacy_leaks`, `citation_issues`, `future_leaks`, `deterministic`, plus 3 new fields after Phase 3 (`compounding_regression`, `hardening_regression`, `knowledge_non_degradation`).

## H

**Hardening** — see "Topic hardening."

**Hebbian** — neuropsychological principle: "neurons that fire together wire together." cogcore's coact matrix is a Hebbian implementation: cells used in the same recall reinforce their pairwise weight.

**HNSW** — Hierarchical Navigable Small World graph. Approximate-nearest-neighbor algorithm. cogcore's `experimental_hnsw` feature would wire `hnsw_rs`; default uses BM25 + concept-expand + graph rerank (no neural embedding).

## I

**Ingest** — the paper-loading path. `cogcore::ingest::paper::ingest(text)` splits sections, extracts claims/equations/theorems/citations, builds MemoryCells, appends to ledger, updates concepts/topics.

**Interner** — `cogcore::index::Interner`. Maps token bytes → `TokenId` (u32). After `rebuild()`, IDs are canonicalized in sorted-bytes order so projections hash identically.

## J

**Jaccard** — set similarity `|A ∩ B| / |A ∪ B|`. cogcore uses MinHash to estimate Jaccard over token bigrams for concept clustering. Threshold τ_form = 0.55.

**Jekko** — the daemon host. Runs ZYAL workflows. cogcore is intended to be embeddable as a Jekko daemon's memory backend.

**Jnoccio** — see `tips/smart_memory` and the user memory `[[jnoccio-unlock-pipeline]]`. The fusion source used by some ZYAL daemons; not directly invoked by cogcore.

## K

**Kernel tokens** — the top-15 TF-IDF tokens that define a concept. New cells attach to a concept if their token Jaccard with `kernel_tokens` exceeds τ_attach = 0.45.

**knowledge_non_degradation** — Phase 3 hard gate (cap 80). Fires when a poisoned paper lowers the score on an unrelated control topic. Indicates structural cross-contamination in the memory.

## L

**Lane** — multiple meanings:
1. **Memory lane** — MEMSPEC term for typed memory store (Core, Episodic, Semantic, ...). cogcore implements these implicitly via `CellKind`.
2. **Arena lane** — one of `arena_lane_00..19`, thin policy wrappers around references.
3. **ZYAL lane** — one variant in `experiments.lanes` (e.g., `question_tournament`, `saturated_blind_checks`).

**Ledger** — the WAL. Append-only, hash-chained, source of truth. All projections (BM25 index, subject map, concept arena, topic arena, coact matrix) are rebuildable from the ledger.

## M

**MemorySystem** — the trait. `crates/memory-benchmark/src/types.rs:154`. Defines the public boundary between the benchmark and any adapter.

**MIRIX** — academic memory system (mentioned in MEMSPECs). 6-lane taxonomy. cogcore's `EventKind` is loosely inspired by MIRIX but uses 16 kinds.

**MinHash** — locality-sensitive hashing for set similarity. cogcore uses 8-hash MinHash on token bigrams for concept clustering. 8 hashes give ~0.18 expected error on Jaccard estimate.

## N

**North-star** — the single composite score in `target/memory-benchmark/northstar.json`. Computed by `score_mix.rs` over T0(0.10), T1(0.30), compounding(0.20), hardening(0.15), qbank(0.20). After hard gate caps.

**Nondeterminism token** — a forbidden source of non-determinism in the worker patch: `SystemTime::now`, `Instant::now`, `rand::`, `chrono::`, `env::var(`, `process::Command`, `unsafe`. AutoResearch rejects any patch containing one.

## O

**Oracle** — pure-Rust function that grades a `RecallResult`. cogcore's benchmark uses oracles in `crates/memory-benchmark/src/oracle/` (privacy, provenance, temporal, theorem_dag, unit, workflow). All deterministic, no LLM.

## P

**`pack_hash`** — `crates/memory-benchmark/src/memory_api.rs:18`. FNV-1a of the canonical JSON form of a `RecallResult` (with `context_pack_hash` field omitted). Used for byte-identity of context packs.

**Pathology** — 10-variant enum of memory failures (`crates/memory-benchmark/src/types.rs:207`): FutureLeak, SupersededClaim, PrivacyLeak, UnitMismatch, SourceHallucination, CoreferenceError, CompressionDrift, RankingIgnored, SkepticBlindness, ModalityConfusion.

**Population ledger** — `.jekko/daemon/memory-benchmark-chase/memory/population-ledger.jsonl`. Existing append-only log of all worker outcomes in the chase.

**Promotion** — in AutoResearch, accepting a worker's patch as the new `autoresearch/chase-best`. Requires: all hard gates clean, no new gate failures, deterministic, ≥0.75 point public delta, reference drift ≤ 0.5, shadow delta ≥ 0, divergence ≤ 5.0.

**Provenance** — axis 2 (weight 10 post-Phase 3, was 12). Tests whether fixtures requiring citation get non-empty `Citations`. cogcore: every cell carries `Source { uri, citation, quality ≥ 0.85 }` and `Citation::from_source(...)` cites all used cells.

## Q

**QBank** — question bank. Generated via the `qbank-*.zyal` workflows from real papers; vetted by agent tournament + blind answering + critic audit.

**Query** — `crates/memory-benchmark/src/types.rs:76`. Has `text`, `intent: QueryIntent`, `mentions`, `token_budget`. Passed to `MemorySystem::recall(...)`.

## R

**`recall_as_of`** — historical recall, transaction-time filtered. Does NOT mutate state (no `RecallTouch`). Test in `tests/ledger_replay.rs` enforces.

**`RecallTouch`** — `WalOp` variant emitted after every successful `recall`. Records `used_ids` and `tx_time`. Makes Hebbian/FSRS/utility mutations replayable, preserving byte-identity. The load-bearing trick for "compounding without breaking determinism."

**Receipt** — `crates/memory-benchmark/src/types.rs:122`. Returned by every state-mutating operation. Hash-chained: `hash = FNV-1a(prev_hash || seq || op)`.

**Reducer** — `crates/memory-benchmark/src/bin/chase_reduce.rs` (new in Phase 4). Aggregates 20 worker reports, runs shadow + reference-drift checks, decides promotion, emits receipts.

## S

**`score_mix`** — `crates/memory-benchmark/src/bin/score_mix.rs`. Computes a weighted-average composite from multiple bench JSON outputs. Already supports arbitrary `--input name:weight:path` flags. The north-star uses it with 5 inputs.

**Shadow suite** — `Split::PrivateGenerated` run with `$MEMORY_BENCHMARK_PRIVATE_SEED`. Detects overfit-to-public-seed. Reducer requires `|public - shadow| ≤ 5.0` for promotion.

**Skeptic** — GEMINI_V3 daemon that adversarially hunts contradictions. cogcore surfaces `Warning::SkeptikSurfaced` (sic — typo in the enum, retained for ABI compatibility) when topic `contradiction_pressure` is high or when a `Counterexample` event matches the query.

**Split** — suite identifier (`crates/memory-benchmark/src/case.rs`). Existing: `PublicSmoke`, `PublicGenerated`, `PublicStress`, `RealPapers`, `PrivateGenerated`. New in Phase 3: `PublicCompounding`, `PublicHardening`.

**State hash** — `cogcore::Adapter::export_state_hash()`. FNV-1a of sorted cell IDs + sorted coact triples + tombstones. Invariant under insertion order. Used for `rebuild()` byte-identity check.

## T

**T0 / T1 / T2** — benchmark tiers.
- T0 = `Split::PublicSmoke`, 100 hand-crafted fixed fixtures.
- T1 = `Split::PublicGenerated`, seeded synthetic, ~500 fixtures.
- T2 = `Split::PublicStress`, same generator at larger scale.

**Tantivy** — Rust full-text search library. Cited heavily in MEMSPECs. cogcore deliberately does NOT use it (zero-dep default); a simple sorted-Vec inverted index suffices for the benchmark scale (≤ 1M cells).

**Tier (AutoResearch)** — mutation surface size:
- T1 = config / hyperparameter, ≤30 lines
- T2 = single algorithm swap, ≤150 lines, 1 file
- T3 = new data structure, ≤600 lines, ≤5 files
- T4 = LLM-suggested edit, ≤1500 lines, ≤8 files

**Tombstone** — `crates/memory-benchmark/src/types.rs:131`. Recorded by `forget(...)`. Includes `deletion_proof` hash. Cells in the tombstone map are filtered out of all recalls.

**Topic** — community of concepts. Forms when ≥4 concepts have pairwise coact ≥ 0.40. Has `strength` ∈ [0, 1] computed by the topic-strength formula.

**Topic hardening** — the property that re-asking the same query over time should yield: smaller `used_ids` set, higher `confidence`, smaller `context_token_count`, byte-stable `context_pack_hash`. Measured by the new `topic_hardening` axis (weight 8) on the `Split::PublicHardening` suite.

**Trusted core** — the read-only subset of `crates/memory-benchmark/` that AutoResearch cannot edit. Defined in `04-autoresearch-loop.md` §1.

**`tx_time`** — transaction time. When the system recorded the event. Used by `recall_as_of` causal mask.

## U

**Utility** — `cell.utility ∈ [0, 1]`. EMA-tracked from feedback outcomes. `TaskSuccess|Verified` bump +0.20; `TaskFailure|Falsified` bump -0.30; `Ignored` bump -0.05. Drives `cell.strength` half-life and recall confidence.

## V

**Vault** — `PrivacyClass::Vault`. Cells with this class short-circuit the redaction path: never rendered; emit `[REDACTED:vault]` placeholder + `OmissionNote` + `Warning::Redacted`.

**`valid_from` / `valid_to`** — world-time validity window on a cell. `recall_at(query, world_t)` drops cells where `world_t < valid_from` or `world_t ≥ valid_to`.

**`verify_determinism`** — binary `crates/memory-benchmark/src/bin/verify_determinism.rs`. Runs a candidate twice on the same suite, byte-compares the JSON. Must exit 0 or the candidate gets `!deterministic` gate (cap 80).

**Voyager** — academic skill-library system. Cited in MEMSPECs. Pattern: skills are executable + tested + reliability-scored; failing skills get deprecated. cogcore stores skills as cells but does NOT execute them (deferred to Phase 6+).

## W

**WAL** — Write-Ahead Log. `cogcore::ledger`. Append-only, hash-chained, source of truth. Format: `[seq:u64][prev_hash:[u8;16]][op_tag:u8][payload_len:u32][payload][hash:[u8;16]]`.

**Warning** — 17-variant enum (`crates/memory-benchmark/src/types.rs:101`). Surfaced by `recall` in `RecallResult.warnings`. Some are required by oracles (e.g., `Contradicted`, `Redacted`, `UnitMismatch`).

**WEIGHTS** — `crates/memory-benchmark/src/scoring/axes.rs:16`. The 10-axis (pre-Phase 3) or 12-axis (post-Phase 3) weight table summing to 100.

**Worktree** — `git worktree`. One isolated copy of the repo. cogcore AutoResearch creates one per worker per cycle at `.jekko/daemon/memory-benchmark-chase/worktrees/<cycle_id>/<worker_id>`.

## Z

**Zero-dep** — no external Cargo dependencies. The benchmark crate enforces this in its `Cargo.toml` (`# Zero external dependencies. Standard library only.`). cogcore default also zero-dep.

**Zettelkasten** — slip-box note-taking method. Inspiration for A-MEM and cogcore's concept emergence. New cells dynamically link to existing concepts; the network self-organizes through usage.

**ZYAL** — Jekko host daemon DSL (v2.6.0). YAML-based. Specifies workflows, fleets, sandboxes, gates, fan_out, reduce, memory stores. `docs/ZYAL/SPEC.md` is canonical.
