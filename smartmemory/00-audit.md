# 00 — State of the Art Audit

What already works in the repo, as of 2026-05-13.

## 1. Benchmark core — `crates/memory-benchmark/`

### Trait

`MemorySystem` (`src/types.rs:154`) — the surface every candidate implements:

```rust
pub trait MemorySystem {
    fn name(&self) -> &'static str;
    fn observe(&mut self, event: &Event) -> Receipt;
    fn recall(&mut self, query: &Query) -> RecallResult;
    fn recall_at(&mut self, query: &Query, world_time: &str) -> RecallResult;
    fn recall_as_of(&mut self, query: &Query, tx_time: &str) -> RecallResult;
    fn belief_as_of(&mut self, query: &Query, tx_time: &str) -> RecallResult { ... }
    fn build_context(&mut self, query: &Query, budget_tokens: u32) -> RecallResult { ... }
    fn feedback(&mut self, pack_id: &str, outcome: &Feedback) -> Receipt;
    fn forget(&mut self, memory_id: &str, reason: &str) -> Tombstone;
    fn rebuild(&mut self) -> Receipt;
    fn export_state_hash(&self) -> String;
}
```

Bitemporal (`recall_at` = world time, `recall_as_of` = transaction time) is first-class. `build_context` is a budget-aware wrapper.

### Event model

`Event` (`src/types.rs:3`) carries `subject`, `body`, `sources: Vec<Source>`, `valid_from/to`, `tx_time`, plus optional `event_time`, `observation_time`, `review_time`, `policy_time`. Provenance arrays: `dependencies`, `supersedes`, `contradicts`, `derived_from`. `privacy_class: PrivacyClass`, `claim_modality: Option<ClaimModality>`, `tags: Vec<String>`.

16 `EventKind` variants:
```
Observation, Claim, Equation, Theorem, Skill, Resource, Dataset, Experiment,
Hypothesis, Counterexample, Lesson, Question, VaultCanary, SchemaMigration,
Supersede { target_event_id, reason }, Feedback
```

5 `ClaimModality` variants (GEMINI-V3 / ANTIGRAVITY-V2 contribution):
```
Observed, AssertedBySource, InferredByAgent, HumanApproved, FormallyVerified
```

5 `PrivacyClass` variants:
```
Public, Internal, Confidential, Secret, Vault
```

17 `Warning` variants emitted on the read path:
```
Superseded, Contradicted, LowConfidence, Redacted, CausalMaskApplied,
UntrustedInstructionLikeContent, SkeptikSurfaced, UnitMismatch,
SchemaMigrated, DependencyInvalidated, CitationUnsupported, CitationBloated,
CompressionDrift, PrivacyTransformBlocked, UnsafeToolRefused, Abstained,
BeliefTimeApplied
```

### Scoring

10 axes (`src/scoring/axes.rs:16`), weighted to exactly 100:

| Axis | Weight | What it measures |
|---|---:|---|
| correctness | 20 | answer must_contain / used_ids must_include matches |
| provenance | 12 | every cited fixture has non-empty Citations |
| math_science | 12 | unit_mismatch detection + science term preservation |
| bitemporal_recall | 10 | causal-mask + valid_from/to filtering |
| contradiction | 10 | required Warnings (contradicted/stale/skeptic_surfaced) |
| english_discourse_coreference | 8 | name/coref preservation |
| privacy_redaction | 8 | canary patterns + Vault short-circuit |
| procedural_skill | 8 | unsafe-skill refusal + skill-name matches |
| feedback_adaptation | 6 | confidence within fixture-declared band |
| determinism_rebuild | 6 | `context_pack_hash` non-empty + stable across rebuild |

Each per-fixture axis returns `Option<f32>`. `None` means the fixture didn't exercise the axis — averaged-out, never penalized. The final total normalizes against only those axes that had ≥1 contributing fixture.

There's also a 12-element `ADVANCED_WEIGHTS` table (concept_learning, transfer_reasoning, formal_math, scientific_reasoning, …) that also sums to 100 — used by `population_report` for an alternative rollup.

### Hard gates

`src/scoring/gates.rs:10` applies score ceilings on structural failures:

```
unsafe_tool_exec > 0  → max 50
privacy_leaks   > 0   → max 60
citation_issues > 0   → max 70
future_leaks    > 0   → max 75
!deterministic        → max 80
```

The strictest cap wins. A privacy leak with a citation issue caps the score at 60, not at 70.

### Suites

| Suite | Use | Fixtures |
|---|---|---|
| `Split::PublicSmoke` (T0) | 100 hand-crafted fixtures in `src/fixture/data.rs` | 100 fixed |
| `Split::PublicGenerated` (T1) | Seeded synthetic — math, science, theorem, privacy, workflow | up to 500 |
| `Split::PublicStress` (T2) | Same generator at larger scale | configurable |
| `Split::RealPapers` | QBank-built from real OA papers under `data/real-paper-bank/` | tens to hundreds |
| `Split::PrivateGenerated` | Env-seeded, never committed; commitment SHA-256 only | configurable |

### Determinism

- `BENCH_NOW = "2026-05-12T00:00:00Z"` (`src/memory_api.rs:14`) — the canonical "now". Hot paths use this; `std::time::SystemTime::now()` is forbidden by code review.
- FNV-1a hashing (`src/hash.rs`) — fast, stable, no deps.
- `SeedRng::from_label` (`src/generated/seed.rs`) — deterministic Gaussian/uniform from a label string.
- All JSON output via sorted `BTreeMap` keys.
- `verify_determinism` bin (`src/bin/verify_determinism.rs`) runs a candidate twice and `cmp`s the byte stream.

### Calibration

`src/lib.rs:130` enforces score bands:
- `baseline` total ∈ [25, 75] — must be weak but not zero.
- `reference_context_pack`, `reference_evidence_ledger`, `reference_claim_skeptic` total ∈ [70, 90] — must be strong but not perfect.

A test (`no_branded_identifiers`, `src/lib.rs:88`) scans the source for banned identifiers (`claude_v3`, `codex_v3`, `gemini_v3`, `codex-memory`, `memory-v3`, `MGV3`, `MemoryGauntlet`, `mnemos_gauntlet`). Naming policy is enforced at the test level.

### Candidates (28 total)

```
baseline                      # deliberately weak vector scan
reference_context_pack        # 15-lane bitemporal ContextPack reference (70-88)
reference_evidence_ledger     # ledger-oriented reference (70-88)
reference_claim_skeptic       # contradiction-first reference (70-88)
ledger_first / exec           # alias for chase preflight
hybrid_index
temporal_graph
compression_first
skeptic_dataset
arena_lane_00..19             # 20 thin policy variants around the references
```

### Binaries (8)

```
bench               # primary candidate runner
generate_suite      # T1/T2 seeded suite generator
verify_determinism  # byte-identity check
qbank_validate      # QBank acceptance gate
score_mix           # weighted composite of multiple bench outputs
population_report   # AutoResearch reducer
prompt_reduce       # diagnostic judge-population aggregator
dump_tasks          # fan_out work generator for chase
```

### Justfile targets (existing)

```
memory-benchmark-fast            # check + test + determinism (compile/test, no scoring run)
memory-benchmark-check
memory-benchmark-test
memory-benchmark-determinism
memory-benchmark-generated       # generate 500 fixtures + baseline + determinism + qbank validate
memory-benchmark-real-papers     # qbank validate + bench with reference_evidence_ledger
memory-benchmark-qbank-smoke     # qbank validate top-50
memory-benchmark-score-mix       # 25 generated + 50 qbank → score_mix composite
memory-benchmark-chase-preflight # generate + multiple candidates → preflight reports
```

## 2. ZYAL pipeline — `docs/ZYAL/examples/memory-benchmark/`

ZYAL is a Jekko-host daemon DSL (v2.6.0). The benchmark directory contains 8 ZYAL files:

### Question-bank workflow

- **`qbank-simple.zyal`** (192 lines) — smoke test: 1 open-access paper → 2 generators + 2 answerers + 1 critic + 1 focused auditor. Acceptance: ≥0.75 auditor agreement, ≥0.90 answerability, ≤0.50 blind-correct for hard questions. License-aware.
- **`qbank-advanced.zyal`** (268 lines) — production: 50 papers, 6 generators + 8 answerers + 4 focused auditors + 2 critics. Tournament + saturated blind checks. 10-worker fleet under `git_worktree`. Jnoccio metrics.
- **`qbank-ultra.zyal`** — adds dispatch, incubator retries, taint quarantine for web content, ask-before-push checkpoints, single-use arm token.

### AutoResearch workflow

- **`autoresearch-basic.zyal`** (158 lines) — 4-lane minimal tournament. Deny-by-default sandbox. Append-only progress memory. `best_score` reducer (promotion at +0.75 points without new hard gates).
- **`autoresearch-chase.zyal`** (158 lines) — 20-lane production tournament. Same gates. Designed for long-running search.

### Scoring workflow

- **`executable-benchmark.zyal`** — pre-existing 300s budget. T0 + T1 + comparison-matrix + curriculum proposals. Already runnable via `cargo --bin bench`.
- **`generated-challenge.zyal`** — private seed via env var, SHA-256 commitment persisted (never seed value). Privacy-preserving.
- **`prompt-scoring.zyal`** — 5 specialist judge lanes (science/math/english/privacy/procedure), 100-point rubric, deterministic vote aggregation via `prompt_reduce`. Diagnostic-only; Rust oracle is primary.

### ZYAL contract (relevant fields)

```yaml
arming:
  preview_hash_required: true
  host_nonce_required: true
  single_use: true
loop.policy: once | forever | until <condition>
permissions: { research, websearch, webfetch, edit: ask }
capabilities: { default: deny, allow: [ ... ], command_floor: { block: [git push, ...] } }
sandbox: { network.outbound: deny, paths: { ... } }
fleet: { max_workers: N, isolation: git_worktree | same_session, jnoccio: { ... } }
fan_out: { split: { ... }, reduce: { strategy: best_score | scatter_gather, ... } }
experiments: [ { lane, iterations, budget_usd, ... } ]
hooks: { on_start, before_iteration, after_iteration }
done: { require: [ ... ], forbid: [ ... ] }
memory: { stores: [ ... ] }
rollback: { plan, verify }
checkpoint: { git: { push: ask } }
```

## 3. MEMSPEC corpus convergence — `tips/smart_memory/`

Reading 30+ memory-system specs across CLAUDE/CODEX/GEMINI/ANTIGRAVITY × V1/V2/V3, plus 23 deep-tip writeups in `tips/smart_memory/v2/`, the field has converged on a stable pattern:

### Architectural consensus

1. **Memory is a compiler, not a database** (CODEX V1's framing, adopted by V2/V3). Raw experience → append-only ledger → typed memory lanes → rebuildable index projections → deterministic ContextPack → agent action → feedback → strengthening → consolidation.

2. **Append-only ledger as canonical truth.** JSONL/binary WAL, hash-chained (BLAKE3 or FNV-1a). All projections (Tantivy/HNSW/CSR-graph/Roaring) are rebuildable.

3. **Typed lanes (6→16 depending on author).** Core, Episodic, Semantic, Belief, Concept, Procedural, Resource. Math/science adds Equation, Theorem, Dataset, Experiment, Counterexample. Self-reflection adds Eval, Distillate, Lesson, Question.

4. **Bitemporal validity (`valid_from / valid_to / tx_from / tx_to`).** Contradictions close old edges (`valid_to = now`); never overwrite. Source of truth is the ledger, not the index.

5. **FSRS + Hebbian + Concept kernels.** FSRS on individual cells and topics; Hebbian co-activation on retrieval; A-MEM/Zettelkasten-style concept kernels via consolidation clustering.

6. **Counterexamples never decay.** Falsified beliefs stay accessible forever; explicitly retrieved when matching queries appear.

7. **ClaimModality.** Observed vs AssertedBySource vs InferredByAgent vs HumanApproved vs FormallyVerified. Prevents conflation of "source said X" with "agent believes X". The GEMINI_V3 / ANTIGRAVITY_V2 contribution; already in the benchmark's `types.rs`.

8. **Skill verification.** Voyager pattern — executable + tested + reliability-scored. Skills with `failure_count > threshold` get deprecated.

9. **Math/science.** Dimensional analysis mandatory; theorem dependency DAG; provenance with byte-span hashes; SymPy or symbolic-crate hooks optional.

10. **Skeptic daemon.** Adversarial self-correction — periodically hunts for contradictions and forces belief revision (GEMINI_V3 innovation).

### Scoring per spec (self-claimed)

| Spec | Self-score / 60 | Notable claim |
|---|---:|---|
| CLAUDE_MEMSPEC | ~54 | "mnemos" — 12 typed stores, FSRS + Hebbian explicit |
| CODEX_MEMSPEC | ~54 | "Memory = compiler" framing |
| CODEX_MEMSPEC_V2 ("MNEMOS-Omega") | 54.3 | Explicit scoring + 8 daemons |
| GEMINI_MEMSPEC_V3 | 60.0 | ClaimModality + Skeptic daemon claimed perfect |
| ANTIGRAVITY_MEMSPEC_V2 | 60.0 | 13 lanes + BitemporalValidity + EdgeKind |

### V2 tip themes (`tips/smart_memory/v2/`)

Across 23 tips totaling ~1.5 MB:
- **Names tried**: MNEMOS-Σ, MNEMOS-Ω, HYPERMNESIA, HELIX, NOESIS-RS, ALETHEIA-Ω, MnemOS-Prime, OpenQG-Mnemosyne. The team did not pick.
- **External systems cited**: MIRIX, Hindsight, HippoRAG, A-MEM, Graphiti/Zep, Voyager, Mem0, Mastra Observational Memory, MemOS, LightRAG, GraphRAG, RAPTOR, ReasoningBank, Memory-R1, CoALA, Titans.
- **Performance targets**: p50 observe ≤3-5ms, p50 warm recall ≤40-90ms, p95 recall ≤150-300ms.
- **Storage stack consensus**: Tantivy + Qdrant or HNSW + Roaring + redb or fjall + Kuzu (optional graph).
- **Rust crate stack**: tantivy, hnsw_rs, qdrant_client, redb, fjall, roaring, blake3, zstd, candle, tokio, gix.

## 4. What you can do today

With nothing else in place:

```bash
# Fast: compile + test + determinism check (no scoring)
just memory-benchmark-fast

# Full scoring of baseline on T0 + T1
just memory-benchmark-generated

# Real papers (QBank)
just memory-benchmark-real-papers

# Composite score (generated + qbank weighted)
just memory-benchmark-score-mix

# Multi-candidate preflight for chase
just memory-benchmark-chase-preflight
```

All deterministic. All reproducible. All zero-dependency. The infrastructure is solid; what's missing is the actual learning system.

See `01-gaps.md` for the missing pieces.
