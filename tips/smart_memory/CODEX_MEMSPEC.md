# CODEX_MEMSPEC: Rust Cognitive Memory Compiler

## Objective

Build a local-first Rust memory system for long-running research and coding agents. The system should learn math and science concepts, recall quickly, strengthen important topics dynamically, preserve provenance, and compound procedural skill over months without turning raw chat history into memory.

The recommended architecture is the **Rust Cognitive Memory Compiler**: a typed, source-preserving memory system that compiles small, cited, task-specific context packs from durable local memory stores.

The core loop is:

```text
Raw events
  -> observer/event segmenter
  -> append-only episode ledger
  -> typed memory router
  -> core / episodic / semantic / procedural / resource / belief / concept / eval stores
  -> temporal graph + concept-note graph + skill library
  -> hybrid indexes: Tantivy BM25 + vector HNSW/Qdrant adapter + graph adjacency
  -> context pack compiler
  -> agent action
  -> feedback, reflection, consolidation, topic strengthening
```

### Non-Goals

- Do not build implementation code as part of this spec.
- Do not store secrets, credentials, private vault data, or raw sensitive user data in repo memory.
- Do not make vector search the source of truth.
- Do not rely on raw chat transcripts as memory.
- Do not destructively overwrite facts or beliefs.
- Do not mutate generated zones or unrelated repo metadata.
- Do not require a hosted database or network service for baseline recall.

### Assumptions

- The first production target is a single-user local agent on a developer machine.
- Rust owns canonical memory IO, indexing coordination, policy, and CLI behavior.
- LLM calls may help extraction, reflection, and consolidation, but local recall must work without an LLM.
- Git-backed Markdown/YAML cards are curated memory artifacts; SQLite and indexes are rebuildable projections.
- Every promoted memory object has provenance, confidence, review state, and mutation receipts.

### Completion Criteria

- Local recall without an LLM is p95 <= 750 ms on warm indexes for common queries.
- Every memory object links to a source episode, resource span, tool run, or explicit human note.
- No destructive fact overwrite exists; changes create supersession or invalidation records.
- Context packs are bounded, cited, freshness-aware, and contradiction-aware.
- Index rebuilds are deterministic from canonical stores and mutation receipts.
- Topic strengthening is explainable from visible score components.
- Math/science memories separate concepts, claims, equations, evidence, questions, failed attempts, and beliefs.

## Score And Defense

Overall score: **9.6 / 10**

| Axis | Score | Rationale |
| --- | ---: | --- |
| Human-like | 9.8 | Uses typed memory lanes, episodes, semantic claims, beliefs, concept consolidation, procedural skills, reflection, and decay/reinforcement. |
| Adaptability | 9.7 | New evidence can update topic strength, invalidate temporal facts, evolve concept notes, create review queues, and revise beliefs without losing old provenance. |
| Fast recall | 9.2 | Uses local SQLite/Tantivy/HNSW/adjacency indexes, intent classification, bounded retrieval budgets, and no LLM requirement for the hot recall path. |
| Buildability | 8.9 | Rust-first components are direct to build, but extraction quality, contradiction handling, and graph/concept consolidation need staged rollout and tests. |
| Auditability | 9.8 | Append-only receipts, source spans, temporal validity windows, deterministic rebuilds, and Git-backed curated cards make memory explainable and reviewable. |

No single memory framework satisfies the whole objective. The source notes converge on a hybrid:

- MIRIX supplies the typed cognitive memory taxonomy.
- Graphiti/Zep supplies temporal truth, entity/fact evolution, and provenance.
- A-MEM supplies self-organizing concept notes and evolving links.
- Hindsight supplies separation of facts, experiences, beliefs, and mental models.
- Nemori supplies event segmentation and prediction-gap learning.
- Mem0 and Mastra Observational Memory supply fast practical recall and compressed observations.
- HippoRAG supplies associative multi-hop graph retrieval.
- Voyager and Memp supply procedural skill memory that compounds through use.

The key design rule is:

```text
Do not make the vector database the memory.
Make structured, source-controlled memory the truth.
Use graphs, vectors, observations, and skills as compiled indexes over it.
```

## Source Synthesis

| Source Pattern | What To Steal | Role In This Spec | What Not To Copy Blindly |
| --- | --- | --- | --- |
| MIRIX | Core, episodic, semantic, procedural, resource, and vault-style memory lanes | Typed memory router and lane-specific schemas | Avoid treating the vault as repo-stored memory; sensitive data must remain outside project memory |
| Graphiti / Zep | Temporal graph, validity windows, entity resolution, provenance | Canonical semantic graph and stale-fact handling | Do not require a hosted graph service for local-first baseline |
| A-MEM | Atomic notes, tags, links, evolving context, Zettelkasten-style self-organization | Concept cards, concept kernels, topic maps, link evolution | Do not run LLM graph evolution on every hot recall path |
| Hindsight | Facts, experiences, beliefs, mental models, Retain/Recall/Reflect | Epistemic separation: observation != claim != belief | Do not collapse beliefs into facts |
| Nemori | Learned event boundaries, episode construction, prediction gaps | Observer/event segmenter and reflection triggers | Do not chunk only by token count |
| Mem0 | Fast memory extraction, hybrid retrieval, low-token context | Practical recall API, dedupe/update policy, low-latency path | Do not let update/delete semantics erase provenance |
| Mastra Observational Memory | Dense dated observations and background reflection | Rolling autobiographical observation layer | Do not make the stable prefix the only retrievable memory |
| HippoRAG | Associative graph activation and multi-hop retrieval | Entity/concept/fact graph expansion and PPR-style scoring | Do not replace exact symbol/equation search with graph ranking alone |
| Voyager / Memp | Executable skills, descriptions, examples, preconditions, feedback | Procedural skill library and skill strengthening | Do not store untested code snippets as trusted skills |
| Generative Agents / MemoryBank | Recency + relevance + importance retrieval, reflection, decay/reinforcement | Topic strength, review cadence, reflection thresholding | Do not use unbounded natural-language memory streams |
| Letta / MemGPT | Core memory blocks, archival memory tools, self-editing state | Pinned core state and agent-visible memory tools | Do not let the agent silently self-edit policy or provenance |
| MemoryOS / MemOS | Lifecycle, tiering, governance | Store lifecycle and promotion/demotion policy | Do not overbuild an OS before the schemas and receipts are stable |
| EverMemOS / LightMem | Episodic trace formation, semantic consolidation, sleep-like compaction | Background consolidation jobs | Do not let summaries become the only source of truth |
| GraphRAG / LightRAG | Corpus-level maps and community summaries | Paper/book/dataset corpus memory | Do not use corpus summaries for source-level claims without citations |
| Titans | Fast neural long-context substrate | Future optional acceleration | Not a replacement for inspectable memory |

## Architecture

### System Shape

```text
smart-memory-core
  schema.rs
  engine.rs
  write_path.rs
  retrieval.rs
  graph.rs
  topic_strength.rs
  consolidation.rs
  context_pack.rs
  store/
    sqlite.rs
    tantivy.rs
    vector.rs
    graph_tables.rs

smart-memory-cli
  ingest
  recall
  reflect
  consolidate
  eval
  explain
```

### Memory Lanes

| Lane | Purpose | Examples | Hot Recall Priority |
| --- | --- | --- | --- |
| Core | Stable mission, user-approved preferences, active constraints | "Prefer local-first tools", project goals, model policy | Always considered, tiny budget |
| Episodic | Immutable events and observations | Search run, failed proof, benchmark, conversation, tool result | Retrieved by time, entity, task similarity |
| Semantic | Claims, facts, equations, entities, relations | "Method X assumes linearity", equation variables, paper claims | Retrieved by entity, BM25, graph, vector |
| Procedural | Skills, workflows, scripts, playbooks | "Extract equations from arXiv PDF", "run benchmark Y" | Retrieved by task intent and preconditions |
| Resource | Papers, datasets, repos, files, APIs | DOI, repo path, dataset manifest, notebook | Retrieved by exact ID, citation, topic |
| Belief | Current epistemic state with evidence links | "I currently believe claim C at confidence 0.72" | Retrieved only when question needs judgment |
| Concept | Self-organizing topic notes and kernels | "Gauge invariance", "RAG temporal validity" | Retrieved by concept/topic intent |
| Eval | What retrievals, answers, and skills worked | Bad answer record, test result, benchmark score | Retrieved for self-correction and ranking |

### Canonical Data Flow

1. `observe` receives an event envelope from chat, tool output, file ingestion, code run, dataset inspection, or explicit human note.
2. The observer detects event boundaries and emits one or more dated observations.
3. The episode ledger appends immutable source records with hashes.
4. The typed memory router proposes memory mutations for relevant lanes.
5. Validators check source spans, duplicates, contradictions, and schema invariants.
6. The mutation transaction writes canonical SQLite rows and append-only JSONL receipts.
7. Index writers update Tantivy, vector, and graph adjacency projections.
8. Consolidation jobs periodically create concept kernels, belief revisions, topic queues, and skill updates.
9. `recall` classifies intent, queries indexes in parallel, reranks candidates, and compiles a bounded context pack.
10. Feedback from actions and evals updates retrieval success, topic strength, skill utility, and reflection triggers.

### Ownership Boundaries

Canonical truth:

- Git-backed Markdown/YAML cards for curated, human-reviewable concept/resource/skill artifacts.
- SQLite WAL database for local metadata, entity/edge tables, topic state, receipts index, and object registry.
- Append-only JSONL mutation logs for audit and replay.

Rebuildable projections:

- Tantivy BM25/full-text indexes.
- Local HNSW vector indexes.
- Graph adjacency/materialized traversal tables.
- Context-pack cache.

Forbidden for implementation agents:

- Do not write generated zones directly.
- Do not store vault/private data in project memory cards.
- Do not mutate unrelated repo agent maps unless explicitly requested.
- Do not couple recall correctness to remote services.

## Rust Modules

### `smart-memory-core/schema.rs`

Owns versioned data structures, IDs, enums, provenance, status, and serialization compatibility. All persisted schemas need explicit `schema_version` and migration tests.

### `smart-memory-core/engine.rs`

Defines the top-level engine trait, transaction boundaries, service dependencies, and runtime orchestration. It should hide store implementation details behind traits.

### `smart-memory-core/write_path.rs`

Owns observation ingestion, event segmentation outputs, typed routing, validation, mutation receipts, and index fanout.

### `smart-memory-core/retrieval.rs`

Owns query classification, retrieval fanout, candidate normalization, reranking, confidence scoring, and failure reasons.

### `smart-memory-core/graph.rs`

Owns entity resolution, temporal fact edges, concept links, provenance links, contradiction links, and adjacency queries.

### `smart-memory-core/topic_strength.rs`

Owns decayed topic scores, review queues, reinforcement events, propagation, and explainability reports.

### `smart-memory-core/consolidation.rs`

Owns background jobs that merge duplicates, create concept kernels, revise beliefs, generate skill cards, and promote or demote memory objects.

### `smart-memory-core/context_pack.rs`

Owns budgeted context-pack construction, citations, omitted-evidence notes, freshness warnings, contradiction summaries, and deterministic packing.

### Store Adapters

- `store/sqlite.rs`: canonical metadata, graph tables, object registry, topic state, receipt index.
- `store/tantivy.rs`: exact/BM25 search over observations, claims, equations, resources, skills, and concept notes.
- `store/vector.rs`: `EmbeddingIndex` trait plus local HNSW implementation; optional Qdrant/pgvector adapters later.
- `store/graph_tables.rs`: adjacency, temporal queries, contradiction/support neighborhoods, PPR-compatible exports.

### `smart-memory-cli`

| Command | Purpose |
| --- | --- |
| `ingest` | Ingest files, notes, tool logs, papers, datasets, or explicit events. |
| `recall` | Return a cited context pack for a query without mutating memory. |
| `reflect` | Run scoped reflection over episodes, contradictions, skills, or topics. |
| `consolidate` | Run deterministic consolidation jobs and index rebuilds. |
| `eval` | Run recall, provenance, contradiction, performance, and skill-memory tests. |
| `explain` | Explain why a memory was stored, retrieved, strengthened, weakened, or omitted. |

## Data Model

### ID And Provenance Rules

- IDs are stable, content-addressable where practical, and prefixed by object type, for example `ep_`, `claim_`, `eq_`, `concept_`, `skill_`.
- Every derived memory links to at least one `Episode`, `ResourceCard`, or human-authored card.
- Source spans include byte offsets where possible, page/section anchors for documents, and line ranges for code resources.
- Status is explicit: `proposed`, `trusted`, `contradicted`, `superseded`, `deprecated`, or `rejected`.
- Confidence is not truth. Confidence only records current support quality.

### Schema Sketch

These are documentation sketches, not implementation code.

```rust
pub struct Episode {
    pub id: EpisodeId,
    pub source_kind: SourceKind,
    pub source_uri: Option<String>,
    pub observed_at: DateTime<Utc>,
    pub ingested_at: DateTime<Utc>,
    pub actor: Option<EntityId>,
    pub title: String,
    pub raw_ref: SourceRef,
    pub content_hash: Hash,
    pub summary: String,
    pub entities: Vec<EntityId>,
    pub tags: Vec<String>,
}

pub struct Observation {
    pub id: ObservationId,
    pub episode_id: EpisodeId,
    pub observed_at: DateTime<Utc>,
    pub statement: String,
    pub salience: f32,
    pub novelty: f32,
    pub prediction_gap: Option<PredictionGap>,
    pub source_span: SourceSpan,
}

pub struct ClaimCard {
    pub id: ClaimId,
    pub statement: String,
    pub domain: Domain,
    pub status: MemoryStatus,
    pub confidence: f32,
    pub support: Vec<EvidenceLink>,
    pub contradiction: Vec<EvidenceLink>,
    pub assumptions: Vec<String>,
    pub valid_time: ValidTime,
    pub provenance: Provenance,
}

pub struct EquationCard {
    pub id: EquationId,
    pub latex: String,
    pub normalized: String,
    pub variables: Vec<VariableDef>,
    pub units: Vec<UnitConstraint>,
    pub assumptions: Vec<String>,
    pub derivation_steps: Vec<DerivationStep>,
    pub linked_claims: Vec<ClaimId>,
    pub provenance: Provenance,
}

pub struct ConceptCard {
    pub id: ConceptId,
    pub name: String,
    pub aliases: Vec<String>,
    pub summary: String,
    pub prerequisites: Vec<ConceptId>,
    pub claims: Vec<ClaimId>,
    pub equations: Vec<EquationId>,
    pub resources: Vec<ResourceId>,
    pub open_questions: Vec<QuestionId>,
    pub topic_state: TopicId,
    pub links: Vec<ConceptLink>,
    pub provenance: Provenance,
}

pub struct TopicState {
    pub id: TopicId,
    pub label: String,
    pub strength: f32,
    pub half_life_hours: f32,
    pub recency: f32,
    pub recurrence: f32,
    pub utility: f32,
    pub novelty: f32,
    pub source_quality: f32,
    pub contradiction_pressure: f32,
    pub retrieval_success: f32,
    pub graph_neighbor_boost: f32,
    pub last_strengthened_at: Option<DateTime<Utc>>,
    pub next_review_at: Option<DateTime<Utc>>,
}

pub struct FactEdge {
    pub id: EdgeId,
    pub subject: EntityId,
    pub relation: RelationType,
    pub object: EntityOrLiteral,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    pub confidence: f32,
    pub support: Vec<EvidenceLink>,
    pub supersedes: Vec<EdgeId>,
}

pub struct BeliefState {
    pub id: BeliefId,
    pub proposition: String,
    pub confidence: f32,
    pub stance: BeliefStance,
    pub supporting_claims: Vec<ClaimId>,
    pub contradicting_claims: Vec<ClaimId>,
    pub changed_by: Vec<MemoryMutationId>,
    pub rationale: String,
    pub updated_at: DateTime<Utc>,
}

pub struct SkillCard {
    pub id: SkillId,
    pub name: String,
    pub description: String,
    pub preconditions: Vec<String>,
    pub procedure: Vec<SkillStep>,
    pub examples: Vec<EpisodeId>,
    pub tests: Vec<SkillTestRef>,
    pub success_rate: f32,
    pub last_used_at: Option<DateTime<Utc>>,
    pub provenance: Provenance,
}

pub struct ResourceCard {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub title: String,
    pub uri: String,
    pub authors: Vec<EntityId>,
    pub published_at: Option<DateTime<Utc>>,
    pub content_hash: Option<Hash>,
    pub trust_profile: SourceQuality,
    pub extracted_claims: Vec<ClaimId>,
    pub extracted_equations: Vec<EquationId>,
}

pub struct QuestionCard {
    pub id: QuestionId,
    pub question: String,
    pub topic: TopicId,
    pub status: QuestionStatus,
    pub why_it_matters: String,
    pub partial_answers: Vec<ClaimId>,
    pub blockers: Vec<String>,
    pub next_actions: Vec<SkillId>,
}

pub struct EvalMemory {
    pub id: EvalId,
    pub task_signature: String,
    pub query: String,
    pub retrieved: Vec<MemoryId>,
    pub answer_quality: EvalScore,
    pub failure_tags: Vec<String>,
    pub corrected_by: Option<EpisodeId>,
    pub updated_at: DateTime<Utc>,
}

pub struct MemoryMutation {
    pub id: MemoryMutationId,
    pub mutation_type: MutationType,
    pub target: MemoryId,
    pub actor: MutationActor,
    pub before_hash: Option<Hash>,
    pub after_hash: Hash,
    pub reason: String,
    pub source: Provenance,
    pub created_at: DateTime<Utc>,
}

pub struct ContextPack {
    pub id: ContextPackId,
    pub query: RecallQuery,
    pub budget: ContextBudget,
    pub items: Vec<ContextItem>,
    pub citations: Vec<Citation>,
    pub contradictions: Vec<ContradictionNote>,
    pub freshness: FreshnessReport,
    pub confidence: f32,
    pub omitted_evidence: Vec<OmittedEvidenceNote>,
    pub generated_at: DateTime<Utc>,
}
```

### State Machines

Memory status:

```text
proposed -> trusted
proposed -> rejected
trusted -> contradicted
trusted -> superseded
contradicted -> trusted
contradicted -> superseded
trusted -> deprecated
```

Belief stance:

```text
unknown -> tentative -> supported -> contested -> revised -> retired
```

Skill status:

```text
draft -> tested -> trusted -> degraded -> retired
```

## Write Path

### `observe(event)`

Input:

```text
EventEnvelope {
  source_kind,
  source_uri,
  observed_at,
  actor,
  payload,
  payload_hash,
  privacy_class,
  extraction_policy
}
```

Steps:

1. Check privacy class. Reject repo persistence for secret/vault material.
2. Segment the event using Nemori-style boundary signals:
   - topic shift
   - task boundary
   - tool invocation boundary
   - prediction gap
   - correction
   - source/document boundary
   - elapsed time
3. Append immutable `Episode` records before any derived memory is committed.
4. Emit `Observation` records with explicit dates and source spans.
5. Route candidate mutations to memory lanes:
   - stable user/project state -> Core
   - event/run/search/build/test -> Episodic
   - assertion/equation/entity/relation -> Semantic
   - reusable workflow -> Procedural
   - paper/dataset/repo/file/API -> Resource
   - evidence-weighted proposition -> Belief
   - reusable abstraction/topic -> Concept
   - retrieval/answer/test outcome -> Eval
6. Validate each candidate:
   - provenance present
   - source span resolvable
   - no duplicate high-confidence object
   - no unsupported belief
   - no destructive fact overwrite
   - no untested procedural promotion
7. Commit canonical rows in one transaction.
8. Append `MemoryMutation` JSONL receipt.
9. Update projection indexes or enqueue rebuild.
10. Emit a `MutationReceipt` with object IDs, receipt path, index updates, and warnings.

### Fact Update Rule

Never update a fact in place to mean a different thing.

Use temporal invalidation and supersession:

```text
old FactEdge:
  valid_from = 2026-01-01
  valid_to   = 2026-05-12
  status     = superseded

new FactEdge:
  valid_from = 2026-05-12
  valid_to   = null
  supersedes = [old_edge_id]
```

### Belief Update Rule

A belief is never a fact. A belief is a current, evidence-weighted stance over claims and observations.

Required links:

- at least one supporting claim or observation unless stance is `unknown`
- all known contradicting claims
- mutation receipt explaining what changed
- confidence delta and rationale

### Procedural Memory Promotion Rule

A candidate skill can become `trusted` only after:

- it has a clear task signature and preconditions
- it links to at least one successful episode
- it has a repeatable test or verification recipe
- failures are recorded as eval memory, not erased

## Read Path

### `recall(query)`

The hot path must be deterministic and local-first. LLM reranking is optional and must not be required for baseline recall.

Steps:

1. Parse query into `RecallQuery`:
   - task intent: answer, explain, plan, implement, debug, compare, cite, reflect
   - domain/topic hints
   - exact symbols/identifiers
   - time scope: now, at time T, before/after T, all history
   - required evidence level
   - context budget
2. Query pinned core memory.
3. Run retrieval fanout in parallel:
   - observation log by time/entity/task
   - Tantivy BM25 over exact terms, equation strings, paper IDs, code names
   - vector HNSW over summaries and concept notes
   - entity graph neighborhood
   - temporal graph facts valid at requested time
   - concept-note graph neighbors
   - procedural skill library
   - eval memory for prior failures and corrections
4. Normalize candidates into a shared score space.
5. Rerank by task-specific weights:
   - exact match
   - semantic similarity
   - graph proximity
   - temporal validity
   - source quality
   - confidence
   - freshness
   - retrieval success history
   - topic strength
   - contradiction pressure
6. Build a bounded `ContextPack`.
7. Include citations, contradictions, freshness, confidence, and omitted-evidence notes.
8. Return enough explanation for `smart-memory-cli explain`.

### Retrieval Weight Profiles

| Intent | Highest Weights | Required Warnings |
| --- | --- | --- |
| Math derivation | exact equation, prerequisite concepts, derivation provenance, units | missing assumptions, unit mismatch, unsupported steps |
| Scientific claim | source quality, support/contradiction, temporal validity | contested evidence, stale papers, low confidence |
| Coding task | skill success, recent evals, resource paths, exact symbols | failed prior attempts, untested skill |
| Planning | core constraints, procedural skills, open questions, topic strength | unresolved blockers, low evidence |
| Debugging | recent episodes, error exact match, failed attempts, skill library | stale environment, unverified workaround |
| Literature review | resource cards, concept graph, claim graph, citation clusters | omitted sources, contradictory clusters |

### Context Pack Contract

A context pack must be small enough to fit the caller's budget and must include:

- `why_included` for each item
- citations or source spans for each factual item
- validity/freshness notes for temporal facts
- contradiction notes when known
- confidence summary
- omitted-evidence notes when important evidence was excluded by budget
- deterministic item order for equal scores

## Topic Strengthening

Topic strength decides what gets consolidated, reviewed, and promoted. It is not a hidden embedding score; it is an explainable decayed score.

### Score Components

| Component | Meaning | Typical Source |
| --- | --- | --- |
| Recency | How recently the topic appeared or was used | observations, retrieval logs |
| Recurrence | How often it appears across independent episodes | episode/topic links |
| Utility | Whether it helped answer, code, debug, or plan | eval memory, skill outcomes |
| Novelty | Whether it changes existing understanding | prediction gaps |
| Source quality | Trust level and specificity of supporting evidence | resource cards |
| Contradiction pressure | Degree of unresolved conflict | claim graph |
| Retrieval success | Whether retrieving it improved outcomes | context-pack eval |
| Graph-neighbor propagation | Strength inherited from linked topics | concept/fact graph |

### Score Sketch

```text
decayed_base =
  previous_strength * exp(-hours_since_update / half_life_hours)

reinforcement =
  w_recency * recency
+ w_recurrence * recurrence
+ w_utility * utility
+ w_novelty * novelty
+ w_source_quality * source_quality
+ w_retrieval_success * retrieval_success
+ w_neighbor * graph_neighbor_boost
- w_contradiction * unresolved_contradiction_pressure

new_strength = clamp(decayed_base + reinforcement, 0.0, 1.0)
```

### Policy

- Strong topics produce concept kernels: compact, cited summaries with prerequisites, equations, claims, resources, and open questions.
- Weak but important topics enter review queues instead of being deleted.
- Contradicted topics trigger reflection and belief revision.
- Topics with high utility but low confidence trigger targeted evidence search.
- Repeated retrieval failure lowers ranking and creates eval memory.
- Repeated skill success strengthens linked topics and procedural cards.

### Explainability

`explain topic <topic_id>` should show:

- current strength
- last update event
- component deltas
- top supporting episodes/resources
- open contradictions
- next review date
- linked concept kernels
- recent retrieval outcomes

## Math And Science Learning

Math/science memory needs epistemic structure, not just notes.

### Required Object Separation

| Object | Stores | Must Not Store |
| --- | --- | --- |
| `ConceptCard` | definitions, prerequisites, concept links, examples | unsupported facts |
| `EquationCard` | LaTeX, normalized form, variables, units, assumptions, derivation steps | hidden assumptions |
| `ClaimCard` | atomic claim, support, contradiction, confidence, validity | agent belief as fact |
| `BeliefState` | current stance over claims | raw source assertion without evidence |
| `ResourceCard` | paper/dataset/repo/API metadata and trust profile | extracted claims without provenance |
| `QuestionCard` | open research target, blockers, next actions | resolved claims |
| `SkillCard` | repeatable procedure and tests | unverified one-off hack as trusted skill |
| `EvalMemory` | retrieval/answer/skill outcomes | silent correction without receipt |

### Concept Kernel

A concept kernel is generated for strong topics and stored as a curated `ConceptCard` or Markdown/YAML card.

Required sections:

- name and aliases
- short definition
- prerequisites
- canonical examples
- equations and variables
- units and dimensional constraints
- claims and confidence
- evidence and contradictions
- datasets/code experiments
- failed attempts and misconceptions
- open questions
- linked skills
- last review date and topic strength explanation

### Equation Handling

Equation memory must preserve:

- original source span
- normalized symbolic form
- variable definitions
- units and dimensional constraints
- assumptions and limiting cases
- derivation steps
- related concepts and claims
- known contradictions or alternative forms
- examples where it was used successfully

Tantivy/BM25 is required for exact symbols and equation strings. Vector search alone is unacceptable for equations.

### Beliefs And Evidence

Scientific belief update rule:

```text
Observation: Source S reports result R under method M.
Claim: R supports proposition P under assumptions A.
Belief: The agent currently accepts P with confidence C because claims C1..Cn support it and claims K1..Km contradict it.
```

The system must be able to answer:

- What did we observe?
- Who or what asserted this?
- What claim was extracted?
- What evidence supports it?
- What evidence contradicts it?
- What does the agent currently believe?
- When and why did that belief change?

## Storage

### Baseline Local Stack

| Need | Choice | Why |
| --- | --- | --- |
| Canonical local metadata | SQLite WAL | Zero-service local durability, transactions, portable backups |
| Audit trail | Append-only JSONL mutation receipts | Replayable, reviewable, easy to diff and archive |
| Exact search | Tantivy | Fast Rust-native BM25/full-text for symbols, terms, IDs, paths |
| Vector search | Local HNSW behind `EmbeddingIndex` | Fast local semantic recall with adapter boundary |
| Temporal graph | SQLite tables plus adjacency projections | Local-first graph queries without requiring Neo4j |
| Curated cards | Git-backed Markdown/YAML | Human review, diffs, code-review workflow |
| Large raw artifacts | File store by content hash | Avoid bloating SQLite and Git with large binary payloads |

### Optional Later Adapters

- Qdrant for larger vector collections.
- Postgres/pgvector for shared team memory.
- Kuzu or Neo4j for heavier graph workloads.
- LanceDB for local vector/table hybrid storage.
- Object storage for large multimodal artifacts.

Adapters must not change the canonical memory contract.

### Rebuild Rules

The following must be rebuildable from canonical stores:

- Tantivy indexes
- vector indexes
- graph adjacency tables
- concept-neighbor caches
- context-pack caches
- recall evaluation aggregates

Rebuild command contract:

```text
smart-memory-cli consolidate --rebuild-indexes --from-canonical
```

Expected result:

- deterministic object counts
- deterministic index manifests
- receipt-linked warnings for skipped/corrupt records
- no silent data loss

## APIs

### Engine Trait

```rust
pub trait MemoryEngine {
    async fn observe(&self, event: EventEnvelope) -> Result<MutationReceipt>;
    async fn recall(&self, query: RecallQuery) -> Result<ContextPack>;
    async fn reflect(&self, scope: ReflectionScope) -> Result<MutationReceipt>;
    async fn consolidate(&self, job: ConsolidationJob) -> Result<ConsolidationReport>;
}
```

### Store Traits

```rust
pub trait EpisodeStore {
    async fn append_episode(&self, episode: Episode) -> Result<EpisodeId>;
    async fn get_episode(&self, id: EpisodeId) -> Result<Option<Episode>>;
}

pub trait MemoryStore {
    async fn apply_mutation(&self, mutation: MemoryMutation) -> Result<MutationReceipt>;
    async fn load_memory(&self, id: MemoryId) -> Result<Option<MemoryObject>>;
}

pub trait SearchIndex {
    async fn upsert(&self, doc: SearchDocument) -> Result<()>;
    async fn search(&self, query: SearchQuery) -> Result<Vec<SearchHit>>;
}

pub trait EmbeddingIndex {
    async fn upsert_embedding(&self, id: MemoryId, vector: EmbeddingVector) -> Result<()>;
    async fn nearest(&self, query: EmbeddingVector, limit: usize) -> Result<Vec<VectorHit>>;
}

pub trait GraphIndex {
    async fn upsert_edge(&self, edge: FactEdge) -> Result<()>;
    async fn neighborhood(&self, seed: GraphSeed, policy: GraphPolicy) -> Result<Vec<GraphHit>>;
}
```

### Recall Query Sketch

```rust
pub struct RecallQuery {
    pub text: String,
    pub intent: RecallIntent,
    pub time_scope: TimeScope,
    pub domains: Vec<Domain>,
    pub required_evidence: EvidenceLevel,
    pub budget: ContextBudget,
    pub include_contradictions: bool,
    pub include_omitted_evidence: bool,
}
```

### Mutation Receipt Sketch

```rust
pub struct MutationReceipt {
    pub id: MemoryMutationId,
    pub created_at: DateTime<Utc>,
    pub objects_created: Vec<MemoryId>,
    pub objects_updated: Vec<MemoryId>,
    pub objects_superseded: Vec<MemoryId>,
    pub receipt_path: PathBuf,
    pub source_hashes: Vec<Hash>,
    pub index_updates: Vec<IndexUpdateReceipt>,
    pub warnings: Vec<MutationWarning>,
}
```

## Testing

### Unit Tests

- Schema serialization round-trips preserve `schema_version`.
- Status machines reject invalid transitions.
- Fact updates create supersession records instead of destructive overwrite.
- Beliefs require support/contradiction links.
- Topic strength explanations equal score inputs.
- Context packs enforce budget and citations.

### Integration Tests

- Ingest a small paper/resource and retrieve its claims with citations.
- Ingest a contradiction and verify old fact validity is closed, not deleted.
- Ingest a failed code run and verify a failed attempt is retrievable for debugging.
- Promote a repeated workflow into a skill only after a passing verification.
- Rebuild all indexes from canonical stores and compare manifests.

### Retrieval Eval Tests

Build a local fixture set with:

- known entities
- exact symbols and equations
- temporally superseded facts
- contradictory claims
- concept prerequisites
- procedural skills
- failed attempts

Required assertions:

- exact equation queries hit BM25 results
- temporal queries return the fact valid at requested time
- contradicted claims produce warnings
- context packs cite every factual item
- known bad prior answers are retrieved for self-correction

### Performance Tests

Warm-index local recall target:

```text
p50 <= 150 ms
p95 <= 750 ms
p99 <= 1500 ms
```

Measure separately:

- SQLite pinned/core lookup
- Tantivy BM25
- vector nearest-neighbor
- graph neighborhood
- rerank
- context-pack assembly

Failure interpretation:

- Slow BM25 means index configuration or query parsing is wrong.
- Slow vector search means HNSW parameters or embedding dimensions need tuning.
- Slow graph traversal means adjacency/cache policy needs limits.
- Slow context packing means budget and citation rendering are doing too much work.
- Low recall with fast latency means scoring/routing is wrong, not storage.

## Validation

For this Markdown artifact:

```bash
rtk test -f /Users/bentaylor/Code/opencode/tips/smart_memory/CODEX_MEMSPEC.md
rtk rg -n "Score|Architecture|Rust|Topic Strength|ClaimCard|ContextPack|Validation" /Users/bentaylor/Code/opencode/tips/smart_memory/CODEX_MEMSPEC.md
rtk wc -l /Users/bentaylor/Code/opencode/tips/smart_memory/CODEX_MEMSPEC.md
```

If future work updates repo owner/test maps for tracked audit coverage, also run:

```bash
rtk just fast
```

Expected artifact:

```text
/Users/bentaylor/Code/opencode/tips/smart_memory/CODEX_MEMSPEC.md
```

Expected result:

- file exists
- required sections and schema names are present
- no generated-zone edits
- no unrelated dirty-worktree changes

## Rollout

### Phase 0: Spec And Fixtures

- Keep this document as the implementation contract.
- Create minimal fixture data for episodes, claims, equations, resources, concepts, and skills.
- Define receipt examples before implementing the engine.

Exit criteria:

- fixture set exercises provenance, contradiction, temporal validity, exact search, concept links, and skills
- validation commands pass

### Phase 1: Canonical Store

- Implement schema structs and SQLite tables.
- Implement append-only JSONL receipts.
- Implement deterministic import/export.
- Implement no-op index adapters for compile-time boundaries.

Exit criteria:

- ingest writes episodes and receipts
- status transitions validated
- destructive fact overwrite impossible by API

### Phase 2: Local Recall

- Implement Tantivy BM25.
- Implement local vector adapter.
- Implement graph adjacency tables.
- Implement recall fanout and deterministic context packs.

Exit criteria:

- warm recall p95 <= 750 ms on fixture set
- context packs cite every factual item
- temporal and contradiction tests pass

### Phase 3: Consolidation

- Implement concept kernels.
- Implement topic strengthening.
- Implement belief revision.
- Implement skill promotion and degradation.

Exit criteria:

- topic explanations are auditable
- belief changes cite mutation receipts
- skill cards require verification

### Phase 4: Agent Integration

- Add CLI commands.
- Add agent-facing context-pack API.
- Add feedback/eval ingestion.
- Add explain commands for receipts, topics, recall, and omissions.

Exit criteria:

- agent can ingest, recall, act, receive feedback, and update eval memory
- failed retrievals create actionable eval records

### Phase 5: Optional Scale Adapters

- Add Qdrant or pgvector adapter.
- Add heavier graph backend only if local graph tables become a bottleneck.
- Add corpus-level GraphRAG summaries for large paper collections.

Exit criteria:

- adapter outputs match local baseline on fixture evals
- canonical store contract remains unchanged

## Risks

| Risk | Consequence | Mitigation |
| --- | --- | --- |
| Memory bloat | Recall slows and context packs get noisy | Topic strength, review queues, bounded context packs, decay, consolidation |
| Unsupported beliefs | Agent becomes overconfident | Belief/fact separation, required evidence links, contradiction warnings |
| Stale facts | Wrong answers from old state | Temporal validity windows and supersession |
| Vector overreliance | Missed exact equations, symbols, IDs | Tantivy/BM25 and structured equation schema |
| LLM extraction errors | Bad memory pollution | Proposed status, confidence, provenance, review, eval memory |
| Skill hallucination | Agent repeats untested procedures | Skill promotion requires successful episodes and tests |
| Non-deterministic rebuilds | Audit failure | canonical stores, manifests, stable ordering, receipt replay |
| Privacy leakage | Secrets become retrievable | privacy classes, vault exclusion, source filters |
| Overcomplicated v1 | System never ships | staged rollout: canonical store, local recall, then consolidation |

## Stop Conditions

Implementation agents should stop and ask before proceeding if:

- `CODEX_MEMSPEC.md` already contains user edits that conflict with this contract.
- requested changes require storing secrets or private vault data in repo memory.
- owner/test maps have unrelated local edits and the user asks to modify them.
- generated/read-only zones would need manual edits.
- recall correctness would depend on a remote service.
- a memory mutation cannot be linked to provenance.

## Handoff Requirements

Every future implementation handoff must include:

- changed file paths
- whether owner/test maps changed
- validation commands run
- command outputs or failure summaries
- generated artifacts created or intentionally avoided
- whether indexes are canonical or rebuildable
- mutation receipt paths for memory-writing tests
- residual risks and next action

Status updates during implementation should report:

- current phase
- changed modules
- proof lane selected from `agent/test-map.json`
- whether generated zones are untouched
- any dirty unrelated files ignored

Final handoff for this spec-only task should state:

- `tips/smart_memory/CODEX_MEMSPEC.md` was created
- owner/test maps were not changed
- validation commands run
- failures, if any
- whether `tips/smart_memory/` remains untracked
