# CODEX_MEMSPEC_V3: MNEMOS-Omega V3
## Rust-First Cognitive Memory Compiler For Long-Running Agents

**Path:** `tips/smart_memory/CODEX_MEMSPEC_V3.md`
**Codename:** `mnemos-omega-v3`
**Status:** standalone engineering specification; supersedes `CODEX_MEMSPEC_V2.md`
**Primary product:** deterministic, cited, bounded `ContextPack`
**Canonical truth:** append-only evidence ledger plus typed `MemoryCell` records
**Default storage:** `redb` canonical object store with rebuildable projections
**Scope:** local-first Rust memory compiler for coding, research, science, math, and long-running agent work

```text
Raw experience is evidence.
The append-only ledger is canonical truth.
MemoryCell is the typed contract.
Indexes are rebuildable projections.
Retrieval is deterministic compilation.
ContextPack is the shipped artifact.
Feedback trains the compiler, never rewrites history.
```

---

## 1. Objective

Build MNEMOS-Omega V3: a Rust-first memory compiler that lets long-running agents compound knowledge over months without treating chat transcripts, vector hits, or prompt summaries as memory. The system records evidence in an append-only ledger, promotes evidence into typed `MemoryCell` objects with bitemporal truth and provenance, maintains rebuildable indexes, and compiles task-shaped `ContextPack` artifacts for downstream reasoning.

The design is optimized for:

- source-backed science and math learning
- exact recall of code symbols, equations, IDs, and prior failures
- temporal truth and historical agent-belief queries
- executable procedural skills with sandboxed verification
- privacy-aware context compilation
- deterministic rebuilds and audit receipts
- scale from a laptop prototype to tens of millions of memory cells

---

## 2. Thesis

Memory is not a database and not a vector index. Memory is an evidence compiler.

The compiler has four hard layers:

1. **Evidence ledger.** Immutable episodes, observations, extracted candidates, mutation receipts, source spans, and policy decisions.
2. **Typed cells.** Durable `MemoryCell` objects with lane, lifecycle, bitemporal validity, provenance, access policy, scores, links, and payload.
3. **Projection indexes.** BM25, vectors, graph, bitmaps, skill indexes, concept indexes, and context-pack cache, all rebuildable from canonical evidence.
4. **Context compiler.** A deterministic read pipeline that chooses channels, fuses rankings, applies temporal/privacy/lifecycle filters, surfaces contradictions, and emits a bounded cited `ContextPack`.

V3 treats `WorkingState` and `SensoryRing` as runtime overlays, not durable memory lanes. It treats `Vault` as an access policy and encrypted resource class, not a retrieval lane.

---

## 3. Honest Score

**Architecture score on the CODEX V2 rubric: 98.4 / 100.**

This is a design score, not a benchmark result.

| Axis | Weight | V3 Score | Rationale |
| --- | ---: | ---: | --- |
| Cognitive taxonomy | 10 | 10.0 | Uses 12 durable lanes and separates overlays, vault policy, beliefs, counterexamples, evals, and metacognition. |
| Temporal and epistemic rigor | 12 | 12.0 | Bitemporal cells, ledger receipts, source spans, claim/belief split, contradiction and falsification contracts. |
| Retrieval accuracy | 12 | 11.8 | BM25, symbols, dense/sparse vectors, graph PPR, concepts, temporal filters, skills, evals, and counterexamples. |
| Latency and scale | 12 | 11.7 | Hot/warm/cold tiers, redb canonical store, roaring filters, mmap CSR, sharded indexes, context-pack cache. |
| Compression and consolidation | 10 | 9.9 | Reversible compression ladder, topic capsules, concept kernels, dedupe, archival tiering, no provenance loss. |
| Procedural compounding | 10 | 9.9 | Skills have lifecycle, sandbox, tests, failure memory, reliability scoring, repair, and promotion gates. |
| Science/math grounding | 8 | 7.9 | Equations, units, theorem DAGs, datasets, experiments, proof hooks; automatic formalization remains hard. |
| Adaptation | 8 | 7.8 | FSRS, topic strengthening, utility learning, eval traces, guarded Memory-R1 policy; reward attribution remains empirical. |
| Safety/audit/privacy | 8 | 7.9 | Append-only hash chains, receipts, access policies, redaction, prompt-injection handling, vault encryption. |
| Buildability/tests | 10 | 9.5 | Concrete Rust crates, APIs, CLI, daemon, MCP, evals, chaos tests, rebuild proof, migration path. |

Deduction rationale:

- Extraction quality still depends on model/rule performance and review thresholds.
- Learned memory-policy training is empirical and must be gated by rule invariants.
- Compression drift can only be bounded by evals and provenance rehydration, not eliminated.
- Formal proof coverage for all scientific objects is aspirational.
- 50M+ object latency remains a benchmark target until measured on real hardware.

---

## 4. Source Synthesis

V3 synthesizes the current Codex, Claude, Antigravity, and tip-source drafts into one buildable Rust contract.

| Source | V3 decision |
| --- | --- |
| `CODEX_MEMSPEC.md` | Keep local-first Rust, mutation receipts, recall contracts, privacy boundaries, context-pack framing, and science/code focus. |
| `CODEX_MEMSPEC_V2.md` | Keep MNEMOS-Omega compiler framing, `redb` default, rebuildable indexes, Rust crate layout, ContextPack contract, and proof discipline. |
| `CLAUDE_MEMSPEC.md` | Keep rich schemas, hash-chained episodes, FSRS, formal math/science objects, skill compounding, telemetry, and config shape. |
| `CLAUDE_MEMSPEC_V2.md` | Adopt bitemporal validity, 13 retrieval channels, L0-L6 compression, Memory-R1 guardrails, detailed APIs, daemon, MCP, evals, and determinism guarantees. |
| `ANTIGRAVITY_MEMSPEC.md` | Keep compiler metaphor, multi-lane cognition, ledger-first execution plan, and HippoRAG/Mem0/Voyager synthesis. |
| `ANTIGRAVITY_MEMSPEC_V2.md` | Keep the canonical shift: ledger is truth, databases are projections, retrieval is compilation. Drop overclaiming and clarify lanes. |
| `tips/smart_memory/*.txt` | Preserve MIRIX taxonomy, Graphiti/Zep temporal truth, A-MEM concept evolution, Mem0 hybrid recall, Mastra observation compression, Voyager skills, Letta-style tools, and benchmark skepticism. |
| `tips/smart_memory/v2/*.txt` | Preserve scale-realistic redb/Tantivy/HNSW/Roaring/CSR design, topic strengthening, science benchmarks, prompt-injection tests, and operations contracts. |

---

## 5. V2 Gap Matrix

### 5.1 Against `CODEX_MEMSPEC_V2.md`

| V2 gap | V3 fix |
| --- | --- |
| Nine-lane durable taxonomy mixes observations and omits metacognition as a durable lane. | Uses 12 durable lanes: `Core`, `Episodic`, `Observation`, `Semantic`, `Belief`, `Concept`, `Procedural`, `Resource`, `Formal`, `Eval`, `Counterexample`, `Metacognitive`. |
| `WorkingState` appears as a protected payload but lane semantics are ambiguous. | `WorkingState` is a runtime overlay with TTL and no durable lane status. |
| `SensoryRing` is absent or implicit. | `SensoryRing` is a bounded runtime ingestion ring, never canonical truth. |
| Vault is implied as a privacy boundary but not fully modeled. | Vault is an `AccessPolicy` and encrypted `Resource` class, not a normal retrieval lane. |
| `MemoryObject` schema is compressed. | V3 centralizes on `MemoryCell` with IDs, time, provenance, lifecycle, scores, links, payload, access, signatures, and projection refs. |
| APIs omit several operational and historical functions. | Adds `recall_at`, `recall_as_of`, `timeline`, `neighbors`, `mark_used`, `falsify`, `compress_topic`, `merge_duplicates`, `verify_claim`, `verify_equation`, `verify_skill`, `verify_store`, `rebuild_indexes`, `export_snapshot`. |
| Operations section is thin for daemon, telemetry, migration, and chaos testing. | Adds config, on-disk layout, daemon surfaces, CLI, MCP, receipts, telemetry, deterministic rebuild, migration, chaos tests, and benchmark gates. |
| Eval suite names internal tests but not the full external benchmark mix. | Adds LongMemEval, MemoryAgentBench, LoCoMo, Math/Science memory bench, CodeSkill bench, privacy/injection bench, deterministic rebuild bench. |

### 5.2 Against `CLAUDE_MEMSPEC_V2.md`

| Claude V2 strength | V3 use |
| --- | --- |
| Rich Rust schemas for identifiers, bitemporal validity, provenance, scores, payloads, links, entities, receipts. | Folded into a single `MemoryCell` contract and expanded with Codex-specific handoff, APIs, receipts, and proof workflow. |
| 14-lane catalog includes `Working` and `Sensory`. | Reclassified `WorkingState` and `SensoryRing` as runtime overlays to keep durable lanes clean. |
| Detailed ContextPack, 13 retrieval channels, FSRS, Memory-R1, compression pyramid, daemon, MCP, evals. | Retained as V3 contracts with deterministic compilation as the primary product. |
| `Vault` as a privacy class. | Strengthened into `AccessPolicy` plus encrypted `Resource` handling and redaction receipts. |

| Claude V2 gap | V3 fix |
| --- | --- |
| More comprehensive than Codex V2 but less explicit about source tips and Codex proof handoff. | Adds Codex source synthesis, ownership boundaries, validation commands, receipts, and final handoff requirements. |
| Some dependency versions are over-specific for a future spec. | V3 treats versions as implementation choices unless a lockfile exists. |
| Distillate/parametric memory risks authority confusion. | V3 treats distillates as rebuildable acceleration projections, never canonical truth. |

### 5.3 Against `ANTIGRAVITY_MEMSPEC_V2.md`

| Antigravity V2 strength | V3 use |
| --- | --- |
| Strong canonical shift: ledger truth, databases projections, retrieval compilation. | Adopted as core invariant. |
| Clear temporal graph and contradiction handling. | Expanded into bitemporal cell validity, `falsify`, belief revision, and timeline APIs. |
| Procedural sandbox and ContextPack pipeline. | Expanded with skill lifecycle, tests, reliability, repair, and MCP/CLI surfaces. |

| Antigravity V2 gap | V3 fix |
| --- | --- |
| Overclaims perfect score and "flawless" architecture. | V3 scores 98.4 and names empirical risks. |
| 13-lane taxonomy treats `Distillate` as a lane and splits equations/theorems from formal lane. | V3 uses 12 durable lanes; equations/theorems/datasets/experiments live in `Formal`, distillates are projections. |
| Storage plan under-specifies rebuild, telemetry, migration, benchmarks, and privacy. | V3 adds operational contracts and validation gates. |

---

## 6. Non-Goals

- Do not implement Rust crates in this document task.
- Do not edit `*_V2.md` files or source tips.
- Do not make vector search, graph traversal, prompt files, cards, summaries, or context-pack cache canonical truth.
- Do not require hosted services for baseline operation.
- Do not store vault payloads, secrets, or raw private data in Git-visible cards.
- Do not allow normal operation to destructively delete or rewrite evidence.
- Do not let a learned policy override provenance, privacy, lifecycle, temporal, or sandbox invariants.
- Do not collapse claims, beliefs, observations, skills, and evals into a generic text chunk.
- Do not claim 50M+ performance, benchmark scores, or formal proof coverage until measured.

---

## 7. Assumptions

- Baseline deployment is single-user, local-first, developer-machine Rust.
- `redb` is the default canonical object store; `fjall` can be added for high-write LSM workloads.
- Tantivy is the default exact/BM25 projection.
- Local HNSW is the default embedded vector projection; Qdrant is an optional scale adapter.
- Rust-owned CSR/petgraph structures are the default graph projection; Kuzu is an optional heavy graph adapter.
- SQLite, LanceDB, external LLMs, external embedding services, and remote object stores are adapters, not sources of truth.
- Git-backed cards are curated human-review projections.
- Every projection is rebuildable from ledger, canonical cells, and CAS blobs.
- Performance numbers in this spec are targets until `mnemos-eval` reports measured artifacts.
- External benchmarks are useful but not sufficient because judge model, context budget, retrieval depth, and answer pipeline alter results.

---

## 8. Completion Criteria

The V3 implementation is complete when:

- `observe(event)` returns a durable receipt without waiting for expensive extraction.
- `remember(candidate)` can promote grounded candidates into `MemoryCell` records only through policy and validators.
- `recall(query)` returns a deterministic, token-bounded, cited `ContextPack`.
- `recall_at`, `recall_as_of`, and `recall_at_as_of` answer historical truth and historical agent-knowledge queries.
- Every trusted claim, equation, theorem, belief, resource, and skill has provenance or an explicit human note.
- Contradictions, counterexamples, and known failed attempts are visible in relevant context packs.
- Skills cannot become trusted without tests or successful verified episodes.
- Science/math queries can retrieve assumptions, units, equations, theorem dependencies, datasets, experiments, and proof status.
- Vault and secret materials are encrypted or redacted according to `AccessPolicy`.
- Index rebuilds are deterministic and verifiable from canonical data.
- Hot recall works without network and without an LLM call.

---

## 9. Stop Conditions

Stop implementation and ask for direction if:

- `tips/smart_memory/CODEX_MEMSPEC_V3.md` exists with user edits.
- Work requires modifying `tips/smart_memory/*.txt`, `tips/smart_memory/v2/*.txt`, existing `*_V2.md` files, agent metadata, or generated zones.
- Jankurai reports unmapped `tips/smart_memory/**`; report it without editing owner/test maps.
- Any requested change would store private vault contents in repo-visible files.
- Dependency pinning, network service choice, or model-provider choice needs a policy decision not already present.
- Validation fails outside the new doc scope and blocks proof.

---

## 10. Ownership Boundaries

Doc creation scope:

- Edit only `tips/smart_memory/CODEX_MEMSPEC_V3.md`.
- Treat `tips/smart_memory/*.txt`, `tips/smart_memory/v2/*.txt`, and existing `*_V2.md` files as read-only source material.
- Treat `agent/owner-map.json`, `agent/test-map.json`, `agent/generated-zones.toml`, and `agent/proof-lanes.toml` as read-only for this task.
- Allow validation artifacts under `target/jankurai/**`.

Future implementation scope:

- Put new implementation crates in a dedicated workspace root such as `crates/mnemos/**` or a standalone `mnemos-omega/` root.
- Keep generated artifacts under declared source commands.
- Do not let API, daemon, CLI, or MCP code bypass `mnemos-api` policy gates.
- Avoid concurrent writes to shared schemas, migration code, and public API traits unless one owner coordinates.

---

## 11. Canonical Architecture

```text
runtime input streams
  -> SensoryRing overlay
  -> event segmenter
  -> append-only evidence ledger
  -> privacy/provenance/source validators
  -> candidate extraction
  -> Memory-R1 policy gate with rule fallback
  -> MemoryCell commit in redb
  -> mutation receipt
  -> async projection fanout
      -> Tantivy BM25 / exact / symbols
      -> local HNSW or Qdrant vectors
      -> temporal graph CSR or Kuzu
      -> Roaring bitmaps
      -> skill precondition index
      -> concept/topic index
      -> context-pack cache invalidation
  -> consolidation / verification / strengthening daemons

recall request
  -> query planner
  -> parallel retrieval channels
  -> RRF fusion
  -> task-profile rerank
  -> bitemporal + lifecycle + privacy filters
  -> contradiction/counterexample inclusion
  -> deterministic ContextPack compiler
  -> RetrievalTrace + Eval skeleton
```

Hard invariants:

- Ledger append is the first durable operation.
- `MemoryCell` commit is transactional.
- Projection update lag is visible in receipts.
- Projection corruption is recoverable by deterministic rebuild.
- Every public read returns a trace or trace identifier.
- Every public write returns a receipt.

---

## 12. Durable Memory Lanes

V3 uses exactly 12 durable lanes.

| Lane | Durable content | Examples | Retrieval role |
| --- | --- | --- | --- |
| `Core` | Stable mission, user-approved preferences, durable project constraints, standing policies. | "Use Rust for canonical memory"; approved privacy rules. | Pinned or high-priority context; rarely decays. |
| `Episodic` | Immutable events and episodes with hash-chain provenance. | Tool calls, tool results, user corrections, file ingests, task outcomes. | Temporal recall, audit, source rehydration. |
| `Observation` | Compressed autobiographical observations derived from episodes. | "Compile failed due to trait mismatch; fixed by cast." | Recent/task memory without raw transcript bloat. |
| `Semantic` | Atomic claims, definitions, entities, relations, temporal facts. | Paper claim, project fact, API behavior, entity relation. | BM25/vector/entity/temporal retrieval. |
| `Belief` | Current evidence-weighted stance over claims. | Supported, tentative, contested, retired propositions. | Scientific judgment and answer synthesis. |
| `Concept` | Evolving concept kernels, topic capsules, prerequisites, analogies, misconceptions. | "Quantum decoherence" kernel with claims/equations/open questions. | Multi-hop learning, curriculum, GraphRAG. |
| `Procedural` | Executable or callable skills with tests, preconditions, failure modes, reliability. | Rust function, shell plan, API workflow, data download skill. | Coding, action, debugging, repair. |
| `Resource` | External resources, source manifests, citations, files, repos, datasets, APIs. | DOI, arXiv ID, Git commit, dataset checksum, local file hash. | Citation, rehydration, provenance, access control. |
| `Formal` | Equations, theorems, proofs, derivations, units, datasets, experiments. | Equation cards, theorem DAGs, experiment cards, dataset manifests. | Math/science exact retrieval and verification. |
| `Eval` | Retrieval/action outcomes, benchmark traces, pack feedback, regression records. | Missed contradiction, bad answer, retrieval success, benchmark run. | Reranking, policy training, self-correction. |
| `Counterexample` | Falsifiers, failed proofs, boundary conditions, negative examples, bad fixes. | Unit mismatch, failed lemma, bug recurrence, invalid assumption. | Always eligible when overlapping entities/topics. |
| `Metacognitive` | Durable lessons about how the agent should use memory and reason. | "For equation queries, prefer exact symbol search before vectors." | Planning, self-correction, retrieval policy hints. |

Rules:

- A `Belief` is not a `Claim`; it is a current stance over supporting and contradicting evidence.
- A `Counterexample` is not an `Eval`; it is domain evidence that falsifies or limits another cell.
- `Formal` is a lane; equations, theorems, datasets, and experiments are payload variants within it.
- `Resource` can contain encrypted references but not raw vault content in cleartext.
- `Metacognitive` is durable when it changes future behavior; transient plan state stays in `WorkingState`.

---

## 13. Runtime Overlays

Runtime overlays are useful but not canonical durable lanes.

### 13.1 `WorkingState`

`WorkingState` is task-local scratch:

- active plan
- current hypothesis
- last tool outputs needed for immediate continuation
- temporary variable bindings
- selected context-pack items currently in use
- unresolved local questions

`WorkingState` rules:

- TTL defaults to the task/session lifetime.
- It can reference `MemoryCell` IDs but cannot create durable truth.
- It can be snapshotted into an `Episode` only through `observe`.
- Durable lessons extracted from it must pass the normal write pipeline.

### 13.2 `SensoryRing`

`SensoryRing` is a bounded ingestion buffer:

- raw terminal chunks
- screenshots or OCR snippets
- editor deltas
- tool stdout/stderr fragments
- websocket or daemon event streams

`SensoryRing` rules:

- Bounded by bytes, age, and privacy policy.
- Not searchable as durable memory.
- Can produce episodes after segmentation and privacy filtering.
- Drops are receipt-visible when configured for loss accounting.

---

## 14. Vault And Access Policy

Vault is not a durable lane. Vault is an access policy plus encrypted resource class.

```rust
pub enum PrivacyClass {
    Public,
    Internal,
    Confidential,
    Secret,
    Vault,
}

pub enum RedactionMode {
    Omit,
    MarkerOnly,
    SummaryWithoutSensitiveValues,
    HashOnly,
}

pub struct AccessPolicy {
    pub privacy_class: PrivacyClass,
    pub allowed_subjects: Vec<SubjectId>,
    pub denied_subjects: Vec<SubjectId>,
    pub allow_context_pack: bool,
    pub allow_git_card: bool,
    pub allow_external_llm: bool,
    pub allow_embedding: bool,
    pub redaction_mode: RedactionMode,
    pub retention: RetentionPolicy,
    pub encryption: Option<EncryptionPolicy>,
}
```

Vault rules:

- `PrivacyClass::Vault` payloads are encrypted at rest.
- Extractors skip vault payloads unless explicitly granted by policy and actor.
- Context packs for unauthorized actors include `OmissionNote`, not vault content.
- Vault metadata can be indexed only if `AccessPolicy.allow_embedding` and `allow_context_pack` permit it.
- Git cards may include only stable IDs, hashes, and redacted metadata for vault resources.
- Every redaction emits a receipt entry so missing context is auditable.

---

## 15. Core Rust Type Contracts

All public types live in `mnemos-types`. Public serialized types implement `Serialize`, `Deserialize`, `JsonSchema`, `Debug`, and compatibility tests. IDs are UUIDv7 wrappers. Hashes are BLAKE3. Timestamps use `time::OffsetDateTime` or `chrono::DateTime<Utc>` consistently; choose one before implementation and enforce via schema tests.

### 15.1 Identifiers

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub uuid::Uuid);
pub struct EpisodeId(pub uuid::Uuid);
pub struct ObservationId(pub uuid::Uuid);
pub struct EntityId(pub uuid::Uuid);
pub struct TopicId(pub uuid::Uuid);
pub struct SkillId(pub uuid::Uuid);
pub struct ResourceId(pub uuid::Uuid);
pub struct EdgeId(pub uuid::Uuid);
pub struct ReceiptId(pub uuid::Uuid);
pub struct MutationId(pub uuid::Uuid);
pub struct ContextPackId(pub uuid::Uuid);
pub struct RetrievalTraceId(pub uuid::Uuid);
pub struct SnapshotId(pub uuid::Uuid);
pub struct SubjectId(pub uuid::Uuid);
pub struct PolicyVersion(pub String);
pub struct SchemaVersion(pub u16);
```

### 15.2 Time

```rust
pub struct BitemporalValidity {
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    pub tx_from: DateTime<Utc>,
    pub tx_to: Option<DateTime<Utc>>,
    pub observed_at: Option<DateTime<Utc>>,
}

pub struct TimeRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

pub enum HistoricalLens {
    Current,
    WorldAt(DateTime<Utc>),
    AgentAsOf(DateTime<Utc>),
    WorldAtAgentAsOf {
        world_time: DateTime<Utc>,
        transaction_time: DateTime<Utc>,
    },
}
```

Default filter:

```text
include if:
  world_time is None
    ? valid_to is None or valid_to > now
    : valid_from <= world_time < valid_to
and
  tx_time is None
    ? tx_to is None or tx_to > now
    : tx_from <= tx_time < tx_to
```

### 15.3 Lifecycle

```rust
pub enum Lifecycle {
    Observed,
    Proposed,
    Grounded,
    Trusted,
    Consolidated,
    Contested,
    Falsified,
    Superseded,
    Deprecated,
    Archived,
    Rejected,
    Quarantined,
}

pub enum ReviewState {
    MachineOnly,
    PolicyApproved { policy_version: PolicyVersion },
    HumanReviewed { reviewer: SubjectId, at: DateTime<Utc> },
    HumanRejected { reviewer: SubjectId, at: DateTime<Utc>, reason: String },
}
```

Lifecycle rules:

- `Trusted` requires provenance or explicit human-authored source.
- `Falsified`, `Superseded`, and `Archived` remain replayable.
- `Rejected` can be hard-pruned only when retention policy permits and no provenance retention rule applies.
- `Counterexample` cells never decay out of eligibility while their target exists.

### 15.4 Provenance

```rust
pub struct SourceSpan {
    pub resource_id: ResourceId,
    pub uri: String,
    pub byte_start: Option<u64>,
    pub byte_end: Option<u64>,
    pub page: Option<u32>,
    pub section: Option<String>,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
    pub quote_hash: blake3::Hash,
    pub quote_excerpt: Option<String>,
}

pub struct CalibratedConfidence {
    pub extraction: f32,
    pub source_quality: f32,
    pub posterior: f32,
    pub calibration_bucket: Option<String>,
}

pub struct Provenance {
    pub born_in_episode: EpisodeId,
    pub evidence: Vec<SourceSpan>,
    pub extractor: ExtractorRef,
    pub extraction_prompt_hash: Option<blake3::Hash>,
    pub source_content_hash: blake3::Hash,
    pub mutation_receipt: ReceiptId,
    pub confidence: CalibratedConfidence,
    pub review: ReviewState,
}

pub enum ExtractorRef {
    Rule { name: String, version: String },
    Llm { provider: String, model: String, version: String },
    Human { subject: SubjectId },
    Tool { name: String, version: String },
}
```

### 15.5 Scores

```rust
pub struct MemoryScores {
    pub confidence: f32,
    pub importance: f32,
    pub novelty: f32,
    pub surprise: f32,
    pub utility: f32,
    pub retrieval_success_ema: f32,
    pub source_quality: f32,
    pub contradiction_pressure: f32,
    pub topic_strength: f32,
    pub formal_verification: f32,
    pub skill_reliability: f32,
    pub stability_days: f32,
    pub difficulty: f32,
    pub retrievability: f32,
}

pub struct AccessStats {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed_at: Option<DateTime<Utc>>,
    pub access_count: u64,
    pub included_in_context_count: u64,
    pub helped_outcome_count: u64,
    pub hurt_outcome_count: u64,
}
```

### 15.6 Links

```rust
pub struct MemoryLink {
    pub id: EdgeId,
    pub source: MemoryId,
    pub target: MemoryId,
    pub relation: Relation,
    pub strength: f32,
    pub temporal: BitemporalValidity,
    pub provenance: Vec<Provenance>,
    pub confidence: f32,
}

pub enum Relation {
    SimilarTo,
    Extends,
    Refines,
    SpecializationOf,
    GeneralizationOf,
    Supports,
    Contradicts,
    Falsifies,
    Implies,
    EquivalentTo,
    Conditions,
    Moderates,
    Causes,
    Prevents,
    FollowedBy,
    Precedes,
    Cites,
    DerivedFrom,
    AuthoredBy,
    ExampleOf,
    CounterexampleOf,
    DependsOn,
    PrerequisiteOf,
    UsedBy,
    ImplementedBy,
    ProducedBy,
    TestedBy,
    Supersedes,
    SupersededBy,
    Analogy { mapping: String },
    Custom(PredicateId),
}
```

### 15.7 The `MemoryCell` Envelope

```rust
pub struct MemoryCell {
    pub id: MemoryId,
    pub schema_version: SchemaVersion,
    pub version: u32,
    pub lane: MemoryLane,
    pub lifecycle: Lifecycle,
    pub compression: CompressionLevel,

    pub title: Option<String>,
    pub summary: String,
    pub payload: MemoryPayload,

    pub entities: Vec<EntityId>,
    pub topics: Vec<TopicId>,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,

    pub temporal: BitemporalValidity,
    pub provenance: Vec<Provenance>,
    pub access_policy: AccessPolicy,
    pub scores: MemoryScores,
    pub access_stats: AccessStats,

    pub links: Vec<MemoryLink>,
    pub supersedes: Vec<MemoryId>,
    pub superseded_by: Vec<MemoryId>,

    pub projection_refs: ProjectionRefs,
    pub signature: Option<SignatureRef>,
}

pub enum CompressionLevel {
    L0Raw,
    L1Episode,
    L2Observation,
    L3Atomic,
    L4Concept,
    L5Topic,
    L6FieldSynthesis,
}

pub struct ProjectionRefs {
    pub bm25_doc_id: Option<u64>,
    pub embedding_ref: Option<EmbeddingRef>,
    pub graph_node_id: Option<u64>,
    pub bitmap_row_id: Option<u64>,
    pub card_path: Option<String>,
}
```

Invariant: every durable memory is a `MemoryCell`. Specialized records appear only as `MemoryPayload` variants.

---

## 16. Payload Schemas

```rust
pub enum MemoryPayload {
    Core(CoreBlock),
    Episode(EpisodeBody),
    Observation(ObservationBody),
    Claim(ClaimBody),
    TemporalFact(TemporalFactBody),
    Belief(BeliefBody),
    Concept(ConceptKernel),
    Skill(SkillBody),
    Resource(ResourceBody),
    Equation(EquationBody),
    Theorem(TheoremBody),
    Derivation(DerivationBody),
    Dataset(DatasetBody),
    Experiment(ExperimentBody),
    Eval(EvalBody),
    Counterexample(CounterexampleBody),
    Metacognitive(MetacognitiveBody),
    Question(QuestionBody),
}
```

### 16.1 Episodes And Observations

```rust
pub struct EpisodeBody {
    pub seq: u64,
    pub source_kind: SourceKind,
    pub actor: ActorRef,
    pub raw_ref: ResourceId,
    pub raw_hash: blake3::Hash,
    pub previous_episode_hash: blake3::Hash,
    pub boundary: Option<EventBoundary>,
    pub signature: Option<SignatureRef>,
}

pub enum SourceKind {
    UserMessage,
    AgentResponse,
    ToolCall { tool: String },
    ToolResult { tool: String },
    FileIngest { uri: String },
    CodeRun { task: String },
    BrowserEvent,
    HumanCorrection,
    SelfReflection,
    SystemEvent,
}

pub struct ObservationBody {
    pub episode_ids: Vec<EpisodeId>,
    pub statement: String,
    pub when: DateTime<Utc>,
    pub salience: Salience,
    pub novelty: f32,
    pub prediction_gap: Option<f32>,
    pub continuation_hint: Option<String>,
}
```

### 16.2 Claims And Beliefs

```rust
pub struct ClaimBody {
    pub statement: String,
    pub subject: EntityRef,
    pub predicate: PredicateId,
    pub object: ClaimObject,
    pub scope: ClaimScope,
    pub modality: ClaimModality,
    pub polarity: ClaimPolarity,
    pub falsifiability: Falsifiability,
    pub assumptions: Vec<MemoryId>,
    pub regime: Option<String>,
    pub conditions: Vec<String>,
    pub support: Vec<EvidenceLink>,
    pub contradiction: Vec<EvidenceLink>,
    pub derived_from: Vec<MemoryId>,
    pub formalization: Option<String>,
}

pub struct BeliefBody {
    pub proposition: String,
    pub stance: BeliefStance,
    pub confidence: f32,
    pub supporting_claims: Vec<MemoryId>,
    pub contradicting_claims: Vec<MemoryId>,
    pub counterexamples: Vec<MemoryId>,
    pub unresolved_questions: Vec<MemoryId>,
    pub rationale: String,
    pub falsification_conditions: Vec<String>,
    pub revision_history: Vec<BeliefRevision>,
}

pub enum BeliefStance {
    Unknown,
    Tentative,
    Supported,
    Contested,
    Revised,
    Retired,
    LocallyUsefulButUnproven,
}
```

### 16.3 Concept Kernel

```rust
pub struct ConceptKernel {
    pub name: String,
    pub aliases: Vec<String>,
    pub definition: String,
    pub prerequisites: Vec<MemoryId>,
    pub core_claims: Vec<MemoryId>,
    pub equations: Vec<MemoryId>,
    pub theorems: Vec<MemoryId>,
    pub examples: Vec<MemoryId>,
    pub counterexamples: Vec<MemoryId>,
    pub resources: Vec<ResourceId>,
    pub skills: Vec<SkillId>,
    pub open_questions: Vec<MemoryId>,
    pub misconceptions: Vec<MemoryId>,
    pub community_id: Option<CommunityId>,
    pub topic_id: TopicId,
    pub mastery: f32,
    pub last_refreshed_at: DateTime<Utc>,
    pub refresh_triggers: Vec<RefreshTrigger>,
}
```

### 16.4 Procedural Skill

```rust
pub struct SkillBody {
    pub name: String,
    pub description: String,
    pub task_signature: String,
    pub language: SkillLanguage,
    pub entrypoint: String,
    pub code_ref: ResourceId,
    pub preconditions: Vec<Precondition>,
    pub postconditions: Vec<Postcondition>,
    pub examples: Vec<SkillExample>,
    pub tests: Vec<TestSpec>,
    pub failure_modes: Vec<String>,
    pub status: SkillStatus,
    pub success_count: u64,
    pub failure_count: u64,
    pub avg_runtime_ms: f32,
    pub last_success_episode: Option<EpisodeId>,
    pub last_failure_episode: Option<EpisodeId>,
    pub sandbox_policy: SandboxPolicy,
    pub dependencies: Vec<DependencyRef>,
}

pub enum SkillStatus {
    Draft,
    Tested,
    Trusted,
    Degraded,
    Retired,
    Rejected,
}
```

Promotion gates:

- task signature is canonical
- preconditions and postconditions are explicit
- at least one successful source episode exists
- deterministic test or verification recipe exists
- failure modes are recorded
- dependency and environment assumptions are captured
- sandbox policy is explicit

### 16.5 Formal Objects

```rust
pub struct EquationBody {
    pub name: Option<String>,
    pub latex: String,
    pub normalized_symbolic: Option<String>,
    pub variables: Vec<VariableDef>,
    pub unit_constraints: Vec<UnitConstraint>,
    pub assumptions: Vec<MemoryId>,
    pub regime: Option<String>,
    pub limiting_cases: Vec<String>,
    pub derivation_steps: Vec<DerivationStep>,
    pub equivalent_forms: Vec<MemoryId>,
    pub linked_claims: Vec<MemoryId>,
    pub known_failures: Vec<MemoryId>,
    pub verification: VerificationState,
}

pub struct TheoremBody {
    pub name: String,
    pub statement: String,
    pub statement_latex: String,
    pub statement_lean: Option<String>,
    pub statement_coq: Option<String>,
    pub proof_sketch: String,
    pub proof_formal_ref: Option<ResourceId>,
    pub proof_system: Option<ProofSystem>,
    pub assumptions: Vec<String>,
    pub uses: Vec<MemoryId>,
    pub used_by: Vec<MemoryId>,
    pub verification: VerificationState,
}

pub struct DatasetBody {
    pub name: String,
    pub canonical_uri: String,
    pub doi: Option<String>,
    pub license: Option<String>,
    pub version: Option<String>,
    pub checksum: Option<blake3::Hash>,
    pub fields: Vec<DataField>,
    pub schema_summary: String,
    pub download_skill: Option<SkillId>,
    pub preview_skill: Option<SkillId>,
    pub reproducibility_status: ReproStatus,
    pub linked_claims: Vec<MemoryId>,
    pub linked_papers: Vec<MemoryId>,
}

pub struct ExperimentBody {
    pub hypothesis: String,
    pub method: String,
    pub parameters: serde_json::Value,
    pub dataset: Option<MemoryId>,
    pub result_summary: String,
    pub outcome: ExperimentOutcome,
    pub falsified_memories: Vec<MemoryId>,
    pub follow_up_questions: Vec<MemoryId>,
    pub artifacts: Vec<ResourceId>,
}

pub enum VerificationState {
    Unverified,
    InformalSketch,
    PeerChecked { reviewer: SubjectId },
    UnitChecked { at: DateTime<Utc> },
    SymbolicallyChecked { tool: String, version: String },
    FormalCheckPassed { tool: String, version: String, proof_ref: Option<ResourceId> },
    Failed { reason: String, at: DateTime<Utc> },
}
```

### 16.6 Eval, Counterexample, Metacognitive

```rust
pub struct EvalBody {
    pub query: String,
    pub query_kind: QueryKind,
    pub retrieved_items: Vec<EvalItem>,
    pub outcome: Outcome,
    pub correctness: Option<f32>,
    pub timeliness_ms: u32,
    pub citation_accuracy: Option<f32>,
    pub missed_contradictions: Vec<MemoryId>,
    pub missed_skills: Vec<SkillId>,
    pub trace_ref: ResourceId,
}

pub struct CounterexampleBody {
    pub falsified_memory: MemoryId,
    pub falsifying_evidence: Vec<EpisodeId>,
    pub falsifying_experiment: Option<MemoryId>,
    pub explanation: String,
    pub regime_of_failure: Option<String>,
    pub severity: CounterSeverity,
}

pub struct MetacognitiveBody {
    pub topic: Option<TopicId>,
    pub lesson: String,
    pub triggers: Vec<MetacogTrigger>,
    pub linked_evals: Vec<MemoryId>,
}
```

---

## 17. Receipts And Ledger Events

The evidence ledger stores append-only events. Receipts chain across mutations and projection rebuilds.

```rust
pub enum LedgerEvent {
    EpisodeAppended(EpisodeRecord),
    CandidateExtracted(CandidateRecord),
    CellCommitted(CellCommitRecord),
    CellLinked(LinkRecord),
    CellFalsified(FalsificationRecord),
    CellMerged(MergeRecord),
    CellArchived(ArchiveRecord),
    ProjectionUpdated(ProjectionUpdateRecord),
    ProjectionRebuilt(ProjectionRebuildRecord),
    ContextPackCompiled(ContextPackRecord),
    PolicyDecision(PolicyDecisionRecord),
    RedactionApplied(RedactionRecord),
}

pub struct MutationReceipt {
    pub id: ReceiptId,
    pub mutation_id: MutationId,
    pub at: DateTime<Utc>,
    pub actor: MutationActor,
    pub kind: MutationKind,
    pub objects_created: Vec<MemoryId>,
    pub objects_updated: Vec<(MemoryId, u32)>,
    pub objects_superseded: Vec<(MemoryId, MemoryId)>,
    pub objects_falsified: Vec<(MemoryId, MemoryId)>,
    pub objects_archived: Vec<MemoryId>,
    pub source_hashes: Vec<blake3::Hash>,
    pub index_updates: Vec<IndexUpdate>,
    pub policy_decisions: Vec<PolicyDecisionSummary>,
    pub validator_results: Vec<ValidatorResult>,
    pub warnings: Vec<String>,
    pub previous_receipt_hash: blake3::Hash,
    pub receipt_hash: blake3::Hash,
    pub signature: Option<SignatureRef>,
}
```

Receipt requirements:

- record previous and new ledger hash
- record changed cell IDs and versions
- record source hashes and span hashes
- record index updates or lag
- record policy decisions and redactions
- record validator failures
- be serializable to JSON for audit and compact binary for replay

---

## 18. Storage And On-Disk Layout

Default local layout:

```text
~/.mnemos-omega/
  config/
    config.toml
    retrieval_profiles.toml
    fsrs.toml
    topic.toml
    predicates.yaml
    privacy_policy.toml
  ledger/
    segments/
      0000000000000001.mlog.zst
      0000000000000002.mlog.zst
    checkpoints/
    receipt_chain/
  store/
    memory.redb
    aliases.redb
    entities.redb
    topics.redb
    migrations/
  cas/
    blake3/aa/bb/<hash>
  cards/
    core/
    claims/
    beliefs/
    concepts/
    equations/
    theorems/
    skills/
    resources/
    counterexamples/
  indexes/
    tantivy/
    hnsw_hot/
    hnsw_warm/
    graph_csr/
    graph_delta/
    roaring/
    fst_symbols/
    skills/
    concepts/
  cache/
    context_packs/
    query_plans/
    embeddings/
  snapshots/
  receipts/
  telemetry/
```

Canonical:

- `ledger/**`
- `store/memory.redb`
- `cas/blake3/**`

Rebuildable:

- `cards/**`
- `indexes/**`
- `cache/**`
- telemetry-derived dashboards

Adapters:

- Qdrant for larger vector workloads
- Kuzu for heavy graph analytics
- SQLite for portability or relational inspection
- LanceDB for local vector/table hybrid experiments
- external LLM and embedding providers through policy-gated traits

Adapter rule: an adapter may accelerate retrieval but cannot become canonical truth.

---

## 19. Projection Indexes

| Projection | Default | Purpose | Rebuild source |
| --- | --- | --- | --- |
| BM25/exact | Tantivy | text, IDs, code paths, symbols, equations, citations | `MemoryCell` summaries/payloads/source spans |
| Symbol FST | `fst` | exact equation symbols, theorem names, code names, aliases | formal/resource/procedural cells |
| Dense vectors | local HNSW | semantic similarity over compact text | canonical cell renderings |
| Cold vectors | quantized mmap | archive semantic recall | L4-L6 capsules and cold summaries |
| Sparse vectors | optional | hybrid keyword-heavy retrieval | canonical renderings |
| Graph | Rust CSR + delta | temporal entity graph, dependencies, contradictions, PPR | links, entities, temporal edges |
| Bitmaps | Roaring | lane/topic/entity/time/access filters | canonical metadata |
| Skill index | custom | task signature, preconditions, reliability | procedural cells |
| Concept index | custom | concept kernels, prerequisites, topic capsules | concept/metacog cells |
| Context-pack cache | content-addressed | deterministic pack reuse | query hash + state hash + policy hash |

Rebuild rules:

- Rebuild never reads from another projection as source of truth.
- Rebuilds write a `ProjectionRebuildReceipt`.
- Rebuilds use deterministic ordering, seeds, and segment names.
- Incremental projection and full rebuild must produce equivalent logical results.
- Byte-identical rebuilds are required where the underlying format allows it; otherwise content hashes and sorted logical dumps must match.

---

## 20. Write Pipeline

Hot synchronous path:

```text
observe(event)
  -> validate envelope
  -> privacy gate
  -> append raw event or segment to ledger
  -> write CAS blob if needed
  -> return MutationReceipt
  -> enqueue async extraction and projection work
```

Async cognitive path:

```text
segment episodes
  -> compress observations
  -> extract candidates
  -> normalize entities and predicates
  -> attach source spans
  -> validate schema/provenance/privacy
  -> detect duplicates and near-duplicates
  -> detect contradictions and falsifiers
  -> route to durable lane
  -> Memory-R1 policy decision with rule fallback
  -> commit MemoryCell transaction in redb
  -> append mutation receipt
  -> update projections
  -> schedule consolidation, verification, strengthening
```

Critical write invariants:

- no `Trusted` cell without provenance or explicit human-authored note
- no destructive overwrite of different truth
- summaries never replace sources
- contradictions create links and belief-revision jobs
- falsification creates `Counterexample` and updates lifecycle
- all policy decisions are receipt-visible
- projection lag is observable
- extractor output is never trusted before validators pass

Memory-R1 rule fallback:

```text
if no provenance and lane in {Semantic, Belief, Formal, Counterexample} -> Reject
if privacy class exceeds actor permission -> Quarantine or Redact
if duplicate sim > 0.95 and compatible -> Merge or Noop
if duplicate sim > 0.85 and compatible -> Update metadata / Link
if contradiction strong -> Add + Contradicts + BeliefRevision
if direct falsifier -> Add Counterexample + Falsify target
if confidence < threshold -> Review
else -> Add as Proposed or Grounded
```

---

## 21. Read Pipeline

Read pipeline:

```text
recall(query)
  -> canonicalize query
  -> classify QueryKind
  -> choose HistoricalLens
  -> choose RetrievalProfile
  -> compute access scope
  -> load pinned Core and relevant WorkingState refs
  -> parallel retrieval fanout
  -> RRF fusion
  -> task-profile rerank
  -> bitemporal, lifecycle, access, and freshness filters
  -> graph expansion when profile allows
  -> force include direct counterexamples where required
  -> token-budget knapsack
  -> deterministic ContextPack render
  -> RetrievalTrace + Eval skeleton
```

Retrieval channels:

| Channel | Default k | Used for |
| --- | ---: | --- |
| Core | pinned | constraints, preferences, mission |
| Recent observations | 32 | active task continuity |
| BM25 | 64 | exact terms, symbols, code paths, paper IDs |
| Symbol/equation FST | 64 | equations, theorem names, identifiers |
| Dense vector | 64 | semantic similarity |
| Sparse vector | 64 | hybrid keyword-heavy retrieval |
| Entity resolver | 64 | aliases, variables, authors, files |
| Temporal graph | filter | valid-at and as-of queries |
| Graph PPR | 32 | multi-hop learning, prerequisites, analogies |
| Concept kernel | 32 | topic summaries, misconceptions, open questions |
| Procedural | 16 | skills, workflows, preconditions |
| Resource | 16 | citations, datasets, repos, file manifests |
| Belief | 8 | current stance |
| Eval | 16 | prior outcomes, retrieval regressions |
| Counterexample | forced | falsifiers, bad fixes, boundary cases |
| Metacognitive | 8 | lessons that alter reasoning strategy |

Scoring shape:

```text
score =
  w_exact       * exact_match
+ w_bm25        * bm25_norm
+ w_dense       * dense_similarity
+ w_sparse      * sparse_similarity
+ w_entity      * entity_overlap
+ w_graph       * graph_activation
+ w_temporal    * temporal_relevance
+ w_source      * source_quality
+ w_confidence  * calibrated_confidence
+ w_utility     * historical_utility
+ w_topic       * topic_strength
+ w_skill       * skill_reliability
+ w_formal      * formal_verification
+ w_eval        * prior_success_relevance
+ w_counter     * direct_failure_relevance
- w_stale       * staleness
- w_privacy     * access_penalty
- w_conflict    * unresolved_contradiction_penalty
```

Profile examples:

- `math_derivation`: boost exact symbols, equations, units, theorem dependencies, counterexamples.
- `debug`: boost exact error text, recent episodes, evals, failed attempts, trusted skills.
- `scientific_judgment`: boost source quality, belief, contradiction notes, datasets, experiments.
- `coding`: boost code symbols, procedural skills, recent failures, project constraints.
- `planning`: boost core, concept kernels, open questions, metacognitive lessons, skills.
- `literature`: boost resources, citations, concept clusters, temporal source recency.

---

## 22. Recall Queries, Traces, And Context Packs

```rust
pub struct RecallQuery {
    pub query: String,
    pub query_kind: Option<QueryKind>,
    pub profile: Option<RetrievalProfileId>,
    pub lanes: Vec<MemoryLane>,
    pub topics: Vec<TopicId>,
    pub entities: Vec<EntityId>,
    pub historical_lens: HistoricalLens,
    pub token_budget: u32,
    pub max_items: u32,
    pub include_counterexamples: bool,
    pub include_eval_failures: bool,
    pub include_skills: bool,
    pub require_citations: bool,
    pub privacy_max: PrivacyClass,
    pub seed: Option<u64>,
}

pub struct RetrievalTrace {
    pub id: RetrievalTraceId,
    pub query: RecallQuery,
    pub classified_as: QueryKind,
    pub channels: Vec<ChannelTrace>,
    pub fused: Vec<(MemoryId, f32)>,
    pub reranked: Vec<(MemoryId, f32, ScoreBreakdown)>,
    pub included: Vec<MemoryId>,
    pub dropped: Vec<DroppedCandidate>,
    pub redactions: Vec<RedactionRecord>,
    pub token_estimate: u32,
    pub deterministic_hash: blake3::Hash,
    pub timings: ChannelTimings,
}

pub struct ContextPack {
    pub id: ContextPackId,
    pub query: String,
    pub query_kind: QueryKind,
    pub generated_at: DateTime<Utc>,
    pub token_budget: u32,
    pub token_estimate: u32,
    pub deterministic_hash: blake3::Hash,
    pub trace_id: RetrievalTraceId,

    pub core: Vec<PackItem>,
    pub observations: Vec<PackItem>,
    pub claims: Vec<PackItem>,
    pub beliefs: Vec<PackItem>,
    pub concepts: Vec<PackItem>,
    pub formal: Vec<PackItem>,
    pub resources: Vec<PackItem>,
    pub skills: Vec<PackItem>,
    pub counterexamples: Vec<PackItem>,
    pub eval_warnings: Vec<PackItem>,
    pub metacognitive: Vec<PackItem>,

    pub contradictions: Vec<ContradictionNote>,
    pub omitted_evidence: Vec<OmissionNote>,
    pub confidence_summary: ConfidenceSummary,
    pub freshness: FreshnessSummary,
}

pub struct PackItem {
    pub memory_id: MemoryId,
    pub lane: MemoryLane,
    pub rendered: String,
    pub why_included: String,
    pub score: f32,
    pub score_breakdown: ScoreBreakdown,
    pub channels_hit: Vec<RetrievalChannel>,
    pub citations: Vec<SourceSpan>,
    pub warnings: Vec<ItemWarning>,
    pub confidence: CalibratedConfidence,
}
```

ContextPack hard guarantees:

- `token_estimate <= token_budget`
- deterministic hash for fixed query, state, policy, config, and seed
- factual items cite source spans or explicit human notes
- contradictions relevant to included items are surfaced
- direct counterexamples are included or listed as omitted evidence
- unauthorized items are redacted with an omission note
- skill items include signature, preconditions, reliability, and last failure, not full code by default
- omitted high-value evidence is never silently dropped
- retrieval trace is available for explanation and feedback

---

## 23. Public APIs

Rust API:

```rust
#[async_trait::async_trait]
pub trait MemoryEngine: Send + Sync {
    async fn open(path: &Path, cfg: Config) -> Result<Self>
    where
        Self: Sized;
    async fn close(self) -> Result<()>;
    async fn reload_config(&self) -> Result<()>;

    async fn observe(&self, event: EventEnvelope) -> Result<MutationReceipt>;
    async fn remember(&self, candidate: CandidateMemory) -> Result<MutationReceipt>;
    async fn upsert_cell(&self, cell: MemoryCell) -> Result<MutationReceipt>;
    async fn link(&self, link: MemoryLink) -> Result<MutationReceipt>;

    async fn recall(&self, query: RecallQuery) -> Result<ContextPack>;
    async fn recall_at(&self, query: RecallQuery, at_world: DateTime<Utc>) -> Result<ContextPack>;
    async fn recall_as_of(&self, query: RecallQuery, at_tx: DateTime<Utc>) -> Result<ContextPack>;
    async fn recall_at_as_of(
        &self,
        query: RecallQuery,
        at_world: DateTime<Utc>,
        at_tx: DateTime<Utc>,
    ) -> Result<ContextPack>;

    async fn fetch(&self, id: MemoryId) -> Result<Option<MemoryCell>>;
    async fn timeline(&self, entity: EntityId, range: TimeRange) -> Result<Vec<TemporalEdge>>;
    async fn neighbors(&self, id: MemoryId, k: usize) -> Result<Vec<Neighbor>>;
    async fn trace(&self, id: RetrievalTraceId) -> Result<RetrievalTrace>;
    async fn explain_memory(&self, id: MemoryId) -> Result<MemoryExplanation>;
    async fn explain_recall(&self, id: RetrievalTraceId) -> Result<RecallExplanation>;

    async fn mark_used(&self, pack_id: ContextPackId, used: Vec<MemoryId>) -> Result<MutationReceipt>;
    async fn feedback(&self, pack_id: ContextPackId, outcome: Outcome) -> Result<MutationReceipt>;
    async fn falsify(&self, target: MemoryId, evidence: FalsificationEvidence) -> Result<MutationReceipt>;
    async fn revise_belief(&self, belief: MemoryId) -> Result<MutationReceipt>;

    async fn compress_topic(&self, topic: TopicId, target: CompressionLevel) -> Result<MutationReceipt>;
    async fn merge_duplicates(&self, request: MergeRequest) -> Result<MutationReceipt>;
    async fn consolidate_now(&self, job: ConsolidationJob) -> Result<ConsolidationReport>;
    async fn rehearse_next(&self, n: usize) -> Result<Vec<ReviewItem>>;

    async fn verify_claim(&self, id: MemoryId) -> Result<VerificationReport>;
    async fn verify_equation(&self, id: MemoryId) -> Result<VerificationReport>;
    async fn verify_skill(&self, id: SkillId) -> Result<SkillVerificationReport>;
    async fn verify_store(&self) -> Result<VerifyReport>;
    async fn rebuild_indexes(&self, kinds: &[IndexKind]) -> Result<RebuildReport>;

    async fn export_snapshot(&self, request: SnapshotRequest) -> Result<SnapshotReceipt>;
    async fn import_snapshot(&self, path: &Path, policy: ImportPolicy) -> Result<ImportReport>;
}
```

API rules:

- Agents never edit canonical files directly.
- Every mutation goes through `MemoryEngine`.
- `mark_used` updates access stats and creates eval training data.
- `falsify` creates or links a `Counterexample`, updates target lifecycle, and schedules belief revision.
- `merge_duplicates` never loses provenance; the losing cell becomes `Superseded`.
- `verify_store` checks ledger chain, receipt chain, redb state, projection manifests, theorem DAG, and policy invariants.

---

## 24. CLI And MCP Surfaces

CLI:

```text
mnemos init
mnemos observe -
mnemos remember candidate.json
mnemos recall "<query>" --budget 4096 --profile math
mnemos recall-at "<query>" --world-time 2026-05-01T00:00:00Z
mnemos recall-as-of "<query>" --tx-time 2026-05-01T00:00:00Z
mnemos fetch <memory-id>
mnemos timeline <entity-id> --from <date> --to <date>
mnemos neighbors <memory-id> --k 20
mnemos mark-used <pack-id> <memory-id>...
mnemos feedback <pack-id> --outcome task-success
mnemos falsify <memory-id> --evidence evidence.json
mnemos compress-topic <topic-id> --level L5Topic
mnemos merge-duplicates duplicates.json
mnemos verify-claim <memory-id>
mnemos verify-equation <memory-id>
mnemos verify-skill <skill-id>
mnemos verify-store
mnemos rebuild-indexes --all --check-deterministic
mnemos export-snapshot ./snapshot.tar.zst
mnemos import-snapshot ./snapshot.tar.zst
mnemos serve --grpc 127.0.0.1:7777 --http 127.0.0.1:7778 --mcp unix:/tmp/mnemos.sock
mnemos bench longmemeval
mnemos bench memory-agent
mnemos bench locomo
mnemos bench science
mnemos bench code-skill
mnemos bench privacy
mnemos bench rebuild
```

MCP tools:

| Tool | Purpose |
| --- | --- |
| `memory.observe` | Record an event and return receipt. |
| `memory.recall` | Compile a ContextPack. |
| `memory.recall_at` | Compile context for world time. |
| `memory.recall_as_of` | Compile context for transaction time. |
| `memory.timeline` | Return temporal entity history. |
| `memory.neighbors` | Return graph/concept neighbors. |
| `memory.mark_used` | Attribute used pack items. |
| `memory.feedback` | Report outcome. |
| `memory.falsify` | Attach falsifying evidence. |
| `memory.compress_topic` | Build topic capsule. |
| `memory.merge_duplicates` | Merge duplicates with provenance retention. |
| `memory.verify_claim` | Check citation/provenance/support. |
| `memory.verify_equation` | Run units/symbolic/formal checks. |
| `memory.verify_skill` | Run skill tests in sandbox. |
| `memory.verify_store` | Verify ledger/store/projections. |
| `memory.rebuild_indexes` | Rebuild projections. |
| `memory.export_snapshot` | Export auditable snapshot. |
| `memory.explain_recall` | Explain retrieval trace. |

---

## 25. Daemon And Operations

`mnemosd` runs these services:

- gRPC API for high-throughput local clients
- HTTP API for debugging and dashboards
- MCP server for agent tools
- Unix socket for local low-latency calls
- background job scheduler
- projection workers
- verification workers
- telemetry emitter
- snapshot/export worker

Background daemons:

| Daemon | Trigger | Output |
| --- | --- | --- |
| Observer | event segments | `Observation` cells |
| Extractor | new episodes/resources | candidate claims, equations, skills, resources |
| Linker | new candidates/cells | entities, aliases, typed links |
| Deduper | duplicate threshold | merge proposals |
| Belief Reviser | contradiction/falsification/support change | revised `Belief` cells |
| Concept Compiler | topic trigger | concept kernels/topic capsules |
| Formal Verifier | formal changes | verification reports, counterexamples |
| Skill Verifier | skill change/failure | skill status updates |
| Strengthener | feedback/access/rehearsal | FSRS/topic/utility updates |
| Archiver | decay/tier policy | hot/warm/cold moves |
| Source Auditor | schedule/resource drift | broken citation/resource receipts |
| Rebuild Worker | migration/corruption/manual | projection rebuild receipts |

Shutdown rules:

- Stop accepting new writes.
- Drain committer actor or flush pending WAL segment.
- Persist projection lag manifest.
- Emit shutdown receipt.
- On restart, replay ledger from last checkpoint and resume projection lag.

---

## 26. Configuration

```toml
[storage]
path = "~/.mnemos-omega"
canonical = "redb"
wal_segment_mb = 64
wal_zstd_level = 3
checkpoint_every_writes = 50000
git_cards = true

[projections]
bm25 = "tantivy"
vector = "hnsw"
graph = "csr"
bitmaps = "roaring"
context_pack_cache = true

[adapters]
qdrant = false
kuzu = false
sqlite = false
lancedb = false
external_llm = false
external_embeddings = false

[retrieval]
token_budget_default = 4096
k_per_channel = 64
graph_ppr_top_k = 32
deterministic_seed = 13
require_citations_by_default = true

[privacy]
default_class = "Internal"
deny_extraction_from = ["Secret", "Vault"]
redact_in_context_pack = true
allow_external_llm_for_secret = false

[strengthen]
fsrs_target_retrievability = 0.9
hebbian_lr = 0.01
topic_recompute_days = 7
topic_mastery_floor = 0.55

[policy]
manager = "rule"
learned_model_path = "policy/manager.safetensors"
fallback = "rule"
train_nightly = false

[formal]
unit_check = true
symbolic_check = true
lean_bin = "lean"
coq_bin = "coqc"
sympy_python = "python3"

[daemon]
grpc = "127.0.0.1:7777"
http = "127.0.0.1:7778"
mcp = "unix:/tmp/mnemos.sock"
auth_token_env = "MNEMOS_TOKEN"

[telemetry]
json_logs = true
trace_dir = "telemetry/traces"
metrics_interval_sec = 10
redact_labels = true
```

Config reload:

- SIGHUP reloads non-structural settings.
- Structural changes such as storage engine, schema version, or embedding dimension require migration plan.
- Every config reload writes a receipt with old/new config hashes.

---

## 27. Compression And Consolidation

Compression ladder:

```text
L0 raw source
  -> L1 episode
  -> L2 observation
  -> L3 atomic cell
  -> L4 concept kernel
  -> L5 topic capsule
  -> L6 field synthesis
```

Compression laws:

- Never compress away provenance.
- Never make a summary canonical over its sources.
- Never hide a counterexample inside a high-level synthesis.
- Every compressed cell links down to evidence.
- Every compressed cell declares omissions and known uncertainty.
- Rehydration from source spans must be possible unless access policy forbids it.

Consolidation jobs:

- observation synthesis from clustered episodes
- duplicate merge proposals
- belief synthesis and revision
- concept kernel refresh
- topic capsule generation
- skill distillation from repeated successes
- theorem dependency closure
- source audit and citation repair
- index compaction and projection rebuild
- cold archive tiering
- metacognitive lesson generation

---

## 28. Strengthening And Policy Learning

### 28.1 FSRS

Use FSRS-like state for durable knowledge and skills:

- `stability_days`
- `difficulty`
- `retrievability`
- `next_review_at`

Feedback updates:

- successful use raises stability and utility
- failed use lowers stability and raises difficulty
- ignored retrieval lowers retrieval-success EMA
- verified memory increases formal/source confidence
- falsified memory changes lifecycle and schedules belief revision

### 28.2 Topic Strengthening

```rust
pub struct TopicState {
    pub id: TopicId,
    pub label: String,
    pub member_entities: Vec<EntityId>,
    pub member_memories: Vec<MemoryId>,
    pub prerequisites: Vec<TopicId>,
    pub strength: f32,
    pub mastery: f32,
    pub half_life_hours: f32,
    pub retrieval_success: f32,
    pub contradiction_pressure: f32,
    pub source_coverage: f32,
    pub formal_coverage: f32,
    pub procedural_coverage: f32,
    pub last_strengthened_at: DateTime<Utc>,
    pub next_review_at: Option<DateTime<Utc>>,
}
```

Topic reinforcement:

```text
new_strength =
  decayed_previous
+ recurrence
+ downstream_utility
+ novelty
+ independent_sources
+ retrieval_success
+ skill_success
+ neighbor_strength
- unresolved_conflicts
- task_failures
```

### 28.3 Memory-R1 Guardrails

Learned policy may propose:

- `Add`
- `UpdateMetadata`
- `Link`
- `Supersede`
- `Merge`
- `Archive`
- `Noop`
- `Review`
- `Reject`

Validators always enforce:

- provenance required for trusted knowledge
- no destructive overwrite
- bitemporal validity rules
- privacy and vault policy
- sandbox rules for skills
- citation and source hash checks
- deterministic receipt generation

Training constraints:

- train offline from eval and feedback receipts
- fixed input data plus seed yields same model bytes
- policy model version is recorded in receipts
- fallback to rule policy on missing model, incompatible version, or validator disagreement

---

## 29. Science And Math Contract

Science and math are first-class, not generic text.

Required object separation:

| Object | Must preserve | Must not be confused with |
| --- | --- | --- |
| `ClaimBody` | source span, assumptions, scope, support/contradiction | current belief |
| `BeliefBody` | stance, rationale, evidence weights, revision history | source claim |
| `EquationBody` | TeX, normalized form, variables, units, regime, derivation | prose summary |
| `TheoremBody` | statement, dependencies, proof status, proof refs | equation |
| `DatasetBody` | version, schema, license, checksum, reproducibility | raw dataset dump |
| `ExperimentBody` | method, params, code/data hashes, outcome | claim |
| `CounterexampleBody` | falsified target, evidence, failure regime, severity | low-priority failure note |

Hard requirements:

- Equations with units must pass dimensional checks or remain `Unverified`.
- Equation queries must use exact BM25/symbol channels, not vector-only retrieval.
- Theorem changes trigger dependency closure over `used_by`.
- Dataset-backed claims require dataset version and checksum when available.
- Experiments require code/data/resource hashes.
- Beliefs require support and contradiction links.
- Formal proof absence is visible in context packs.
- Counterexamples matching entities/topics are aggressively surfaced.

Verification functions:

- `verify_claim`: checks provenance, source span hash, support/contradiction links, and citation availability.
- `verify_equation`: checks syntax, units, symbol normalization, assumptions, known failures, optional formal proof.
- `verify_skill`: checks sandbox, tests, dependencies, preconditions, and recent failure rate.

---

## 30. Privacy, Security, And Prompt Injection

Security invariants:

- Memory content is data, never instructions.
- Retrieved text cannot change system policy.
- Prompt-injection strings inside memories are rendered with data boundaries.
- Secrets and vault content are redacted before LLM exposure.
- External LLM/embedder calls require access-policy permission.
- Skill execution is sandboxed and receipt-visible.
- Network access is allowlisted per skill and daemon adapter.
- Ledger and receipt chains are verified on startup when configured.

Prompt-injection handling:

- Classify untrusted source content as quoted data.
- Strip or neutralize instruction-like text from pack control fields.
- Put raw source snippets only in item render fields with citations.
- Add `ItemWarning::UntrustedInstructionLikeContent`.
- Adversarial tests assert that pack structure and system behavior do not change.

Privacy failure modes:

- secret in source blob
- sensitive metadata in embedding text
- unauthorized context-pack inclusion
- external LLM extraction on confidential material
- vault path leak in Git card
- skill exfiltration attempt

Each failure mode needs a regression fixture.

---

## 31. Rust Workspace Tree

```text
mnemos-omega/
  Cargo.toml
  rust-toolchain.toml
  config/
    default.toml
    retrieval_profiles.toml
    fsrs.toml
    topic.toml
    predicates.yaml
    privacy_policy.toml
  crates/
    mnemos-types/
      src/{ids.rs,time.rs,lanes.rs,lifecycle.rs,provenance.rs,access.rs,scores.rs,links.rs,memory_cell.rs,payloads.rs,context_pack.rs,receipts.rs}
    mnemos-ledger/
      src/{append_log.rs,segments.rs,hash_chain.rs,wal.rs,receipt.rs,replay.rs,verify.rs,checkpoint.rs}
    mnemos-store/
      src/{canonical.rs,redb_store.rs,fjall_store.rs,cas.rs,cards.rs,migrations.rs,snapshot.rs}
    mnemos-index-bm25/
      src/{tantivy.rs,schema.rs,symbols.rs,rebuild.rs}
    mnemos-index-vector/
      src/{hnsw.rs,quantized.rs,qdrant.rs,embedding_cache.rs,rebuild.rs}
    mnemos-index-graph/
      src/{entity.rs,temporal_edge.rs,csr.rs,delta.rs,ppr.rs,kuzu.rs,rebuild.rs}
    mnemos-index-bitmap/
      src/{roaring.rs,filters.rs,rebuild.rs}
    mnemos-observe/
      src/{event.rs,sensory_ring.rs,segmenter.rs,observer.rs,privacy_filter.rs}
    mnemos-extract/
      src/{claim.rs,equation.rs,theorem.rs,skill.rs,resource.rs,entity_linker.rs,validators.rs}
    mnemos-write/
      src/{pipeline.rs,router.rs,normalizer.rs,grounding.rs,policy_gate.rs,committer.rs}
    mnemos-recall/
      src/{query.rs,planner.rs,fanout.rs,rrf.rs,rerank.rs,filters.rs,context_pack.rs,trace.rs}
    mnemos-cognition/
      src/{scheduler.rs,consolidation.rs,dedupe.rs,belief_revision.rs,concept_kernel.rs,topic_strength.rs,fsrs.rs,archive.rs}
    mnemos-formal/
      src/{units.rs,symbolic.rs,theorem_dag.rs,lean.rs,coq.rs,dataset.rs,experiment.rs}
    mnemos-skill/
      src/{sandbox.rs,runner.rs,test_harness.rs,registry.rs,repair.rs,dependency.rs}
    mnemos-policy/
      src/{rule_policy.rs,features.rs,learned.rs,rewards.rs,attribution.rs,trainer.rs}
    mnemos-api/
      src/{engine.rs,config.rs,errors.rs,tools.rs}
    mnemos-daemon/
      src/{main.rs,grpc.rs,http.rs,mcp.rs,auth.rs,telemetry.rs,shutdown.rs}
    mnemos-cli/
      src/{main.rs,observe.rs,recall.rs,verify.rs,rebuild.rs,snapshot.rs,bench.rs}
    mnemos-eval/
      src/{fixtures.rs,longmemeval.rs,memory_agent_bench.rs,locomo.rs,science.rs,code_skill.rs,privacy.rs,rebuild.rs,metrics.rs}
  tests/
    schema_roundtrip.rs
    ledger_replay.rs
    temporal_queries.rs
    contradiction_policy.rs
    citation_required.rs
    privacy_redaction.rs
    context_pack_determinism.rs
    rebuild_indexes.rs
    crash_replay.rs
    skill_promotion.rs
    theorem_dependency.rs
  benches/
    observe_ack.rs
    recall_1m.rs
    recall_10m.rs
    ppr_graph.rs
    context_pack.rs
    rebuild_indexes.rs
  docs/
    architecture.md
    schemas.md
    operations.md
    evals.md
```

---

## 32. Dependencies

Preferred crates:

- core: `serde`, `serde_json`, `schemars`, `uuid`, `time` or `chrono`, `thiserror`, `anyhow`
- runtime: `tokio`, `rayon`, `tracing`, `tracing-subscriber`
- hash/log/compression: `blake3`, `zstd`, `crc32fast`, `postcard` or `bincode`
- storage: `redb`, optional `fjall`, `gix`
- search/index: `tantivy`, `hnsw_rs`, `roaring`, `fst`, `memmap2`
- embeddings: `candle-core`, `candle-transformers`, `tokenizers`
- graph/numerics: `petgraph`, `ndarray`, `wide`
- crypto: `ed25519-dalek`, optional `age`
- API: `tonic`, `axum`, MCP schema crate chosen at implementation
- CLI: `clap`
- tests: `proptest`, `criterion`, `insta`, `tempfile`

Do not pin exact versions in this spec unless implementation begins and repository lockfile policy is known.

---

## 33. Testing And Evaluation Suite

Unit/property tests:

- schema roundtrips preserve `schema_version`
- `MemoryCell` invariants by lane and lifecycle
- bitemporal interval logic
- provenance span hash validation
- access-policy redaction
- receipt hash-chain verification
- supersession and falsification idempotency
- context-pack budget and determinism
- no rejected/quarantined memory in normal recall
- counterexamples eligible under entity/topic overlap

Integration tests:

- observe -> extract -> commit -> index -> recall
- contradictory science claims create contested belief, not deletion
- `recall_at` returns historical world truth
- `recall_as_of` returns historical agent knowledge
- math query retrieves equation, assumptions, units, theorem dependencies
- debug query retrieves exact error, prior failed attempts, and skill
- vault query redacts unauthorized content
- skill promotion only after sandbox success
- theorem dependency downgrade propagates
- corrupted projection rebuilds from ledger

External and custom benchmarks:

| Benchmark | Purpose | Gate |
| --- | --- | --- |
| LongMemEval | long-term interactive memory, updates, temporal reasoning, abstention | report F1/accuracy by category |
| MemoryAgentBench | retrieval, test-time learning, long-range understanding, selective forgetting | report task-level metrics |
| LoCoMo | long conversation memory and temporal/personal consistency | report overall and temporal scores |
| Math/Science Memory Bench | equations, units, theorem DAG, citations, experiments, contradictions | equation hit@k, unit mismatch detection, citation coverage |
| CodeSkill Bench | procedural recall, skill execution, failure recovery, sandbox safety | skill success rate and false promotion count |
| Privacy/Injection Bench | prompt injection, vault leakage, redaction, external LLM policy | zero successful leaks/injections |
| Deterministic Rebuild Bench | ledger replay, projection rebuild, context-pack cache consistency | matching hashes/logical dumps |
| Scale Bench | 1M, 10M, 50M synthetic cell corpora | latency, memory, rebuild throughput |

Benchmark claims require committed artifacts:

- config hash
- dataset or fixture hash
- model/provider versions
- hardware summary
- command line
- result JSON
- failure interpretation

---

## 34. Chaos Tests

Required chaos scenarios:

- kill daemon during WAL append
- kill daemon after ledger append before redb commit
- kill daemon after redb commit before projection update
- corrupt Tantivy segment
- corrupt HNSW file
- delete graph CSR snapshot
- truncate context-pack cache entry
- inject duplicate flood
- inject contradiction storm
- inject prompt-injection source
- inject secret into source blob
- run skill with forbidden filesystem write
- run skill with forbidden network exfiltration
- change embedding model dimension
- migrate schema with old ledger segment

Pass criteria:

- canonical ledger remains verifiable or fails closed
- replay produces deterministic canonical state
- corrupted projections are detected and rebuildable
- privacy failures are blocked and receipt-visible
- skill sandbox denies unsafe behavior

---

## 35. Performance Targets

Targets for local NVMe developer machine; not claims until benchmarked:

| Operation | p50 | p95 | Notes |
| --- | ---: | ---: | --- |
| `observe(event)` hot receipt | <= 5 ms | <= 25 ms | no LLM in hot path |
| `remember(candidate)` grounded commit | <= 20 ms | <= 100 ms | excluding extraction |
| `recall(simple fact)` | <= 35 ms | <= 180 ms | hot indexes |
| `recall(debug exact error)` | <= 45 ms | <= 220 ms | BM25/eval heavy |
| `recall(math/science multi-hop)` | <= 150 ms | <= 750 ms | graph PPR bounded |
| `ContextPack` compile after candidates | <= 25 ms | <= 100 ms | 4k-8k budget |
| `verify_store` startup quick check | <= 2 s | <= 5 s | checkpoint based |
| full projection rebuild at 10M | target <= 30 min | target <= 2 h | hardware dependent |

Scale tiers:

- hot: 1M-3M active cells
- warm: 10M cells
- cold: 50M+ source-backed cells
- archive: 100M+ source spans or external artifacts

50M+ latency remains empirical risk until measured.

---

## 36. Migration

Migration principles:

- ledger segments are immutable
- schema changes are append-only migrations with receipts
- old cells remain readable through versioned deserializers
- projection rebuild follows migration
- snapshots include schema and config hashes
- migration can dry-run and emit a plan

Migration commands:

```text
mnemos migrate plan --from <version> --to <version>
mnemos migrate dry-run --out migration-report.json
mnemos migrate apply --receipt migration-receipt.json
mnemos verify-store --after-migration
mnemos rebuild-indexes --all --check-deterministic
```

Failure handling:

- if cell migration fails, quarantine new version and preserve old cell
- if projection migration fails, delete projection and rebuild
- if ledger verification fails, stop and require operator decision
- if access-policy migration weakens privacy, fail closed

---

## 37. Roadmap

Phase 0: spec and fixtures

- finalize schemas
- create fixture corpus with contradictions, equations, skills, privacy cases
- create eval harness skeleton

Phase 1: ledger and canonical store

- `mnemos-types`
- `mnemos-ledger`
- `mnemos-store`
- receipts and replay
- schema roundtrip/property tests

Phase 2: local retrieval

- Tantivy BM25
- symbol FST
- local HNSW
- roaring filters
- basic graph adjacency
- deterministic ContextPack compiler

Phase 3: write pipeline

- observe
- candidate validation
- provenance grounding
- duplicate/contradiction detection
- policy fallback
- projection fanout

Phase 4: science and skills

- equations, units, theorem DAG
- datasets and experiments
- skill sandbox, tests, promotion/degradation
- `verify_claim`, `verify_equation`, `verify_skill`

Phase 5: cognition daemons

- consolidation
- concept kernels
- belief revision
- FSRS/topic strengthening
- eval feedback
- metacognitive lessons

Phase 6: operations and scale

- daemon, CLI, MCP
- telemetry
- snapshots/export/import
- deterministic rebuild
- chaos tests
- LongMemEval/MemoryAgentBench/LoCoMo/custom benches

---

## 38. Parallel Work Packets

Use parallel work only when write scopes are disjoint.

| Packet | Owner scope | Files |
| --- | --- | --- |
| A: schemas | IDs, lanes, time, provenance, payloads, receipts, context packs | `crates/mnemos-types/**`, schema tests |
| B: ledger/store | WAL, hash chain, redb, CAS, replay, snapshots | `crates/mnemos-ledger/**`, `crates/mnemos-store/**` |
| C: indexes | BM25, vectors, graph, bitmaps, rebuild manifests | `crates/mnemos-index-*/**` |
| D: write/read | observe, policy gate, recall planner, context compiler, traces | `crates/mnemos-write/**`, `crates/mnemos-recall/**` |
| E: cognition | consolidation, belief revision, FSRS, topics, archive | `crates/mnemos-cognition/**` |
| F: formal/skills | units, theorem DAG, datasets, experiments, sandbox, repair | `crates/mnemos-formal/**`, `crates/mnemos-skill/**` |
| G: interfaces/eval | API, daemon, CLI, MCP, benchmarks | `crates/mnemos-api/**`, `crates/mnemos-daemon/**`, `crates/mnemos-cli/**`, `crates/mnemos-eval/**` |

Conflict risks:

- `mnemos-types` blocks all packets; stabilize first.
- public API trait changes require coordination.
- migration and schema changes cannot be parallelized casually.
- retrieval profile weights affect eval baselines.
- access-policy code is security-critical and must not be forked.

---

## 39. Validation Commands

For this doc-only V3 change:

```bash
rtk git diff -- tips/smart_memory/CODEX_MEMSPEC_V3.md
rtk jankurai proof . --changed tips/smart_memory/CODEX_MEMSPEC_V3.md --out target/jankurai/proof-v3.json --md target/jankurai/proof-v3.md
rtk just fast
```

Expected artifacts:

- `tips/smart_memory/CODEX_MEMSPEC_V3.md`
- `target/jankurai/proof-v3.json`
- `target/jankurai/proof-v3.md`

Failure interpretation:

- Jankurai unmapped-path findings for `tips/smart_memory/**` indicate repo metadata does not track these concept docs; report without editing maps.
- `just fast` failures outside the new doc scope should be reported with command and first relevant failure.
- If `just fast` changes unrelated tracked files, report drift and do not revert it.

Future implementation validation:

```bash
rtk cargo fmt --all --check
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo test --workspace
rtk cargo test --workspace --all-features
rtk cargo bench --workspace --no-run
rtk mnemos verify-store
rtk mnemos rebuild-indexes --all --check-deterministic
rtk mnemos bench longmemeval
rtk mnemos bench memory-agent
rtk mnemos bench locomo
rtk mnemos bench science
rtk mnemos bench code-skill
rtk mnemos bench privacy
rtk mnemos bench rebuild
```

---

## 40. Logging, Receipts, And Handoff

Implementation logs must include:

- files read
- files changed
- commands run
- validation results
- generated artifacts
- unrelated workspace drift
- unresolved failures
- performance claims with artifact paths or explicit "not measured"

Runtime receipts must include:

- receipt ID
- actor/tool identity
- policy version
- previous and new ledger hash
- previous and new receipt hash
- source hashes and span hashes
- cells created/updated/superseded/falsified/archived
- projection updates or lag
- validators passed/failed
- redactions and omissions
- timestamp and signature when enabled

Final handoff for an implementation phase must include:

- summary of crates and APIs implemented
- exact commands and outcomes
- known limitations
- migration/rebuild instructions
- safety note for vault/privacy handling
- benchmark artifacts or statement that benchmarks were not run
- ranked follow-up tasks by risk

---

## 41. Final Design Statement

MNEMOS-Omega V3 is a Rust-first, append-only, provenance-backed cognitive memory compiler. Its durable unit is `MemoryCell`; its truth source is the evidence ledger; its indexes are rebuildable projections; its cognition is organized into 12 durable lanes plus runtime overlays; and its primary output is a deterministic, cited `ContextPack` that lets agents act with compressed, inspectable, temporally correct memory instead of raw context-window accumulation.
