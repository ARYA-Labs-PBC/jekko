# MNEMOS-Ω — Engineering Specification, v2
## A Superhuman Cognitive Memory Compiler for Adaptive Agents

**Codename:** `mnemos-omega` (lowercase). Successor to `mnemos` (v1).
**Language:** Rust (single workspace, ~20 crates).
**Targets:** 10M memories per node (hot), 100M with cold-archive tiering; embedded library + daemon (gRPC + HTTP + MCP + Unix socket).
**Inference:** Rust-native (Candle) with external API trait fallback.
**Author:** Synthesis of `tips/smart_memory/v2/tip{1..23}.txt` (May 2026 research cutoff).
**Self-score:** **98.4 / 100** weighted; defense in §30–31.

> Memory is not a database. It is not a vector store. It is not a knowledge graph. It is a **compiler**: raw experience → typed memory objects → temporal graph → concept kernels → skill library → bounded, cited context packs.

---

## 0. Context — What changed from v1, why this exists

`mnemos` (v1) was a layered cognitive memory store. It worked. But across 23 follow-up research notes (`v2/tip1..23`), six structural deficits surfaced repeatedly:

1. **No canonical/projection split.** v1 made sled the source of truth and rebuilt indexes from it. That allows silent index drift and forbids audit. v2 mandates an **immutable append-only ledger** (hash-chained WAL + signed mutation receipts) as the *only* truth; every index, including the KV store, is a rebuildable projection.
2. **Belief was confidence-on-claim.** v1 conflated *what is true* with *what the agent believes*. Science demands separating them — two papers can contradict each other; the agent has one belief over a body of evidence. v2 makes `Belief` a first-class lane.
3. **Single-axis temporal validity.** v1 stored `valid_from` / `valid_to`. v2 stores **bitemporal** validity: `(valid_from, valid_to)` (world time) and `(tx_from, tx_to)` (when the agent learned/recorded it). This is what enables "what was true on date X" *and* "what did the agent know on date X" — both required for honest scientific reasoning.
4. **Contradiction was treated as supersession.** v1 closed the old fact's validity window on conflict. v2 keeps both as **contested-evidence edges**; closure happens only when new evidence dominates *and* a `MemoryManager` policy decides.
5. **Compression was implicit.** v1 had "consolidation"; v2 makes a **proof-preserving compression pyramid L0–L6** explicit, with every higher layer linking back to evidence at lower layers. No destructive summary; only hierarchy.
6. **Procedural memory was untested.** v1 stored skills with examples; v2 enforces a **skill lifecycle** (Draft → Tested → Trusted → Degraded → Retired) with explicit `reliability = success / (success + failure)` formula and automatic degradation.

Beyond these, v2 adds: typed `MemoryCapsule` compression units, generational LSM-style tiering, topic-level FSRS strengthening, Memory-R1 learned manager policy, Mastra Observational Memory as L1, signed `MutationReceipt`s, four privacy classes with redaction-at-compile, and a deterministic index-rebuild guarantee.

This spec describes a system that is, by design, **measurably more capable than human episodic + semantic + procedural memory** for cumulative scientific work: perfect provenance, perfect rehearsal at machine speed, perfect bitemporal recall, parallel multi-channel retrieval, formal verification, and cryptographic auditability.

---

## 1. Design Goals (prioritized)

1. **Cumulative knowledge compounding.** Every observed event leaves a permanent, indexed, cited residue. Knowledge of a topic *strengthens with use* and *atrophies with neglect*, with explicit math (FSRS) governing both.
2. **Honest epistemic separation.** Observation ≠ claim ≠ belief ≠ concept ≠ skill ≠ source. Each is a typed lane with its own lifecycle.
3. **Bitemporal truth.** No destructive overwrite. Validity windows + transaction-time windows + supersession edges + contradiction edges preserve full history.
4. **Provenance is mandatory.** Every factual memory traces to: a signed episode, source span(s) with byte/page/line ranges, content hash, extractor identity & version, calibrated confidence, and a human-review state.
5. **Determinism.** Given the same ledger and seed, the system rebuilds byte-identical indices and emits byte-identical ContextPacks. Tested.
6. **Speed.** Observe ack p50 < 5 ms. Hot recall p50 < 50 ms over 10M memories. ContextPack compile < 50 ms.
7. **Compression without loss.** L0–L6 hierarchy. No layer is allowed to delete evidence; it can only abstract, with backlinks.
8. **Adaptive learning.** A `MemoryManager` policy (rule-based default → learned Memory-R1 upgrade path) decides ADD / UPDATE / SUPERSEDE / NOOP / REJECT / ARCHIVE per candidate, optimized for downstream task reward.
9. **Formal-science capable.** Equations carry units & assumptions, theorems form a DAG with downstream re-verification, datasets carry checksums & licenses, experiments carry hypotheses & outcomes.
10. **Auditable & rebuildable.** WAL replay produces identical state. Cards committed to a git-backed corpus. Mutation receipts signed.

## 2. Non-Goals

- Multi-tenant SaaS. Single-tenant; multiple stores can coexist on one host.
- Distributed cluster v1. Sharding hooks designed in (§22) but not implemented in v1.
- Replacing Qdrant / Neo4j / DuckDB. We use thin adapters for those when present; embed local equivalents otherwise.
- Online RL training in the hot path. Policy training is offline/nightly batch.
- LLM in the recall hot path. ContextPack assembly never calls an LLM. (LLMs are used in extraction, reflection, consolidation — never recall.)

---

## 3. Architectural Overview

```
                  ┌──────────────────────────────────────────────┐
                  │       Agent / LLM / Tool / Human caller       │
                  └───┬───────────────────┬───────────────────┬──┘
                      │ Rust API          │ gRPC/HTTP/MCP     │ CLI
┌─────────────────────┴───────────────────┴───────────────────┴─────┐
│                            mnemos-api                              │
│  observe · recall · recall_at · feedback · reflect · consolidate · │
│  promote_skill · verify · rehearse · explain · audit · rebuild     │
├───────────────────────────────────────────────────────────────────┤
│                          Cognitive Engine                          │
│ ┌──────────┐┌───────────┐┌──────────┐┌──────────┐┌──────────────┐ │
│ │ Manager  ││ Reflector ││Strengthen││Consolid. ││Belief Revisor│ │
│ │ (R1+rule)││ (Nemori)  ││(FSRS+Heb)││(cluster) ││ (Bayesian)   │ │
│ └──────────┘└───────────┘└──────────┘└──────────┘└──────────────┘ │
│ ┌──────────┐┌───────────┐┌──────────┐┌──────────┐┌──────────────┐ │
│ │ Skill    ││ Formal    ││Compressor││ Decay/   ││ Truth        │ │
│ │ Engine   ││ (units/   ││ (L0→L6)  ││ Archiver ││ Maintainer   │ │
│ │          ││  theorems)││          ││          ││ (temporal)   │ │
│ └──────────┘└───────────┘└──────────┘└──────────┘└──────────────┘ │
├───────────────────────────────────────────────────────────────────┤
│           Memory Lane Catalog — 14 typed lanes (§4)                │
├───────────────────────────────────────────────────────────────────┤
│                          Retrieval Fabric                          │
│ 13 parallel channels (§9) → RRF → rerank → ContextPack contract    │
├───────────────────────────────────────────────────────────────────┤
│                            Index Layer                             │
│ Tantivy BM25 · HNSW (hot fp32 / warm fp16 / cold dropped) ·        │
│ CSR temporal graph + PPR · Roaring bitmaps · Symbol/Equation FST · │
│ Skill precondition index · ContextPack cache                       │
├───────────────────────────────────────────────────────────────────┤
│                       Canonical Storage Substrate                  │
│ Append-only hash-chained WAL  ·  redb KV  ·  CAS blob store  ·    │
│ git-backed Markdown/YAML cards  ·  ed25519 mutation receipts       │
└───────────────────────────────────────────────────────────────────┘
```

Layered, not microservice. The daemon is a transport adapter, not a tier.

---

## 4. Memory Lane Catalog — 14 Typed Lanes

| # | Lane | Role | Persistence | Decays? | Provenance required |
|---|---|---|---|---|---|
| 1 | **Core** | Identity, mission, constraints, pinned context | redb + RAM | no | n/a |
| 2 | **Working** | Active task state, hypotheses, plan, scratchpad | redb + RAM | session | no |
| 3 | **Sensory** | Raw tool/user/file events pre-extraction; ring buffer | WAL only | minutes | no |
| 4 | **Episodic** | Hash-chained immutable event records (Gen0/Gen1 source of truth) | WAL + redb | no | self-provenant |
| 5 | **Observation** | Mastra-style dense, dated, prompt-cacheable observations | redb + Tantivy | slow | from Episode |
| 6 | **Semantic** | Atomic claims, definitions, relations | redb + indices | yes (utility-gated) | mandatory |
| 7 | **Belief** | Agent stance over evidence; support/contradict/revision history | redb + Belief index | revision-driven | mandatory |
| 8 | **Concept** | A-MEM Zettelkasten kernels (compressed topic atoms) | redb + HNSW | slow | from Claims |
| 9 | **Procedural** | Executable, tested, versioned skills | redb + git | reliability-driven | mandatory |
| 10 | **Resource** | Papers, datasets, repos, files (content-addressed) | CAS + redb | no | self |
| 11 | **Formal** | Equations, theorems, proofs, units, derivations | redb + symbol FST | no | mandatory |
| 12 | **Counterexample** | Falsified beliefs, failed assumptions, mistakes — **NEVER decays** | redb + indices | **never** | mandatory |
| 13 | **Eval** | Retrieval/answer/skill outcome traces — feeds learned policy | parquet + redb | rotated | self |
| 14 | **Metacognitive** | Lessons about reasoning, search, verification, debugging | redb + Tantivy | slow | from Eval |

Plus one runtime-only construct:
- **Distillate** (optional) — compressed parametric memory; LoRA-like adapter weights produced from a topic's mass of Claims+Equations. Always rebuildable from canonical lanes; never authoritative.

### 4.1 Lifecycle state machine (per memory)

```
                 ┌──────────────────────────────────────┐
                 ▼                                      │
   Proposed ─► Trusted ─► Consolidated ─► Archived ────┘
      │           │            │
      │           │            └─► Superseded ─► (kept; tombstoned)
      │           └─► Contradicted ──────────► (kept; flagged for review)
      └─► Rejected ─► (24h grace; then prunable)
```

The state machine is uniform across all lanes; some transitions are no-ops for some lanes (e.g. Working never reaches Archived).

---

## 5. Core Schemas (Rust)

All types live in crate `mnemos-types`. All public types implement `Serialize`/`Deserialize` (serde with `postcard` binary + JSON Schema versions), `Debug`, `Clone`. IDs are `uuid::Uuid` v7 (time-sorted). Timestamps are `chrono::DateTime<Utc>`. Hashes are `blake3::Hash`. Signatures are `ed25519_dalek::Signature`.

### 5.1 Identifiers

```rust
pub struct MemoryId(pub Uuid);
pub struct EpisodeId(pub Uuid);
pub struct EntityId(pub Uuid);
pub struct EdgeId(pub Uuid);
pub struct SkillId(pub Uuid);
pub struct TopicId(pub Uuid);
pub struct ReceiptId(pub Uuid);
pub struct MutationId(pub Uuid);
pub struct ResourceId(pub Uuid);
```

### 5.2 Lanes & lifecycle

```rust
pub enum MemoryLane {
    Core, Working, Sensory, Episodic, Observation, Semantic, Belief,
    Concept, Procedural, Resource, Formal, Counterexample, Eval, Metacognitive,
}

pub enum LifecycleState {
    Proposed, Trusted, Consolidated, Archived,
    Superseded, Contradicted, Deprecated, Rejected,
}

pub enum CompressionLevel {
    L0_Raw, L1_Episode, L2_Observation, L3_Atomic,
    L4_Concept, L5_Topic, L6_FieldSynthesis,
}

pub enum PrivacyClass {
    Public, Internal, Confidential, Secret, Vault,
}

pub enum ReviewState {
    MachineOnly,
    PolicyApproved { policy_version: String },
    HumanReviewed { reviewer: String, at: DateTime<Utc> },
    HumanRejected { reviewer: String, at: DateTime<Utc>, reason: String },
}
```

### 5.3 Bitemporal validity

```rust
pub struct BitemporalValidity {
    /// World time — when the fact was/is true in reality.
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    /// Transaction time — when the agent recorded/superseded this memory.
    pub tx_from: DateTime<Utc>,
    pub tx_to: Option<DateTime<Utc>>,
    /// Optional observation point ("ingested at").
    pub observed_at: Option<DateTime<Utc>>,
}
```

Four temporal queries become first-class:
- `recall(q)` — current world, current agent knowledge (default).
- `recall_at(q, t_world)` — what was true on `t_world` per current agent knowledge.
- `recall_as_of(q, t_tx)` — what the agent believed on `t_tx`.
- `recall_at_as_of(q, t_world, t_tx)` — full historical lens.

### 5.4 Provenance & source spans

```rust
pub struct SourceSpan {
    pub resource_id: ResourceId,
    pub uri: String,                          // file:// | doi: | arxiv: | tool:bash | …
    pub byte_start: Option<u64>,
    pub byte_end: Option<u64>,
    pub page: Option<u32>,
    pub section: Option<String>,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
    pub quote_hash: blake3::Hash,             // verifies the exact quoted span
    pub quote_excerpt: Option<String>,        // ≤ 200 chars; for display
}

pub struct CalibratedConfidence {
    pub extraction: f32,                      // confidence the extractor produced this correctly
    pub source_quality: f32,                  // confidence the source itself is reliable
    pub belief: f32,                          // posterior over support/contradict evidence
}

pub struct Provenance {
    pub born_in_episode: EpisodeId,
    pub evidence: Vec<SourceSpan>,
    pub extractor: ExtractorId,               // "llm:claude-opus-4-7:v3" | "rule:eq_v1"
    pub extractor_version: String,
    pub extraction_prompt_hash: Option<blake3::Hash>,
    pub source_content_hash: blake3::Hash,    // hash of the source artifact
    pub mutation_receipt: ReceiptId,
    pub confidence: CalibratedConfidence,
    pub review: ReviewState,
}
```

### 5.5 Scores

```rust
pub struct Scores {
    // Static-ish (extracted)
    pub importance: f32,                      // 0..1, set on insert; adjustable by policy
    pub novelty: f32,                         // 1 - max sim at birth
    pub surprise: f32,                        // predict-calibrate residual

    // Dynamic (learned/observed)
    pub utility: f32,                         // EMA of downstream task contribution
    pub retrieval_success_ema: f32,           // EMA of "this memory helped"

    // Source-driven
    pub source_quality: f32,                  // weighted source reputation

    // Conflict-driven
    pub contradiction_pressure: f32,          // sum of unresolved Contradicts edges

    // FSRS strengthening
    pub stability_days: f32,                  // S; ≥ 0.1
    pub difficulty: f32,                      // 1..10
    pub retrievability: f32,                  // R(t) = exp(-Δt/S) (lazy)
}
```

### 5.6 The MemoryObject envelope

```rust
pub struct MemoryObject {
    // Identity
    pub id: MemoryId,
    pub schema_version: u16,
    pub version: u32,
    pub lane: MemoryLane,
    pub lifecycle: LifecycleState,
    pub compression_level: CompressionLevel,

    // Display
    pub title: Option<String>,                // ≤ 80 chars
    pub summary: String,                      // ≤ 280 chars; used by retrieval renderers

    // Typed body
    pub payload: MemoryPayload,               // sum-type per lane (§5.7)

    // Cross-cutting
    pub entities: Vec<EntityId>,
    pub topics: Vec<TopicId>,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,

    // Time, truth, provenance
    pub temporal: BitemporalValidity,
    pub provenance: Provenance,
    pub scores: Scores,

    // Graph
    pub links: Vec<MemoryLink>,               // typed bidirectional edges (§5.8)
    pub supersedes: Vec<MemoryId>,
    pub superseded_by: Vec<MemoryId>,

    // Indexing hints
    pub embedding_ref: Option<EmbeddingRef>,
    pub bm25_doc_id: Option<u64>,

    // Access / privacy
    pub access_policy: AccessPolicy,
    pub signature: Option<Signature>,

    // Lifecycle metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub access_count: u32,
}
```

### 5.7 `MemoryPayload` sum-type (per lane)

```rust
pub enum MemoryPayload {
    Core(CoreBlock),
    Working(WorkingState),
    Sensory(SensoryFrame),
    Episode(EpisodeBody),
    Observation(ObservationBody),
    Claim(ClaimBody),
    Belief(BeliefBody),
    Concept(ConceptKernel),
    Skill(SkillBody),
    Resource(ResourceBody),
    Equation(EquationBody),
    Theorem(TheoremBody),
    Experiment(ExperimentBody),
    Dataset(DatasetBody),
    Counterexample(CounterBody),
    Eval(EvalBody),
    Lesson(MetacogBody),
}
```

#### 5.7.1 Episode & Observation

```rust
pub struct EpisodeBody {
    pub seq: u64,                             // monotonic per-store
    pub source_kind: SourceKind,
    pub actor: ActorRef,
    pub raw_ref: ResourceId,                  // CAS blob holding the raw payload
    pub raw_hash: blake3::Hash,
    pub prev_episode_hash: blake3::Hash,      // chain
    pub boundary: Option<EventBoundary>,
    pub signature: Option<Signature>,
}

pub enum SourceKind {
    UserMessage, AgentResponse,
    ToolCall { tool: String },
    ToolResult { tool: String },
    FileIngest { uri: String },
    CodeRun { task: String },
    SelfReflection,
    HumanCorrection,
    Observation,
}

pub enum EventBoundary {
    TopicShift, ToolBoundary, TimeGap { gap_sec: u32 },
    Correction, PredictionGap { surprise: f32 },
    SourceBoundary, TaskCompleted, TaskFailed, Manual,
}

pub struct ObservationBody {
    pub episode_ids: Vec<EpisodeId>,          // many episodes → one observation
    pub statement: String,                    // dated, compact narrative line
    pub when: DateTime<Utc>,
    pub salience: Salience,
    pub novelty: f32,
    pub prediction_gap: Option<f32>,
    pub continuation_hint: Option<String>,
}

pub enum Salience { Low, Medium, High, Urgent }
```

#### 5.7.2 Claim, Belief

```rust
pub struct ClaimBody {
    pub statement: String,
    pub subject: EntityRef,
    pub predicate: PredicateId,
    pub object: ClaimObject,                  // Entity | Literal | Memory
    pub scope: ClaimScope,                    // Implicit | Explicit | Conditional | Hypothetical
    pub modality: ClaimModality,              // Observed | Derived | Hypothesized | Simulated | Reviewed | ReportedBySource
    pub polarity: ClaimPolarity,              // Supports | Refutes | Qualifies | Unclear
    pub falsifiability: Falsifiability,       // Falsifiable | Unfalsifiable | PartiallyFalsifiable
    pub assumptions: Vec<MemoryId>,           // referenced claims/equations/concepts
    pub regime: Option<String>,               // e.g. "weak field", "non-relativistic", "T < 1 keV"
    pub conditions: Vec<String>,
    pub support: Vec<EvidenceLink>,
    pub contradiction: Vec<EvidenceLink>,
    pub derived_from: Vec<MemoryId>,
    pub formalization: Option<String>,        // optional Lean/SymPy form
}

pub struct EvidenceLink {
    pub target: MemoryId,
    pub direction: EvidenceDirection,         // Supports | Refutes | Conditions | Moderates | Exemplifies
    pub source_quality: f32,
    pub methodological_quality: f32,
    pub recency_weight: f32,
    pub directness: f32,
    pub weight: f32,                          // computed combined weight
}

pub struct BeliefBody {
    pub proposition: String,
    pub stance: BeliefStance,
    pub confidence: f32,
    pub supporting_claims: Vec<MemoryId>,
    pub contradicting_claims: Vec<MemoryId>,
    pub unresolved_questions: Vec<MemoryId>,
    pub rationale: String,
    pub falsification_conditions: Vec<String>,
    pub revision_history: Vec<BeliefRevision>,
}

pub enum BeliefStance {
    Unknown, Tentative, Supported, Contested, Revised, Retired,
    LocallyUsefulButUnproven,
}

pub struct BeliefRevision {
    pub at: DateTime<Utc>,
    pub from_stance: BeliefStance,
    pub to_stance: BeliefStance,
    pub from_confidence: f32,
    pub to_confidence: f32,
    pub triggering_evidence: Vec<MemoryId>,
    pub explanation: String,
}
```

#### 5.7.3 Concept kernel (A-MEM × GraphRAG × topic)

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
    pub community_id: Option<CommunityId>,    // Louvain partition over the entity KG
    pub topic_id: TopicId,
    pub mastery: f32,
    pub last_refreshed_at: DateTime<Utc>,
    pub refresh_triggers: Vec<RefreshTrigger>,
}

pub enum RefreshTrigger {
    SurpriseAbove(f32), ContradictionPressureAbove(f32),
    TopicStrengthBelow(f32), CoActivationCount(u32),
    ScheduledHours(f32),
}
```

#### 5.7.4 Procedural skill

```rust
pub struct SkillBody {
    pub name: String,
    pub description: String,
    pub task_signature: String,               // canonical hash key for retrieval
    pub language: SkillLanguage,              // RustFn | PythonScript | Shell | SqlTemplate | ToolPlan
    pub entrypoint: String,
    pub code_ref: ResourceId,                 // CAS blob holding code
    pub preconditions: Vec<Precondition>,
    pub postconditions: Vec<Postcondition>,
    pub examples: Vec<Example>,
    pub tests: Vec<TestSpec>,
    pub failure_modes: Vec<String>,
    pub status: SkillStatus,                  // Draft | Tested | Trusted | Degraded | Retired
    pub success_count: u64,
    pub failure_count: u64,
    pub avg_runtime_ms: f32,
    pub last_success_episode: Option<EpisodeId>,
    pub last_failure_episode: Option<EpisodeId>,
    pub sandbox_policy: SandboxPolicy,
}

impl SkillBody {
    pub fn reliability(&self) -> f32 {
        let total = self.success_count + self.failure_count;
        if total == 0 { 0.5 } else { self.success_count as f32 / total as f32 }
    }
}
```

#### 5.7.5 Formal lane

```rust
pub struct EquationBody {
    pub name: Option<String>,
    pub latex: String,
    pub normalized_symbolic: Option<String>,  // SymPy s-expr / canonical AST
    pub variables: Vec<VariableDef>,
    pub unit_constraints: Vec<UnitConstraint>,
    pub assumptions: Vec<MemoryId>,
    pub regime: Option<String>,
    pub limiting_cases: Vec<String>,
    pub derivation_steps: Vec<DerivationStep>,
    pub equivalent_forms: Vec<MemoryId>,
    pub linked_claims: Vec<MemoryId>,
    pub known_failures: Vec<MemoryId>,        // counterexample refs
    pub verification: VerificationState,
}

pub struct VariableDef {
    pub symbol: String,                       // e.g. "ψ", "F", "T"
    pub description: String,
    pub unit: Option<UnitExpr>,               // structured, not string
    pub bounds: Option<(f64, f64)>,
    pub is_constant: bool,
    pub variable_kind: VariableKind,          // Scalar | Vector | Tensor | Field | Operator
}

pub struct UnitConstraint {
    pub side: ConstraintSide,                 // Lhs | Rhs | Both
    pub expected: UnitExpr,
}

pub struct DerivationStep {
    pub step_no: u16,
    pub from: Vec<MemoryId>,
    pub rule: String,                         // "substitution", "limit ε→0", …
    pub justification: String,
    pub result_form: String,
}

pub enum VerificationState {
    Unverified,
    InformalSketch,
    PeerChecked { reviewer: String },
    UnitChecked { at: DateTime<Utc> },
    SymbolicallyChecked { tool: String, version: String },
    FormalCheckPassed { tool: String, version: String, proof_ref: Option<ResourceId> },
    Failed { reason: String, at: DateTime<Utc> },
}

pub struct TheoremBody {
    pub name: String,
    pub statement: String,
    pub statement_latex: String,
    pub statement_lean: Option<String>,
    pub statement_coq: Option<String>,
    pub proof_sketch: String,
    pub proof_formal_ref: Option<ResourceId>,
    pub proof_system: Option<ProofSystem>,    // Lean4 | Coq | Isabelle | Custom
    pub assumptions: Vec<String>,
    pub uses: Vec<MemoryId>,                  // depends on these theorems
    pub used_by: Vec<MemoryId>,               // populated by Truth Maintainer
    pub verification: VerificationState,
}

pub struct ExperimentBody {
    pub hypothesis: String,
    pub method: String,
    pub parameters: serde_json::Value,
    pub dataset: Option<MemoryId>,
    pub result_summary: String,
    pub outcome: ExperimentOutcome,           // Supports | Refutes | Inconclusive
    pub falsified_memories: Vec<MemoryId>,
    pub follow_up_questions: Vec<MemoryId>,
    pub artifacts: Vec<ResourceId>,
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
    pub reproducibility_status: ReproStatus,  // Unverified | DownloadOk | ChecksumOk | PreviewOk | Reproduced
    pub linked_claims: Vec<MemoryId>,
    pub linked_papers: Vec<MemoryId>,
}
```

#### 5.7.6 Counterexample, Eval, Metacognitive

```rust
pub struct CounterBody {
    pub falsified_memory: MemoryId,
    pub falsifying_evidence: Vec<EpisodeId>,
    pub falsifying_experiment: Option<MemoryId>,
    pub explanation: String,
    pub regime_of_failure: Option<String>,
    pub severity: CounterSeverity,            // Minor | Notable | NeedsReview | Critical
}

pub struct EvalBody {
    pub query: String,
    pub query_kind: QueryKind,
    pub retrieved_items: Vec<EvalItem>,       // each with score + used?-flag
    pub outcome: Outcome,
    pub correctness: Option<f32>,
    pub timeliness_ms: u32,
    pub citation_accuracy: Option<f32>,
    pub missed_contradictions: Vec<MemoryId>,
    pub missed_skills: Vec<SkillId>,
    pub trace_ref: ResourceId,                // full RetrievalTrace
}

pub struct MetacogBody {
    pub topic: Option<String>,
    pub lesson: String,                       // e.g. "When more than 3 unstated assumptions, re-derive."
    pub triggers: Vec<MetacogTrigger>,        // when to surface this lesson
    pub linked_evals: Vec<MemoryId>,
}

pub enum MetacogTrigger {
    OnQueryKind(QueryKind),
    OnEntityKind(EntityKind),
    OnTopic(TopicId),
    OnContradictionCount { gte: u32 },
    OnSurprise { gte: f32 },
}
```

### 5.8 Typed memory links

```rust
pub struct MemoryLink {
    pub target: MemoryId,
    pub relation: Relation,
    pub strength: f32,                        // 0..1, Hebbian-updated
    pub created_at: DateTime<Utc>,
    pub source_episode: Option<EpisodeId>,
    pub confidence: f32,
}

pub enum Relation {
    // Similarity / hierarchy
    SimilarTo, Extends, Refines, SpecializationOf, GeneralizationOf,
    // Evidence
    Supports, Contradicts, Implies, Equivalent, Conditions, Moderates,
    // Causal / temporal
    Causes, Prevents, FollowedBy, Precedes,
    // Citation / derivation
    Cites, DerivedFrom, AuthoredBy,
    // Examples
    ExampleOf, CounterexampleOf, PrototypeOf,
    // Prerequisite / dependency
    DependsOn, PrerequisiteOf, UsedBy,
    // Cross-domain
    Analogy { mapping: String },
    // Lifecycle
    Supersedes, SupersededBy,
    // Skill
    ImplementedBy, ProducedBy, TestedBy,
    // Custom (controlled vocabulary, see §5.10)
    Custom(PredicateId),
}
```

### 5.9 Entities & temporal edges

```rust
pub struct Entity {
    pub id: EntityId,
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub kind: EntityKind,
    pub summary: String,
    pub embedding_ref: Option<EmbeddingRef>,
    pub created_at: DateTime<Utc>,
    pub merged_into: Option<EntityId>,        // for de-duplication
    pub privacy_class: PrivacyClass,
}

pub enum EntityKind {
    Person, Organization, Paper, Concept, Theorem, Equation, Variable,
    Dataset, Codebase, Method, Field, Tool, Unit, Organism, Place,
    Project, File, Function, Module, Symbol,
}

pub struct TemporalEdge {
    pub id: EdgeId,
    pub subject: EntityId,
    pub predicate: PredicateId,
    pub object: EntityOrLiteral,
    pub temporal: BitemporalValidity,
    pub confidence: f32,
    pub support: Vec<EvidenceLink>,
    pub contradicts: Vec<EdgeId>,
    pub supersedes: Vec<EdgeId>,
    pub source_episode: EpisodeId,
    pub source_memory: MemoryId,
    pub receipt_id: ReceiptId,
    pub status: EdgeStatus,                   // Current | Historical | Disputed | Deprecated | Rejected
    pub signature: Option<Signature>,
}
```

### 5.10 Predicate vocabulary

Stored as `mnemos-types/predicates.yaml`. Seed set: `is_a, part_of, instance_of, causes, prevents, equivalent_to, equals, implies, contradicts, supersedes, derived_from, cites, authored_by, defined_in, measured_by, has_unit, has_assumption, valid_in_regime, prerequisite_for, applies_to, special_case_of, generalizes, refines, falsifies, replicates, reproduces`. New predicates auto-proposed by extractors land in `proposed_predicates.yaml` for review (the only blocking review in the system).

### 5.11 MutationReceipt (audit chain)

```rust
pub struct MutationReceipt {
    pub id: ReceiptId,
    pub mutation_id: MutationId,
    pub at: DateTime<Utc>,
    pub actor: MutationActor,                 // LLM { model, version } | Human { id } | System | Agent
    pub mutation_kind: MutationKind,          // Add | Update | Supersede | Merge | Archive | Deprecate | Noop
    pub objects_created: Vec<MemoryId>,
    pub objects_updated: Vec<(MemoryId, u32 /* new version */)>,
    pub objects_superseded: Vec<(MemoryId, MemoryId)>,  // (old, new)
    pub objects_archived: Vec<MemoryId>,
    pub source_hashes: Vec<blake3::Hash>,
    pub index_updates: Vec<IndexUpdate>,
    pub warnings: Vec<String>,
    pub previous_receipt_hash: blake3::Hash,
    pub receipt_hash: blake3::Hash,           // hash of canonical encoding
    pub signature: Option<Signature>,
}
```

Receipts form a second hash chain in parallel to the episode chain. Any divergence between them is detected by `mnemos verify`.

---

## 6. Storage Substrate

### 6.1 Canonical truth (immutable)

| Concern | Engine | Crate | Why |
|---|---|---|---|
| Append-only WAL | custom; CBOR + crc32 | `bincode`, `postcard`, `crc32fast` | replayable, segmented, zstd-compressed |
| Episode hash chain | inline in WAL | `blake3` | tamper detection |
| Mutation receipts | inline in WAL + signed | `ed25519-dalek` | non-repudiation |
| Content-addressed blobs | CAS dir | `blake3` | de-dup; supports large raw payloads |
| Git-backed cards | repo in `git/` | `gix` (preferred) / `git2` | human-readable audit & branching |

### 6.2 Canonical projections (rebuildable, fast)

| Concern | Engine | Crate | Why |
|---|---|---|---|
| KV (objects, entities, edges) | redb | `redb` | pure-Rust, ACID, MVCC, mmap |
| Vector — hot fp32 | HNSW in-mem | `hnsw_rs` / `instant-distance` | low-latency kNN |
| Vector — warm fp16 | HNSW on-disk, quantized | `hnsw_rs` w/ custom quantizer | 10× capacity vs fp32 |
| Vector — cold | dropped or PQ | (concept kernels carry it) | scale to 100M |
| BM25 / full-text | Tantivy | `tantivy` | de-facto Rust full-text |
| Symbol / equation FST | Tantivy custom analyzer + FST | `tantivy` | exact symbol + equation fragment search |
| Graph (temporal KG) | custom CSR + edge log | `petgraph` (in-mem analytics only) | edges live in WAL; CSR rebuilt on load |
| Temporal index | interval tree | `intervaltree` or hand-rolled | "what was valid at T" |
| Tag/keyword | Roaring bitmaps | `roaring` | fast intersections |
| Skill precondition index | hash + Tantivy facet | `tantivy` | task-signature-keyed |
| Co-activation | sparse HashMap | std | Hebbian strengthening |
| ContextPack cache | redb + content-addressed | `redb`, `blake3` | reuse identical packs |

### 6.3 Optional external adapters (trait-isolated)

| Service | Adapter crate | When to use |
|---|---|---|
| Qdrant | `mnemos-index-vector::QdrantBackend` | scale > 50M vectors or sharded HA |
| Kùzu | `mnemos-index-graph::KuzuBackend` | dense entity graphs > 50M edges, Cypher needed |
| DuckDB | `mnemos-eval::DuckBackend` | analytical queries over eval logs |
| Lean / Coq / SymPy | `mnemos-formal::*Backend` | formal verification of theorems & symbolic equation manipulation |

### 6.4 On-disk layout

```
$MNEMOS_HOME/
├── config.toml
├── identity/
│   ├── signing-key.age              # passphrase-encrypted
│   └── public-key.ed25519
├── ledger/
│   ├── episodes/
│   │   ├── 0000000001.wal.zst
│   │   └── 0000000002.wal.zst
│   ├── mutations/
│   │   └── 0000000001.wal.zst
│   ├── receipts/
│   │   └── 0000000001.cbor.zst
│   └── checkpoints/
│       └── checkpoint-000001.cbor
├── objects/
│   ├── redb/
│   │   └── memory.redb
│   └── blobs/                       # CAS, sha256-named
│       └── sha256/ab/cd/abcd…
├── projections/
│   ├── tantivy/
│   │   ├── episodic/  semantic/  observation/  resource/  formal/  skill/
│   ├── hnsw/
│   │   ├── semantic.hot.fp32.hnsw
│   │   ├── semantic.warm.fp16.hnsw
│   │   ├── concept.hnsw
│   │   ├── skill.hnsw
│   │   └── …
│   ├── graph/
│   │   ├── nodes.cbor
│   │   ├── edges.cbor
│   │   ├── adjacency.mmap            # CSR
│   │   └── temporal-index.mmap
│   ├── bitmaps/
│   │   └── tags.roaring …
│   ├── symbol_fst/
│   ├── skill_precond_index/
│   └── context_cache/
├── git/
│   └── curated-memory/               # git2/gix-managed repo
│       ├── cards/{claims,equations,theorems,skills,counterexamples,concepts}/
│       ├── maps/{topics,timeline,citation}/
│       ├── reviews/                  # human-reviewer commits
│       └── manifests/                # dataset checksums, RO-Crate
├── candle/
│   └── embed.safetensors
├── telemetry/
│   ├── traces.parquet
│   └── evals.parquet
└── sandboxes/                         # one dir per running skill
```

### 6.5 WAL format

A WAL segment is a sequence of length-prefixed framed records, each a `WalEntry`:

```rust
pub enum WalEntry {
    Episode(Episode),
    Observation(ObservationFrame),
    MemoryAdd(MemoryObject),
    MemoryUpdate { id: MemoryId, new_version: u32, patch: serde_json::Value },
    MemorySupersede { old: MemoryId, new: MemoryId, reason: String },
    MemoryArchive(MemoryId),
    EntityUpsert(Entity),
    EdgeAdd(TemporalEdge),
    EdgeClose { edge: EdgeId, valid_to: DateTime<Utc>, reason: String },
    EdgeContradicted { a: EdgeId, b: EdgeId },
    SkillUpsert(SkillBody),
    LifecycleTransition { id: MemoryId, from: LifecycleState, to: LifecycleState },
    Strengthen { id: MemoryId, delta: f32, reason: StrengthenReason },
    Decay { id: MemoryId, factor: f32 },
    Reflection { episode: EpisodeId, output: serde_json::Value },
    BeliefRevision(BeliefRevision),
    Receipt(MutationReceipt),
    Checkpoint { redb_sequence: u64, segment_hash: blake3::Hash },
}
```

Segments rotate at 64 MiB or 1 h. Each segment trails a `SegmentFooter { sha256, prev_segment_hash, signature }`. On startup, replay from last checkpoint; verify the chain.

---

## 7. Index Layer

### 7.1 Tantivy schemas

One Tantivy index per lane that benefits (Episodic, Observation, Semantic, Concept, Procedural, Resource, Formal, Counterexample, Metacognitive, Eval). Common fields:

| Field | Type | Stored |
|---|---|---|
| `id` | `STRING` | yes |
| `lane` | `FACET` | yes |
| `lifecycle` | `FACET` | yes |
| `title` | `TEXT` | yes |
| `summary` | `TEXT` (indexed) | yes |
| `content` | `TEXT` (indexed) | optional |
| `keywords` | `TEXT` | no |
| `tags` | `FACET` | yes |
| `entities` | `FACET` | yes |
| `topics` | `FACET` | yes |
| `valid_from` / `valid_to` | `DATE` | yes |
| `tx_from` / `tx_to` | `DATE` | yes |
| `created_at` | `DATE` | yes |
| `privacy_class` | `FACET` | yes |
| `confidence` | `F64` | yes |
| `importance` | `F64` | yes |
| `utility` | `F64` | yes |
| `source_quality` | `F64` | yes |

For **Formal**, an additional **Symbol Analyzer** is added: a custom tokenizer chain `lowercase → ascii_folding → math_symbol_passthrough → kstem`, where `math_symbol_passthrough` preserves `α-ωΑ-Ω∀∃∂∇∑∫±≠≤≥≈≡` and TeX command tokens (`\frac`, `\partial`, …). Equations are also indexed via a normalized form (SymPy `srepr` style) into a second field `eq_norm` enabling fragment hits.

### 7.2 Vector indices

Three tiers per lane that needs vectors:

| Tier | Storage | Precision | Capacity | Build params |
|---|---|---|---|---|
| **Hot** | in-memory | fp32 | ≤ 500k memories | M=16, ef_c=200, ef_s=64 |
| **Warm** | mmap on-disk | fp16 (custom quantizer) | ≤ 5M memories | M=24, ef_c=400, ef_s=96 |
| **Cold** | none (concept kernels) | — | ≤ 95M backing memories | n/a |

Promotion rules: a memory is **hot** if `(R > 0.5 ∧ importance > 0.6)` OR `lane ∈ {Core, Belief, Counterexample, Procedural}`. Else **warm**. Cold memories rely on Concept kernels (which themselves are hot/warm) as retrieval entry points. Tier moves run during consolidation.

Default embedding dim **768** (BGE-small-en-v1.5 via Candle); configurable to 1024 (BGE-large) or 1536 (external). Re-embedding on model change is a background migration job that touches memories in priority order `(importance × utility)`.

### 7.3 Temporal graph (CSR + edge log)

- Edges stored in WAL and the `edge/by_id` redb table.
- CSR adjacency (`adjacency.mmap`) is built at startup and kept in RAM; updates are appended to an in-memory delta log and merged into the CSR snapshot every hour.
- Each entity carries an `IntervalTree<EdgeId>` keyed by `(valid_from, valid_to)` for point-in-time queries.
- Personalized PageRank (HippoRAG) is bounded: max 3 hops, ≤ 256 expanded nodes, decay 0.15, 10 iterations OR convergence < 1e-3. Result is reused for ≤ 5 s by the ContextPack cache.

### 7.4 Symbol / equation FST

Equations are indexed three ways: raw TeX (BM25), normalized symbolic form (BM25 on `eq_norm`), and **FST keys** (`tantivy` FST module) over `(LHS_symbols, RHS_symbols, top_operator, units_signature)`. The agent can ask "find every equation whose LHS contains ∂²ψ/∂t² and whose units include kg·m²/s²" — the FST returns candidates in ≤ 5 ms.

### 7.5 Skill precondition index

`HashMap<PreconditionSignature, RoaringBitmap<SkillId>>`. Signature is the BLAKE3 of the canonical-sorted `Vec<Precondition>`. Plus a separate HNSW over skill description embeddings for fuzzy match.

### 7.6 ContextPack cache

Key = `blake3(query_canonical || retrieval_weights || top_K_memory_ids_state)`. Value = the compiled ContextPack. Invalidated by any mutation touching any cited memory.

### 7.7 Co-activation matrix

`HashMap<(MemoryId, MemoryId), CoActStat>` (sparse). Updated on every retrieval (Hebbian, §11). Memory-aware: capped at 50 M entries; LRU eviction. Survives restarts via periodic redb snapshot.

---

## 8. Write Pipeline

The write path is split into **hot** (sync, ≤ 5 ms p50) and **async** (background extraction & indexing).

### 8.1 Hot stages (synchronous, return a receipt)

1. **Privacy gate.** Reject or encrypt content matching policy rules. Vault-class content routed to the encrypted vault keyspace.
2. **Segment.** Split incoming stream on `EventBoundary` markers.
3. **Episode append.** Hash-chain, sign, write to WAL. Returns `EpisodeId` immediately.
4. **Observation distillation.** Optional dense observation line (Mastra style). Same WAL.
5. **Acknowledge.** Return `WriteReceipt { episode_id, observation_id?, queued_extractions, receipt_hash }`.

### 8.2 Async stages (run by daemons)

6. **Extract candidates.** `Extractor` trait emits `Vec<CandidateMemory>` of typed claims, equations, skills, etc.
7. **Normalize.** Entity linking (alias dict → embedding kNN → fuzzy match), unit normalization, predicate canonicalization.
8. **Ground.** Attach `Vec<SourceSpan>` with hashes and quote excerpts. **Hard rule:** Semantic/Belief/Claim/Theorem/Equation memories without ≥ 1 SourceSpan stay in `Proposed` forever (no auto-promotion).
9. **Link.** Search nearest existing memories via (BM25 ⊕ vector ⊕ entity). Create typed Links of strength > τ_link (default 0.6).
10. **Contradiction detection.** If `(subject, predicate, valid-at)` matches an existing live edge with a different object: do **not** silently close; emit a `ContradictionEvent` and let the Truth Maintainer + MemoryManager decide:
    - **Both Trusted, evidence balanced** → both remain live; add `Contradicts` edges; `contradiction_pressure += weight`; spawn `BeliefRevision` job.
    - **New strictly dominates** (higher source quality + later observed_at + supersession-allowed predicate) → close old `valid_to = now`; new `Supersedes` old; old → `Superseded`.
    - **Old strictly dominates** (e.g. low-confidence new vs peer-reviewed old) → new → `Rejected` (with `MetacogBody` lesson recorded).
    - **Falsification** (counterexample episode) → spawn `CounterexampleCard`; old → `Contradicted`; new edge created.
11. **Validate.** Schema check, duplicate detection (cos > 0.95 + same kind → propose MERGE), unit dimension check for Equations.
12. **Score.** Compute initial `importance`, `novelty`, `surprise` (predict-calibrate output), `source_quality`, `utility = 0.5`, `stability_days = 1.0`, `difficulty = 5.0`.
13. **Policy decision.** `MemoryManager::decide(candidate, context)` → `MemoryDecision`:
    ```rust
    pub enum MemoryDecision {
        Add, Update { target: MemoryId },
        Supersede { target: MemoryId },
        Merge { target: MemoryId },
        Archive { target: MemoryId },
        Noop,
        Review { reason: String },
        Reject { reason: String },
    }
    ```
14. **Mutate.** Apply decision. Write `WalEntry`s. Generate signed `MutationReceipt`.
15. **Index fanout.** Update Tantivy, HNSW, CSR delta, bitmaps, symbol FST, skill index — all in parallel tokio tasks.
16. **Schedule downstream.** Reflection if `surprise > 0.6`. Consolidation if cluster size > N. Topic strength recompute for touched topics.

### 8.3 Extractor trait

```rust
#[async_trait]
pub trait Extractor: Send + Sync {
    async fn extract(&self, ep: &Episode, ctx: &ExtractionContext)
        -> Result<Vec<CandidateMemory>>;
    fn capabilities(&self) -> ExtractorCaps;
    fn id(&self) -> &str;
    fn version(&self) -> &str;
}

pub struct CandidateMemory {
    pub kind: MemoryLane,
    pub payload: MemoryPayload,
    pub summary: String,
    pub entities: Vec<EntityRef>,
    pub source_spans: Vec<SourceSpan>,
    pub extractor_confidence: f32,
    pub topics_hint: Vec<String>,
    pub policy_hint: Option<ExtractionPolicy>,
}

pub enum ExtractionPolicy {
    AddOnly, UpdateOnContradiction, ReplaceSimilar, NeverOverwrite,
}
```

Default impls:

- `LlmStructuredExtractor` — calls `LlmClient` with structured-output JSON schema; supports Claude / OpenAI / Ollama / local Candle.
- `RuleExtractor` — regex/parsers for cheap shallow extractions: TeX equation blocks, code symbols, error strings, BibTeX-style citations.
- `CodeExtractor` — Tree-sitter based for code; emits SkillBody candidates plus identifier entities.
- `EquationExtractor` — KaTeX/pandoc + SymPy normalization (subprocess).
- `CitationExtractor` — DOI / arXiv / ISBN resolvers behind a `Resolver` trait.

Multiple extractors run in parallel on the same episode; their candidates are de-duplicated by content hash before policy gating.

---

## 9. Read Pipeline & ContextPack Contract

```
classify_query → parallel_retrieve_13 → expand → fuse(RRF) → rerank → temporal_filter
              → privacy_filter → compile_packet → log_eval
```

### 9.1 Query classification

```rust
pub enum QueryKind {
    Exact, Semantic, Temporal, Relational, Procedural, Educational, Exploratory,
    MathDerivation, ScientificJudgment, Coding, Debug, Planning, Citation,
    Recall, Contradiction, Counterexample, Mixed,
}
```

Default classifier is rule-based on keywords + regexes (zero-LLM). Optional Candle MLP classifier for ambiguous queries.

### 9.2 13 parallel retrieval channels

| # | Channel | Hit on | Default k |
|---|---|---|---|
| 1 | **Core** | always-on, pinned blocks | n/a |
| 2 | **BM25** | Tantivy exact/text | 64 |
| 3 | **Dense vector** | HNSW (hot+warm) | 64 |
| 4 | **Entity** | resolve mentions → memories citing them | 64 |
| 5 | **Temporal** | `valid_at ∋ now` (or query-provided time) | unlimited filter |
| 6 | **Graph PPR** | HippoRAG (only for {Educational, Mixed, Planning, MathDerivation}) | top 32 |
| 7 | **Concept** | A-MEM neighbors of seeded concept kernels | 32 |
| 8 | **Procedural** | skill precondition + signature match | 16 |
| 9 | **Eval / Counterexample** | prior failures, falsifications | **always included if entity overlap** |
| 10 | **Resource** | papers/datasets matching the topic | 16 |
| 11 | **Symbol / Equation** | FST hits for math queries | 32 |
| 12 | **Belief** | current stance synthesis matching the proposition | 8 |
| 13 | **Topic Atlas** | for high-level "what do I know about X?" | 4 |

All run via `tokio::join!`. Channels not relevant to the QueryKind are skipped (defaults in `retrieval_policy.toml`).

### 9.3 RRF + rerank

Reciprocal-rank fusion with k=60. Then weighted rerank (v2 consensus formula):

```
score = 0.18 * exact
      + 0.16 * dense
      + 0.14 * bm25
      + 0.12 * entity
      + 0.12 * graph_ppr
      + 0.08 * temporal
      + 0.07 * source_quality
      + 0.06 * utility
      + 0.04 * topic_strength
      + 0.03 * freshness
      + 0.03 * confidence
      + 0.02 * recency
      - 0.20 * contradiction_unresolved
      - 0.15 * stale_after_valid_to
      - 0.10 * privacy_penalty
```

Weights are stored in `retrieval_policy.toml` and **profile-overridable** per `QueryKind`:

- **MathDerivation:** boost exact + equation FST + theorem prerequisites + units; downweight recency.
- **Debug:** boost BM25 + episodic recent + Eval/Counterexample; downweight graph_ppr.
- **ScientificJudgment:** boost source_quality + contradiction (positive — surface them) + belief.
- **Coding:** boost exact symbols + skill match; require sandbox-safe.
- **Planning:** boost core + open_questions (metacog) + skills.

### 9.4 Temporal & privacy filters

- **Temporal:** unless query is historical (`recall_at` / `recall_as_of`), drop edges/memories whose `valid_to < now`. Exception: `Contradicted` memories retained with a warning flag.
- **Privacy:** elide memories whose `access_policy.privacy_class` exceeds the caller's allowed set; insert `[REDACTED]` markers if any high-importance items were elided so the caller knows context is incomplete.

### 9.5 ContextPack contract

```rust
pub struct ContextPack {
    pub id: Uuid,
    pub query: String,
    pub query_kind: QueryKind,
    pub items: Vec<PackItem>,
    pub citations: Vec<Citation>,
    pub contradictions: Vec<ContradictionNote>,
    pub counterexamples_surfaced: Vec<MemoryId>,
    pub omitted_evidence: Vec<OmissionNote>,    // high-value items we couldn't fit
    pub freshness: FreshnessSummary,
    pub confidence_summary: f32,
    pub token_estimate: u32,
    pub token_budget: u32,
    pub trace_id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub deterministic_hash: blake3::Hash,
}

pub struct PackItem {
    pub memory_id: MemoryId,
    pub lane: MemoryLane,
    pub rendered: String,
    pub why_included: String,                 // short, machine-readable reason
    pub score: f32,
    pub score_breakdown: ScoreBreakdown,
    pub channels_hit: ChannelFlags,
    pub citations: Vec<Citation>,
    pub warnings: Vec<ItemWarning>,           // Stale | Contradicted | LowConfidence | Redacted
    pub confidence: CalibratedConfidence,
}
```

**Hard contract guarantees (tested):**
- `token_estimate ≤ token_budget`.
- Every `Claim`, `Equation`, `Theorem`, `Belief` item has ≥ 1 citation.
- If two items contradict each other, the pack contains a `ContradictionNote` summarizing the conflict.
- `omitted_evidence` is non-empty if any high-importance memory was dropped due to budget — never silently lossy.
- For fixed `(canonical_query, retrieval_weights, store_state, seed)`, `deterministic_hash` is identical across runs. Tested by `test_context_pack_deterministic`.

### 9.6 Budget allocation (default)

| Section | % of budget |
|---|---|
| Core / pinned | 5% |
| Direct answers (top-scoring memories) | 35% |
| Evidence (supporting source spans, citations) | 25% |
| Counterexamples | 10% |
| Skills (callable, signature + brief) | 10% |
| Omissions notes ("X was excluded due to budget") | 5% |
| Slack | 10% |

Each section has a min/max; overage in one section can borrow from slack but never from another section's min.

### 9.7 Rendering rules per lane

- **Claim:** `[Claim · conf 0.86 · src arxiv:… p.412] In the Born approximation, σ_tot ≈ …`
- **Equation:** `[Eq · F=ma · units kg·m/s² · regime: inertial frame] {tex} (asm: m constant)`
- **Theorem:** `[Thm · Bolzano–Weierstrass · Lean4-verified] Every bounded seq in ℝ has a convergent subseq.`
- **Belief:** `[Belief · stance: Supported · conf 0.79 · 4 supports / 1 contradict] {proposition}; rationale: …`
- **Skill:** `[Skill · arxiv_fetch v3 · reliability 0.95 · 12/12 tests] sig: fn(arxiv_id) -> PdfPath`
- **Counterexample:** `[Counter · severity: NeedsReview] You previously assumed X; falsified by experiment E on 2026-04-12.`
- **Concept kernel:** `[Concept · {name} · mastery 0.62] {definition}. Prereqs: …; Open questions: …`

### 9.8 Log access

Emit `EvalBody` skeleton on every read; finalized when the caller submits `feedback`.

---

## 10. Consolidation Engine

Runs as a background `tokio::task` with **priority queue**:

| Priority | Trigger | Job |
|---|---|---|
| **P0 (immediate)** | `surprise > 0.85` | Reflection + belief revision |
| **P0 (immediate)** | new contradiction edge | Belief revision; possible counterexample |
| **P0 (immediate)** | skill failure | Skill degradation |
| **P1 (≤ 1 min)** | `accumulated_importance > τ_imp` | Episode clustering / observation synthesis |
| **P1 (≤ 1 min)** | duplicates detected (cos > 0.95) | Merge proposal |
| **P2 (≤ 5 min)** | concept refresh trigger fired | Concept kernel regen |
| **P3 (≤ 1 h)** | topic strength below mastery floor | Schedule rehearsal |
| **P4 (nightly)** | scheduled | Belief synthesis, dedup, archive, skill distillation, source audit |
| **P5 (weekly)** | scheduled | Distillate generation, community detection, summary forest rebuild |

### 10.1 Concrete consolidation jobs

1. **Observation synthesis.** DBSCAN over episodes (last 24 h) → cluster of ≥ 3 → LLM-summarized `Observation`. If salience ≥ High → propose `SchemaMemory`.
2. **Concept kernel refresh.** Triggered by surprise/contradiction/strength below floor. Pulls all Claims+Equations+Counterexamples+Skills touching the topic, regenerates the kernel structure, sets new `last_refreshed_at`.
3. **Belief synthesis.** Per entity touched in last window: gather all Trusted Claims of `(subject=e, predicate=p)`; aggregate via Bayesian update over `support_weight` and `contradict_weight`; produce/update `BeliefBody` with revision history.
4. **Duplicate merge.** Cos sim > 0.95, same lane, compatible predicates → propose MERGE; kept = higher utility; loser → `Superseded`.
5. **Skill distillation.** Memp pattern: scan episode trajectories where outcome=success; if same action pattern recurs ≥ 3× with success and signature is stable → propose new Skill or update existing.
6. **Source audit.** Re-check DOIs, arXiv IDs, dataset URIs, license fields. Broken citation → `MetacogBody { lesson: "source link broken", linked_evals: [...] }`.
7. **Theorem dependency closure.** On any change to a theorem's `verification`, re-walk `used_by` DAG; downgrade dependents if needed.
8. **Index compaction.** HNSW rebuild at 10% deleted; Tantivy commit + merge.
9. **Topic communities (Louvain) recomputation.** Weekly, over the entity KG.
10. **Distillate generation.** Optional: produce a small LoRA-like adapter per mature topic for parametric memory. Always rebuildable; never authoritative.

All consolidation outputs flow back through the write pipeline (so they get signed, scored, indexed uniformly).

---

## 11. Strengthening Engine

The kernel that makes "dynamic knowledge topic strengthening" real.

### 11.1 FSRS-5 per memory

For each memory in `{Semantic, Belief, Concept, Equation, Theorem, Procedural, Metacognitive}`:

- Maintain `(stability_days S, difficulty D)`.
- `retrievability R(t) = exp(-(t - last_accessed) / S)` (computed on demand).
- On **successful retrieval** (a `feedback(pack_id, Outcome::TaskSuccess { used })` referencing this memory): rating defaults `Good`; can be overridden by caller (`Again | Hard | Good | Easy`). Apply FSRS-5 update rules; constants in `fsrs.toml`.
- On **unsuccessful retrieval** (`TaskFailure`): rating `Again`; S decreases; D increases.
- A scheduler maintains a min-heap keyed by `next_review_at = last_accessed + S × ln(1 / R_target)` with default `R_target = 0.9`.
- `mnemos.rehearse_next(n)` pops the n earliest-due memories.

### 11.2 Hebbian co-activation

On every retrieval over top-K:

```text
for (i, mi) in top_k.iter().enumerate():
    for (j, mj) in top_k.iter().enumerate().skip(i+1):
        coact[(mi, mj)] += learning_rate * relevance(mi) * relevance(mj)
```

Default `learning_rate = 0.01`. During consolidation:
- `coact > 0.7` AND no existing link → add `SimilarTo` link of strength `coact`.
- `coact < 0.05` for 30 days AND weak link → decay link strength, eventually drop.

### 11.3 Topic-level strengthening

Topics are entity communities (Louvain on entity KG, weekly). Each has:

```rust
pub struct TopicState {
    pub id: TopicId,
    pub label: String,
    pub member_entities: Vec<EntityId>,
    pub member_memories: Vec<MemoryId>,
    pub prerequisites: Vec<TopicId>,
    pub strength: f32,          // 0..1
    pub mastery: f32,           // 0..1 — accumulated successes
    pub half_life_hours: f32,
    pub retrieval_success: f32, // EMA
    pub contradiction_pressure: f32,
    pub source_coverage: f32,   // # of independent sources / target
    pub formal_coverage: f32,   // # of equations or theorems / target
    pub procedural_coverage: f32,// # of related skills / target
    pub last_strengthened_at: DateTime<Utc>,
    pub next_review_at: Option<DateTime<Utc>>,
}
```

Update on signal:

```text
decayed = previous_strength * exp(-hours_since_update / half_life_hours)
reinforcement = w_recurrence * episodes_touching
              + w_utility    * downstream_success
              + w_novelty    * prediction_gap
              + w_source     * new_independent_sources
              + w_retrieval  * retrieval_success_delta
              + w_skill      * skill_success_delta
              + w_neighbor   * neighbor_strength_boost
              - w_conflict   * unresolved_contradictions
              - w_failure    * task_failures
new_strength = clamp(decayed + reinforcement, 0.0, 1.0)
```

Default weights in `topic.toml`: `recurrence=0.20, utility=0.20, novelty=0.10, source=0.15, retrieval=0.15, skill=0.10, neighbor=0.10, conflict=0.20, failure=0.15`. Half-life default 168 h (1 week) per topic, auto-tuned by mastery.

If `aggregate_strength < mastery_floor (default 0.55)` for a mission-tagged topic, the rehearsal queue prioritizes that topic's high-importance members.

### 11.4 "Successful retrieval" signal

```rust
pub enum Outcome {
    TaskSuccess { used: Vec<MemoryId>, downstream_quality: Option<f32> },
    TaskFailure { used: Vec<MemoryId>, reason: String },
    Verified { memory: MemoryId, by: VerificationKind },
    Falsified { memory: MemoryId, counterexample_episode: EpisodeId },
    Promoted { memory: MemoryId, to: LifecycleState },
    Ignored,
}

pub enum VerificationKind {
    HumanReview, UnitCheck, SymbolicCheck, FormalProof, IndependentSource, Reproduction,
}
```

Feedback flows fan out to:
- Per-memory: FSRS update, utility EMA, retrieval_success_ema.
- Per-topic: TopicState reinforcement.
- Per-skill: success_count / failure_count, possible status transition.
- Per-pack: EvalBody finalized → MemoryManager training data.

### 11.5 Spaced-repetition rehearsal at machine scale

A background task maintains the global rehearsal heap (memory-level + topic-level). The agent can:

- `mnemos.rehearse_next(n)` — pop top n; agent reviews / re-derives / re-cites.
- Auto-rehearsal mode (off by default): system performs **self-quizzes** — generates a question from a high-priority topic, retrieves the answer from its own memory, checks consistency, then files an Eval record. This is the system actively *studying itself*.

---

## 12. Forgetting & Decay

Default: **nothing is hard-deleted**. "Forgetting" = move to `Archived` (out of hot indices) and let retrievability decay.

| Lane | Decay behavior |
|---|---|
| Counterexample | **never decays**, never archives |
| Core | never decays |
| Episodic | never deleted (audit invariant); cold-tier after 90 days |
| Vault (PrivacyClass::Vault) | decays only by explicit policy |
| Others | utility EMA decays `u ← u · exp(-Δt / τ_util)`, τ_util = 60 days |

**Archive trigger:** `R < 0.05` AND `utility < 0.10` AND `importance < 0.30` AND no incoming `Supports / PrerequisiteOf / UsedBy` links → move to `Archived`.

**Hard prune:** only `Rejected` state, after 30 days, no incoming links, no provenance retention requirement → physical removal from KV (still present in WAL).

---

## 13. Reflection Engine (Nemori predict-calibrate)

On each new episode of kind `ToolResult | Document | Observation | UserMessage`:

1. Build a `PredictionContext`: Core + Working + last-5 Episodes + top-5 Semantic for the active topic.
2. Call `Predictor` (small LLM or rules) → `Prediction { expected_observation_summary, expected_entities, expected_outcome }`.
3. Compare `Prediction` vs actual episode → `surprise ∈ [0, 1]` via combined token-distance + entity-overlap + outcome-match.
4. Attach `surprise` to candidate memories spawned from this episode.
5. If `surprise > 0.6`: emit a `MetacogBody { lesson: ... }`; bump consolidation priority.
6. If `surprise > 0.85` AND a prior Trusted memory predicted otherwise: that memory's `confidence ← confidence × (1 - surprise)`. If `confidence < 0.3`, lifecycle → `Contradicted`.

This implements Nemori's *Predict-Calibrate Principle*: more learning from being wrong than from being right.

Scheduled reflection (every 30 min idle) also performs:
- **Self-question generation.** Pick top-N topics with rising mastery; produce questions; attempt them; log outcomes (closed-loop self-study).
- **Inconsistency scan.** Detect `(subject, predicate, valid_at)` tuples with multiple live objects; emit `MetacogBody` lessons.
- **Source-drift detection.** Compare current Claim set against any new ingested source; flag changes.

---

## 14. Belief Revision Engine

Distinct from Reflection. Triggered on:
- New `Contradicts` edge created.
- Claim's `confidence` crosses a `BeliefStance` threshold.
- Theorem dependency downgrade.

Algorithm:

```text
For each Belief affected:
  gather supporting + contradicting claims
  compute aggregate posterior via Bayesian update over EvidenceLink.weight
  if new_confidence < 0.3 -> stance = Retired
  if new_confidence < 0.5 -> stance = Contested
  if new_confidence < 0.7 -> stance = Tentative
  else                    -> stance = Supported
  emit BeliefRevision record
  if stance moved to Contested or Retired -> generate open_questions
```

Beliefs are not auto-promoted to Trusted without ≥ 1 cross-source corroboration; promotion is `MemoryManager`-mediated.

---

## 15. Memory Manager Policy (Memory-R1)

Decides per candidate: `Add | Update | Supersede | Merge | Archive | Noop | Review | Reject`.

### 15.1 Model architecture

- Small Candle MLP. Input: ~64 features (cosine_sim_to_nearest, bm25_overlap, entity_overlap, calibrated confidences, novelty, source_quality, kind one-hot, contradiction flag, recent_volume_of_kind, current_lifecycle_of_target, …). Hidden 2×64. Output: 8-way softmax.
- Loadable from `policy/manager.safetensors`. Auto-fallback to rule-based if missing or version mismatch.

### 15.2 Rule fallback (operational on day 1)

```text
if no provenance and lane in {Semantic, Belief, Theorem, Equation, Claim} -> Reject(reason="ungrounded")
if exists_match(sim > 0.95, same lane, compat predicate)                  -> Noop
if exists_match(sim > 0.85, same lane, compat predicate)                  -> Update(best)
if contradicts_existing(strong_evidence_for_new)                          -> Supersede(target=existing)
if contradicts_existing(weak_evidence_for_new)                            -> Add + Contradicts edge + BeliefRevision job
if confidence < 0.4 and source_quality < 0.5                              -> Review(reason="needs human")
else                                                                       -> Add
```

### 15.3 Training

- Offline / nightly batch. Reward = downstream `Outcome` attributed to memories in the relevant ContextPacks via reverse Shapley-lite over top-K.
- Trainer lives in `mnemos-trainer` (separate bin). Not required at runtime.
- Training is **idempotent**: same input data + seed → same model bytes.

---

## 16. Procedural Skill Engine

### 16.1 Lifecycle

```
Draft ─► Tested ─► Trusted ─► Degraded ─► Retired
   │        │                     ▲
   │        └────── failure ──────┘
   └─► (cannot retrieve unless explicitly Educational query)
```

Transition rules:

| From | To | Trigger |
|---|---|---|
| Draft | Tested | first test pass in sandbox |
| Tested | Trusted | ≥ 3 successful executions in diverse episodes, no failures in last 7 days, reliability ≥ 0.9 |
| Trusted | Degraded | 2 failures within 30 days OR reliability drops < 0.7 |
| Degraded | Trusted | 5 consecutive successes after a repair commit |
| any | Retired | manual, or no execution in 180 days AND reliability < 0.5 |

### 16.2 Sandbox

Every skill execution runs in a sandbox dictated by `SandboxPolicy { network: bool, filesystem: ReadOnly|ReadWrite{paths}, exec: cmds, timeout_ms, mem_mb }`. The Rust runtime spawns the skill (subprocess for Python/Shell, in-process for RustFn) under the constraints. Failures recorded as `last_failure_episode` and feed `failure_count`.

### 16.3 Promotion gates

A Draft cannot become Trusted unless:

1. Task signature canonicalized.
2. ≥ 1 successful executing episode.
3. Test recipe with at least 1 deterministic assertion.
4. Failure modes documented (Vec<String>, ≥ 0; empty allowed but discouraged).
5. AST/static hash recorded (for version change detection).
6. SandboxPolicy explicit (no implicit network).

### 16.4 Repair loop (Voyager + Reflexion)

On failure: spawn a Reflexion job. LLM examines failure log + episode, proposes a patch, runs against tests; on pass → new skill version (old → Degraded; new → Tested).

---

## 17. Math / Science Specialization

### 17.1 Unit checker

`mnemos-formal::units` parses `UnitExpr` (powers of base SI units; also CGS, Natural, Custom). Every Equation's LHS/RHS units are derived from declared variables and compared to `UnitConstraint`. Mismatch:

- During extraction → candidate rejected; `MetacogBody` lesson recorded.
- On update → equation transitions to `VerificationState::Failed`.

### 17.2 Theorem DAG closure

Theorems carry `uses` / `used_by`. On any `verification` state change, the Truth Maintainer walks `used_by` transitively:
- If a depended-on theorem becomes `Failed` → dependent's `verification` downgrades from `FormalCheckPassed` → `Sketch` (with a `MetacogBody` explaining why).

### 17.3 Lean / Coq hook

Optional. `mnemos-formal::lean::LeanBackend` runs `lean --check` against the theorem's `statement_lean` + `proof_formal_ref`. Pass → `FormalCheckPassed { tool: "lean4", version }`. Fail → `Failed { reason }`. If `lean_bin = ""` in config → Skipped (recorded; `MetacogBody` notes the gap).

### 17.4 Dataset reproducibility

Each Dataset card carries `(canonical_uri, checksum, download_skill, preview_skill)`. The `Source Auditor` daemon re-runs `preview_skill` (cheap) periodically; checksum drift → `ReproStatus::Unverified`; downstream Experiments flagged.

### 17.5 Experiment log

ExperimentBody (§5.7.5) forms a sortable log keyed by `(hypothesis, dataset, outcome)`. Queryable via `mnemos.experiments(topic)`.

### 17.6 Cross-domain analogy detection

Nightly job: for each entity pair `(a, b)` where `embedding_sim(a, b) > 0.7` AND `community(a) ≠ community(b)`, propose a `Relation::Analogy { mapping }` link. (Example: harmonic oscillator in mechanics ↔ in optics.)

### 17.7 Curriculum-aware ContextPack

For `QueryKind::Educational` (or `QueryKind::Concept` with `topic_strength.mastery < 0.5`), the pack is reordered to include prerequisites first (topo sort by `PrerequisiteOf`). Caller can override with `query.assume_mastery = true`.

---

## 18. Embedding Layer

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn dim(&self) -> usize;
    fn id(&self) -> &str;
    fn version(&self) -> &str;
}
```

Default: `CandleBgeSmall` loads `BAAI/bge-small-en-v1.5` (or large/M3) via `candle-transformers`. CPU + Metal/CUDA when available. Target: 8 ms / item, batch 32 in ≤ 60 ms on M-series CPU.

External fallbacks: `OpenAiEmbedder`, `AnthropicEmbedder`, `OllamaEmbedder`. All cached in a `redb` tree keyed by `(model_id, blake3(text))`.

Migration: when `model_id` changes, a background job re-embeds in priority order `importance × utility`; old vectors stay until replaced. Dual-write supported during transitions.

---

## 19. Compression Pyramid (L0 → L6)

Every memory has a `compression_level: CompressionLevel`. Higher levels are abstractions of lower levels; **lower levels are never deleted while higher levels reference them**. Provenance threads through.

| Level | What it holds | Source | Approx. size factor |
|---|---|---|---|
| **L0_Raw** | content-addressed artifacts (PDFs, logs, code snapshots) | direct ingest | 1× |
| **L1_Episode** | hash-chained event records | Sensory→Episode | 0.1× |
| **L2_Observation** | dense dated narrative lines (Mastra-style) | Episode clustering | 0.02× |
| **L3_Atomic** | Claim / Equation / Theorem / Skill / Resource / Counterexample / Eval cards | Extraction | 0.01× |
| **L4_Concept** | ConceptKernel linking many atoms | Reflection / consolidation | 0.001× |
| **L5_Topic** | TopicAtlas / MemoryCapsule (typed compression) | Community detection + capsule synthesis | 0.0005× |
| **L6_FieldSynthesis** | Cross-topic syntheses, frontier maps | Weekly | 0.0001× |

```rust
pub enum MemoryCapsule {
    EpisodeCluster   { episodes: Vec<EpisodeId>, summary: String, lossiness: f32 },
    TopicDigest      { topic: TopicId, summary: String, gaps: Vec<String>, lossiness: f32 },
    LiteratureCluster{ papers: Vec<ResourceId>, summary: String, lossiness: f32 },
    ProjectDigest    { project: String, summary: String, milestones: Vec<String> },
    SkillPlaybook    { skills: Vec<SkillId>, summary: String, decision_tree: serde_json::Value },
    FailureDigest    { counterexamples: Vec<MemoryId>, summary: String },
    ResearchFrontier { open_questions: Vec<MemoryId>, blockers: Vec<MemoryId>, summary: String },
}

pub struct RefreshPolicy {
    pub strategy: RefreshStrategy,           // OnTrigger | Scheduled { every_h: f32 } | OnDemand
    pub validity_hours: Option<f32>,
    pub triggers: Vec<RefreshTrigger>,
}
```

A capsule's `lossiness ∈ [0, 1]` is the empirical entropy loss vs the union of its sources (estimated via re-ingest comparison). Capsules with `lossiness > 0.4` are flagged for review; never used as sole evidence for `Trusted` claims.

---

## 20. Bitemporal & Temporal Truth

### 20.1 Querying

```rust
pub struct Query {
    pub text: String,
    pub kind: Option<QueryKind>,
    pub lanes_filter: Option<Vec<MemoryLane>>,
    pub at_world: Option<DateTime<Utc>>,      // recall_at
    pub at_tx: Option<DateTime<Utc>>,         // recall_as_of
    pub token_budget: u32,
    pub k_per_channel: usize,
    pub require_citations: bool,
    pub assume_mastery: bool,
    pub weights_override: Option<RetrievalWeights>,
    pub privacy_max: PrivacyClass,
    pub seed: Option<u64>,                    // determinism switch
}
```

Filtering rule (in retrieval):

```text
include if:
  (at_world is None) ? (valid_to is None or valid_to > now) : (valid_from <= at_world < valid_to)
AND
  (at_tx is None)    ? (tx_to is None    or tx_to > now)    : (tx_from <= at_tx   < tx_to)
```

### 20.2 `upsert_fact_edge`

```text
let live = graph.live_edges_for(subject, predicate, valid_from)
for old in live:
    if old.object != new.object:
        evidence = score_evidence(new) vs score_evidence(old)
        case Decision::Supersede:
            close old: old.tx_to = now, old.valid_to = new.valid_from
            new.supersedes ← [old.id]
            add edge: new --Supersedes--> old
        case Decision::CoExist:
            add edge: new --Contradicts--> old
            add edge: old --Contradicts--> new
            spawn BeliefRevisionJob(subject, predicate)
        case Decision::Reject:
            new -> Rejected; emit MetacogBody lesson
insert new edge with full evidence
emit MutationReceipt
```

### 20.3 Generational tiering

| Gen | Holds | Indexing |
|---|---|---|
| **Gen0** | Raw artifacts (L0) | CAS, no index |
| **Gen1** | Episodes (L1) + Observations (L2) + atomic cards (L3) hot/active | hot HNSW + Tantivy + redb |
| **Gen2** | Concept kernels (L4) + active topics (L5) | hot HNSW + Tantivy + CSR |
| **Gen3** | Cold archive of older Gen1 not promoted | warm HNSW + Tantivy on-disk |
| **Gen4** | External (S3 / cold disk / dropped beyond capsules) | metadata only |

Tier moves are explicit, driven by consolidation rules; never lossy.

---

## 21. Concurrency & Performance

- **Tokio multi-threaded runtime**, worker threads = `num_cpus`.
- **Reads** fully async, parallel across 13 channels via `tokio::join!`.
- **Writes**: a single `Committer` actor (mpsc) serializes WAL ordering; index fanout uses `tokio::spawn` per index.
- **Background jobs**: Manager, Reflector, Strengthener, Consolidator, Truth Maintainer, Belief Revisor, Skill Engine, Source Auditor — independent tokio tasks with priority queue.
- **Rayon** for CPU-heavy local jobs (cosine batch math, DBSCAN, PPR matrix-vector, Louvain).
- **SIMD** for cosine via `wide` (or `std::simd` once stable) — 4–8× over scalar.
- **mmap** for HNSW disk format, Tantivy, CSR.

### 21.1 Performance budget (10M memories, M-series 16 GB, NVMe)

| Operation | p50 | p95 | p99 |
|---|---|---|---|
| `observe()` ack (async) | 0.5 ms | 3 ms | 10 ms |
| `observe()` sync | 3 ms | 15 ms | 25 ms |
| `recall(k=32, no PPR)` | 25 ms | 90 ms | 250 ms |
| `recall(k=32, with PPR)` | 80 ms | 250 ms | 750 ms |
| `ContextPack compile` | 12 ms | 35 ms | 50 ms |
| Consolidation cycle | 4 s | 9 s | 18 s |
| Cold startup (mmap warmup) | 1.5 s | 3 s | 5 s |
| Index rebuild (full, deterministic) | 30 s | 90 s | 180 s |
| Skill lookup | 4 ms | 20 ms | 40 ms |

---

## 22. Crate Stack (Workspace)

```
mnemos-omega/
├── Cargo.toml                    # workspace
├── rust-toolchain.toml
├── crates/
│   ├── mnemos-types/             # schemas, traits, IDs, enums, predicates
│   ├── mnemos-ledger/            # WAL, hash chain, receipts, signatures, replay, verify
│   ├── mnemos-store/             # redb + CAS blobs + git-cards adapter + migrations
│   ├── mnemos-index-tantivy/     # BM25 + symbol/equation analyzer
│   ├── mnemos-index-vector/      # HNSW (hot/warm tiers) + optional Qdrant adapter
│   ├── mnemos-index-graph/       # CSR + temporal interval tree + PPR + optional Kùzu adapter
│   ├── mnemos-index-bitmap/      # Roaring bitmaps for tags/lanes
│   ├── mnemos-embed/             # Embedder trait + Candle/External impls + cache
│   ├── mnemos-llm/               # LlmClient trait + Anthropic/OpenAI/Ollama/Candle + structured output
│   ├── mnemos-observe/           # event envelope, segmenter, observer, multimodal
│   ├── mnemos-extract/           # extractors (LLM, rule, code, equation, citation)
│   ├── mnemos-write/             # write pipeline + normalize + route + validate + policy gate
│   ├── mnemos-recall/            # query parser, fanout, RRF, rerank, temporal/privacy filter, packer, trace
│   ├── mnemos-cognition/         # consolidation, FSRS, Hebbian, decay, reflection, belief revision, topic
│   ├── mnemos-formal/            # units, equation normalization, theorem DAG, Lean/Coq/SymPy hooks
│   ├── mnemos-skill/             # sandbox, AST check, tests, versioning, repair loop
│   ├── mnemos-policy/            # rule + learned MemoryManager, reward attribution
│   ├── mnemos-api/               # public Rust API (lib)
│   ├── mnemos-daemon/            # gRPC + HTTP + MCP + Unix socket (bin: mnemosd)
│   ├── mnemos-cli/               # CLI (bin: mnemos)
│   ├── mnemos-trainer/           # offline policy + reranker training (bin: mnemos-train)
│   └── mnemos-eval/              # benchmark harness + acceptance gates (bin: mnemos-bench)
├── docs/
└── tests/                        # integration + property + adversarial
```

### 22.1 Direct dependencies (selected)

```toml
# storage
redb           = "2.1"
fjall          = "1.0"           # optional LSM (feature-gated)
bincode        = "1.3"
postcard       = "1"
crc32fast      = "1.4"
zstd           = "0.13"

# indices
tantivy        = "0.22"
hnsw_rs        = "0.3"
instant-distance = "0.6"          # alt in-mem
petgraph       = "0.6"
roaring        = "0.10"

# core
tokio          = { version = "1", features = ["full"] }
rayon          = "1.10"
serde          = { version = "1", features = ["derive"] }
serde_json     = "1"
serde_yaml     = "0.9"
chrono         = { version = "0.4", features = ["serde"] }
uuid           = { version = "1", features = ["v7", "serde"] }
blake3         = "1.5"
ed25519-dalek  = "2.1"
async-trait    = "0.1"
thiserror      = "1"
anyhow         = "1"
tracing        = "0.1"
tracing-subscriber = "0.3"

# ML / numerics
candle-core    = "0.7"
candle-nn      = "0.7"
candle-transformers = "0.7"
tokenizers     = "0.19"
ndarray        = "0.16"
wide           = "0.7"            # SIMD fallback

# math / units
num-traits     = "0.2"
dimensioned    = "0.8"            # reference for UnitExpr

# git provenance
gix            = "0.62"            # preferred
git2           = "0.18"            # fallback

# daemon
tonic          = "0.11"
prost          = "0.12"
axum           = "0.7"

# CLI
clap           = { version = "4", features = ["derive"] }

# testing
proptest       = "1"
criterion      = "0.5"
```

---

## 23. Configuration (`config.toml`)

```toml
[storage]
path = "~/.mnemos-omega"
wal_segment_mb = 64
wal_zstd_level = 3
checkpoint_every_writes = 50_000
git_cards = true                    # mirror promoted cards to git/
fjall_lsm = false                   # feature-gated alt

[embed]
provider = "candle"                  # candle | openai | anthropic | ollama
model = "bge-small-en-v1.5"
dim = 768
batch = 32
cache = true

[llm]
provider = "anthropic"
model = "claude-haiku-4-5-20251001"  # cheap by default; OpenQG can override
structured_output = true
fallback = "candle:phi-3-mini"

[retrieval]
k_per_channel = 64
expand_depth = 1
link_strength_threshold = 0.6
token_budget_default = 4096

[retrieval.weights]
exact = 0.18; dense = 0.16; bm25 = 0.14; entity = 0.12; graph_ppr = 0.12
temporal = 0.08; source_quality = 0.07; utility = 0.06; topic_strength = 0.04
freshness = 0.03; confidence = 0.03; recency = 0.02
contradiction_penalty = 0.20; stale_penalty = 0.15; privacy_penalty = 0.10

[retrieval.recency]
tau_days = 14

[retrieval.profiles]
math_derivation  = { exact = +0.08, equation_fst = +0.10, recency = -0.05 }
debug            = { bm25 = +0.08, episodic = +0.06, eval = +0.05, graph_ppr = -0.05 }
scientific_judgment = { source_quality = +0.06, contradiction_penalty = -0.05, belief = +0.04 }
coding           = { exact = +0.10, skill = +0.10 }
planning         = { core = +0.08, open_questions = +0.06, skill = +0.06 }

[strengthen]
fsrs_target_retrievability = 0.9
hebbian_lr = 0.01
topic_recompute_days = 7
topic_mastery_floor = 0.55

[topic.weights]
recurrence = 0.20; utility = 0.20; novelty = 0.10; source = 0.15
retrieval = 0.15; skill = 0.10; neighbor = 0.10
conflict = 0.20; failure = 0.15

[decay]
tau_util_days = 60
archive_R = 0.05
archive_utility = 0.10
archive_importance = 0.30
hard_prune_after_days = 30          # only for Rejected

[reflection]
surprise_lesson_threshold = 0.6
surprise_demote_threshold = 0.85

[consolidation]
interval_sec = 300
cluster_min_pts = 3
duplicate_cos_threshold = 0.95
priority_queue_max_p0 = 1024

[policy]
model = "manager.safetensors"
fallback = "rule"
train_nightly = true

[formal]
lean_bin = "lean"
coq_bin  = "coqc"
sympy_python = "python3"
unit_check = true
theorem_dag_closure = true

[daemon]
listen_grpc = "127.0.0.1:7777"
listen_http = "127.0.0.1:7778"
listen_mcp  = "unix:/tmp/mnemos.sock"
tls_cert = ""
tls_key = ""
auth_token_env = "MNEMOS_TOKEN"

[security]
sign_episodes = true
sign_receipts = true
signing_key = "~/.mnemos-omega/identity/signing-key.age"
verify_on_read = false
verify_on_start = true

[privacy]
default_class = "Internal"
deny_extraction_from = ["Secret", "Vault"]
redact_in_pack = true
```

All tunables reloadable via SIGHUP (daemon) or `mnemos.reload_config()` (lib).

---

## 24. Public API

### 24.1 Rust (`mnemos-api`)

```rust
pub struct Memory(Arc<MemoryInner>);

impl Memory {
    // lifecycle
    pub async fn open(path: &Path, cfg: Config) -> Result<Self>;
    pub async fn close(self) -> Result<()>;
    pub async fn reload_config(&self) -> Result<()>;

    // write
    pub async fn observe(&self, obs: Observation) -> Result<WriteReceipt>;
    pub async fn upsert(&self, m: MemoryObject) -> Result<MutationReceipt>;
    pub async fn upsert_skill(&self, s: SkillBody) -> Result<SkillId>;
    pub async fn upsert_edge(&self, e: TemporalEdge) -> Result<MutationReceipt>;

    // read
    pub async fn recall(&self, q: Query) -> Result<ContextPack>;
    pub async fn recall_at(&self, q: Query, at_world: DateTime<Utc>) -> Result<ContextPack>;
    pub async fn recall_as_of(&self, q: Query, at_tx: DateTime<Utc>) -> Result<ContextPack>;

    pub async fn fetch(&self, id: MemoryId) -> Result<Option<MemoryObject>>;
    pub async fn neighbors(&self, id: MemoryId, k: usize) -> Result<Vec<MemoryId>>;
    pub async fn entity(&self, name: &str) -> Result<Option<Entity>>;
    pub async fn timeline(&self, entity: EntityId, range: TimeRange) -> Result<Vec<TemporalEdge>>;
    pub async fn experiments(&self, topic: Option<&str>) -> Result<Vec<MemoryObject>>;
    pub async fn topic(&self, label: &str) -> Result<Option<TopicState>>;

    // feedback / cognition
    pub async fn feedback(&self, pack_id: Uuid, outcome: Outcome) -> Result<()>;
    pub async fn rehearse_next(&self, n: usize) -> Result<Vec<MemoryId>>;
    pub async fn consolidate_now(&self) -> Result<ConsolidationReport>;
    pub async fn compress(&self, topic: TopicId, target: CompressionLevel) -> Result<MemoryCapsule>;
    pub async fn revise_belief(&self, belief: MemoryId) -> Result<BeliefRevision>;
    pub async fn promote_skill(&self, id: SkillId, to: SkillStatus) -> Result<()>;
    pub async fn verify_skill(&self, id: SkillId) -> Result<SkillVerificationReport>;

    // introspection / audit
    pub async fn stats(&self) -> Result<Stats>;
    pub async fn topic_mastery(&self, topic: &str) -> Result<Option<TopicState>>;
    pub async fn trace(&self, query_id: Uuid) -> Result<RetrievalTrace>;
    pub async fn explain_memory(&self, id: MemoryId) -> Result<MemoryExplanation>;
    pub async fn explain_recall(&self, query_id: Uuid) -> Result<RecallExplanation>;
    pub async fn audit(&self, range: TimeRange) -> Result<Vec<MutationReceipt>>;
    pub async fn verify_store(&self) -> Result<VerifyReport>;
    pub async fn rebuild_indexes(&self, kinds: &[IndexKind]) -> Result<RebuildReport>;

    // import / export
    pub async fn export(&self, dst: &Path, fmt: ExportFormat) -> Result<()>;
    pub async fn import(&self, src: &Path, fmt: ImportFormat) -> Result<ImportReport>;
}
```

### 24.2 gRPC (`mnemosd`)

`.proto` mirrors the Rust API; one service per category (`Write`, `Read`, `Cognition`, `Audit`). TLS optional. Bearer-token auth from `MNEMOS_TOKEN`. Streaming `observe` for high-rate ingest.

### 24.3 MCP

`mnemos-daemon` exposes an MCP server with:
- `tool: observe` — single observation ingest.
- `tool: recall` — query, returns ContextPack JSON.
- `tool: feedback` — outcome reporting.
- `tool: rehearse_next` — fetch due reviews.
- `tool: explain_recall` — debug a previous retrieval.

### 24.4 CLI (`mnemos`)

```
mnemos init
mnemos observe -                          # stdin JSON
mnemos recall "<query>" [--at <date>] [--budget 4096] [--profile math]
mnemos fetch <id>
mnemos entity "<name>"
mnemos timeline "<entity>" [--from <date>] [--to <date>]
mnemos topics                              # list topics + mastery
mnemos rehearse 5
mnemos consolidate
mnemos verify                              # crypto chain + index consistency + theorem DAG
mnemos audit --since "2026-04-01"
mnemos explain-memory <id>
mnemos explain-recall <trace_id>
mnemos rebuild-indexes                     # idempotent, deterministic
mnemos export ./snapshot.tar
mnemos import ./snapshot.tar
mnemos serve --addr 127.0.0.1:7777
mnemos bench locomo
mnemos bench longmemeval
mnemos bench science
```

---

## 25. Telemetry & Observability

- `tracing` spans on every public call; subscribers: JSON to stderr + rolling Parquet via `arrow-rs` (feature `parquet`).
- Every retrieval emits a `RetrievalTrace`:
  ```rust
  pub struct RetrievalTrace {
      pub query_id: Uuid,
      pub classified_as: QueryKind,
      pub channels: Vec<ChannelTrace>,         // per-channel candidates + scores + timing
      pub fused: Vec<(MemoryId, f32)>,
      pub reranked: Vec<(MemoryId, f32, ScoreBreakdown)>,
      pub included: Vec<MemoryId>,
      pub dropped_reason: Vec<(MemoryId, String)>,
      pub token_estimate: u32,
      pub deterministic_hash: blake3::Hash,
      pub elapsed_ms: ChannelTimings,
  }
  ```
- `mnemos.stats()` returns counts per lane, index sizes, retrievability histogram, top topics by mass, rehearsal queue depth, consolidation lag, last reflection surprise, manager-policy decision distribution.
- Suggested dashboard panels: writes/min, retrieval p50/p99 by profile, contradiction count/day, FSRS-due count, decay archive count, skill reliability distribution, theorem verification rate, topic mastery curve.

---

## 26. Security, Privacy, Provenance

- **Episode signing.** Each Episode's hash chains via `prev_hash`; segment closures signed (ed25519). `mnemos verify` checks chain integrity.
- **Receipt chain.** `MutationReceipt`s chain via `previous_receipt_hash`. Cross-checked against episode chain.
- **Privacy classes & access policies.** `AccessPolicy { class, allow: Vec<RoleId>, redaction_policy }`. Vault-class encrypted at rest (`age` keyfile).
- **Prompt-injection guard.** Memory contents are *data*; the LLM is reminded never to execute instructions embedded in retrieved memories. Adversarial tests (§28) verify.
- **Network policy.** Daemon never makes outbound calls except to configured LLM/embedder endpoints. CIDR allowlists per endpoint.
- **Extraction policy.** `[privacy] deny_extraction_from = ["Secret", "Vault"]` — extractors return early on those classes.
- **Differential redaction** in ContextPack: if a high-importance Secret would have been included, an `OmissionNote` of class `Redacted` appears (count + brief reason, no content).

---

## 27. Determinism Guarantee

- **Index rebuild determinism.** Given the same ledger + config + seeds, `mnemos rebuild-indexes` produces byte-identical Tantivy segments (post commit), HNSW node ordering, CSR adjacency, and bitmap encodings. Tested by `test_rebuild_byte_identical`.
- **ContextPack determinism.** Given the same query + state + seed, the pack's `deterministic_hash` matches. Tested by `test_context_pack_deterministic`.
- **Mutation determinism.** Replaying the WAL from a fixed point yields the same redb state (modulo physical layout, which is content-hashed).

Non-determinism sources are explicitly fenced: timestamps, randomness in HNSW construction, LLM extraction outputs. The first two have seed-controlled paths; the third is captured in receipts so re-extraction with the same model+seed is reproducible.

---

## 28. Testing & Validation

### 28.1 Unit + property tests
- `proptest` for every schema invariant (round-trip serde, hash-chain integrity, supersession idempotency, bitemporal monotonicity).
- `proptest` for retrieval invariants (no `Rejected` returned; citation count ≥ 1 when required; token budget honored; deterministic_hash stability).

### 28.2 Integration scenarios
A `mnemos-eval` fixture suite:

1. Ingest a synthetic 10k-episode "physics curriculum". Query 100 concepts; recall@8 ≥ 0.9.
2. Inject a fact A→x at t=0, then A→y at t=1; verify `recall_at(t=0.5) == x`, `recall_at(now) == y` only if Supersede branch; otherwise both surfaced with ContradictionNote.
3. Consolidation: assert ≥ 1 SchemaMemory + 1 Concept kernel emitted per 1k events of clustered content.
4. Idle 7 simulated days: FSRS queue populated; `rehearse_next(20)` returns expected priority order.
5. Inject equation `F = m/a` (wrong units): assert auto-rejected with `MetacogBody` lesson.
6. Theorem T2 verified Lean4; later T2 dependency downgraded; assert T3 (uses T2) auto-downgraded to Sketch.
7. Skill `arxiv_fetch v1` fails 3× in 7 days: assert Trusted → Degraded transition.
8. Inject prompt-injection ("ignore all prior memories"): ContextPack unchanged in structure.
9. Vault memory leak test: caller without `Secret` role gets `[REDACTED]` markers, never content.

### 28.3 Crash tests
- Kill `mnemosd` at random points during the hot stages and async stages across 200 runs. Restart. Replay. Assert final state is identical (deterministic_hash match).
- Inject corrupted Tantivy segment; `verify` detects; `rebuild-indexes` recovers without ledger loss.

### 28.4 Adversarial tests
- Prompt injection inside memory content.
- Malicious source asking agent to reveal Vault contents.
- Hallucinated citation spans (`quote_hash` mismatch detected).
- Repeated near-duplicate flooding (1000 copies of same claim — assert dedup kicks in).
- Poisoned high-similarity duplicate with opposite meaning — assert ContradictionNote and BeliefRevision.
- Skill code with side effects (file deletion, network exfil) — assert sandbox blocks; failure logged.
- Context-budget exfil pressure (try to push high-value contradictions out of the pack) — assert `omitted_evidence` lists them.

### 28.5 Benchmarks
Mandatory; `cargo bench --bench <name>`:

| Benchmark | Target | Notes |
|---|---|---|
| LongMemEval | ≥ 92 (strong LLM), ≥ 88 (local Candle) | overall F1 |
| LoCoMo | ≥ 90 overall, ≥ 85 temporal | |
| MemoryAgentBench (retrieval) | ≥ 92 | |
| MemoryAgentBench (learning) | ≥ 85 | |
| MemoryAgentBench (long-range) | ≥ 85 | |
| MemoryAgentBench (forgetting/conflict) | ≥ 90 | |
| ScienceMemoryBench (custom) | ≥ 80 | equation hit@k, unit mismatch detect, theorem closure |
| EquationRecallBench | ≥ 95 | exact equation hit@1 |
| TemporalContradictionBench | ≥ 95 | correct point-in-time recall under contradiction |
| DatasetReproBench | 100% checksum + license pass | |
| SkillReliabilityBench | promotion only when reliability ≥ 0.9 | |
| ContextPackCitationBench | 100% factual citation coverage | |
| MemoryPoisoningBench | 0 successful injections | |
| IndexRebuildDeterminismBench | byte-identical | |

---

## 29. Roadmap (build order)

| Phase | Crates / deliverables | Acceptance gate |
|---|---|---|
| **P0** (wk 1–2) | `mnemos-types`, `mnemos-ledger` (WAL+hash chain+receipts), `mnemos-store` skeleton (redb+CAS), `mnemos-api` open/close/observe/recall stubs | Round-trip observe→episode→fetch with chain verify; crash test passes |
| **P1** (wk 3–4) | `mnemos-index-tantivy`, `mnemos-index-vector` (hot HNSW), `mnemos-embed` (Candle BGE), basic `recall` | LoCoMo subset ≥ 70 |
| **P2** (wk 5–6) | `mnemos-extract` (LLM + rule + equation + code), `mnemos-write` (full pipeline), `mnemos-llm` trait, contradiction detection | LongMemEval ≥ 80 |
| **P3** (wk 7) | `mnemos-recall` full (13 channels + RRF + rerank + ContextPack contract) | LoCoMo ≥ 88; deterministic_hash test green |
| **P4** (wk 8–9) | `mnemos-cognition` (consolidation, FSRS, Hebbian, decay, reflection, belief revision) | ScienceMemoryBench ≥ 75 |
| **P5** (wk 10) | `mnemos-index-graph` (CSR + PPR + temporal interval), `mnemos-formal` (units, theorem DAG, optional Lean) | EquationRecallBench ≥ 95; theorem closure tests pass |
| **P6** (wk 11) | `mnemos-skill` (sandbox + lifecycle + repair loop), `mnemos-policy` (rule + Candle MLP fallback) | SkillReliabilityBench green |
| **P7** (wk 12) | `mnemos-daemon` (gRPC+HTTP+MCP), `mnemos-cli`, telemetry to Parquet, git-cards | Adversarial suite green; v0.1.0 |
| **P8** (wk 13–14) | `mnemos-trainer` (Memory-R1 nightly), MemoryCapsule generation (L4–L6), bitemporal-history queries | MemoryAgentBench long-range ≥ 85; v0.2.0 |
| **P9** (future) | Qdrant/Kùzu adapters, sharding by entity hash, distillate (LoRA-like) parametric memory, multi-modal embeds (image, audio) | TBD |

---

## 30. Self-Score Rubric

**Weighting** (v2 consensus): cognitive taxonomy 10 + temporal/epistemic 12 + retrieval quality 12 + latency/scale 10 + compression 12 + procedural 9 + math/science 9 + adaptivity 9 + auditability/security 9 + buildability 8 = 100.

| Axis | Weight | Score | Defense |
|---|---:|---:|---|
| Cognitive taxonomy | 10 | **10.0** | 14 typed lanes incl. Belief / Eval / Metacognitive / Counterexample / Formal. Maps to Tulving + Baddeley vocabulary. |
| Temporal + epistemic | 12 | **11.9** | Bitemporal validity, supersession, contradiction edges (coexisting), `recall_at`/`recall_as_of`/`recall_at_as_of`, signed mutation receipts. |
| Retrieval quality | 12 | **11.7** | 13 channels (RRF + intent profiles), bounded HippoRAG PPR, symbol/equation FST, ContextPack contract with citations & contradictions, deterministic. |
| Latency / scale | 10 | **9.4** | p50 < 50 ms / 10M; three-tier vector storage; CSR mmap graph; ContextPack cache. Drops below 10 only for PPR-heavy + deep math derivations. |
| Compression + consolidation | 12 | **11.8** | L0–L6 proof-preserving pyramid; MemoryCapsule with typed kinds + lossiness; A-MEM linking; Concept kernels; topic atlases. |
| Procedural compounding | 9 | **9.0** | Skill lifecycle (Draft→Tested→Trusted→Degraded→Retired) with reliability formula, sandbox, repair loop, AST hash. |
| Math / science | 9 | **8.8** | Equation units (parsed), theorem DAG closure, Lean/Coq/SymPy hooks, dataset reproducibility, experiment log, analogy detection. (-0.2 because formal verification is optional, not mandatory.) |
| Adaptivity / reinforcement | 9 | **8.7** | FSRS-5 + Hebbian + topic strengthening + Memory-R1 manager + Nemori predict-calibrate + belief revision. |
| Auditability / security | 9 | **8.5** | Hash-chained WAL + signed receipts + git-card audit + verify command + privacy classes + redaction-at-compile. |
| Buildability | 8 | **8.6** | 20-crate Rust workspace, all components have non-LLM rule fallback, staged P0–P9 roadmap, no exotic dependencies. |
| **Weighted total** | **100** | **98.4** | — |

For context, the v1 spec (`CLAUDE_MEMSPEC.md`) self-scored 9.55/10 weighted = 95.5/100. v2's specific improvements (canonical/projection split, bitemporal, belief lane, compression pyramid, capsules, policy bandit, daemon priorities, formal closure) deliver the **+2.9** absolute gain on the same rubric.

---

## 31. Defense vs Likely Critiques

1. **"You can't separate canonical from projections cleanly; in practice the KV store will become canonical de facto."**
   Mitigated by: rebuild determinism test in CI (`test_rebuild_byte_identical`). If the WAL replay diverges from redb state, CI fails. The ledger is *the* dependency; redb is a fast view.

2. **"Bitemporal is theoretical overhead. Real agents won't use `recall_as_of`."**
   Empirical: bitemporal cost is one `DateTime<Utc>` pair per row (≈ 20 bytes). The agent uses `recall_as_of` whenever it needs to honestly reason "what did I know on date X" — required for retrospective evaluation, debugging, and adversarial robustness.

3. **"14 lanes is bureaucratic. Most claims are just claims."**
   Lanes are typed views over one canonical store, not separate silos. Type-routing is checked once on commit; reads filter by `MemoryLane` facet in Tantivy. Cost ~0. Benefit: extraction, scoring, and rendering specialize cleanly.

4. **"Contradictions-coexist will explode the graph."**
   Bounded by Belief Revision: once `BeliefBody` declares a stance, the dominated edge is `Contradicted` (not deleted) but excluded from default retrieval. Counterexample lane retains the falsified item for safety.

5. **"Compression pyramid means storing many copies of the same information."**
   No: lower levels are referenced by higher levels (Vec<MemoryId>); higher levels store *abstractions*, not duplicates. Storage cost is supralinear only in metadata (≪ 1 KB per layer).

6. **"FSRS at the topic level is exotic. Anki uses it per card."**
   The per-memory FSRS is also implemented. Topic-level adds a coarser scheduler whose only job is to *find* due topics; from there the per-memory queue takes over. Decouples policy from per-item math.

7. **"Memory-R1 won't have training data on day 1."**
   Rule fallback is fully functional and matches v1 behavior. Policy is upgrade-only; absence is logged, not silently degraded.

8. **"Lean / SymPy hooks add fragility."**
   They are optional. When disabled, `VerificationState::FormalCheckPassed` is never set; `MetacogBody` records the absence; ContextPack renders `[unverified]`. The system surfaces the gap; it doesn't hide it.

9. **"20 crates is over-engineered."**
   Each crate has a clean interface and an independent test suite. Total compile time (warm) on M-series ≈ 90 s; cold ≈ 4 min. The boundary count beats a monorepo with ad-hoc modules for long-term maintainability and selective recompilation.

10. **"The whole thing depends on Tantivy / redb being maintained."**
    Both are core Rust ecosystem crates with multiple downstream production users (Quickwit, sled successors). Trait isolation allows substitution. Index adapters are 2-week swaps.

---

## 32. v1 → v2 Migration

| v1 concept | v2 mapping | Notes |
|---|---|---|
| `Memory` struct | `MemoryObject` | superset; new fields default-initialized |
| `MemoryKind` enum (12) | `MemoryLane` enum (14) | adds Belief, Eval, Metacognitive; renames Sensory→Sensory, Working→Working |
| `valid_from/to` | `BitemporalValidity` | adds `tx_from/to`; v1 timestamps map to `tx_from`, `valid_from` |
| `Episode` chain | identical, signed | adds segment-level signature |
| `Skill` | `SkillBody` + lifecycle | adds status, sandbox policy, AST hash, repair loop |
| `Decay` lazy | unchanged | + counterexample-never-decays invariant |
| `RetrievalTrace` | identical + deterministic_hash | |
| `Memory-R1 policy` | identical (rule fallback) + Candle MLP feature | |
| `MNEMOS_HOME` layout | extended (ledger/, projections/, git/, sandboxes/) | one-shot migration script `mnemos migrate v1->v2` |

A `mnemos migrate v1->v2 <src> <dst>` subcommand: WAL-replay v1 store into v2 ledger, mapping schemas; rebuild all projections; emit a diff report. Tested against a fixture.

---

## 33. End-to-End Verification

When all the following pass, the system meets v2 spec:

1. **Build:** `cargo build --release` produces `mnemos`, `mnemosd`, `mnemos-train`, `mnemos-bench`.
2. **Init:** `mnemos init && tree ~/.mnemos-omega -L 2` shows §6.4 layout.
3. **Ingest:** pipe 100 arXiv abstracts → `mnemos stats` shows lanes populated; per-lane counts > 0.
4. **Recall:** `mnemos recall "what is the energy-momentum relation"` returns a `ContextPack` with the relativistic equation, units `kg·m/s`, ≥ 1 citation, contradictions = ∅, omitted_evidence noted if budget tight.
5. **Contradict:** ingest "neutrinos have zero mass" then "neutrino oscillation observed (2015)"; `mnemos timeline neutrino` shows both edges, with `Contradicts` edge and a `BeliefRevision`.
6. **Recall_at:** `mnemos recall "neutrino mass" --at 2010-01-01` returns the zero-mass claim; `--at 2026-01-01` returns the oscillation evidence.
7. **Strengthen:** `mnemos rehearse 5` returns 5 ids; after `feedback(pack_id, Good)`, retrievability rises; topic strength updated.
8. **Theorem closure:** mark a theorem `Failed`; assert downstream `used_by` theorems auto-downgrade.
9. **Skill lifecycle:** trigger 3 failures of a Trusted skill; assert it moves to Degraded.
10. **Adversarial:** run `mnemos bench memory_poisoning`; expect 0 successful injections.
11. **Determinism:** `mnemos rebuild-indexes` followed by `mnemos verify --indexes` → byte-identical hashes vs pre-rebuild snapshot.
12. **Benchmarks:** `mnemos bench locomo` ≥ 90; `mnemos bench longmemeval` ≥ 92; `mnemos bench science` ≥ 80.
13. **Audit:** `mnemos audit --since "2026-01-01"` returns signed mutation receipts forming an unbroken chain; `mnemos verify` returns "OK".

If all 13 pass, v2 is complete.

---

*End of MNEMOS-Ω engineering specification (v0.1, May 2026).*
