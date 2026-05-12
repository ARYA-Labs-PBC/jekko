# CODEX_MEMSPEC_V2: MNEMOS-Omega
## Rust-Native Superhuman Memory Compiler for Long-Running Agents

**Path:** `tips/smart_memory/CODEX_MEMSPEC_V2.md`
**Source basis:** `tips/smart_memory/v2/tip{1..23}.txt`
**Codename:** `mnemos-omega`
**Scope:** local-first Rust memory system, rebuildable indexes, mutation receipts, context packs, privacy boundaries
**Thesis:** memory is a compiler, not a database

```text
Raw experience is evidence.
Typed memory is truth.
Indexes are projections.
Retrieval is compilation.
ContextPack is the compiled artifact.
Feedback improves the compiler.
```

---

## 1. Objective

Design the best Rust-native superhuman memory system from `tips/smart_memory/v2/*.txt`: fast, local-first, adaptive, source-backed, and useful for long-running agents that compound knowledge across science, math, coding, research, and general domains.

MNEMOS-Omega is a cognitive memory compiler. It does not treat vector search, graph traversal, or prompt files as canonical truth. It uses a typed memory ledger plus rebuildable projections to produce deterministic, cited `ContextPack` artifacts for downstream agents.

---

## 2. Source Synthesis

The V2 notes converge on these required ideas:

| Concept | Decision |
| --- | --- |
| MIRIX / CMC lanes | Use typed cognitive memory lanes, but avoid too many physical stores. |
| CODEX_MEMSPEC | Keep local-first Rust, rebuildable indexes, mutation receipts, context packs, privacy boundaries. |
| CLAUDE_MEMSPEC | Keep rich schemas, hash-chained episodes, FSRS strengthening, formal math/science, skill compounding. |
| ANTIGRAVITY V2 | Keep compiler framing, procedural sandbox, equation/theory emphasis, multi-lane cognition. |
| Graphiti / Zep | Make temporal validity and provenance first-class. |
| A-MEM / Zettelkasten | Use evolving concept kernels and topic graph links. |
| HippoRAG | Use bounded PPR-style graph activation for multi-hop recall. |
| Mem0 / Mastra OM | Preserve low-latency observation compression and practical recall. |
| Voyager / Memp | Treat reusable workflows as tested executable skills. |
| Memory-R1 | Add learned memory policy only behind hard invariants. |

---

## 3. Architecture Score

This is a design score, not a benchmark claim.

| Axis | Weight | Score | Rationale |
| --- | ---: | ---: | --- |
| Cognitive taxonomy | 10 | 9.9 | Covers core, working overlay, episodic, semantic, formal, concept, procedural, resource, belief, eval, and counterexamples without store sprawl. |
| Temporal and epistemic rigor | 12 | 11.8 | Bi-temporal records, claim/belief split, source spans, contradiction handling, no destructive overwrite. |
| Retrieval accuracy | 12 | 11.6 | Hybrid BM25, vector, entity, temporal, graph, skill, eval, and counterexample recall with task-specific profiles. |
| Latency and scale | 12 | 11.4 | Hot/warm/cold tiers, per-lane indexes, roaring filters, cached PPR, no LLM on hot recall. |
| Compression and consolidation | 10 | 9.8 | Observation compression, concept kernels, topic capsules, dedupe, archive/rehydrate. |
| Procedural compounding | 10 | 9.8 | Skills are tested, sandboxed, versioned, scored, degraded, and linked to failures. |
| Science/math grounding | 8 | 7.8 | Equations, units, theorem DAGs, datasets, experiments, formal hooks; deduction because automatic formalization is hard. |
| Adaptation | 8 | 7.8 | FSRS, utility learning, eval traces, topic mastery, guarded learned policy. |
| Safety/audit/privacy | 8 | 7.8 | Hash chains, receipts, access policy, vault boundaries, explainable packs. |
| Buildability/tests | 10 | 9.2 | Clean Rust crate tree, staged milestones, deterministic rebuilds, explicit tests. |

---

## 4. Non-Goals

- Do not build implementation code inside this spec task.
- Do not make a vector database, graph database, or prompt file the source of truth.
- Do not require hosted services for baseline recall.
- Do not store secrets or private vault payloads in repo cards.
- Do not let an agent silently mutate durable truth.
- Do not collapse observations, claims, beliefs, skills, and summaries into one chunk type.
- Do not delete contradicted or superseded scientific evidence.

---

## 5. Assumptions And Defaults

- Rust owns canonical storage, retrieval, policy, CLI, daemon, and MCP/API surfaces.
- Baseline is single-user local-first on a developer machine.
- External LLM/embedding services are optional adapters; local Candle embeddings are the default offline path.
- `redb` is the primary pure-Rust ACID canonical store; `fjall` is optional for high-write LSM workloads.
- Git-backed cards are curated projections, not the only truth.
- All indexes are rebuildable from ledger + canonical objects + CAS blobs.
- The system can later add Qdrant/Kuzu/SQLite adapters, but they must never become canonical truth.

---

## 6. Completion Criteria

- `observe(event)` returns a durable receipt without waiting for LLM extraction.
- `recall(query)` returns a deterministic, bounded, cited `ContextPack`.
- Every trusted memory has provenance or an explicit human note.
- Current beliefs are separate from source claims and historical episodes.
- Contradictions and counterexamples are visible, not hidden.
- Math/science recall can retrieve equations, assumptions, units, theorem dependencies, datasets, experiments, and failed proof strategies.
- Index rebuilds are deterministic and verifiable.
- Skills cannot become trusted without tests or verification recipes.
- Hot recall works without network and without an LLM.

---

## 7. Stop Conditions

Stop implementation and ask for direction if:

- Existing `tips/smart_memory/CODEX_MEMSPEC_V2.md` appears with user edits.
- Work requires editing generated or read-only zones.
- Any requested change would store private vault contents in repo-visible files.
- Rust dependencies require network pinning decisions not already approved.
- A validation failure appears unrelated to the memory spec and blocks proof.

---

## 8. Read-First Files

1. `AGENTS.md`
2. `agent/JANKURAI_STANDARD.md`
3. `/Users/bentaylor/.codex/RTK.md`
4. `tips/smart_memory/CODEX_MEMSPEC.md`
5. `tips/smart_memory/CLAUDE_MEMSPEC.md`
6. `tips/smart_memory/ANTIGRAVITY_MEMSPEC.md`
7. `tips/smart_memory/ANTIGRAVITY_MEMSPEC_V2.md`
8. `tips/smart_memory/v2/tip1.txt` through `tips/smart_memory/v2/tip23.txt`
9. `agent/owner-map.json`, `agent/test-map.json`, `agent/generated-zones.toml`, `agent/proof-lanes.toml`

---

## 9. Files To Create Or Edit

Doc phase:

- Create `tips/smart_memory/CODEX_MEMSPEC_V2.md`.

Future implementation phase:

- Create a new Rust workspace under a dedicated root such as `crates/mnemos/` or a standalone `mnemos-omega/` workspace.
- Do not edit `tips/smart_memory/v2/*.txt`; they are source notes.
- Do not edit generated zones unless their source command is run.

---

## 10. Ownership Boundaries

- `tips/smart_memory/**` is concept/spec material. Treat source tips as read-only inputs.
- `target/**` is proof/tool artifact output.
- Generated zones listed in `agent/generated-zones.toml` are read-only unless regenerated by their declared command.
- Concurrent-agent conflict risk is highest in shared docs, root manifests, and agent metadata. Keep this spec isolated to `CODEX_MEMSPEC_V2.md`.

---

## 11. Architecture

MNEMOS-Omega uses nine canonical lanes with richer payload variants.

| Lane | Purpose |
| --- | --- |
| Core | Stable mission, user-approved preferences, project constraints. |
| Episodic | Immutable episodes, observations, tool runs, sessions, failures. |
| Semantic | Atomic claims, entities, definitions, relations, temporal facts. |
| Formal | Equations, units, theorems, proofs, derivations, datasets, experiments. |
| Concept | Zettelkasten concept kernels, prerequisites, analogies, topic capsules. |
| Procedural | Tested skills, workflows, scripts, commands, preconditions, failure modes. |
| Resource | Papers, repos, files, APIs, datasets, hashes, citations, source manifests. |
| Belief | Current evidence-weighted stance over claims, with rationale and uncertainty. |
| Eval | Retrieval outcomes, bad answers, benchmark traces, counterexamples, lessons. |

Protected payload types include `WorkingState`, `Observation`, `ClaimCard`, `TemporalFact`, `EquationCard`, `TheoremCard`, `DatasetCard`, `ExperimentCard`, `CounterexampleCard`, `QuestionCard`, `SkillCard`, `ConceptKernel`, `BeliefState`, and `EvalMemory`.

---

## 12. Storage Design

Canonical truth:

- `ledger/*.jsonl.zst`: append-only mutation and episode ledger, BLAKE3 hash chained.
- `store/memory.redb`: canonical object registry, lane tables, metadata, lifecycle state.
- `cas/blake3/**`: content-addressed source blobs, extracted spans, artifacts.
- `cards/**`: curated Git-backed Markdown/YAML projections for human review.
- `receipts/**`: durable write, rebuild, verification, redaction, and migration receipts.

Rebuildable projections:

- `indexes/tantivy/**`: BM25, exact symbols, code paths, paper IDs, equations.
- `indexes/hnsw/**`: hot HNSW semantic indexes, sharded by lane/topic.
- `indexes/cold_vectors/**`: quantized cold vector capsules.
- `indexes/graph/**`: mmap CSR graph snapshots plus delta overlay.
- `indexes/roaring/**`: entity/topic/time/access-policy filters.
- `cache/context_packs/**`: explainable, invalidatable context-pack cache.

---

## 13. Rust Workspace Tree

```text
mnemos-omega/
├── Cargo.toml
├── rust-toolchain.toml
├── config/
│   ├── default.toml
│   ├── retrieval_profiles.toml
│   ├── fsrs.toml
│   ├── predicates.yaml
│   └── privacy_policy.toml
├── crates/
│   ├── mnemos-types/
│   │   └── src/{ids.rs,lanes.rs,time.rs,provenance.rs,memory.rs,payloads.rs,context_pack.rs,receipts.rs}
│   ├── mnemos-ledger/
│   │   └── src/{append_log.rs,hash_chain.rs,wal.rs,receipt.rs,replay.rs,verify.rs}
│   ├── mnemos-store/
│   │   └── src/{canonical.rs,redb_store.rs,fjall_store.rs,cas.rs,cards.rs,migrations.rs,snapshot.rs}
│   ├── mnemos-index/
│   │   └── src/{tantivy_bm25.rs,symbol_index.rs,vector_hnsw.rs,vector_cold.rs,roaring_filters.rs,temporal_index.rs,rebuild.rs}
│   ├── mnemos-graph/
│   │   └── src/{entity.rs,temporal_edge.rs,adjacency.rs,csr_snapshot.rs,ppr.rs,contradiction.rs,communities.rs}
│   ├── mnemos-observe/
│   │   └── src/{event_segmenter.rs,observer.rs,source_parser.rs,privacy_filter.rs}
│   ├── mnemos-extract/
│   │   └── src/{claim_extractor.rs,equation_extractor.rs,theorem_extractor.rs,skill_extractor.rs,entity_linker.rs,validators.rs}
│   ├── mnemos-write/
│   │   └── src/{pipeline.rs,router.rs,normalizer.rs,grounding.rs,scorer.rs,committer.rs}
│   ├── mnemos-retrieve/
│   │   └── src/{query.rs,planner.rs,fanout.rs,rrf.rs,rerank.rs,filters.rs,context_pack.rs,traces.rs}
│   ├── mnemos-consolidate/
│   │   └── src/{scheduler.rs,dedupe.rs,concept_kernel.rs,belief_revision.rs,skill_distill.rs,topic_strength.rs,rehearsal.rs,archive.rs}
│   ├── mnemos-formal/
│   │   └── src/{units.rs,dimensional_analysis.rs,symbolic.rs,lean.rs,coq.rs,dataset_schema.rs,experiment_log.rs}
│   ├── mnemos-skills/
│   │   └── src/{sandbox.rs,runner.rs,test_harness.rs,registry.rs,repair.rs}
│   ├── mnemos-policy/
│   │   └── src/{rule_policy.rs,bandit.rs,features.rs,rewards.rs,attribution.rs}
│   ├── mnemos-api/
│   │   └── src/{engine.rs,tools.rs,config.rs,errors.rs}
│   ├── mnemos-mcp/
│   │   └── src/{server.rs,tool_schema.rs}
│   ├── mnemos-daemon/
│   │   └── src/{main.rs,grpc.rs,http.rs,auth.rs,shutdown.rs}
│   ├── mnemos-cli/
│   │   └── src/{main.rs,ingest.rs,recall.rs,explain.rs,consolidate.rs,verify.rs,bench.rs}
│   └── mnemos-eval/
│       └── src/{fixtures.rs,longmemeval.rs,memory_agent_bench.rs,math_memory_bench.rs,code_skill_bench.rs,metrics.rs}
├── cards/{concepts,claims,equations,theorems,skills,resources,questions}/
├── tests/{temporal_supersession.rs,contradiction_policy.rs,citation_required.rs,context_budget.rs,rebuild_indexes.rs,crash_replay.rs,skill_promotion.rs,theorem_dependency.rs,privacy_redaction.rs}
├── benches/{recall_10m.rs,ingest_async.rs,ppr_graph.rs,context_pack.rs,consolidation.rs}
└── docs/{architecture.md,schemas.md,retrieval.md,consolidation.md,agent_tools.md,evals.md,operations.md}
```

---

## 14. Core Types

```rust
pub enum MemoryLane {
    Core, Episodic, Semantic, Formal, Concept, Procedural, Resource, Belief, Eval,
}

pub enum Lifecycle {
    Observed, Proposed, Trusted, Consolidated, Contested, Revised, Superseded, Archived, Rejected,
}

pub struct MemoryObject {
    pub id: MemoryId,
    pub lane: MemoryLane,
    pub schema_version: u32,
    pub lifecycle: Lifecycle,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub valid_time: Option<TimeInterval>,
    pub transaction_time: TimeInterval,
    pub provenance: Vec<Provenance>,
    pub access: AccessPolicy,
    pub scores: MemoryScores,
    pub links: Vec<MemoryLink>,
    pub payload: MemoryPayload,
}

pub enum MemoryPayload {
    Core(CoreMemory),
    Episode(EpisodeMemory),
    Observation(ObservationMemory),
    Claim(ClaimCard),
    TemporalFact(TemporalFact),
    Equation(EquationCard),
    Theorem(TheoremCard),
    Dataset(DatasetCard),
    Experiment(ExperimentCard),
    Concept(ConceptKernel),
    Skill(SkillCard),
    Resource(ResourceCard),
    Belief(BeliefState),
    Counterexample(CounterexampleCard),
    Question(QuestionCard),
    Eval(EvalMemory),
}
```

---

## 15. Public API

```rust
#[async_trait::async_trait]
pub trait MemoryEngine: Send + Sync {
    async fn observe(&self, event: EventEnvelope) -> Result<MutationReceipt>;
    async fn recall(&self, query: RecallQuery) -> Result<ContextPack>;
    async fn remember(&self, candidate: CandidateMemory) -> Result<MutationReceipt>;
    async fn feedback(&self, pack_id: ContextPackId, outcome: Outcome) -> Result<MutationReceipt>;
    async fn reflect(&self, scope: ReflectionScope) -> Result<ReflectionReport>;
    async fn consolidate(&self, job: ConsolidationJob) -> Result<ConsolidationReport>;
    async fn rehearse(&self, request: RehearsalRequest) -> Result<Vec<ReviewItem>>;
    async fn promote_skill(&self, trace: SkillTrace) -> Result<MutationReceipt>;
    async fn verify(&self, target: MemoryId) -> Result<VerificationReport>;
    async fn explain(&self, target: ExplainTarget) -> Result<Explanation>;
    async fn rehydrate(&self, target: MemoryId) -> Result<SourceBundle>;
}
```

CLI and MCP tools must mirror this API:

```text
mnemos observe
mnemos recall
mnemos remember
mnemos feedback
mnemos reflect
mnemos consolidate
mnemos rehearse
mnemos promote-skill
mnemos verify
mnemos explain
mnemos rehydrate
mnemos audit
```

---

## 16. Write Path

Synchronous hot path:

1. Validate `EventEnvelope`.
2. Apply privacy filter.
3. Append episode to ledger.
4. Return `MutationReceipt`.
5. Update recent observation ring if cheap.

Asynchronous cognitive path:

1. Segment events into observations.
2. Extract claims, entities, equations, resources, skills, questions, failures.
3. Ground every candidate to source spans or explicit human notes.
4. Detect duplicates and near-duplicates.
5. Detect contradictions without auto-deleting old evidence.
6. Commit typed memory objects in one transaction.
7. Emit mutation receipts.
8. Fan out to BM25, vector, entity, temporal, graph, skill, and topic indexes.
9. Schedule consolidation when surprise, contradiction, utility, or topic thresholds fire.

Fact update rule:

```text
Personal/project facts can supersede old facts by closing valid_time.
Scientific claims usually do not supersede each other.
They coexist as evidence until a BeliefState weighs them.
```

---

## 17. Read Path

`recall(query)` runs:

1. Parse intent, time scope, evidence level, token budget, privacy scope.
2. Select retrieval profile: fact, science claim, math derivation, coding, debugging, literature, planning.
3. Fan out in parallel: core, recent episodes, BM25, vector, entity, temporal graph, concept graph, PPR, skills, eval/counterexamples.
4. Fuse candidates with RRF.
5. Rerank with task profile weights.
6. Apply lifecycle, access, freshness, contradiction, and confidence filters.
7. Compile deterministic `ContextPack`.
8. Record retrieval trace for feedback.

Scoring shape:

```text
score =
  w_lexical * bm25
+ w_vector * semantic_similarity
+ w_entity * entity_overlap
+ w_graph * graph_activation
+ w_time * temporal_relevance
+ w_source * source_quality
+ w_utility * historical_utility
+ w_skill * skill_reliability
+ w_formal * verification_status
+ w_counterexample * direct_failure_relevance
- w_stale * staleness
- w_privacy * access_penalty
- w_conflict * unresolved_contradiction_penalty
```

---

## 18. ContextPack Contract

A `ContextPack` must:

- Fit the requested token budget.
- Cite every factual item.
- Show known contradictions when relevant.
- Include direct counterexamples and failed attempts for risky reasoning.
- Include skill signatures, preconditions, reliability, and last failure; do not paste full skill bodies by default.
- Include omitted-evidence notes when important evidence was excluded for budget.
- Be deterministic for the same query, memory state, profile, and seed.
- Carry a retrieval trace for explainability and feedback.

---

## 19. Compression And Adaptation

Compression ladder:

```text
raw source -> episode -> observation -> atomic memory -> concept kernel -> topic capsule -> curriculum map
```

Rules:

- Summaries are never canonical truth.
- Every compressed object links downward to evidence.
- Counterexamples do not decay out of recall.
- Decay changes retrieval priority, not truth.
- Topic strengthening operates on clusters, not only individual memories.
- A learned policy may propose `Add`, `Update`, `Link`, `Archive`, `Noop`, or `AskReview`, but validators enforce provenance, privacy, temporal rules, and no destructive overwrite.

Consolidation daemons:

- Observer: dense dated observations.
- Linker: entities, aliases, symbols, prerequisites.
- Deduper: merge aliases without deleting provenance.
- Belief reviser: support/contradiction aggregation.
- Concept compiler: concept kernels and topic capsules.
- Skill distiller: repeated successful traces to skills.
- Formal verifier: units, theorem dependencies, proof status, experiment hashes.
- Strengthener: FSRS, utility, retrieval success, review queues.
- Archiver: hot/warm/cold tier management.

---

## 20. Science And Math Requirements

Formal objects must be first-class.

- `EquationCard`: TeX, normalized symbolic form, variables, units, dimensions, assumptions, limiting cases, regime of validity, derivation links.
- `TheoremCard`: statement, proof sketch, optional formal proof, proof system, dependency DAG, invalidation status.
- `DatasetCard`: version, schema, license, hash, generation process, linked experiments.
- `ExperimentCard`: method, parameters, code commit, data hash, result, replication status.
- `CounterexampleCard`: failed proof, falsified assumption, bad fix, boundary condition, linked concepts.
- `QuestionCard`: unresolved target, blockers, candidate resources, next actions.

Hard invariants:

- Trusted equations with units must pass dimensional checks or carry `Unverified`.
- Theorems must track dependency closure.
- Experiments require source/code/data hashes.
- Paper claims preserve source spans.
- Beliefs require support and contradiction links.
- Formal proof absence is visible, never hidden.
- Counterexamples are aggressively retrieved for matching entities/topics.

---

## 21. Procedural Memory

A skill can become trusted only if:

1. It has at least one successful source episode.
2. Preconditions are explicit.
3. Steps are reproducible.
4. A test, assertion, or verification recipe exists.
5. Failure modes are recorded.
6. Dependencies and environment assumptions are captured.
7. It passes sandbox validation.

Skill lifecycle:

```text
Draft -> Tested -> Trusted -> Degraded -> Retired
```

Degrade a skill when dependencies drift, failures rise, eval memory reports misuse, or a safer skill supersedes it.

---

## 22. Performance Targets

Design targets for a 10M-memory local node:

| Operation | Target |
| --- | ---: |
| `observe(event)` receipt | p50 <= 5 ms, p95 <= 25 ms |
| `recall(simple fact)` | p50 <= 35 ms, p95 <= 180 ms |
| `recall(debug exact error)` | p50 <= 40 ms, p95 <= 200 ms |
| `recall(science/multi-hop)` | p50 <= 120 ms, p95 <= 500 ms |
| `context_pack_compile` after candidates | p50 <= 20 ms |
| Hot memory scale | 1M-3M objects |
| Total local memory scale | 10M-50M objects |
| Cold archive | 100M+ source-backed objects |
| Default context budget | 4k-8k tokens |

Scale tactics:

- Per-lane vector indexes.
- Topic-sharded HNSW.
- Roaring filters for entity/topic/time/access.
- Tantivy for symbols, paths, identifiers, equations.
- Temporal interval indexes for valid-at queries.
- Cached PPR for hot graph communities.
- Cold archive removes low-utility items from hot HNSW.
- Rehydrate source only when needed.

---

## 23. Dependencies

Rust crates to prefer:

- Core: `serde`, `schemars`, `uuid`, `time`, `thiserror`, `anyhow`
- Runtime: `tokio`, `rayon`, `tracing`
- Hash/log/compression: `blake3`, `zstd`
- Storage: `redb`, optional `fjall`, `gix`
- Search/index: `tantivy`, `hnsw_rs`, `roaring`, `fst`, `memmap2`
- Embeddings: `candle-core`, `candle-transformers`, `tokenizers`
- API: `axum`, `tonic`, MCP schema crate chosen by implementation context
- Tests: `proptest`, `criterion`, `insta`, `tempfile`

Do not pin exact versions in the spec unless implementation begins and lockfile policy is known.

---

## 24. Implementation Steps

1. Create `tips/smart_memory/CODEX_MEMSPEC_V2.md` with this spec.
2. If implementing, create `mnemos-types` first: `MemoryId`, `MemoryLane`, `Lifecycle`, `MemoryObject`, `MemoryPayload`, `ContextPack`, `MutationReceipt`.
3. Add schema roundtrip tests and compatibility fixtures before storage.
4. Build `mnemos-ledger`: append log, BLAKE3 hash chain, receipt replay, crash recovery.
5. Build `mnemos-store`: redb canonical tables, CAS source blobs, Git card projection.
6. Build `mnemos-index`: Tantivy, HNSW, roaring filters, temporal index, rebuild manifest.
7. Build `mnemos-graph`: temporal edges, contradiction links, CSR snapshot, bounded PPR.
8. Build `mnemos-write`: observe pipeline, validators, duplicate detection, contradiction policy, receipts.
9. Build `mnemos-retrieve`: query planner, fanout, RRF, rerank, context compiler, traces.
10. Build `mnemos-consolidate`: concept kernels, belief revision, topic strength, archive/rehydrate.
11. Build `mnemos-formal` and `mnemos-skills`: units, theorem dependencies, experiments, skill sandbox.
12. Expose `mnemos-api`, `mnemos-cli`, `mnemos-mcp`, and daemon surfaces.
13. Add evals and benches before claiming performance.

---

## 25. Parallel Work Packets

Only split work when write scopes are disjoint:

| Packet | Owner scope | Files |
| --- | --- | --- |
| A: Types and schemas | Canonical data contracts | `crates/mnemos-types/**`, schema tests |
| B: Ledger and store | Durability | `crates/mnemos-ledger/**`, `crates/mnemos-store/**` |
| C: Index and retrieval | Search/read path | `crates/mnemos-index/**`, `crates/mnemos-retrieve/**` |
| D: Graph/formal/skills | Specialized cognition | `crates/mnemos-graph/**`, `crates/mnemos-formal/**`, `crates/mnemos-skills/**` |
| E: API/CLI/eval | Interfaces and proof | `crates/mnemos-api/**`, `crates/mnemos-cli/**`, `crates/mnemos-eval/**` |

No packet may edit another packet's crate without coordination.

---

## 26. Edge Cases

- Contradictory scientific papers under different assumptions.
- Stale personal/project facts versus historically valid facts.
- Missing source span for an extracted claim.
- Hallucinated summary with no supporting evidence.
- Embedding model upgrade causing index drift.
- Corrupted or partial index rebuild.
- Skill succeeds once but fails under changed environment.
- Theorem dependency invalidated after a lemma changes.
- Dataset version mismatch.
- Prompt-injected memory content requesting exfiltration.
- Sensitive resource metadata leaking into context.
- Multi-hop graph query exceeding latency budget.
- Counterexample hidden by high-level summary.
- Belief confidence overfit to low-quality sources.

---

## 27. Test Plan

Unit tests:

- Schema serialization roundtrips.
- Temporal interval logic.
- Source span validation.
- Score normalization.
- Access policy redaction.
- Skill lifecycle transitions.

Property tests:

- Ledger replay is deterministic.
- No committed trusted memory lacks provenance.
- Supersession never deletes old evidence.
- Context packs never exceed budget.
- Index rebuild equals incremental index for same ledger.
- Counterexample direct matches are always eligible for inclusion.

Integration tests:

- Observe -> extract -> commit -> index -> recall.
- Contradictory science claims produce contested belief, not deletion.
- Debug query retrieves exact error, prior failed attempts, and trusted skill.
- Math query retrieves equation, assumptions, units, theorem dependencies.
- Privacy query redacts secret/vault items.
- Crash during commit replays or rolls back cleanly.

Benchmarks:

- `observe(event)` receipt latency.
- BM25/entity/vector recall latency at 1M and 10M objects.
- Bounded PPR latency.
- Context pack compilation time.
- Index rebuild throughput.
- Consolidation batch throughput.

Adversarial tests:

- Prompt injection inside memory content.
- Poisoned low-quality source.
- Near-duplicate flood.
- Contradiction storm.
- Skill with unsafe shell command.
- Secret in source blob.
- Formal object with inconsistent units.

---

## 28. Validation Commands

For doc-only change:

```bash
rtk git diff -- tips/smart_memory/CODEX_MEMSPEC_V2.md
rtk jankurai proof . --changed tips/smart_memory/CODEX_MEMSPEC_V2.md --out target/jankurai/proof-plan.json --md target/jankurai/proof-plan.md
rtk just fast
```

For future Rust implementation:

```bash
rtk cargo fmt --all --check
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo test --workspace
rtk cargo test --workspace --all-features
rtk cargo bench --workspace --no-run
rtk just fast
```

Expected artifacts:

- `tips/smart_memory/CODEX_MEMSPEC_V2.md`
- `target/jankurai/proof-plan.json`
- `target/jankurai/proof-plan.md`
- Future implementation: benchmark output under `target/criterion/**`

Failure interpretation:

- Formatting, clippy, or test failures in new crates are implementation blockers.
- Jankurai unmapped-path findings for `tips/smart_memory/**` indicate repo metadata does not currently track these concept docs; do not edit owner/test maps unless requested.
- Benchmark failures do not block v1 correctness but block performance claims.

---

## 29. Logging And Receipts

Repo work handoff must report:

- Source files studied.
- File created or edited.
- Validation commands run.
- Validation results and unresolved failures.
- Any generated artifacts.
- Any paths intentionally not touched.

Runtime MNEMOS receipts must include:

- `receipt_id`
- previous ledger hash
- new ledger hash
- event ids
- memory ids inserted or updated
- indexes updated
- source spans
- policy decisions
- validator results
- timestamp
- actor/tool identity

---

## 30. Final Handoff Requirements

A completed implementation handoff must include:

- Summary of implemented crates and APIs.
- Exact commands run with results.
- Known limitations.
- Performance numbers only if measured.
- Migration/rebuild instructions.
- Safety note for privacy/vault handling.
- Open follow-up tasks ranked by risk.
