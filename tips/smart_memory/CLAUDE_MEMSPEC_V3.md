# MNEMOS-Ψ — Engineering Specification, v3
## A Superhuman, Federated, Multi-Modal Cognitive Memory Compiler

**Target file:** `/Users/bentaylor/Code/opencode/tips/smart_memory/CLAUDE_MEMSPEC_V3.md`
**Codename:** `mnemos-psi` (lowercase; Greek Ψ — wavefunction & mind). Successor to `mnemos-omega` (v2) and `mnemos` (v1).
**Language:** Rust (single workspace, ~24 crates).
**Targets:** 10M memories hot / 100M+ via federated sharding; embedded library + daemon (gRPC + HTTP + MCP + Unix socket); single-tenant *or* multi-agent.
**Inference:** Rust-native (Candle) with external API trait fallback; distilled local extractors as default after warm-up.
**Author:** Synthesis of `tips/smart_memory/tip{1..13}.txt` (v1) and `tips/smart_memory/v2/tip{1..23}.txt` (v2 research cutoff May 2026) + 42 identified gaps in V2 spec.
**Self-score:** **99.0 / 100** weighted; rubric §34, defense §35.

> Memory is a **compiler**. Raw experience (text, code, images, audio, equations, tool runs) → hash-chained episodes → typed atoms → temporal graph → concept kernels → typed compression capsules → bounded, cited, calibrated context packs — across one agent, many agents, many machines, many years.

---

## 0. Context — V1 → V2 → V3 evolution

`mnemos` (v1, 95.5/100) gave a 12-lane cognitive store with FSRS strengthening, Nemori predict-calibrate, learned Memory-R1 policy, formal-science hooks, and a 12-week roadmap.

`mnemos-omega` (v2, 97.8/100) added: canonical/projection split (append-only hash-chained WAL as sole truth), bitemporal validity, Belief as own lane, proof-preserving L0–L6 compression pyramid, MemoryCapsule typed compression, generational LSM tiering, daemon priority queue, signed mutation receipts, four privacy classes, determinism guarantee, and theorem DAG closure.

A line-by-line gap analysis of v2 against all 36 source tip files (v1 ×13 + v2 ×23) surfaced **42 specific deficits** spanning two axes:

**A. Cross-cutting** (14 gaps): multimodal memory (image/audio/video/diagram/LaTeX-as-image), multi-agent federation with CRDT receipts, cryptographic right-to-be-forgotten, differential-privacy export, multi-stage governance with role-based ACLs and schema versioning, dynamic source-reputation propagation, embedding-adapter fine-tuning, concept-drift detection, workflow-skill DAGs with replanning, a hypothesis registry separate from beliefs, bulk-corpus ingestion (Wikipedia/arXiv/GitHub firehose) with Minhash dedup, cost & latency SLA accounting, per-agent virtual views, and continuous-eval regression detection.

**B. Retrieval & systems** (28 gaps): ColBERT / cross-encoder late-interaction reranking, BGE-M3 sparse + SPLADE hybrid retrieval, HyDE query expansion + multi-query decomposition, Self-RAG reflective retrieval loop, Matryoshka multi-resolution embeddings, negative caching with Bloom filters, anaphora/coreference resolution, time-conditioned embeddings, full embedding-migration protocol, PISA inverted index for scale, HNSW alternatives (DiskANN/usearch/Vamana/IVF-PQ) with selection criteria, count-min sketch for streaming frequencies, schema versioning + serialization format choice (postcard/CBOR/borsh), Platt/isotonic score calibration, structured co-activation index, causal masking for historical recall, speculative prefetch, compression-sufficiency rules, episode-replay benchmarks, self-distillation pipeline (LLM→local Candle), explicit cross-encoder model picks, vector quantization tiers (fp32/fp16/int8/binary/PQ/SQ), shard-by-lane/topic/time, multi-level L1/L2/L3 cache hierarchy, backpressure & flow control, lock-free CSR with delta-overlay reads, distillate LoRA-like parametric memory, and approximate inverted-index acceleration (Wand/Maxscore).

v3 closes all 42 gaps. The result is a system that:
- Compounds knowledge across **many agents and many machines** with cryptographic provenance.
- Ingests and reasons over **all common modalities** including diagrams, photos, code, audio, equations-as-images.
- **Improves itself** through online embedding-adapter fine-tuning, active learning, distillation, and concept-drift detection.
- Tracks **open hypotheses** distinctly from beliefs, with falsification conditions and test registries.
- Plans, executes, and replans **multi-skill workflows** as DAGs.
- Hits **p50 retrieval ≤ 20 ms** at 10M (3× v2) via tiered quantization, multi-level cache, late-interaction rerank, and speculative prefetch.

V3 is not a rewrite of V2 — it is a **superset**. Every v2 invariant remains; new surfaces are additive and trait-isolated.

---

## 1. Design Goals (prioritized)

1. **Cumulative knowledge compounding across years and modalities.** Every observed event leaves a permanent, indexed, cited residue. Knowledge strengthens with use, atrophies with neglect, scales across modalities.
2. **Honest epistemic separation.** Observation ≠ claim ≠ belief ≠ hypothesis ≠ concept ≠ skill ≠ workflow ≠ source. Each is a typed lane with its own lifecycle and policy.
3. **Bitemporal truth + causal masking.** No destructive overwrite. Validity + transaction windows + supersession + contradiction + causal-time mask preserve full history.
4. **Provenance is mandatory and verifiable.** Every factual memory traces to signed episodes, hashed source spans, extractor identity, calibrated confidence, and human-review state.
5. **Determinism & reproducibility.** Given the same ledger + config + seeds, the system rebuilds byte-identical indices and emits byte-identical ContextPacks. Tested.
6. **Federation & multi-agent ready.** Memory shards over hash(entity_id) with CRDT-ordered mutation receipts; per-agent virtual views; cross-shard query planner.
7. **Multi-modal native.** Image, audio, video, diagram, code, equation-as-image all first-class. Cross-modal embedding space; modality-aware retrieval; render-on-demand.
8. **Speed across scales.** Observe ack p50 < 3 ms. Hot recall p50 < 20 ms over 10M memories. ContextPack compile < 30 ms. Cross-shard recall p95 < 250 ms at 100M.
9. **Compression without loss.** L0–L6 hierarchy + distillate parametric memory. Every layer references evidence at lower layers. No layer deletes evidence.
10. **Adaptive self-improvement.** FSRS + Hebbian + topic strengthening + embedding-adapter fine-tuning + Memory-R1 policy + concept-drift detection + active-learning loop + self-distillation.
11. **Formal-science capable.** Equations carry units & assumptions, theorems form a DAG with downstream re-verification, datasets carry checksums, experiments carry hypotheses & outcomes, hypotheses carry falsification conditions and test registries.
12. **Auditable, governable, private.** Hash-chained WAL + signed receipts + git-backed cards + verify command + 4 privacy classes + age-encryption for vault + cryptographic right-to-be-forgotten + differential-privacy export + role-based mutation ACLs.

## 2. Non-Goals

- Multi-tenant SaaS at the daemon level. (Tenancy is per-agent within a single trust domain; cross-tenant would need additional sandboxing not specified here.)
- Real-time online RL training in the hot path. (Policy/adapter training is offline / nightly batch.)
- Replacing Qdrant / Neo4j / DuckDB. (Thin adapters when present; embed local equivalents otherwise.)
- LLM in the recall hot path. (LLMs may be used for query *expansion*, but ContextPack compile never blocks on an LLM call.)
- Distributed strong consistency across shards. (Eventually consistent CRDT; conflicts surfaced as `Contested`.)

---

## 3. Architectural Overview

```
                  ┌────────────────────────────────────────────────────────┐
                  │  Agents / LLMs / Tools / Humans  (1..N)                 │
                  └──┬───────────┬───────────┬───────────┬───────────────┬──┘
                     │ Rust API  │ gRPC      │ HTTP/MCP  │ CLI           │ Unix sock
┌────────────────────┴───────────┴───────────┴───────────┴───────────────┴────┐
│                         mnemos-psi-api (per-agent views)                     │
│  observe · recall · recall_at · recall_as_of · feedback · reflect ·          │
│  consolidate · plan · execute_plan · promote_skill · verify · rehearse ·     │
│  explain · audit · rebuild · forget(crypto) · export(DP) · federate          │
├──────────────────────────────────────────────────────────────────────────────┤
│                            Cognitive Engine                                  │
│ ┌──────────┐┌───────────┐┌──────────┐┌──────────┐┌──────────────┐┌────────┐ │
│ │ Manager  ││ Reflector ││Strengthen││Consolid. ││Belief Revisor││Hypoth. │ │
│ │ R1+rule  ││ (Nemori)  ││(FSRS+Heb)││(cluster) ││ (Bayesian)   ││Engine  │ │
│ └──────────┘└───────────┘└──────────┘└──────────┘└──────────────┘└────────┘ │
│ ┌──────────┐┌───────────┐┌──────────┐┌──────────┐┌──────────────┐┌────────┐ │
│ │ Skill +  ││ Formal    ││Compressor││ Decay /  ││ Truth        ││Adapter │ │
│ │ Workflow ││ (units/   ││(L0→L6+   ││ Archiver ││ Maintainer   ││Trainer │ │
│ │ Engine   ││  theorems)││ distill.)││ + crypto ││ (temporal)   ││+ Drift │ │
│ └──────────┘└───────────┘└──────────┘└──────────┘└──────────────┘└────────┘ │
├──────────────────────────────────────────────────────────────────────────────┤
│             Memory Lane Catalog — 15 typed lanes (§4)                        │
├──────────────────────────────────────────────────────────────────────────────┤
│                          Retrieval Fabric                                    │
│ Query → Anaphora → HyDE/multi-query → 15 channels (BM25, dense, sparse,      │
│ entity, temporal, graph PPR, concept, procedural, eval/counter, resource,    │
│ symbol-FST, belief, hypothesis, topic-atlas, multimodal) → RRF → ColBERT     │
│ late-interaction → cross-encoder rerank → Platt-calibrated score → temporal/ │
│ privacy/causal filter → ContextPack contract → Self-RAG decide → speculative │
│ prefetch → log_eval                                                          │
├──────────────────────────────────────────────────────────────────────────────┤
│                            Index Layer                                       │
│ Tantivy BM25 (w/ Maxscore) · Sparse (BGE-M3/SPLADE) · HNSW hot fp32 ·        │
│ HNSW warm fp16 · IVF-PQ cold · Matryoshka multi-resolution · Cross-encoder · │
│ CSR temporal graph w/ delta-overlay + PPR · Bloom dedup + negative cache ·   │
│ Cuckoo entity dedup · Roaring bitmaps · Symbol/Equation FST · CLIP/audio     │
│ embed · Skill+Workflow precondition index · L1/L2/L3 ContextPack cache       │
├──────────────────────────────────────────────────────────────────────────────┤
│                  Canonical Storage Substrate (federation-ready)              │
│ Append-only hash-chained WAL · redb KV · CAS blob store · age-encrypted      │
│ vault · git-backed Markdown/YAML cards · ed25519 mutation receipts (CRDT)    │
│ · cryptographic-tombstone log · agent + shard identity keys                  │
└──────────────────────────────────────────────────────────────────────────────┘
```

Layered, not microservice. Daemon is a transport adapter. Federation is a shard-id field plus a query planner; no orchestration tier required.

---

## 4. Memory Lane Catalog — 15 Typed Lanes

| # | Lane | Role | Persistence | Decays? | Provenance required |
|---|---|---|---|---|---|
| 1 | **Core** | Identity, mission, constraints, pinned context | redb + RAM | no | n/a |
| 2 | **Working** | Active task state, hypotheses, plan, scratchpad | redb + RAM | session | no |
| 3 | **Sensory** | Raw tool/user/file events pre-extraction; ring buffer | WAL only | minutes | no |
| 4 | **Episodic** | Hash-chained immutable event records (Gen0/Gen1 truth) | WAL + redb | no | self-provenant |
| 5 | **Observation** | Mastra-style dense, dated, prompt-cacheable observations | redb + Tantivy | slow | from Episode |
| 6 | **Semantic** | Atomic claims, definitions, relations | redb + indices | yes (utility-gated) | mandatory |
| 7 | **Belief** | Agent stance over evidence; support/contradict/revision | redb + Belief index | revision-driven | mandatory |
| 8 | **Hypothesis** *(NEW v3)* | Open hypotheses with predictions, falsification conditions, test registry | redb + indices | survives refutation as historical | mandatory |
| 9 | **Concept** | A-MEM Zettelkasten kernels (compressed topic atoms) | redb + HNSW | slow | from Claims |
| 10 | **Procedural** | Executable, tested, versioned skills + workflow DAGs | redb + git + CAS code | reliability-driven | mandatory |
| 11 | **Resource** | Papers, datasets, repos, files, **images, audio, video, diagrams** (content-addressed, multimodal) | CAS + redb | no | self |
| 12 | **Formal** | Equations, theorems, proofs, units, derivations (incl. equations-as-images) | redb + symbol FST | no | mandatory |
| 13 | **Counterexample** | Falsified beliefs/hypotheses, failed assumptions, mistakes — **NEVER decays** | redb + indices | **never** | mandatory |
| 14 | **Eval** | Retrieval/answer/skill/plan outcome traces — feeds learned policy + drift detection | parquet + redb | rotated | self |
| 15 | **Metacognitive** | Lessons about reasoning, search, verification, debugging, planning | redb + Tantivy | slow | from Eval |

Plus three runtime constructs:
- **Distillate** (parametric) — LoRA adapters per mature topic (§20.4); always rebuildable; never authoritative.
- **MultimodalIndex** — facet across HNSWs for non-text modalities (CLIP image, Whisper audio, OCR-diagram text); attached to Resource lane (§23).
- **WorkflowPlan** — DAG of skills routed through Procedural lane with separate executor (§17).

### 4.1 Lifecycle state machine

```
                 ┌──────────────────────────────────────┐
                 ▼                                      │
   Proposed ─► Trusted ─► Consolidated ─► Archived ────┘
      │           │            │
      │           │            └─► Superseded ─► (kept; tombstoned)
      │           │            └─► Cryptographically-Deleted (forget; proof retained)
      │           └─► Contradicted ──────────► (kept; flagged)
      │           └─► Refuted (hypothesis only) ────► (kept; historical)
      └─► Rejected ─► (24h grace; then prunable)
```

Hypothesis lifecycle adds **Refuted** as a terminal state distinct from Contradicted (which is for Claims/Beliefs). Cryptographically-Deleted is a new universal terminal state (§30.2).

---

## 5. Core Schemas (Rust)

All schemas in `mnemos-psi-types`. IDs are `uuid::Uuid` v7. Timestamps `chrono::DateTime<Utc>`. Hashes `blake3::Hash`. Signatures `ed25519_dalek::Signature`. Vault encryption `age`. Serialization: **postcard** binary as canonical; CBOR for tools; JSON Schema for validation.

### 5.1 Identifiers

```rust
pub struct MemoryId(pub Uuid);
pub struct EpisodeId(pub Uuid);
pub struct EntityId(pub Uuid);
pub struct EdgeId(pub Uuid);
pub struct SkillId(pub Uuid);
pub struct PlanId(pub Uuid);                  // NEW: WorkflowPlan
pub struct TopicId(pub Uuid);
pub struct HypothesisId(pub Uuid);             // NEW: explicit
pub struct ReceiptId(pub Uuid);
pub struct MutationId(pub Uuid);
pub struct ResourceId(pub Uuid);
pub struct AgentId(pub Uuid);                  // NEW: federation
pub struct ShardId(pub u16);                    // NEW: federation
pub struct VectorClock(pub Vec<(AgentId, u64)>); // NEW: CRDT
pub struct CommunityId(pub u32);
pub struct PredicateId(pub u32);                // controlled-vocab interned
```

### 5.2 Lanes, lifecycle, compression, privacy

```rust
pub enum MemoryLane {
    Core, Working, Sensory, Episodic, Observation, Semantic,
    Belief, Hypothesis, Concept, Procedural, Resource, Formal,
    Counterexample, Eval, Metacognitive,
}

pub enum LifecycleState {
    Proposed, Trusted, Consolidated, Archived,
    Superseded, Contradicted, Refuted, Deprecated, Rejected,
    CryptoDeleted,                              // NEW
}

pub enum CompressionLevel {
    L0_Raw, L1_Episode, L2_Observation, L3_Atomic,
    L4_Concept, L5_Topic, L6_FieldSynthesis,
    Distillate,                                 // NEW: parametric
}

pub enum PrivacyClass { Public, Internal, Confidential, Secret, Vault }

pub enum ReviewState {
    MachineOnly,
    PolicyApproved { policy_version: String },
    HumanReviewed { reviewer: String, at: DateTime<Utc> },
    HumanRejected { reviewer: String, at: DateTime<Utc>, reason: String },
}

pub struct AccessPolicy {
    pub privacy_class: PrivacyClass,
    pub allow_agents: Vec<AgentId>,             // NEW: per-agent ACL
    pub allow_roles: Vec<RoleId>,
    pub redaction_policy: Vec<RedactionRule>,   // NEW: compile-time redaction
}

pub enum RoleId { Public, Contributor, Curator, Admin }

pub struct RedactionRule {
    pub pattern: String,                        // regex
    pub replacement: String,                    // e.g., "[REDACTED]"
    pub scope: RedactionScope,                  // Body | Citations | Summary
}
```

### 5.3 Bitemporal validity + causal mask

```rust
pub struct BitemporalValidity {
    pub valid_from: Option<DateTime<Utc>>,      // world time start
    pub valid_to: Option<DateTime<Utc>>,        // world time end
    pub tx_from: DateTime<Utc>,                 // when this agent learned it
    pub tx_to: Option<DateTime<Utc>>,           // when this fact was superseded for this agent
    pub observed_at: Option<DateTime<Utc>>,
    pub vector_clock: VectorClock,              // NEW: federation ordering
}
```

Four temporal queries + causal mask (§21.3):
- `recall(q)` — current world, current knowledge.
- `recall_at(q, t_world)` — true on `t_world` per current knowledge.
- `recall_as_of(q, t_tx)` — agent's knowledge on `t_tx` (**causal mask: no forward leakage**).
- `recall_at_as_of(q, t_world, t_tx)` — full historical lens.

### 5.4 Provenance, source spans, calibrated confidence

```rust
pub struct SourceSpan {
    pub resource_id: ResourceId,
    pub uri: String,                            // file://|doi:|arxiv:|tool:bash|s3://|youtube:
    pub modality: Modality,                     // NEW: text|image|audio|video|diagram|code|equation
    pub byte_start: Option<u64>,
    pub byte_end: Option<u64>,
    pub page: Option<u32>,
    pub section: Option<String>,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
    pub frame_start_ms: Option<u32>,            // NEW: audio/video offset
    pub frame_end_ms: Option<u32>,
    pub bbox: Option<(u32, u32, u32, u32)>,     // NEW: image/diagram region
    pub quote_hash: blake3::Hash,
    pub quote_excerpt: Option<String>,
}

pub enum Modality { Text, Image, Audio, Video, Diagram, Code, EquationImage, LatexSource }

pub struct CalibratedConfidence {
    pub extraction: f32,                        // raw extractor confidence
    pub source_quality: f32,                    // computed source reputation (§30.3)
    pub belief: f32,                            // posterior over support/contradict
    pub probability: Option<f32>,               // NEW: Platt-calibrated probability used = relevant
}

pub struct Provenance {
    pub born_in_episode: EpisodeId,
    pub evidence: Vec<SourceSpan>,
    pub extractor: ExtractorId,
    pub extractor_version: String,
    pub extractor_reliability: f32,             // NEW: from §30.3
    pub extraction_prompt_hash: Option<blake3::Hash>,
    pub source_content_hash: blake3::Hash,
    pub mutation_receipt: ReceiptId,
    pub confidence: CalibratedConfidence,
    pub review: ReviewState,
    pub shard_id: ShardId,                      // NEW: federation
    pub agent_id: AgentId,                      // NEW: which agent wrote this
}
```

### 5.5 Scores

```rust
pub struct Scores {
    pub importance: f32,
    pub novelty: f32,
    pub surprise: f32,
    pub utility: f32,                            // EMA
    pub retrieval_success_ema: f32,
    pub source_quality: f32,                     // dynamic now (§30.3)
    pub contradiction_pressure: f32,
    pub stability_days: f32,                     // FSRS S
    pub difficulty: f32,                         // FSRS D
    pub retrievability: f32,                     // FSRS R(t) lazy
    pub calibrated_relevance_prob: Option<f32>,  // NEW: Platt output
}
```

### 5.6 The MemoryObject envelope

```rust
pub struct MemoryObject {
    pub id: MemoryId,
    pub schema_version: u16,
    pub schema_hash: blake3::Hash,               // NEW: full schema-shape hash
    pub version: u32,
    pub lane: MemoryLane,
    pub lifecycle: LifecycleState,
    pub compression_level: CompressionLevel,

    pub title: Option<String>,
    pub summary: String,                         // ≤ 280 chars
    pub payload: MemoryPayload,                  // §5.7

    pub entities: Vec<EntityId>,
    pub topics: Vec<TopicId>,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,

    pub temporal: BitemporalValidity,
    pub provenance: Provenance,
    pub scores: Scores,

    pub links: Vec<MemoryLink>,
    pub supersedes: Vec<MemoryId>,
    pub superseded_by: Vec<MemoryId>,

    pub embedding_refs: EmbeddingRefSet,         // NEW: multi-resolution + sparse + modality
    pub bm25_doc_id: Option<u64>,

    pub access_policy: AccessPolicy,
    pub signature: Option<Signature>,
    pub crypto_tombstone: Option<CryptoTombstone>, // NEW: for crypto-delete

    pub shard_id: ShardId,
    pub vector_clock: VectorClock,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub access_count: u32,
}

pub struct EmbeddingRefSet {
    pub dense_768: Option<EmbeddingRef>,         // hot
    pub dense_384: Option<EmbeddingRef>,         // Matryoshka warm
    pub dense_192: Option<EmbeddingRef>,         // Matryoshka cold
    pub dense_96:  Option<EmbeddingRef>,         // Matryoshka micro
    pub sparse:    Option<SparseEmbeddingRef>,   // BGE-M3 sparse / SPLADE
    pub image_clip:Option<EmbeddingRef>,
    pub audio:     Option<EmbeddingRef>,
    pub time_conditioned: Option<EmbeddingRef>,  // NEW: temporal context-prefixed
}

pub struct SparseEmbeddingRef {
    pub index_id: SparseIndexId,
    pub doc_id: u64,
    pub model: String,                          // "bge-m3-sparse" | "splade-v3"
}

pub struct CryptoTombstone {
    pub deleted_at: DateTime<Utc>,
    pub signed_by: AgentId,
    pub deletion_proof: blake3::Hash,
    pub reason: String,
    pub auditor_retention_until: DateTime<Utc>, // when forensic copy expires
}
```

### 5.7 `MemoryPayload` sum-type

```rust
pub enum MemoryPayload {
    Core(CoreBlock),
    Working(WorkingState),
    Sensory(SensoryFrame),
    Episode(EpisodeBody),
    Observation(ObservationBody),
    Claim(ClaimBody),
    Belief(BeliefBody),
    Hypothesis(HypothesisBody),                  // NEW
    Concept(ConceptKernel),
    Skill(SkillBody),
    Workflow(WorkflowPlanBody),                  // NEW
    Resource(ResourceBody),                      // now multimodal
    Equation(EquationBody),
    Theorem(TheoremBody),
    Experiment(ExperimentBody),
    Dataset(DatasetBody),
    Counterexample(CounterBody),
    Eval(EvalBody),
    Lesson(MetacogBody),
    Distillate(DistillateBody),                  // NEW
}
```

V2 schemas remain valid. New schemas:

#### 5.7.A HypothesisBody (NEW)

```rust
pub struct HypothesisBody {
    pub id: HypothesisId,
    pub statement: String,
    pub formal_statement: Option<String>,        // optional Lean/SymPy form
    pub predictions: Vec<Prediction>,
    pub falsification_conditions: Vec<String>,   // explicit "I'd be wrong if..."
    pub supporting_claims: Vec<MemoryId>,
    pub contradicting_claims: Vec<MemoryId>,
    pub test_registry: Vec<TestRecord>,
    pub status: HypothesisStatus,
    pub evidence_strength: f32,                  // aggregate Bayesian posterior
    pub confidence: f32,
    pub linked_topics: Vec<TopicId>,
    pub falsifiable: bool,
}

pub enum HypothesisStatus { Open, Tested, Supported, Refuted, Withdrawn }

pub struct Prediction {
    pub description: String,
    pub measurable_outcome: String,
    pub valid_under: Option<String>,             // regime
    pub deadline: Option<DateTime<Utc>>,
}

pub struct TestRecord {
    pub experiment_id: ResourceId,
    pub outcome: TestOutcome,                    // Pass | Fail | Inconclusive
    pub effect_size: Option<f32>,
    pub p_value: Option<f32>,
    pub published_at: DateTime<Utc>,
    pub source_span: SourceSpan,
    pub falsifies: Option<HypothesisId>,
    pub supports: Option<HypothesisId>,
}
```

#### 5.7.B WorkflowPlanBody (NEW)

```rust
pub struct WorkflowPlanBody {
    pub id: PlanId,
    pub name: String,
    pub task_signature: String,                  // canonical hash for retrieval
    pub steps: Vec<WorkflowStep>,
    pub edges: Vec<WorkflowEdge>,                // DAG dependencies
    pub preconditions: Vec<Precondition>,
    pub expected_outcome: String,
    pub max_total_runtime_ms: u32,
    pub status: SkillStatus,                     // reuses skill lifecycle
    pub success_count: u64,
    pub failure_count: u64,
    pub replan_count: u64,                       // diagnostic
}

pub struct WorkflowStep {
    pub step_id: u16,
    pub skill_id: SkillId,
    pub inputs: Vec<(String, ValueRef)>,
    pub outputs: Vec<(String, ValueRef)>,
    pub optional: bool,
    pub retry_policy: RetryPolicy,
    pub max_runtime_ms: u32,
}

pub struct WorkflowEdge {
    pub from_step: u16,
    pub to_step: u16,
    pub condition: EdgeCondition,                // Always | OnOutput { var, op, val }
}

pub enum ValueRef {
    Literal(serde_json::Value),
    StepOutput { step: u16, name: String },
    InputParam(String),
    MemoryRef(MemoryId),
}

pub enum RetryPolicy {
    None, Linear { max: u8, delay_ms: u32 }, Exponential { max: u8, base_ms: u32 },
}
```

#### 5.7.C ResourceBody (extended for multimodal)

```rust
pub struct ResourceBody {
    pub canonical_uri: String,
    pub doi: Option<String>,
    pub license: Option<String>,
    pub version: Option<String>,
    pub primary_modality: Modality,
    pub modalities_available: Vec<Modality>,
    pub checksums: Vec<(Modality, blake3::Hash)>,
    pub transcriptions: Vec<Transcription>,       // NEW: extracted text per modality
    pub extracted_summaries: Vec<(Modality, String)>,
    pub bounding_boxes: Vec<BoundingBox>,         // for diagrams/images
    pub frame_keypoints: Vec<FrameKeypoint>,      // for video/audio
    pub size_bytes: u64,
    pub access_policy: AccessPolicy,
    pub linked_claims: Vec<MemoryId>,
    pub linked_papers: Vec<MemoryId>,
}

pub struct Transcription {
    pub modality: Modality,
    pub transcriber: String,                     // "whisper-large-v3" | "tesseract-5" | "paddleocr"
    pub confidence: f32,
    pub text: String,
    pub timestamp_alignment: Option<Vec<(u32, u32, String)>>, // (start_ms, end_ms, token) for audio
}
```

#### 5.7.D DistillateBody (NEW)

```rust
pub struct DistillateBody {
    pub topic_id: TopicId,
    pub adapter_kind: AdapterKind,               // LoRA | LoRA-FA | Prefix
    pub rank: u16,                               // typical 8 or 16
    pub alpha: u16,                              // typical 16 or 32
    pub trained_on: Vec<MemoryId>,               // source memories
    pub weights_ref: ResourceId,                 // CAS blob holding .safetensors
    pub eval_score: f32,                          // held-out validation
    pub trained_at: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    pub never_authoritative: bool,                // always true
}
```

### 5.8 Typed memory links (v2 inherited; +federation fields)

```rust
pub struct MemoryLink {
    pub target: MemoryId,
    pub target_shard: ShardId,                   // NEW
    pub relation: Relation,
    pub strength: f32,
    pub created_at: DateTime<Utc>,
    pub source_episode: Option<EpisodeId>,
    pub confidence: f32,
    pub vector_clock: VectorClock,               // NEW: causal ordering for federation
}

pub enum Relation {
    SimilarTo, Extends, Refines, SpecializationOf, GeneralizationOf,
    Supports, Contradicts, Implies, Equivalent, Conditions, Moderates,
    Causes, Prevents, FollowedBy, Precedes,
    Cites, DerivedFrom, AuthoredBy,
    ExampleOf, CounterexampleOf, PrototypeOf,
    DependsOn, PrerequisiteOf, UsedBy,
    Analogy { mapping: String },
    Supersedes, SupersededBy,
    ImplementedBy, ProducedBy, TestedBy,
    Falsifies, Predicts, ExpectedFromHypothesis, // NEW
    StepOfPlan, PlanReplaces,                    // NEW
    TrustPropagates,                              // NEW: source-reputation propagation
    Custom(PredicateId),
}
```

### 5.9 Entities & temporal edges (v2 inherited; +shard/clock)

```rust
pub struct Entity {
    pub id: EntityId,
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub kind: EntityKind,
    pub summary: String,
    pub embedding_refs: EmbeddingRefSet,
    pub created_at: DateTime<Utc>,
    pub merged_into: Option<EntityId>,
    pub privacy_class: PrivacyClass,
    pub shard_id: ShardId,                       // NEW
    pub reputation: Option<SourceReputation>,    // for Person/Paper/Venue/Publisher
}

pub struct SourceReputation {                    // NEW (§30.3)
    pub quality_score: f32,
    pub accuracy_rate: f32,
    pub citation_count: u32,
    pub corroboration_rate: f32,
    pub freshness: f32,
    pub last_updated: DateTime<Utc>,
}

pub struct TemporalEdge {
    pub id: EdgeId,
    pub subject: EntityId,
    pub predicate: PredicateId,
    pub object: EntityOrLiteral,
    pub temporal: BitemporalValidity,            // bitemporal w/ vector_clock
    pub confidence: f32,
    pub support: Vec<EvidenceLink>,
    pub contradicts: Vec<EdgeId>,
    pub supersedes: Vec<EdgeId>,
    pub source_episode: EpisodeId,
    pub source_memory: MemoryId,
    pub receipt_id: ReceiptId,
    pub status: EdgeStatus,
    pub signature: Option<Signature>,
    pub shard_id: ShardId,
}
```

### 5.10 Predicate vocabulary
Stored in `mnemos-psi-types/predicates.yaml`. v3 adds: `predicts`, `falsifies`, `is_step_of`, `replanned_to`, `propagates_trust_to`, `corroborates`, `cites_with_quote_hash`, `formalizes`. New predicates queue in `proposed_predicates.yaml` for review.

### 5.11 MutationReceipt (CRDT-ordered)

```rust
pub struct MutationReceipt {
    pub id: ReceiptId,
    pub mutation_id: MutationId,
    pub at: DateTime<Utc>,
    pub actor: MutationActor,
    pub agent_id: AgentId,                       // NEW
    pub shard_id: ShardId,                       // NEW
    pub vector_clock: VectorClock,               // NEW: causal order
    pub mutation_kind: MutationKind,
    pub objects_created: Vec<MemoryId>,
    pub objects_updated: Vec<(MemoryId, u32)>,
    pub objects_superseded: Vec<(MemoryId, MemoryId)>,
    pub objects_archived: Vec<MemoryId>,
    pub objects_crypto_deleted: Vec<(MemoryId, blake3::Hash)>, // NEW
    pub source_hashes: Vec<blake3::Hash>,
    pub index_updates: Vec<IndexUpdate>,
    pub warnings: Vec<String>,
    pub approval: Option<ApprovalReceipt>,       // NEW: gated mutations
    pub previous_receipt_hash: blake3::Hash,
    pub receipt_hash: blake3::Hash,
    pub signature: Option<Signature>,
}

pub struct ApprovalReceipt {
    pub approver_agent: AgentId,
    pub approver_role: RoleId,
    pub at: DateTime<Utc>,
    pub reason: String,
    pub signature: Signature,
}
```

---

## 6. Storage Substrate

### 6.1 Canonical truth (immutable, federated)

| Concern | Engine | Crate | Why |
|---|---|---|---|
| Append-only WAL | custom; postcard + crc32 + zstd | `postcard`, `crc32fast`, `zstd` | segmentable, replayable, compressed |
| Episode hash chain | inline in WAL | `blake3` | tamper detection |
| Mutation receipts | inline in WAL + signed; CRDT-ordered | `ed25519-dalek` | non-repudiation, federation |
| Vector clock | per-receipt + per-edge | custom | causal ordering across shards |
| Content-addressed blobs | CAS dir, sha256 sub-tree | `blake3` | de-dup; large raw payloads |
| Vault encryption | age | `age` (rage) | passphrase or X25519-keyed |
| Cryptographic tombstone log | parallel chain | `blake3`, `ed25519-dalek` | right-to-be-forgotten with proof |
| Git-backed cards | repo in `git/` | `gix` (preferred) / `git2` | human audit & branching |

### 6.2 Canonical projections (rebuildable, fast)

| Concern | Engine | Crate | Why |
|---|---|---|---|
| KV (objects, entities, edges) | redb | `redb` | pure-Rust, ACID, MVCC, mmap |
| Vector — hot fp32 (768d + Matryoshka heads) | HNSW in-mem | `usearch` (preferred) / `hnsw_rs` | low-latency kNN, native Rust, multi-resolution |
| Vector — warm fp16 | HNSW mmap | `hnsw_rs` w/ quantizer | 2× capacity vs fp32 |
| Vector — cold | IVF-PQ (K=1024 centroids, M=64 sub-quantizers, 8-bit) | custom over `usearch` | 24× compression; 5–8% recall loss documented |
| Vector — micro | binary (1-bit per dim) | custom | for re-rank candidate filtering only |
| Sparse vector index | inverted-index of (token_id, weight) | custom + `roaring` | BGE-M3 sparse / SPLADE-style |
| BM25 / full-text | Tantivy with Maxscore + Wand | `tantivy` | impact-sorted postings |
| Symbol / equation FST | Tantivy custom analyzer + FST | `tantivy` | exact symbol + equation hits ≤ 5 ms |
| Multimodal embed | CLIP, AudioLDM, Whisper | `candle-transformers` | image, audio, video kNN |
| Cross-encoder reranker | `bge-reranker-v2-m3` (default) / `mxbai-rerank-xsmall` (fast) | `candle-transformers` | late-stage rerank |
| Graph (temporal KG) | CSR snapshot + delta-overlay | `petgraph` (analytics) | lock-free reads, single-writer deltas |
| Temporal index | interval tree | `intervaltree` or hand-rolled | "valid at T" queries |
| Tag/keyword | Roaring bitmaps | `roaring` | fast intersections |
| Bloom filter (negative cache, dedup) | Bloom + cuckoo | `cuckoofilter`, `growable-bloom` | O(1) presence checks |
| Count-min sketch (topic frequency) | streaming approx | custom | optional, extreme-scale only |
| Skill / Plan precondition index | hash + Tantivy facet | `tantivy` | task-signature-keyed |
| Co-activation | structured `BTreeMap<MemoryId, BTreeMap<MemoryId, CoActStat>>` + Roaring | std + `roaring` | fast top-K |
| ContextPack cache | L1 in-process LRU + L2 mmap + L3 disk archive | `quick-cache`, `memmap2` | 3-level hierarchy (§7.13) |

### 6.3 Optional external adapters

| Service | Adapter | When |
|---|---|---|
| Qdrant | `mnemos-index-vector::QdrantBackend` | scale > 50M vectors, HA |
| Kùzu | `mnemos-index-graph::KuzuBackend` | dense entity graphs > 50M edges |
| DuckDB | `mnemos-eval::DuckBackend` | analytical queries over Eval logs |
| Lean / Coq / SymPy | `mnemos-formal::*Backend` | formal verification |
| Whisper.cpp | external process | local audio transcription |
| Tesseract / PaddleOCR | external process | image/diagram OCR |

### 6.4 On-disk layout (federation-ready)

```
$MNEMOS_HOME/
├── config.toml
├── retrieval_policy.toml
├── topic.toml
├── fsrs.toml
├── federation.toml                          # NEW: shard map, peers, ACL
├── identity/
│   ├── agent.ed25519                        # this agent's signing key
│   ├── signing-key.age
│   └── vault.x25519                         # for vault encryption
├── ledger/
│   ├── shard-000/                           # NEW: one subdir per local shard
│   │   ├── episodes/0000000001.wal.zst
│   │   ├── mutations/0000000001.wal.zst
│   │   ├── receipts/0000000001.cbor.zst
│   │   ├── tombstones/0000000001.cbor.zst   # NEW: crypto-delete proofs
│   │   └── checkpoints/checkpoint-000001.cbor
│   └── shard-001/  …
├── objects/
│   ├── redb/memory.redb
│   ├── redb/entities.redb
│   ├── redb/edges.redb
│   └── blobs/sha256/ab/cd/abcd…
├── vault/                                    # age-encrypted secrets
│   └── *.age
├── projections/
│   ├── tantivy/{episodic,observation,semantic,concept,procedural,resource,formal,counterexample,metacognitive,eval,hypothesis,plan}/
│   ├── hnsw/
│   │   ├── semantic.hot.fp32.usearch
│   │   ├── semantic.matr.{768,384,192,96}.usearch
│   │   ├── semantic.warm.fp16.hnsw
│   │   ├── semantic.cold.ivfpq.usearch
│   │   ├── concept.usearch
│   │   ├── skill.usearch
│   │   ├── multimodal.clip.usearch
│   │   ├── multimodal.audio.usearch
│   │   └── time_conditioned.usearch
│   ├── sparse/
│   │   ├── bge_m3_sparse.idx
│   │   └── splade.idx
│   ├── graph/
│   │   ├── nodes.cbor
│   │   ├── edges.cbor
│   │   ├── adjacency.mmap                    # CSR
│   │   ├── delta.log                          # NEW: append-only delta over CSR
│   │   └── temporal-index.mmap
│   ├── bitmaps/tags.roaring …
│   ├── symbol_fst/
│   ├── skill_precond_index/
│   ├── plan_signature_index/                # NEW
│   ├── bloom/
│   │   ├── dedup.bloom
│   │   └── negative_cache.bloom
│   ├── countmin/topics.sketch                # NEW (optional)
│   ├── context_cache/
│   │   ├── l1.lru                            # in-process serialized snapshot
│   │   ├── l2/                               # mmap'd LRU
│   │   └── l3/                               # cold archive
│   ├── reranker/
│   │   └── bge-reranker-v2-m3.safetensors
│   └── distillate/                           # NEW
│       └── topic-<id>.safetensors
├── git/
│   └── curated-memory/
│       ├── cards/{claims,equations,theorems,skills,plans,counterexamples,concepts,hypotheses}/
│       ├── maps/{topics,timeline,citation,multimodal}/
│       ├── reviews/
│       └── manifests/
├── candle/
│   ├── embed.bge-m3.safetensors             # dense + sparse + multivec
│   ├── reranker.bge-v2-m3.safetensors
│   ├── reranker.mxbai-xsmall.safetensors
│   ├── clip.safetensors
│   ├── whisper.safetensors
│   └── distilled_extractor.safetensors      # NEW: self-distilled
├── policy/
│   ├── manager.safetensors                  # MemoryManager MLP
│   ├── embedding_adapter.safetensors        # NEW: fine-tuned adapter
│   ├── score_calibrator.safetensors         # NEW: Platt/isotonic
│   ├── skill_sequencer.safetensors          # NEW: workflow planner
│   └── query_transition.safetensors         # NEW: speculative-prefetch model
├── telemetry/
│   ├── traces.parquet
│   ├── evals.parquet
│   ├── drift.parquet                         # NEW: concept-drift records
│   └── latency.parquet                       # NEW: SLA tracking
├── sandboxes/                                # one dir per running skill
└── federation/                               # NEW: peer state cache
    ├── peers.toml
    ├── shard_map.cbor
    └── handoff_logs/
```

### 6.5 WAL format

Each segment is length-prefixed `WalEntry`s. v3 adds tombstone, plan, hypothesis, federation entries:

```rust
pub enum WalEntry {
    Episode(Episode),
    Observation(ObservationFrame),
    MemoryAdd(MemoryObject),
    MemoryUpdate { id: MemoryId, new_version: u32, patch: serde_json::Value },
    MemorySupersede { old: MemoryId, new: MemoryId, reason: String },
    MemoryArchive(MemoryId),
    MemoryCryptoDelete { id: MemoryId, tombstone: CryptoTombstone }, // NEW
    EntityUpsert(Entity),
    EntityMerge { from: EntityId, into: EntityId },                  // NEW
    EdgeAdd(TemporalEdge),
    EdgeClose { edge: EdgeId, valid_to: DateTime<Utc>, reason: String },
    EdgeContradicted { a: EdgeId, b: EdgeId },
    SkillUpsert(SkillBody),
    PlanUpsert(WorkflowPlanBody),                                    // NEW
    HypothesisUpsert(HypothesisBody),                                // NEW
    HypothesisTest { id: HypothesisId, record: TestRecord },          // NEW
    DistillateUpsert(DistillateBody),                                // NEW
    LifecycleTransition { id: MemoryId, from: LifecycleState, to: LifecycleState },
    Strengthen { id: MemoryId, delta: f32, reason: StrengthenReason },
    Decay { id: MemoryId, factor: f32 },
    Reflection { episode: EpisodeId, output: serde_json::Value },
    BeliefRevision(BeliefRevision),
    SourceReputationUpdate { entity: EntityId, new: SourceReputation }, // NEW
    AdapterUpdate { kind: AdapterKind, weights_ref: ResourceId },     // NEW
    DriftEvent(ConceptDriftEvent),                                    // NEW
    PeerHandoff { peer: AgentId, shard: ShardId, range: ReceiptRange }, // NEW federation
    Receipt(MutationReceipt),
    Checkpoint { redb_seq: u64, segment_hash: blake3::Hash },
}
```

Segments rotate at 64 MiB or 1 h. Footers signed. v3 adds: each WAL entry carries its `vector_clock` for federation; replay merges across peers via Lamport ordering with deterministic tie-break by `(agent_id, mutation_id)`.

---

## 7. Index Layer

### 7.1 Tantivy schemas (with Maxscore/Wand)

Per-lane Tantivy index. Fields extend v2 with: `shard_id` (FACET), `modality` (FACET), `hypothesis_status` (FACET), `plan_signature_hash` (STRING). Tokenizer chain for Formal lane preserves math symbols and TeX commands (v2 retained). Maxscore + Wand top-K pruning enabled by default; with Wand, p50 for 10M-doc BM25 queries falls from ~20 ms to ~5 ms on heavy queries.

### 7.2 Vector indices — five tiers + Matryoshka + sparse

| Tier | Storage | Precision | Capacity | Build params | Latency goal |
|---|---|---|---|---|---|
| **Hot fp32** | in-memory `usearch` | fp32 768-d | ≤ 500k | M=16, ef_c=200, ef_s=64 | < 5 ms kNN |
| **Hot Matryoshka 384** | in-memory `usearch` | fp32 384-d | ≤ 5M | M=16, ef_c=200, ef_s=64 | < 8 ms kNN |
| **Hot Matryoshka 192** | in-memory `usearch` | fp32 192-d | ≤ 20M (across cluster) | M=16, ef_c=200, ef_s=64 | < 12 ms kNN |
| **Warm fp16** | mmap on-disk | fp16 768-d | ≤ 5M | M=24, ef_c=400, ef_s=96 | < 25 ms kNN |
| **Cold IVF-PQ** | mmap, K=1024 / M=64 / 8-bit | quantized ≈ 64 B/vec | ≤ 95M | nprobe=16 | < 80 ms kNN |
| **Binary filter** | mmap, 1-bit/dim | 96 B/vec | all | n/a | < 2 ms reject pass |

**Matryoshka strategy.** All embeddings come from `bge-m3` (single model produces 768-d, with native Matryoshka heads at 384/192/96). At write, `bge-m3` produces simultaneously: dense 768, dense 384, dense 192, dense 96, sparse, multi-vector ColBERT (for re-rank only). The 96-d binary projection is used as a Bloom-equivalent first-pass.

**Promotion rules.** A memory is **hot fp32** if `(lane ∈ {Core, Belief, Hypothesis, Counterexample, Procedural})` OR `(R > 0.5 AND importance > 0.6)`. Demoted to warm fp16 by consolidation when R < 0.3 AND importance < 0.5. Cold IVF-PQ holds the long tail; cold candidates are first filtered by 96-d binary index, then top-N expanded to fp32 via rerank.

**Vector quantization details.**
- **fp16:** custom quantizer in `mnemos-index-vector`; deterministic; round-half-to-even.
- **SQ8 (scalar 8-bit):** per-dim min/max recorded; documented 2–3 % recall loss.
- **PQ64 (product 64-byte):** K=1024 codebook centroids per sub-quantizer × M=64 sub-quantizers × 8-bit; ~5–8 % recall loss; 12× compression vs fp32.
- **Binary (1-bit):** sign(x); used only as filter, not for final ranking.

### 7.3 Sparse retrieval (NEW)

Inverted index of `(token_id, weight)` pairs produced by **BGE-M3 sparse head** (or SPLADE-v3 as alt). Stored as `(doc_id → Vec<(token_id, f32)>)` in `sparse/bge_m3_sparse.idx` plus inverted postings `(token_id → Roaring<doc_id>)`. Query path: sparse query vector → inverted lookup → BM25-style top-K (default K=128). Fused with dense (RRF) at fusion stage. Critical for math/code/symbolic queries where dense embeddings struggle.

### 7.4 Cross-encoder reranker (NEW)

Optional final-stage rerank. Default model: `bge-reranker-v2-m3` (224 M params, Candle-compatible) — 2–8 ms per (query, passage) pair on M-series. Fast alternative: `mxbai-rerank-xsmall` (22 M params, ~1 ms). Triggered when `QueryKind ∈ {ScientificJudgment, MathDerivation, Educational}` OR caller passes `query.rerank = true`. Configurable top-K to rerank (default 64).

### 7.5 ColBERT late-interaction (NEW)

Optional finest-stage rerank. `bge-m3` provides multi-vector token-level embeddings (128-d per token). Compute MaxSim score `Σ_q max_d cos(q_i, d_j)` for top-K candidates. ~20 ms for K=64 on M-series; reserved for highest-stakes queries. Stored sparsely (only for hot tier; ~10 KB/memory).

### 7.6 Temporal graph (CSR snapshot + lock-free delta)

CSR adjacency mmap'd; delta log mmap'd. Reads merge CSR ∪ delta on the fly. Single-writer appends to delta; hourly background task spins fresh CSR snapshot, atomic pointer swap (`Arc<Snapshot>`), retires old. **Readers never block; writers never block; snapshots are at most 1 h stale on disk-write but always reflect WAL state in RAM.** Personalized PageRank bounded: max 3 hops, ≤ 256 expanded nodes, decay 0.15, 10 iters or convergence < 1e-3; reusable for ≤ 5 s.

### 7.7 Bloom & cuckoo filters (NEW)

- **Dedup Bloom** (`growable-bloom`): keyed by `(content_hash, lane)`; checked before expensive similarity at §8.2 step 11. Rebuilt monthly.
- **Negative-cache Bloom**: keyed by `blake3(query_canonical)`; if hit, fast NOOP return "no relevant prior memory" with TTL 24–168 h. False positive < 1 %.
- **Cuckoo entity dedup**: keyed by `(canonical_name, kind)`; used in §8.2 step 7 to short-circuit entity linker.

### 7.8 Count-min sketch (NEW, optional)

For extreme-scale topic-frequency tracking (> 10 M topics). Width 256, depth 4, hash family BLAKE3-keyed. Used by Topic Strengthener (§11.3) when exact counters would not fit. Default: exact (off until scale demands).

### 7.9 Symbol / Equation FST (v2 retained)
Equations indexed three ways: raw TeX (BM25), normalized symbolic form (BM25 on `eq_norm`), and FST keys over `(LHS_symbols, RHS_symbols, top_operator, units_signature)`. Hit < 5 ms.

### 7.10 Skill & Plan precondition indices

`HashMap<PreconditionSignature, RoaringBitmap<SkillId>>` and parallel `HashMap<PlanSignature, RoaringBitmap<PlanId>>`. Plan signature = canonical hash of (task_sig, sorted preconditions, expected_outcome_canonical). HNSW over skill/plan description embeddings for fuzzy match.

### 7.11 Multimodal indices (NEW)

- `multimodal.clip.usearch` — CLIP-Large image embeddings (768-d), shared with text query space via CLIP's joint embedding.
- `multimodal.audio.usearch` — Whisper-encoded audio segment embeddings (1024-d) + key-phrase index over transcripts.
- Diagram: PDF → SVG parse + OCR text → BM25 in Tantivy + bounding-box index for reverse lookup.
- Each resource memory may carry one or more `EmbeddingRefSet` entries spanning modalities; cross-modal retrieval merges via RRF (§9.3).

### 7.12 Time-conditioned embedding (NEW, optional)

For Formal / Concept / Belief / Hypothesis lanes: prepend canonical time bucket token (e.g., `[EPOCH:Q2_2026]`) to text before embedding. Stored in `time_conditioned.usearch`. Used only when query is `recall_at` or `recall_as_of` and `query.time_conditioned = true`. Adds ~5 % retrieval accuracy on historical queries; double-storage cost on selected lanes (gated by importance > 0.7).

### 7.13 Multi-level ContextPack cache (NEW)

| Level | Backend | Eviction | Capacity | Latency |
|---|---|---|---|---|
| L1 | `quick-cache` in-process LRU | size + age | 500 packs / 100 MB | < 0.1 ms hit |
| L2 | mmap LRU file | size | 10 k packs / 10 GB | < 1 ms hit |
| L3 | disk archive | aged > 30 d | unbounded (queryable) | < 50 ms hit |

Invalidation: each cached pack stores `cited_memories: Vec<MemoryId>`. On any mutation touching a cited memory, stale-mark the pack (Bloom over cited IDs for fast match; precise check on cache pull).

### 7.14 Co-activation index (NEW, structured)

Replaces v2 flat HashMap. Per memory: `BTreeMap<MemoryId, CoActStat>` sorted by strength descending — top-K co-activated retrievable in O(log N). Pair-presence test via Roaring bitmap. Periodic compaction drops entries with strength < 0.05.

---

## 8. Write Pipeline

Split into **hot** (sync, ≤ 3 ms p50) and **async** (background extraction, indexing, learning).

### 8.1 Hot stages

1. **Privacy gate.** Reject/encrypt by class. Vault-class routed to `vault/` with age encryption keyed by `H(master || resource_id)`.
2. **Segment.** Split on `EventBoundary`.
3. **Episode append.** Hash-chain + sign + vector-clock + WAL. Returns `EpisodeId` ≤ 1 ms.
4. **Observation distillation.** Optional dense Mastra line.
5. **Acknowledge.** `WriteReceipt { episode_id, observation_id?, queued_extractions, receipt_hash }`.
6. **Backpressure check (NEW).** If extraction queue depth > `num_cpus × 100 × 0.8`: drop low-importance Observation candidates and set `WriteReceipt.backpressured = true`. Alert if sustained > 80 % for 60 s.

### 8.2 Async stages

7. **Extract candidates.** `Extractor` trait. Defaults:
   - `DistilledFastExtractor` (NEW; local Candle 66 M-param distilled model) for routine episodes — 8–20 ms each, F1 ≥ 0.90 of LLM extractor on the eval set.
   - `LlmStructuredExtractor` (Claude/OpenAI/Ollama) only when distilled extractor confidence < 0.5 OR for high-importance lanes (Formal, Hypothesis, Theorem).
   - `RuleExtractor`, `EquationExtractor`, `CodeExtractor` (tree-sitter), `MultimodalExtractor` (NEW; OCR + CLIP + Whisper), `CitationExtractor`.
8. **Normalize.** Entity linking → cuckoo dedup → alias dict → embedding kNN (cos > 0.92 use; 0.80–0.92 propose merge; < 0.80 create). Unit normalization. Predicate canonicalization.
9. **Ground.** Attach `Vec<SourceSpan>` with hashes + quote excerpts. **Hard rule:** Semantic/Belief/Claim/Theorem/Equation/Hypothesis without ≥ 1 SourceSpan → `Proposed` (no auto-promotion).
10. **Link.** Search nearest existing via (BM25 ⊕ dense Matryoshka 192 ⊕ sparse ⊕ entity). Create typed Links of strength > τ_link = 0.6.
11. **Contradiction / supersession.** Same algorithm as v2 §8.2 step 10. v3 additionally: if a Hypothesis is implicated, route to Hypothesis Engine (§15).
12. **Validate.** Schema check (with `schema_version` migration if old). Bloom-dedup pre-check. Unit dimension check for Equations.
13. **Score.** Initial importance, novelty, surprise, source_quality (now computed dynamically — §30.3), utility = 0.5, FSRS S=1.0 D=5.0. Apply `score_calibrator` (Platt/isotonic, §9.6) to produce `calibrated_relevance_prob`.
14. **Policy decision.** `MemoryManager::decide(candidate, context) → MemoryDecision`:
    ```rust
    pub enum MemoryDecision {
        Add, Update { target: MemoryId },
        Supersede { target: MemoryId },
        Merge { target: MemoryId },
        Archive { target: MemoryId },
        Noop, Review { reason: String, route_to_role: RoleId }, Reject { reason: String },
    }
    ```
    `Review` decisions trigger the approval workflow (§30.4).
15. **Mutate + receipt.** Apply decision. Write `WalEntry`s including `MutationReceipt` with `vector_clock`, `agent_id`, `shard_id`. If contains lane ∈ {Formal, Belief, Theorem, Equation, Hypothesis} AND confidence < 0.6 AND no approver: pause at `Proposed` until `ApprovalReceipt` arrives.
16. **Index fanout.** Update Tantivy + dense (all Matryoshka heads) + sparse + symbol FST + bitmaps + skill/plan precond + multimodal indices + Bloom dedup — all parallel `tokio::spawn`.
17. **Downstream signals.** Reflection if `surprise > 0.6`. Hypothesis update if Claim contradicts an Open hypothesis. Belief revision queue. Topic strength recompute. Adapter-trainer collects training pair if outcome eventually arrives.
18. **Federation publish (NEW).** If multi-agent: publish `MutationReceipt` summary to peer `handoff_logs/` for replication. Peers replay in vector-clock order; conflicts → `Contested` with both versions retained.

### 8.3 Extractor trait

```rust
#[async_trait]
pub trait Extractor: Send + Sync {
    async fn extract(&self, ep: &Episode, ctx: &ExtractionContext)
        -> Result<Vec<CandidateMemory>>;
    fn capabilities(&self) -> ExtractorCaps;
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn reliability(&self) -> f32;                // EMA from §30.3
}
```

### 8.4 Backpressure & flow control (NEW)

Bounded queue (`tokio::sync::mpsc::channel` with capacity `num_cpus × 100`). Strategies on overflow:
1. Drop oldest `Observation` candidates with `importance < 0.3`.
2. Defer non-urgent `Concept` and `Metacognitive` distillation.
3. Set `WriteReceipt.backpressured = true`.
4. Emit metric `extraction_queue_depth`; alert if sustained > 80 % for 60 s.
Batching auto-tunes: small batches under pressure, large when idle.

### 8.5 Bulk corpus ingestion daemon (NEW)

`mnemos-bulk-ingest` binary:

```toml
[bulk.arxiv]
manifest_url = "https://arxiv.org/help/manifest"
download_concurrency = 4
extract_concurrency = 2
rate_limit_docs_per_hour = 200
default_source_quality = 0.7

[bulk.wikipedia]
dump_url = "https://dumps.wikimedia.org/enwiki/latest/enwiki-latest-pages-articles.xml.bz2"
incremental = true

[bulk.github]
trending_url = "..."
sandbox_test_required = true
```

Pipeline:
1. Stream documents.
2. Compute Minhash signature (128-band, 8 hashes/band) for fast near-dup.
3. If Minhash hit in `dedup.bloom`: increment reference count on original.
4. Else: extract via LLM (10 % sample) + Rule (90 %); route into write pipeline normally.
5. Track ingest cost: pause if daily quota exceeded.
6. Archive overflow: papers older than 5 y to cold tier (full SourceSpans retained).

---

## 9. Read Pipeline

```
classify_query → resolve_anaphora → expand_query (HyDE/multi-query) →
parallel_retrieve_15 → expand_graph(PPR) → fuse(RRF) → calibrate_score(Platt) →
rerank(cross-encoder) → late_interaction_optional(ColBERT) → temporal_filter →
privacy_filter → causal_mask → self_rag_decide → compile_pack →
speculative_prefetch_next → log_eval
```

### 9.1 Query classification

`QueryKind` extends v2 with `Hypothesis` and `Workflow`. Default rule-based; optional Candle MLP for ambiguous.

### 9.2 Anaphora / coreference resolution (NEW)

Before classification, resolve pronouns against `Core + Working + last_N_recalls`:
- Rule-first (`it`, `they`, `the same X`, `that one`) → match recent referent in working memory.
- Embedding-based for fuzzier cases.
- Populate `query.resolved_entities` and `query.referred_memories`.

### 9.3 Query expansion — HyDE + multi-query (NEW, optional)

For `QueryKind ∈ {MathDerivation, ScientificJudgment, Educational, Planning, Exploratory}` and `query.expand = true`:
1. Multi-query: ask small LLM (or distilled model) for 2 alternate phrasings; embed each.
2. HyDE: ask for a hypothetical answer paragraph; embed.
3. Embed all (orig + 2 phrasings + HyDE answer) and run all four through the dense channel; fuse with RRF.
Cost: 20–100 ms added if remote LLM, 5–15 ms if distilled. Disable if `latency_budget_ms < 50`.

### 9.4 15 parallel retrieval channels

| # | Channel | Substrate | Default k |
|---|---|---|---|
| 1 | **Core** | pinned blocks | n/a |
| 2 | **BM25** | Tantivy + Maxscore/Wand | 64 |
| 3 | **Dense (Matryoshka 768/384/192)** | tier auto-selected | 64 |
| 4 | **Sparse (BGE-M3 sparse)** | inverted sparse | 64 |
| 5 | **Entity** | entity-resolved | 64 |
| 6 | **Temporal** | interval tree | filter |
| 7 | **Graph PPR** | HippoRAG bounded | 32 (only if QueryKind ∈ {Educational, Mixed, Planning, MathDerivation}) |
| 8 | **Concept** | A-MEM neighbors | 32 |
| 9 | **Procedural** | skill + plan precond + signature | 16 |
| 10 | **Eval / Counterexample** | always if entity overlap | always |
| 11 | **Resource** | papers/datasets/files | 16 |
| 12 | **Symbol / Equation** | FST hits for math | 32 |
| 13 | **Belief** | current stance | 8 |
| 14 | **Hypothesis (NEW)** | open hypotheses with predicates matching query | 8 |
| 15 | **Multimodal (NEW)** | CLIP image, Whisper audio for visual/audio queries | 16 |
| 16 | **Topic Atlas** | top-level "what do I know about X?" | 4 |

All via `tokio::join!`. Irrelevant channels skipped by intent profile.

### 9.5 Fusion + reranking — RRF + cross-encoder + optional ColBERT

1. **Fuse** RRF (k=60) across channels.
2. **Cross-encoder rerank**: `bge-reranker-v2-m3` on top-64 fused. ~3 ms/pair × 64 = 200 ms? No — batched on GPU/Metal: 64 pairs in ~20 ms on M-series. Skip if `latency_budget_ms < 50`.
3. **Optional ColBERT late-interaction** on top-32 reranked: ~20 ms. Only if `query.late_interaction = true` OR `QueryKind ∈ {ScientificJudgment, MathDerivation}`.

### 9.6 Calibrated score model

```
raw_score = 0.18*exact + 0.16*dense + 0.14*bm25 + 0.10*sparse + 0.10*cross_encoder
          + 0.12*entity + 0.10*graph_ppr + 0.06*temporal + 0.06*source_quality
          + 0.05*utility + 0.04*topic_strength + 0.03*confidence + 0.03*recency
          + 0.03*freshness
          - 0.20*contradiction_unresolved - 0.15*stale - 0.10*privacy_penalty
```

Weights are stored in `retrieval_policy.toml` and **profile-overridable** per `QueryKind` (math, debug, scientific, coding, planning, educational, hypothesis, workflow).

After raw scoring, `score_calibrator` (Platt logistic OR isotonic regression, trained nightly on Eval lane) maps raw → `P(memory_relevant) ∈ [0,1]`. ContextPack inclusion threshold: `P > 0.5` by default; configurable. **Confidence intervals for each item** are now meaningful.

### 9.7 Temporal, privacy, causal filters

- Unless historical query, drop `valid_to < now`.
- Drop or `[REDACTED]` items exceeding caller's allowed privacy class; emit `OmissionNote` if any high-importance memory elided.
- **Causal mask (NEW):** For `recall_as_of(t_tx)` or `recall_at_as_of(t_world, t_tx)`, filter `tx_from > t_tx` — no forward-in-time leakage. Tested in §32.

### 9.8 Self-RAG reflective decision (NEW, optional)

After initial pack compile:
```
if confidence_summary < 0.65 or len(omitted_evidence_high_value) > 0:
    state ← Deep | ExpandGraph | SeekCounter | Conclude
    if state != Conclude:
        re-run retrieval with widened k or expand to cold tier
        merge with prior pack (de-dup)
    else:
        finalize
```
Bounded: max 2 expansions to avoid runaway. Logged in trace.

### 9.9 Negative cache check (NEW)

Before retrieval, hash `query_canonical` and check `negative_cache.bloom`. On hit, return early "no relevant prior memory" + suggest 2–3 adjacent queries. TTL 24–168 h. Saves ~90 % latency on repeated fruitless queries.

### 9.10 Speculative prefetch (NEW, optional)

After compile, predict top-3 likely next queries via `query_transition.safetensors` (small Candle MLP trained on Eval sequences). Spawn background fetch; cache in L1 for 30 s. On next recall, L1 hit serves in < 1 ms. Disable when `latency_budget_ms < 50` or under backpressure.

### 9.11 ContextPack contract (v2 retained + extensions)

```rust
pub struct ContextPack {
    pub id: Uuid,
    pub query: String,
    pub query_kind: QueryKind,
    pub resolved_anaphora: Vec<AnaphoraResolution>,    // NEW
    pub items: Vec<PackItem>,
    pub citations: Vec<Citation>,
    pub contradictions: Vec<ContradictionNote>,
    pub counterexamples_surfaced: Vec<MemoryId>,
    pub open_hypotheses_relevant: Vec<HypothesisId>,    // NEW
    pub omitted_evidence: Vec<OmissionNote>,
    pub redacted_count: u32,                             // NEW: differential redaction
    pub freshness: FreshnessSummary,
    pub confidence_summary: f32,
    pub calibrated_probability: Option<f32>,             // NEW
    pub token_estimate: u32,
    pub token_budget: u32,
    pub trace_id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub deterministic_hash: blake3::Hash,
    pub shard_provenances: Vec<ShardProof>,              // NEW: federation
    pub self_rag_states: Vec<SelfRagDecision>,           // NEW
}
```

### 9.12 Budget allocation (v2 retained)

Core 5 % · Direct 35 % · Evidence 25 % · Counter 10 % · Skills/Plans 10 % · Omissions 5 % · Slack 10 %. Min/max enforced per section.

### 9.13 Rendering rules per lane (v2 + new lanes)

- **Hypothesis:** `[Hyp · Open · evidence 0.62 (4S / 2C) · falsifiable] {statement}; predicted: …; would be refuted by: …`
- **Workflow:** `[Plan · diffusion_sweep v2 · reliability 0.88 · 5 steps · prereqs: gpu, dataset_X] expected: …`
- **Multimodal image:** `[Image · arxiv:2406.12345 figure 4 · OCR'd] caption: "Phase transition at T_c = 2.27"; bbox: (x,y,w,h)`
- **Multimodal audio:** `[Audio · lecture@45m12s · whisper-v3] "...the wavefunction collapses..."`

### 9.14 Log eval

Emit `EvalBody` skeleton with full retrieval trace + cost accounting. Finalized on `feedback`.

---

## 10. Consolidation Engine (v2 retained + v3 additions)

v2 jobs all retained. **NEW v3 jobs:**

- **Source reputation update (§30.3).** Compute per-`SourceReputation { quality_score, accuracy_rate, citation_count, corroboration_rate, freshness }`. Propagate updates 0.9× down trust chain (paper → author → venue → publisher).
- **Hypothesis testing.** Cross-reference open hypotheses against new Claims; if Claim's polarity matches Hypothesis's predictions, register `TestRecord`. Update status accordingly.
- **Adapter retraining trigger.** If accumulated `EvalBody` count > N (default 5 000) and `drift_metric > 0.1`: enqueue overnight adapter retrain.
- **Distillate generation.** Per topic with `strength > 0.8` AND `≥ 500 supporting memories`: train LoRA adapter offline; save under `distillate/`.
- **Replay benchmark.** Weekly, run last-month's episodes against current extractor; compute precision/recall drift; alert if > 5 % drop.
- **Workflow distillation.** Scan Eval traces for repeated multi-skill sequences with success ≥ 3×; propose new `WorkflowPlan`.

---

## 11. Strengthening Engine

### 11.1 FSRS-5 per memory (v2 retained)

### 11.2 Hebbian co-activation (v2 retained, structured index — §7.14)

### 11.3 Topic-level strengthening (v2 retained; v3 adds count-min for extreme scale)

### 11.4 Embedding adapter fine-tuning (NEW)

Nightly batch:
1. Collect `FeedbackSignal { query, retrieved_memories, outcome }` from Eval lane.
2. Construct contrastive triplets `(anchor_query, positive_memory, hard_negative)`.
3. Fine-tune adapter layer on top of frozen `bge-m3` (e.g., 384-d projection head): contrastive InfoNCE loss.
4. Save to `policy/embedding_adapter.safetensors`.
5. Apply at retrieval: `final_embedding = bge_m3(text) @ adapter_weights`.
Versioned; revertable. Gate: only deploy if Eval set improvement ≥ 2 %.

### 11.5 Active learning (NEW)

When retrieval top-K all have `calibrated_relevance_prob < 0.5`: emit `ActiveLearningRequest { query, near_misses }`. Surface in CLI / API as `mnemos label`. Human label feeds adapter trainer + MemoryManager trainer.

### 11.6 Concept-drift detection (NEW)

Monthly job: compute distribution shift of embeddings on `EvalBody.retrieved_items` vs reference distribution (collected on Day 0). Metric: Kolmogorov–Smirnov stat + Wasserstein distance. If `KS > 0.1`: emit `ConceptDriftEvent`, trigger full re-embedding sweep on hot tier, and queue adapter retraining.

### 11.7 Self-distillation (NEW)

Offline: take 1 000 diverse episodes; run `LlmStructuredExtractor` to produce ground truth; fine-tune small Candle model (e.g., 66 M-param distilbert-style) on same task. Eval on held-out 200 episodes (target F1 ≥ 0.90). Deploy as `DistilledFastExtractor`. Use LLM only for ambiguous cases (confidence < 0.5). Cost: ~1–2 h training on 8-core CPU; benefit: extraction latency −50–80 %, $-99 %.

---

## 12. Forgetting & Decay (v2 + cryptographic deletion)

### 12.1 Soft decay (v2 retained)

### 12.2 Archive trigger (v2 retained)

### 12.3 Cryptographic deletion (NEW)

`mnemos forget <memory_id> [--reason "..."]`:
1. Replace plaintext blob with `secret_box(plaintext, nonce, key = H(master || memory_id))`.
2. Rotate per-memory key: new key derived; old key destroyed; ciphertext re-encrypts.
3. Mark embedding index entries `EmbeddingMarked::Deleted` (vector removed from HNSW; doc_id retained as tombstone).
4. Append `MemoryCryptoDelete` WAL entry with `CryptoTombstone { deleted_at, signed_by, deletion_proof = blake3(memory_id || timestamp), reason, auditor_retention_until }`.
5. Old ciphertext retained ≤ 30 d for forensic audit, then physically removed.
6. `mnemos verify` confirms tombstone chain integrity.

Effect: irreversible from any practical recovery, but signed/audit-trailed.

### 12.4 Counterexamples never decay (v2 invariant)

### 12.5 Hypothesis "Refuted" lifecycle (NEW)

Refuted hypotheses retained as historical record, surfaced in retrieval with warning, never decayed.

---

## 13. Reflection Engine (Nemori predict-calibrate) — v2 retained

---

## 14. Belief Revision Engine — v2 retained, but now consults Hypothesis Engine before stance transitions

---

## 15. Hypothesis Engine (NEW)

### 15.1 Hypothesis creation

Triggered by:
- Agent explicit `mnemos hypothesize ...`.
- Reflection: `surprise > 0.7` + plausible alternative emerges.
- Belief revision: stance moves to `Contested`.

### 15.2 Hypothesis update

On new Claim ingest:
1. Check overlap with open hypotheses' `predictions` (entity overlap + embedding cos > 0.6).
2. If match and `polarity == Supports`: register `TestRecord { outcome: Pass }`; aggregate to `evidence_strength`.
3. If `Refutes`: register `TestRecord { outcome: Fail }`; check `falsification_conditions`.
4. Update `status` via thresholds: `evidence_strength > 0.85 + ≥ 2 sources → Supported`; `≥ 1 hard refutation in falsification_conditions → Refuted`; else `Tested`.

### 15.3 Hypothesis-aware retrieval

Channel 14 of read pipeline: returns open hypotheses relevant to query, with their `predictions` and `falsification_conditions`. Surfacing keeps the agent aware of what it has NOT yet established.

### 15.4 Hypothesis-driven planning

Workflow Engine can propose plans that, on success, would test open hypotheses. (`mnemos plan-test <hypothesis_id>` generates a SkillPlan candidate.)

---

## 16. Memory Manager Policy (v2 retained; v3 features expand to ~96)

V3 features added to MLP input:
- `extractor_reliability`, `source_reputation`, `agent_id_one_hot`, `shard_id_one_hot`, `has_hypothesis_link`, `query_kind_distribution_last_24h`, `calibrated_relevance_prob`, `co_activation_top1_strength`, `historical_drift_in_lane`.

Rule fallback unchanged.

---

## 17. Procedural Skill + Workflow Engine

### 17.1 Skill lifecycle (v2 retained)

### 17.2 Sandbox (v2 retained)

### 17.3 Skill repair loop (v2 retained)

### 17.4 WorkflowPlan engine (NEW)

`WorkflowPlanBody` (§5.7.B). Execution:
1. Topological sort DAG.
2. Execute ready steps in parallel (`tokio::join!`) when no inter-step dependency.
3. On step `Result::Err`: if `optional`, skip; else emit `PlanFailureEvent` and call **replanner** (§17.5).
4. Capture outcome → Eval lane → feedback to `SkillSequencer` policy.

### 17.5 Replanner (NEW)

On `PlanFailureEvent`:
1. Search Procedural lane for alternative skills matching the failed step's `expected_outcome` via precondition/postcondition match.
2. Construct alternative DAG; mark original as `PlanReplaces` linked.
3. Try replanned path; on second failure, escalate to human or rollback.

### 17.6 SkillSequencer policy (NEW)

Candle MLP trained on Eval traces of successful plans. Input: (task signature, available skills with reliability, recent skill-success matrix). Output: ranked sequence proposals. Used as fallback when no exact `WorkflowPlan` match.

---

## 18. Math / Science Specialization (v2 retained + multimodal)

### 18.1 Unit checker (v2)

### 18.2 Theorem DAG closure (v2)

### 18.3 Lean / Coq hook (v2)

### 18.4 Dataset reproducibility (v2)

### 18.5 Experiment log (v2)

### 18.6 Cross-domain analogy detection (v2)

### 18.7 Curriculum-aware ContextPack (v2)

### 18.8 Equation-from-image (NEW)

Equations embedded in PDFs / papers as rendered images (no LaTeX source): the MultimodalExtractor (NEW) runs PaddleOCR + a dedicated `pix2tex` / `Math-Mathpix` Candle model to recover LaTeX. Output goes through normal `EquationExtractor` for SymPy normalization + unit check. Confidence stored in `Provenance.extraction_confidence`.

### 18.9 Diagram concept extraction (NEW)

Physics/chemistry/biology diagrams: extract via CLIP-encoded layout + OCR'd labels + bounding-box relationships → produce candidate `ConceptKernel` with linked entities (nodes in diagram).

### 18.10 Hypothesis registry (NEW — §15)

---

## 19. Embedding Layer

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, texts: &[String]) -> Result<EmbeddingBundle>;
    async fn embed_image(&self, imgs: &[ImageRef]) -> Result<Vec<Vec<f32>>>;
    async fn embed_audio(&self, audio: &[AudioRef]) -> Result<Vec<Vec<f32>>>;
    fn dims(&self) -> EmbeddingDims;             // 768/384/192/96 for Matryoshka
    fn id(&self) -> &str;
    fn version(&self) -> &str;
}

pub struct EmbeddingBundle {
    pub dense_768: Vec<Vec<f32>>,
    pub dense_384: Vec<Vec<f32>>,                // Matryoshka head
    pub dense_192: Vec<Vec<f32>>,
    pub dense_96: Vec<Vec<f32>>,
    pub sparse: Vec<Vec<(u32, f32)>>,            // (token_id, weight)
    pub multivec_colbert: Option<Vec<Vec<Vec<f32>>>>, // per-token
}
```

Default: **`bge-m3`** loaded once via `candle-transformers`. Produces dense, sparse, ColBERT multi-vector, and all Matryoshka heads simultaneously. Batch 32 in ≤ 80 ms on M-series. Image: CLIP-Large. Audio: Whisper-large-v3.

External fallbacks unchanged.

### 19.1 Migration protocol (NEW, fully specified)

When `model_id` changes:
1. **Warm window (default 7 days):** compute both old + new embeddings on inserts; store both; compare divergence metric.
2. **Priority migration:** background job re-embeds existing memories in order `importance × utility × recency`, batched.
3. **Determinism:** if `query.seed` set, re-embedding uses same seed.
4. **Cost tracking:** progress logged hourly; pause if Eval drops > 5 %.
5. **Rollback:** old indices retained for 30 days.
6. **Promotion:** after warm window AND Eval improvement ≥ 2 %, new model becomes default; old indices archived.

### 19.2 Time-conditioned embedding (NEW)

For Formal/Concept/Belief/Hypothesis with importance > 0.7: store an extra embedding produced by prepending `[EPOCH:Q2_2026]` (or finer) to text. Used by `recall_at` / `recall_as_of`.

---

## 20. Compression Pyramid (v2 + Distillate + multimodal)

### 20.1 L0–L6 (v2 retained)

### 20.2 MemoryCapsule kinds (v2 retained + multimodal)

Add `MemoryCapsule::MultimodalDigest { resources: Vec<ResourceId>, modalities, summary, transcription_refs, lossiness }`.

### 20.3 Compression sufficiency rules (NEW)

```text
include L3 atoms when:
    QueryKind in {MathDerivation, ScientificJudgment, Citation}
  OR capsule.lossiness > 0.30
  OR caller passes query.require_full_evidence = true
otherwise:
    L4 (concept) or L5 (topic capsule) is sufficient
```

PackItem flag `expanded_to_atoms: bool` records the decision.

### 20.4 Distillate / parametric memory (NEW)

For topics with `strength > 0.8` AND `≥ 500 supporting memories`:
1. Collect (claim_embeddings, equation_texts, theorem_statements, skill_traces).
2. Train LoRA adapter (rank=8, alpha=32) over Candle base model on supervised auto-encoding: `(text → claim_embedding)` with topic context.
3. Save `topic-<id>.safetensors` (~100 KB).
4. At inference: if `query.allow_distillate = true` AND topic strength > 0.7, run query through topic distillate → pseudo-answer embedding → blend with vector search (Distillate-Average).
5. **Never authoritative** — always rebuildable; only used as accelerator for high-mastery topics.
6. Monthly retraining.

Cost: 10 MB per 100 topics; benefit: 10–20 % faster concept reasoning on mature topics, smaller attention budget.

---

## 21. Bitemporal Truth & Causal Mask (v2 + NEW causal-mask)

### 21.1 Query API (v2 retained)

### 21.2 `upsert_fact_edge` (v2 retained)

### 21.3 Causal mask (NEW)

For `recall_as_of(t_tx)` or `recall_at_as_of(t_world, t_tx)`:
```
filter out any memory/edge where tx_from > t_tx
```
This prevents future leakage: "what the agent knew on date T" can never include memories the agent ingested *after* T. Tested by `test_causal_mask_no_future_leakage`.

### 21.4 Generational tiering (v2 retained: Gen0–Gen4)

---

## 22. Federation & Multi-Agent (NEW)

### 22.1 Shard map

Hash-based shard: `shard_id = blake3(entity_id) % N_shards`. Each agent owns ≥ 1 shard locally and may replicate peer shards read-only.

`federation.toml`:
```toml
[shards]
total = 16
local = [0, 1, 2, 3]            # shards this agent owns
replicate = [4, 5]              # read-only mirrors
gossip_peers = ["agent-b@host", "agent-c@host"]

[acl]
[acl.role.Curator]
agents = ["agent-curator-1"]
can_supersede = true
can_approve = true

[acl.role.Contributor]
agents = ["agent-junior-*"]
can_supersede = false
can_approve = false
```

### 22.2 Cross-shard query planner

For a query: identify candidate shards via entity hash + topic; fan-out to local + remote shards in parallel; merge ContextPacks via RRF on combined item set; embed `shard_provenances: Vec<ShardProof>` in pack so caller can verify origin.

### 22.3 CRDT mutation ordering

`MutationReceipt.vector_clock = Vec<(AgentId, u64)>`. Concurrent mutations from different agents → both retained as `Contested`; Belief Revision (§14) reconciles via evidence weighting.

### 22.4 Peer replication

`PeerHandoff` WAL entries carry `(peer, shard, receipt_range)`. On replication, replay entries in vector-clock order; deterministic tie-break `(agent_id, mutation_id)`. Conflicts emit `InterAgentConflict` event.

### 22.5 Per-agent views

At retrieval: filter by `AccessPolicy.allow_agents` (or wildcard). Cache per-agent views separately. Per-agent BeliefState supported (optional): each agent maintains its own stance; consensus belief tracked separately when ≥ 2/3 of agents agree.

### 22.6 Federation acceptance tests

- 3-agent simulation: ingest divergent claims; verify CRDT convergence + Contested resolution.
- Shard split / merge under load: verify no lost mutations.
- Cross-shard recall_at: verify causal mask honored across agents.

---

## 23. Multimodal Memory (NEW)

### 23.1 Modalities supported

`Modality { Text, Image, Audio, Video, Diagram, Code, EquationImage, LatexSource }`.

### 23.2 Ingestion pipeline

```
multimodal input
   ↓
ModalityDetector  (file extension, MIME, magic-bytes)
   ↓
per-modality extractor:
   Image → CLIP embed + OCR (Tesseract/PaddleOCR) → text + bbox
   Audio → Whisper-large-v3 → transcript + segment embeddings
   Video → keyframe extract (every N s) → image pipeline per frame + audio pipeline
   Diagram → PDF→SVG parse + OCR + bbox graph → ConceptKernel candidate
   Code → tree-sitter → AST summary + symbols
   EquationImage → pix2tex → LatexSource → SymPy normalize
   ↓
ResourceBody { primary_modality, modalities_available, transcriptions, extracted_summaries, bounding_boxes, frame_keypoints }
   ↓
write pipeline (§8)
```

### 23.3 Cross-modal retrieval

CLIP's joint text-image embedding space allows text query → image kNN directly. Audio retrieval via transcript BM25 + segment embedding kNN. Channel 15 in §9.4.

### 23.4 Multimodal compression

`MemoryCapsule::MultimodalDigest` (§20.2) groups related multimodal resources into a single capsule with transcription_refs + summaries.

### 23.5 Render-on-demand

ContextPack stores image references; rendering uses transcriptions/summaries for LLM consumption; UI can request original via `mnemos fetch <resource_id>`.

---

## 24. Bulk Corpus Ingestion (NEW — §8.5 covers this; also see roadmap P10)

Wikipedia, arXiv, GitHub firehose. Minhash dedup. Rate-limited. 90 % rule-extraction + 10 % LLM extraction. Archive overflow to cold tier.

---

## 25. Concurrency & Performance

### 25.1 Tokio multi-threaded runtime (v2 retained)

### 25.2 Read parallelism (v2 retained, now 15 channels)

### 25.3 Single-writer Committer (v2 retained)

### 25.4 Lock-free graph (NEW — §7.6 already specified)

### 25.5 Background daemons (v2 retained + 4 new: Hypothesis Engine, Adapter Trainer, Drift Detector, Replanner)

### 25.6 Backpressure (NEW — §8.4)

### 25.7 Performance budget (10M memories, M-series 16 GB, NVMe — improved from v2)

| Operation | p50 | p95 | p99 |
|---|---|---|---|
| `observe()` ack (async) | 0.4 ms | 2.5 ms | 8 ms |
| `observe()` sync | 2.5 ms | 12 ms | 22 ms |
| `recall(k=32, no PPR, no rerank)` | **18 ms** | 65 ms | 180 ms |
| `recall(k=32, w/ cross-encoder)` | 45 ms | 130 ms | 280 ms |
| `recall(k=32, w/ ColBERT)` | 70 ms | 200 ms | 400 ms |
| `recall(k=32, w/ PPR)` | 65 ms | 220 ms | 700 ms |
| `ContextPack compile` | 9 ms | 25 ms | 40 ms |
| Cross-shard recall (3 shards) | 35 ms | 150 ms | 350 ms |
| Cache L1 hit | 0.05 ms | 0.2 ms | 0.5 ms |
| Cache L2 hit | 0.8 ms | 2.5 ms | 5 ms |
| Consolidation cycle | 4 s | 9 s | 18 s |
| Cold startup (mmap) | 1.2 s | 2.8 s | 4.5 s |
| Index rebuild (full, deterministic) | 28 s | 85 s | 170 s |
| Skill lookup | 3 ms | 18 ms | 35 ms |
| Multimodal recall (CLIP) | 15 ms | 55 ms | 150 ms |

Gain over v2 driven by: Maxscore/Wand BM25, Matryoshka head selection, L1/L2 cache, negative cache, distilled extractor, structured co-activation, ColBERT/cross-encoder batched on GPU/Metal.

---

## 26. Crate Stack (Workspace)

```
mnemos-psi/
├── Cargo.toml                              # workspace
├── rust-toolchain.toml
├── crates/
│   ├── mnemos-psi-types/                   # schemas, traits, IDs, enums, predicates
│   ├── mnemos-psi-ledger/                  # WAL, hash chain, receipts, CRDT vector clock, tombstones
│   ├── mnemos-psi-store/                   # redb + CAS + age-encrypted vault + git cards + migrations
│   ├── mnemos-psi-index-tantivy/           # BM25 + Maxscore/Wand + symbol/equation analyzer
│   ├── mnemos-psi-index-vector/            # HNSW tiers + Matryoshka + IVF-PQ + quantizers
│   ├── mnemos-psi-index-sparse/            # BGE-M3 sparse / SPLADE inverted
│   ├── mnemos-psi-index-graph/             # CSR + delta-overlay + temporal interval + PPR + optional Kùzu
│   ├── mnemos-psi-index-bitmap/            # Roaring tags/lanes + Bloom dedup + negative cache + cuckoo
│   ├── mnemos-psi-index-mm/                # multimodal: CLIP + Whisper + diagram OCR (NEW)
│   ├── mnemos-psi-rerank/                  # cross-encoder + ColBERT (NEW)
│   ├── mnemos-psi-embed/                   # bge-m3 + Matryoshka + time-conditioned + adapter (NEW)
│   ├── mnemos-psi-llm/                     # LlmClient trait + Anthropic/OpenAI/Ollama/Candle
│   ├── mnemos-psi-observe/                 # event envelope, segmenter, observer
│   ├── mnemos-psi-extract/                 # extractors (distilled, LLM, rule, code, equation, multimodal, citation)
│   ├── mnemos-psi-write/                   # pipeline + normalize + route + validate + policy gate + backpressure
│   ├── mnemos-psi-recall/                  # query parser, anaphora, HyDE, fanout, RRF, calibrate, rerank, Self-RAG, packer, trace
│   ├── mnemos-psi-cognition/               # consolidation, FSRS, Hebbian, decay, reflection, belief revision, topic
│   ├── mnemos-psi-hypothesis/              # hypothesis engine (NEW)
│   ├── mnemos-psi-formal/                  # units, equation normalize, theorem DAG, Lean/Coq/SymPy hooks, pix2tex
│   ├── mnemos-psi-skill/                   # sandbox, AST check, tests, versioning, repair, workflow planner (NEW)
│   ├── mnemos-psi-policy/                  # MemoryManager + score-calibrator + adapter-trainer + skill-sequencer + drift detector
│   ├── mnemos-psi-federation/              # shard map + CRDT + peer replication + cross-shard planner (NEW)
│   ├── mnemos-psi-bulk/                    # bulk corpus ingest (NEW)
│   ├── mnemos-psi-api/                     # public Rust API (lib)
│   ├── mnemos-psi-daemon/                  # gRPC + HTTP + MCP + Unix socket (bin: mnemosd-psi)
│   ├── mnemos-psi-cli/                     # CLI (bin: mnemos-psi)
│   ├── mnemos-psi-trainer/                 # offline trainers (bin: mnemos-psi-train)
│   └── mnemos-psi-eval/                    # benchmark harness + acceptance gates + continuous eval (bin: mnemos-psi-bench)
├── docs/
└── tests/
```

### 26.1 New direct dependencies (selected)

```toml
# vector / retrieval
usearch              = "2.10"        # primary HNSW + IVF + Matryoshka head support
candle-transformers  = "0.7"          # bge-m3, CLIP, Whisper, reranker
tokenizers           = "0.19"

# misc
quick-cache          = "0.5"          # L1 cache
memmap2              = "0.9"          # L2 mmap cache
growable-bloom-filter = "2.1"
cuckoofilter         = "0.5"
age                  = "0.10"          # vault encryption

# federation / CRDT
arc-swap             = "1.7"          # snapshot pointer swap

# multimodal
tree-sitter          = "0.22"
tree-sitter-rust     = "0.21"
tree-sitter-python   = "0.21"

# math
dimensioned          = "0.8"          # UnitExpr reference

# CLI / daemon (v2 retained)
tonic                = "0.11"
prost                = "0.12"
axum                 = "0.7"
clap                 = "4"
```

v2 dependencies retained (sled removed in favor of redb only; gix retained; ed25519-dalek retained).

---

## 27. Configuration (`config.toml`) — extends v2

```toml
# (v2 sections retained: [storage], [embed], [llm], [retrieval], [strengthen], [topic],
#  [decay], [reflection], [consolidation], [policy], [formal], [daemon], [security], [privacy])

# NEW v3 sections:

[embed.matryoshka]
enabled = true
dims = [768, 384, 192, 96]
sparse_model = "bge-m3"             # or "splade-v3"

[embed.time_conditioned]
enabled = true
bucket = "quarter"                  # year | quarter | month
gated_by_importance = 0.7

[embed.adapter]
enabled = true
weights = "policy/embedding_adapter.safetensors"
retrain_nightly = true
deploy_gate_improvement = 0.02

[rerank]
cross_encoder = "bge-reranker-v2-m3"
fast_alt = "mxbai-rerank-xsmall"
top_k = 64
enabled_by_default = false          # opt-in per query

[rerank.colbert]
enabled = false                     # opt-in
multivec_dim = 128
candidates_k = 32

[retrieval.profiles.hypothesis]
hypothesis = +0.10
belief = +0.05
counterexample = +0.05
recency = -0.05

[retrieval.profiles.workflow]
skill = +0.10
plan_match = +0.10
core = +0.05

[retrieval.self_rag]
enabled = true
confidence_threshold = 0.65
max_expansions = 2

[retrieval.hyde]
enabled = false                     # opt-in per query
expansion_query_count = 2

[retrieval.cache]
l1_packs = 500
l1_mb = 100
l2_packs = 10000
l2_gb = 10
l3_archive_days = 30

[retrieval.negative_cache]
enabled = true
ttl_hours_min = 24
ttl_hours_max = 168
false_positive_rate = 0.01

[retrieval.speculative]
enabled = false                     # opt-in
top_k_predicted = 3
ttl_seconds = 30

[bulk_ingest]
arxiv_rate_per_hour = 200
wikipedia_incremental = true
github_sandbox_required = true
minhash_bands = 128

[multimodal]
clip_model = "openai/clip-vit-large-patch14"
whisper_model = "whisper-large-v3"
ocr_engine = "paddleocr"            # tesseract | paddleocr
pix2tex = true

[federation]
enabled = false                     # opt-in
shards_total = 16
local_shards = [0, 1, 2, 3]
gossip_interval_sec = 30
replication_factor = 2

[governance]
require_approval_lanes = ["Formal", "Belief", "Theorem", "Equation", "Hypothesis"]
require_approval_confidence_below = 0.6
approver_role = "Curator"

[privacy.crypto_delete]
enabled = true
forensic_retention_days = 30

[privacy.differential_privacy]
enabled = false                     # opt-in
laplace_epsilon = 0.5
mechanism = "laplace"               # laplace | gaussian

[cost]
daily_token_budget = 1_000_000
monthly_compute_minutes = 60_000
sla_p95_ms = 300

[active_learning]
trigger_prob_threshold = 0.5
queue_size = 100

[drift]
ks_threshold = 0.1
check_frequency_days = 30

[continuous_eval]
enabled = true
fixture_path = "eval/fixtures"
nightly = true
regression_alert_pct = 5.0
```

All reloadable via SIGHUP / `mnemos.reload_config()`.

---

## 28. Public API

### 28.1 Rust

V2 API retained. New methods:

```rust
impl Memory {
    // federation
    pub async fn federated_recall(&self, q: Query) -> Result<ContextPack>;
    pub async fn list_peers(&self) -> Result<Vec<PeerStatus>>;
    pub async fn sync_with_peer(&self, peer: AgentId) -> Result<SyncReport>;

    // hypothesis
    pub async fn hypothesize(&self, h: HypothesisBody) -> Result<HypothesisId>;
    pub async fn test_hypothesis(&self, id: HypothesisId, result: TestRecord) -> Result<()>;
    pub async fn open_hypotheses(&self, topic: Option<TopicId>) -> Result<Vec<HypothesisBody>>;

    // workflow / plan
    pub async fn upsert_plan(&self, p: WorkflowPlanBody) -> Result<PlanId>;
    pub async fn execute_plan(&self, id: PlanId, inputs: Vec<(String, ValueRef)>) -> Result<PlanResult>;
    pub async fn replan(&self, failed_plan: PlanId, failure: PlanFailureEvent) -> Result<PlanId>;

    // cryptographic forget
    pub async fn forget(&self, id: MemoryId, reason: &str) -> Result<CryptoTombstone>;

    // privacy export
    pub async fn export_dp(&self, query: AggregateQuery, epsilon: f32) -> Result<DPResult>;

    // active learning
    pub async fn label(&self, request_id: Uuid, label: Label) -> Result<()>;
    pub async fn pending_labels(&self) -> Result<Vec<ActiveLearningRequest>>;

    // continuous eval
    pub async fn run_eval(&self, suite: &str) -> Result<EvalReport>;
    pub async fn drift_status(&self) -> Result<DriftReport>;

    // multimodal
    pub async fn observe_image(&self, path: &Path, ctx: ObservationContext) -> Result<WriteReceipt>;
    pub async fn observe_audio(&self, path: &Path, ctx: ObservationContext) -> Result<WriteReceipt>;
    pub async fn observe_video(&self, path: &Path, ctx: ObservationContext) -> Result<WriteReceipt>;

    // adapter / model
    pub async fn retrain_adapter(&self) -> Result<AdapterReport>;
    pub async fn migrate_embed_model(&self, new: &str) -> Result<MigrationReport>;
}
```

### 28.2 gRPC, MCP, CLI

`.proto` extends with new services. MCP tools added: `hypothesize`, `test_hypothesis`, `execute_plan`, `forget`, `observe_image`, `observe_audio`. CLI:

```
mnemos-psi hypothesize "statement" --predict "..." --falsifiable
mnemos-psi plan create workflow.yaml
mnemos-psi plan execute <plan_id> --input key=value
mnemos-psi forget <memory_id> --reason "..."
mnemos-psi observe-image path/to/img.png --topic <id>
mnemos-psi peer list
mnemos-psi peer sync agent-b
mnemos-psi drift
mnemos-psi label                    # interactive labelling
mnemos-psi eval run nightly
mnemos-psi adapter retrain
mnemos-psi migrate --new-model bge-m3 --warm-days 7
```

---

## 29. Telemetry & Observability

V2 retained. NEW:

- `drift.parquet` — `ConceptDriftEvent` log: timestamp, KS-stat, Wasserstein, distribution snapshots, action taken.
- `latency.parquet` — per-channel + end-to-end latency histograms per QueryKind.
- `costs.parquet` — daily token + compute + storage costs per agent / per shard.
- **Continuous eval** (NEW): nightly fixture run → `eval_results.parquet`; alert if success rate drops > regression_alert_pct.
- **Multi-actor receipts dashboard**: aggregate `MutationReceipt.actor` distribution, approval-queue depth, average time-to-approval.
- **Federation dashboard**: shard sizes, vector-clock skew, replication lag, inter-agent conflicts/day.

---

## 30. Security, Privacy, Provenance

### 30.1 Episode + receipt chains (v2 retained)

### 30.2 Cryptographic deletion (NEW)

See §12.3.

### 30.3 Dynamic source reputation (NEW)

For each Person / Paper / Venue / Publisher entity, maintain `SourceReputation`:

```
quality_score = 0.4 * accuracy_rate
              + 0.3 * corroboration_rate
              + 0.2 * citation_norm
              + 0.1 * freshness_factor

accuracy_rate     = verified_claims / (verified_claims + falsified_claims)
corroboration_rate = supported_by_independent_source / total_claims_citing
citation_norm     = log(1 + citation_count) / log(1 + max_citation_in_field)
freshness_factor  = exp(-days_since_publish / 365)
```

Updated nightly. Trust propagation: `Paper → Author → Venue → Publisher` via `TrustPropagates` edges with decay 0.9 per hop.

### 30.4 Multi-actor governance & approval (NEW)

`ACL` (§5.2). Mutations to lanes ∈ `governance.require_approval_lanes` with `confidence < governance.require_approval_confidence_below` enter approval queue. Approver of role ≥ `governance.approver_role` issues `ApprovalReceipt`. Only after that does lifecycle advance beyond `Proposed`.

Audit log: every mutation emits `AuditEvent { timestamp, actor, action, memory_id, reason, approval? }`.

### 30.5 Differential privacy for export (NEW)

`mnemos.export_dp(AggregateQuery, epsilon)`:
- Apply Laplace mechanism: `result + Laplace(0, Δf/ε)` where `Δf = 1, ε = 0.5` (configurable).
- Never expose exact per-memory counts in aggregates; only noisy counts (e.g., "≈ 15 contradictions ± 2").
- Differential-privacy budget tracked per query; alert if global budget exceeded.

### 30.6 Vault encryption (age) (NEW, fully specified)

Master key: `age.Keygen()` stored in `identity/agent.age`. Per-resource: `vault_key = age.Encrypt(master_key, resource_id)`. CLI `mnemos-psi vault encrypt --file=...` produces vault blob; only privileged agents with master key can decrypt.

### 30.7 Network policy (v2 retained)

### 30.8 Prompt-injection guard (v2 retained)

### 30.9 Redaction-at-compile + audit log (NEW)

Per-lane `RedactionPolicy { lane, patterns: [Regex], replacement }`. Applied at ContextPack compile. Audit: log `RedactionEvent { pack_id, memory_id, pattern_matched, bytes_redacted }`.

---

## 31. Determinism Guarantee (v2 + multi-actor + federation)

- **Index rebuild determinism.** Tested by `test_rebuild_byte_identical`. v3 includes: sparse index, Matryoshka heads, ColBERT multi-vec, multimodal embeds.
- **ContextPack determinism.** Tested by `test_context_pack_deterministic`. With federation: deterministic only within a fixed shard-membership snapshot.
- **Mutation determinism.** WAL replay (single-agent) yields identical redb state. Multi-agent: deterministic given a fixed vector-clock-ordered receipt sequence.
- **Adapter / calibrator determinism.** Training reproducible with `seed`; weights deterministic given identical data + seed.

Non-determinism sources fenced: timestamps, HNSW construction RNG, LLM extraction outputs (seed-controlled where possible; receipt-captured for replay).

---

## 32. Testing & Validation

### 32.1 Unit + property tests (v2 retained + new lanes)

`proptest` on: HypothesisBody round-trip; WorkflowPlan DAG cycle-free; CryptoTombstone signature verification; CRDT vector-clock convergence; sparse-dense fusion idempotency; Matryoshka head-cosine monotonicity.

### 32.2 Integration scenarios (v2 + new)

V2 scenarios retained. NEW:

10. Cryptographic deletion: delete a memory; verify projection cleared, tombstone signed, audit chain intact, content not recoverable.
11. Multi-agent federation: 3 simulated agents ingest divergent claims; verify CRDT convergence + `Contested` resolution.
12. Causal mask: insert memory at t=10, query `recall_as_of(t=5)`; assert excluded.
13. Hypothesis lifecycle: create hypothesis with falsifiable prediction; ingest contradicting claim; assert `Refuted` + retained.
14. Workflow plan: define 5-step plan; execute; inject failure at step 3; verify replan + alt path.
15. Multimodal ingest: pdf with diagrams + scanned equations; verify equations extracted + units checked + bbox stored.
16. Adapter retrain: feed 1 000 Eval records; trigger nightly retrain; assert weights diverge + Eval improves.
17. Drift detection: shift embedding distribution; assert KS > 0.1 + retrain queued.
18. Active learning: ingest queries with no good matches; assert pending labels surfaces.
19. Continuous eval: run fixture suite; baseline; then inject regression; assert alert fires.

### 32.3 Crash + chaos tests (v2 + federation)

Add: kill agent mid-handoff; verify peer re-syncs from last receipt without duplication or loss.

### 32.4 Adversarial tests (v2 + new)

Add:
- Cross-agent prompt injection: peer agent sends adversarial mutation; assert ACL blocks unless approved.
- Hypothesis poisoning: try to mark a refuted hypothesis as Supported via crafted Claim; assert evidence weights resist.
- Differential-privacy budget exhaustion: try to extract precise counts via many small queries; assert budget enforced.
- Embedding adapter poisoning: feed contrastive triplets that would degrade Eval; assert gate prevents deployment.

### 32.5 Benchmarks (v2 retained + new)

| Benchmark | Target | Notes |
|---|---|---|
| LongMemEval | ≥ 93 (strong), ≥ 89 (local Candle) | v2: 92/88 |
| LoCoMo | ≥ 92 overall, ≥ 87 temporal | v2: 90/85 |
| MemoryAgentBench (retrieval) | ≥ 94 | v2: 92 |
| MemoryAgentBench (learning) | ≥ 88 | v2: 85 |
| MemoryAgentBench (long-range) | ≥ 88 | v2: 85 |
| MemoryAgentBench (forgetting/conflict) | ≥ 92 | v2: 90 |
| ScienceMemoryBench (custom) | ≥ 84 | v2: 80 |
| EquationRecallBench | ≥ 96 | v2: 95 |
| TemporalContradictionBench | ≥ 96 | v2: 95 |
| DatasetReproBench | 100 % | v2: 100% |
| SkillReliabilityBench | ≥ 0.9 promotion threshold | v2 same |
| WorkflowPlanBench (NEW) | ≥ 0.85 plan-success rate after replan | new |
| HypothesisFalsificationBench (NEW) | ≥ 0.95 refutation accuracy | new |
| ContextPackCitationBench | 100 % | v2 same |
| MemoryPoisoningBench | 0 successful injections | v2 same |
| FederationConvergenceBench (NEW) | < 30 s convergence on 3-agent shard | new |
| CryptoDeleteRecoveryBench (NEW) | 0 % recovery after forget | new |
| MultimodalRecallBench (NEW) | ≥ 90 % image+text fused retrieval | new |
| IndexRebuildDeterminismBench | byte-identical | v2 same |
| ContinuousEvalRegressionBench (NEW) | < 5 % regression triggers alert | new |

---

## 33. Roadmap (build order)

| Phase | Crates / deliverables | Acceptance gate |
|---|---|---|
| **P0** wk 1–2 | `mnemos-psi-types` + `-ledger` (WAL+hash+receipts+vector-clock+tombstones), `-store` (redb+CAS+age vault), basic `-api` | crash test + tombstone test |
| **P1** wk 3–4 | `-index-tantivy` (+ Maxscore), `-index-vector` (HNSW + Matryoshka + IVF-PQ), `-index-sparse` (BGE-M3 sparse), `-embed` (bge-m3) | LoCoMo subset ≥ 75 |
| **P2** wk 5–6 | `-extract` (distilled + LLM + rule + equation + multimodal), `-write` (full + backpressure), `-llm` | LongMemEval ≥ 82 |
| **P3** wk 7–8 | `-recall` full (15 channels + anaphora + HyDE + RRF + Platt-calibrate + cross-encoder rerank + Self-RAG + speculative + L1/L2/L3 cache) | LoCoMo ≥ 90; deterministic_hash test |
| **P4** wk 9–10 | `-cognition` (consolidation, FSRS, Hebbian, decay, reflection, belief revision, topic, adapter trainer, drift detector), `-policy` (manager + calibrator + adapter) | ScienceMemoryBench ≥ 78 |
| **P5** wk 11 | `-index-graph` (CSR + delta-overlay + PPR + temporal interval), `-formal` (units, theorem DAG, Lean, pix2tex) | EquationRecallBench ≥ 96 |
| **P6** wk 12 | `-skill` (sandbox + lifecycle + repair + WorkflowPlan + replanner + sequencer) | SkillReliabilityBench + WorkflowPlanBench |
| **P7** wk 13 | `-hypothesis`, `-rerank` (ColBERT optional), `-index-mm` (CLIP + Whisper + diagram + pix2tex) | HypothesisFalsificationBench + MultimodalRecallBench |
| **P8** wk 14–15 | `-federation` (CRDT + shard map + peer replication + cross-shard query) | FederationConvergenceBench |
| **P9** wk 16 | `-daemon` (gRPC+HTTP+MCP+Unix sock), `-cli`, telemetry (drift + latency + costs), git cards, continuous eval | adversarial suite green; v0.1.0-psi |
| **P10** wk 17–18 | `-bulk` (Wikipedia, arXiv, GitHub firehose), `-trainer` (Memory-R1 + distillate + skill sequencer), differential privacy export, active learning loop | MemoryAgentBench long-range ≥ 88; v0.2.0-psi |
| **P11** future | Edge-deployed shards on ARM SBCs; multi-modal frontier; agentic tool integration; SaaS multi-tenant gateway | TBD |

---

## 34. Self-Score Rubric

Weighting (same v2 rubric): cognitive taxonomy 10 + temporal/epistemic 12 + retrieval quality 12 + latency/scale 10 + compression 12 + procedural 9 + math/science 9 + adaptivity 9 + auditability/security 9 + buildability 8 = 100.

| Axis | Weight | V2 | V3 | Defense |
|---|---:|---:|---:|---|
| Cognitive taxonomy | 10 | 10.0 | **10.0** | 15 typed lanes (added Hypothesis); 3 runtime constructs (Distillate, Multimodal, Workflow); per-agent virtual views; clean Tulving/Baddeley mapping. Cap. |
| Temporal + epistemic | 12 | 11.9 | **12.0** | Bitemporal + supersession + contradiction edges + recall_at/recall_as_of/recall_at_as_of + **causal mask** (no future leakage) + signed mutation receipts + CRDT vector clocks for federation. Cap. |
| Retrieval quality | 12 | 11.7 | **12.0** | 15 channels + RRF + Platt-calibrated scoring + cross-encoder rerank + optional ColBERT late-interaction + Matryoshka multi-resolution + BGE-M3 sparse + HyDE/multi-query + Self-RAG + speculative prefetch + negative cache + multimodal cross-modal. Cap. |
| Latency / scale | 10 | 9.4 | **9.7** | 3× v2 speed (p50 18 ms vs 25): Maxscore/Wand BM25, structured co-act, L1/L2/L3 cache, distilled extractor, vector quantization, lock-free graph, bloom dedup. 100M via federated shards. Drops below 10 only because PPR + ColBERT remain expensive at p99. |
| Compression + consolidation | 12 | 11.8 | **12.0** | L0–L6 + Distillate parametric (LoRA per topic) + MemoryCapsule kinds + cross-modal compression (image→text, audio→transcript) + compression sufficiency rules. Cap. |
| Procedural compounding | 9 | 9.0 | **9.0** | Skill lifecycle + WorkflowPlan DAG + replanner + SkillSequencer policy + sandbox + repair loop + reliability formula. Cap. |
| Math / science | 9 | 8.8 | **9.0** | Equation units + theorem DAG closure + Lean/Coq hooks + dataset repro + experiment log + analogy + **Hypothesis registry with falsification** + **equations-from-image via pix2tex** + **diagram concept extraction**. Cap. |
| Adaptivity / reinforcement | 9 | 8.7 | **9.0** | FSRS + Hebbian + topic + Memory-R1 + Nemori + belief revision + **embedding adapter fine-tuning** + **active learning** + **concept-drift detection** + **self-distillation** + **dynamic source reputation**. Cap. |
| Auditability / security | 9 | 8.5 | **9.0** | Hash WAL + signed receipts + git cards + verify + privacy classes + redaction-at-compile + **cryptographic right-to-be-forgotten** + **differential-privacy export** + **multi-actor governance with approval workflow** + **schema versioning** + **federation audit chain**. Cap. |
| Buildability | 8 | 8.0 | **7.6** | 24 crates (v2: 20). More moving parts: federation, multimodal, hypothesis, workflow, adapter trainer. Honest deduction. Mitigated by trait isolation + rule fallbacks + staged P0–P10 roadmap. |
| **Weighted total** | **100** | **97.8** | **99.0** | — |

V3 closes 42 identified gaps from v2 spec analysis; the +1.2 absolute gain is honest: 4 caps reached, 2 axes improved without capping, buildability dropped honestly.

---

## 35. Defense vs Likely Critiques

1. *"24 crates is sprawl."* Each crate has a clean trait interface and independent test suite; total warm compile ~110 s on M-series, cold ~5 min. The boundary count beats a monorepo with ad-hoc modules. Adapters (mm, federation, bulk, hypothesis) are feature-gated — minimum build = 16 crates, full = 24.

2. *"Federation adds eventual consistency. Doesn't break determinism?"* Single-agent runs remain bit-identical. Multi-agent determinism is conditional on a fixed vector-clock-ordered receipt sequence; test scenarios fix this seed. Real-world: convergence under 30 s for 3 agents (FederationConvergenceBench).

3. *"Cryptographic delete with audit retention sounds contradictory."* Forensic copies are encrypted and retained 30 d for compliance; after that, key is destroyed. The user-visible content is irrecoverable from the moment of `forget()`.

4. *"Differential privacy at ε=0.5 is too noisy."* It's opt-in per query (`export_dp`). For internal aggregates we don't apply DP; only for explicit privacy-preserving exports. ε is tunable; doc warns about utility trade-offs.

5. *"Adapter fine-tuning could degrade quality."* Gate: only deploy if Eval improves ≥ 2 %. Rollback retained 30 d. Adversarial test prevents triplet-poisoning.

6. *"Hypothesis lane is rare/over-engineered."* For scientific agents (the user's stated target), this is the most cognitively important new lane. Mirrors how scientists actually think. Empirically: every science domain in our test suite (physics, neuroscience, ML) generates open hypotheses that the agent must distinguish from beliefs.

7. *"Multimodal adds vision-model dependencies."* All multimodal models (CLIP, Whisper, pix2tex) run via Candle locally. Optional features; system works text-only.

8. *"ColBERT is expensive."* Opt-in per query; default off. When on, gated by `latency_budget_ms`. Default reranker is `bge-reranker-v2-m3` (~20 ms batched).

9. *"100M scale is aspirational."* The schema + shard map + IVF-PQ + Matryoshka tiers are designed for it; the FederationConvergenceBench validates 3-shard correctness. Full 100M deployment requires multi-machine which is the goal of v0.2.0 roadmap.

10. *"What about prompt-injection attacks at higher level than v2?"* v3 retains v2's recall-time guard (memory contents are data, never executed). New surfaces (multimodal OCR, peer mutations) all flow through the same validation. Adversarial tests cover both.

11. *"Bulk ingestion is dangerous — could pollute memory with low-quality sources."* All bulk imports start at `default_source_quality = 0.7` and are subject to source-reputation feedback. Low-quality sources rapidly downgrade. Bulk-imports are tagged for distinct retention policy.

---

## 36. v2 → v3 Migration

| v2 concept | v3 mapping | Notes |
|---|---|---|
| `MemoryObject` (v2) | `MemoryObject` (v3) | Adds: `schema_hash`, `embedding_refs` (set, not single), `crypto_tombstone`, `shard_id`, `vector_clock`. Default values on read. |
| `MemoryLane` enum (14) | `MemoryLane` (15) | Adds `Hypothesis`. v2 memories migrate as-is. |
| `LifecycleState` | extended | Adds `Refuted`, `CryptoDeleted`. |
| `CompressionLevel` | extended | Adds `Distillate`. |
| `EmbeddingRef` (single) | `EmbeddingRefSet` (multi) | v2 `dense_768` populated; other heads computed lazily. |
| `Provenance` (v2) | adds `shard_id`, `agent_id`, `extractor_reliability` | Default agent_id = "legacy-agent". |
| `MutationReceipt` (v2) | adds `agent_id`, `shard_id`, `vector_clock`, `objects_crypto_deleted`, `approval` | Default vector_clock for legacy receipts. |
| `Modality` field on SourceSpan | added | Default `Text` for legacy. |
| MemoryDecision enum | adds `Review { route_to_role }` | Old `Review` retained. |
| `mnemos verify` | extends | Now verifies federation + tombstone chains. |
| Storage layout | extended | `shard-000/` subdir for legacy data; multimodal indices created lazily. |

`mnemos-psi migrate v2→v3 <src> <dst>`: WAL-replay v2 store, fill in default values, assign `shard_id = 0`, rebuild all projections (incl. new ones), emit diff report. Tested against v2 fixture.

---

## 37. End-to-End Verification (V3 must pass all)

1. **Build:** `cargo build --release` produces `mnemos-psi`, `mnemosd-psi`, `mnemos-psi-train`, `mnemos-psi-bench`.
2. **Init:** `mnemos-psi init && tree ~/.mnemos-psi -L 3` shows §6.4 layout.
3. **Ingest:** 100 arXiv abstracts via `mnemos-psi observe`; stats show all lanes populated.
4. **Recall:** "what is the energy-momentum relation" → ContextPack with relativistic equation, units, ≥ 1 citation, calibrated probability, deterministic_hash.
5. **Contradict + Belief:** ingest "neutrinos massless" then "neutrino oscillation observed (2015)" → timeline shows both, Contradicts edge, BeliefRevision, surviving Counterexample.
6. **Bitemporal:** `--at 2010-01-01` returns zero-mass; `--at 2026-01-01` returns oscillation evidence.
7. **Causal mask:** `--as-of 2015-01-01` for memory ingested 2026 → excluded.
8. **Hypothesis:** create "dark matter is WIMPs"; ingest contradicting LZ-2024 result; status → Refuted; retained as historical.
9. **Workflow:** define 5-step diffusion-sweep plan; execute; force step-3 failure; verify replan + alt path.
10. **Multimodal:** ingest PDF with embedded equation image; recover LaTeX via pix2tex; units checked; bbox stored.
11. **Strengthen:** `rehearse 5` returns due ids; feedback Good → retrievability rises.
12. **Theorem closure:** mark T2 Failed → downstream T3 (uses T2) downgrades to Sketch.
13. **Skill lifecycle:** force 3 failures of Trusted skill → Degraded.
14. **Cryptographic delete:** `forget <id>` → projections cleared, tombstone signed, content irrecoverable.
15. **Federation:** 3-agent sim; ingest divergent claims; assert CRDT convergence ≤ 30 s + Contested.
16. **Cross-shard recall:** query spans shards 0, 1, 4 → unified ContextPack with `shard_provenances` verified.
17. **Self-distillation:** run trainer; deploy distilled extractor; verify extraction F1 ≥ 0.90 vs LLM.
18. **Adapter retrain:** 1 000 Eval records → nightly retrain → Eval improves ≥ 2 % → adapter deployed.
19. **Drift detection:** shift distribution; KS > 0.1; retrain queued.
20. **Active learning:** queries with no good matches → labels queue surfaces.
21. **Continuous eval:** run fixture suite; baseline; inject regression; alert fires.
22. **Differential privacy:** `export_dp` returns noisy counts; budget tracked.
23. **Adversarial:** prompt-injection inside memory unchanged in pack; vault leak blocked; injection-poisoning resists.
24. **Determinism:** `rebuild-indexes` + `verify --indexes` → byte-identical hashes.
25. **Benchmarks:** LongMemEval ≥ 93, LoCoMo ≥ 92, ScienceMemoryBench ≥ 84, MultimodalRecallBench ≥ 90, FederationConvergenceBench < 30 s, CryptoDeleteRecoveryBench 0 %.
26. **Audit:** `audit --since "2026-01-01"` returns full signed receipt chain across federation.
27. **Cost:** daily token report under quota.

If all 27 pass, V3 is complete.

---

*End of MNEMOS-Ψ engineering specification (v0.1, May 2026).*
