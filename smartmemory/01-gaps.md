# 01 — Critical Gaps, Break Points, Room for Improvement

What is missing or broken in the in-flight memory work, and what fixing each gap unlocks.

## 1. The biggest gap — no actual learning crate

The repo has a fully-wired benchmark, a mature ZYAL pipeline, and 30+ design specs. What it does NOT have is a memory system. The "candidates" in `crates/memory-benchmark/src/{adapters,candidates}/` are scoring puppets that hard-code behavior to match the rubric:

- `reference_context_pack` (`src/adapters/reference_context_pack.rs`, 480 lines) — implements bitemporal filtering, canary redaction, supersession detection, utility EMA. Scores 70-88. It is a *reference scorer*, not a learning system. It does not get smarter as you feed it papers; it does not compound knowledge; it does not harden topics.
- `reference_evidence_ledger`, `reference_claim_skeptic` — same pattern, different tradeoffs.
- `arena_lane_00..19` — thin policy wrappers around the references (e.g., `lane_12` truncates answers at 96 chars and scales confidence by 0.94).

**Consequence**: AutoResearch has nothing meaningful to mutate. Tuning a reference scorer's `citation_quality_floor` from 0.85 to 0.83 is not "chasing SOTA"; it's hyperparameter spelunking. Until there is a real memory system, the chase is theater.

**Unlock**: implement `crates/cogcore/` (see `02-cogcore-design.md`). That crate must learn — observe a stream of related papers, compound knowledge across cells via Hebbian coactivation, harden topics via FSRS, and emerge concept kernels through offline consolidation.

## 2. Benchmark gaps — won't reward the traits the user actually wants

The 10-axis benchmark scores correctness, provenance, bitemporal recall, contradiction, math/science, English discourse, privacy redaction, procedural skill, feedback adaptation, and determinism rebuild. It does not score:

### 2.1 Compounding

Each T0 fixture is independent. The harness calls `observe` zero-or-more times per fixture, then `recall` once, then moves on. A system that gets dramatically smarter on the 51st neutrino paper after ingesting 50 priors scores *identically* to one that processes each in isolation.

**Fix**: a new `Split::PublicCompounding` suite where each case has an ingest stream of N related events plus M queries whose answers require multi-hop traversal of the stream. Hop depth ∈ {1, 2, 3, 4}; depth-weighted score `[1.0, 1.5, 2.25, 3.4]`. See `03-benchmark-12axis.md`.

### 2.2 Topic hardening

Queries fire once. Re-asking "what is the PMNS matrix?" 5 times across synthetic time steps with intervening reinforcement events should produce smaller, more confident, more concentrated answers each time. Not measured.

**Fix**: `Split::PublicHardening` — 5-timestep cases with 1-2 reinforcement events between each query. Oracle scores `support_concentration` (cite-set shrinks), `confidence_growth`, `token_reduction`, and `determinism`.

### 2.3 Cross-domain transfer

The `Domain` enum exists (`Science / Math / English / Privacy / Procedural`) but every fixture's `must_include` stays in one domain. A real memory system should retrieve a math identity to answer a physics question.

**Fix**: a `cross_domain_transfer` fixture-kind in the compounding suite — math fixture establishes a result, physics fixture asks a question whose `must_include` references the math event id. Tests graph traversal across domain boundaries.

### 2.4 No real-time paper ingestion

The `RealPapers` suite loads `data/real-paper-bank/`, which contains already-distilled QBank questions, not raw paper text. The "ingest 50 arxiv abstracts in order, then answer related queries" loop is not a fixture.

**Fix**: a `paper_distillation` fixture-kind — three events ingest `{abstract, methods, result}` of one synthetic paper, then a fourth event introduces a second paper citing the first. Query requires combining `paper_A.methods` and `paper_B.citation`.

### 2.5 MemoryPoisoningBench wired only in the enum

`PublicBench::MemoryPoisoningBench` exists at `types.rs:296` but no fixture exercises it. A deliberately-wrong paper should be detected and contradicted; current adapters silently absorb it.

**Fix**: a `poisoned_paper` fixture-kind — high-quality fact + low-quality contradicting paper. Oracle requires `Warning::Contradicted` on the bad source AND zero regression on an unrelated control topic (the "knowledge non-degradation" gate, see §3).

### 2.6 No <5min north-star scoring run

`just memory-benchmark-fast` is compile + test + determinism — no scoring. `memory-benchmark-generated` runs 500 fixtures (slow, full). `memory-benchmark-score-mix` runs a 25+50 smoke (not the full composite).

**Fix**: a new `memory-benchmark-northstar` target that runs T0(100) + T1(120) + compounding(25) + hardening(20) + qbank(50) → `score_mix` composite in <5min wall clock warm cache.

## 3. AutoResearch gaps

### 3.1 No orchestrator

`docs/ZYAL/examples/memory-benchmark/autoresearch-chase.zyal` describes the workflow (20 workers, scatter-gather, best-score reducer, append-only memory, hard gates), but the actual cycle driver (mutate → build → score → reduce → promote) is unimplemented. There is no Rust binary, no shell script, no Python orchestrator that turns the ZYAL spec into running cycles.

**Fix**: `tools/autoresearch/` Rust binary with subcommands `tick` (run one cycle), `daemon` (loop), `seed` (initialize `autoresearch/chase-best`), `forensics` (bundle on abort). See `04-autoresearch-loop.md`.

### 3.2 No mutation strategy ladder

Workers ostensibly "try things" but there is no proposer, no tier separation, no rejection rules at file or line granularity. A worker could in principle mutate the scorer itself.

**Fix**: 4-tier ladder with strict allowlists —
- T1 = config sweep (≤30 lines, hyperparameters only)
- T2 = single-algorithm swap (≤150 lines, 1 file)
- T3 = new data structure (≤600 lines, ≤5 files in `cogcore/src/`)
- T4 = LLM-suggested edit (≤1500 lines, ≤8 files; blocked from clock/random/new-dep tokens)

Universal rejection: any change in the trusted benchmark core.

### 3.3 No shadow suite check

`Split::PrivateGenerated` is in the case enum (`case.rs`) but never run in the chase. Public-vs-shadow divergence is the only signal that catches overfit-to-public-seed.

**Fix**: `memory-benchmark-shadow` target runs the candidate with `$MEMORY_BENCHMARK_PRIVATE_SEED` (env-only, never committed). Reducer requires `|public - shadow| ≤ 5.0` for promotion; >5 for 3 consecutive cycles pauses the loop.

### 3.4 No calibration anti-tamper

Nothing today stops a candidate from mutating `apply_hard_gates` to be lenient, or from edge-casing `weighted_fraction` to inflate its own score. The reducer needs to rerun the 4 reference adapters in the worker tree and reject if any drifts.

**Fix**: reducer's promote-or-reject contract includes a step that runs `baseline / reference_context_pack / reference_evidence_ledger / reference_claim_skeptic` against the worker's tree. If `baseline` exits [25, 75] OR any reference exits [70, 90] (the existing calibration band), reject for `ReferenceDrift`. This catches global scorer mutations.

### 3.5 No worktree management

The chase ZYAL specifies `fleet.isolation: git_worktree` but no script creates / cleans / target-caches them. The directory `.jekko/daemon/memory-benchmark-chase/` exists; the management code does not.

**Fix**: orchestrator does `git worktree add --detach .jekko/daemon/memory-benchmark-chase/worktrees/<cycle_id>/<worker_id> autoresearch/chase-best`. Per-worker `CARGO_TARGET_DIR=.jekko/daemon/memory-benchmark-chase/target/<cycle_id>/<worker_id>`. On cleanup: keep last 5 cycles for forensics, prune older.

## 4. Design gaps — MEMSPEC handwaving

### 4.1 Topic hardening formula

Every MEMSPEC names the components (FSRS, Hebbian, decay, reinforcement, contradiction pressure) but none specifies the closed-form weights. CLAUDE_MEMSPEC §11.4 comes closest with "mastery = f(retrievability)" but does not give the function.

**Fix** (concrete, see `05-formulas.md`):
```
topic.strength = clamp(
    decayed_base
    + 0.20·recency + 0.18·recurrence + 0.12·utility + 0.08·novelty
    + 0.10·src_quality + 0.20·retr_success
    − (0.30·contradiction_pressure + 0.10·superseded_fraction),
    0, 1)
```

### 4.2 Concept-kernel emergence threshold

When does N similar cells become a Concept? When does a Topic crystallize from concepts? The specs say "Zettelkasten" or "A-MEM" but don't give the threshold or algorithm.

**Fix** (concrete, see `05-formulas.md`):
- **Concept** opens when ≥3 cells share a token-bigram Jaccard ≥ 0.55. Kernel = intersection of top-15 TF-IDF tokens. Approximate-NN via MinHash sketch (8 hashes, stdlib-only).
- **Topic** opens when ≥4 concepts have pairwise coact ≥ 0.40. Community detection via greedy modularity on the concept graph.

### 4.3 LLM-free hot path

GEMINI_V3 and ANTIGRAVITY assume LLM extraction at write time. The benchmark requires byte-identical determinism — an LLM call breaks that. CODEX allows rule-based fallbacks; CLAUDE allows local Candle. Inconsistent.

**Fix**: hot path (`observe`, `recall`, `recall_as_of`) is 100% deterministic with rule-based extractors. LLMs are gated behind `budget::check_and_consume()` and only invoked in the offline `consolidate.rs::pass_llm_enrich` pass. Default budget = 0 calls. The benchmark sees the LLM path unreachable, preserving byte-identity.

### 4.4 Skill sandbox security undefined

Voyager-pattern skill verification is named but no spec defines:
- Threat model (what attacker capabilities to defend against)
- Allowed operations (filesystem? network? subprocess?)
- Capability boundaries (Linux capabilities? seccomp? WASM?)

**Fix**: defer concrete skill execution until after Phase 4 (autoresearch). For now, skills are *retrieved* but never *executed* by the memory system — execution is the host's responsibility. The MemorySystem trait only stores `EventKind::Skill` events; it does not invoke them.

### 4.5 The determinism-vs-learning paradox

The system must learn (Hebbian, FSRS, utility) on recall. Yet `verify_determinism` requires byte-identical output on rerun. If recall mutates state, the second run has different state, and outputs differ.

**Fix** (the load-bearing insight): every recall-induced mutation is itself a WAL op (`WalOp::RecallTouch { used_ids, tx_time }`). The mutations are deterministic given the input event stream, so replaying the ledger reproduces them byte-for-byte. `rebuild()` truncates projections, replays from seq 0, and lands on the same `export_state_hash()`. The benchmark hits this consistently because every fixture calls `observe` and `recall` in a fixed order. Tests in `tests/ledger_replay.rs` enforce.

## 5. Naming gaps

Specs proposed 9 names (MNEMOS-Σ/Ω, HYPERMNESIA, HELIX, NOESIS-RS, ALETHEIA-Ω, MnemOS-Prime, OpenQG-Mnemosyne, CMC). The team never picked. The benchmark's `no_branded_identifiers` test (`src/lib.rs:88`) bans `mnemos_gauntlet`, `memory_v3`, `Memory Gauntlet`, etc. The naming rule (in user memory) forbids names tied to first consumer.

**Fix**: `cogcore` (cognitive core). Two syllables, generic, available, passes the banned-identifier scan, neutral with respect to Jekko/jankurai/zyal.

## 6. Quick reference — gap → fix mapping

| Gap | Fix location |
|---|---|
| No learning crate | `crates/cogcore/` (`02-cogcore-design.md`) |
| No compounding axis | `Split::PublicCompounding` + `compounding` axis (`03-benchmark-12axis.md`) |
| No hardening axis | `Split::PublicHardening` + `topic_hardening` axis |
| No cross-domain transfer | `cross_domain_transfer` fixture-kind |
| No real-time paper ingest fixture | `paper_distillation` fixture-kind |
| Poisoning bench unwired | `poisoned_paper` fixture-kind + `knowledge_non_degradation` gate |
| No <5min northstar | `memory-benchmark-northstar` Justfile target |
| No AutoResearch orchestrator | `tools/autoresearch/` (`04-autoresearch-loop.md`) |
| No mutation ladder | 4-tier ladder with line caps |
| No shadow check | `memory-benchmark-shadow` + reducer gate |
| No calibration anti-tamper | Reducer reruns 4 references; reject on drift |
| No worktree management | Orchestrator `worktree add` + per-cycle `CARGO_TARGET_DIR` |
| Vague hardening formula | Closed form (`05-formulas.md`) |
| Vague concept emergence | MinHash + Jaccard thresholds (`05-formulas.md`) |
| LLM-on-hot-path | `ExtractorBackend` trait + `Budget::ZERO` default |
| Skill sandbox undefined | Defer execution; memory only stores skills |
| Determinism vs learning | `WalOp::RecallTouch` makes mutations replayable |
| Naming undecided | `cogcore` |

See remaining files for design details.
