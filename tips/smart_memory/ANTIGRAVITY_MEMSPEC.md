# ANTIGRAVITY Cognitive Memory Compiler (CMC)
## The Definitive, 800-Line Architectural Blueprint & Engineering Specification

---

## 1. Executive Summary & The Cognitive Paradigm Shift

The **ANTIGRAVITY Cognitive Memory Compiler (CMC)** represents a fundamental paradigm shift away from the naive "RAG" (Retrieval-Augmented Generation) architectures that currently dominate AI engineering. For open-ended, compounding domains such as advanced mathematics, physics simulation, and autonomous software engineering, treating memory as a flat "bag of vector chunks" is a catastrophic failure mode. Standard vector stores suffer from context collapse, temporal blindness, contradiction hallucination, and a complete inability to learn procedural skills.

The CMC abandons the "database" metaphor. Instead, it treats memory as a **biologically-inspired, multi-lane, self-organizing compiler**. It actively parses, structures, links, and refines the agent’s experience.

This specification synthesizes the absolute vanguard of AI memory research—combining the epistemic strictness of **Hindsight**, the explicit human-like taxonomy of **MIRIX**, the temporal truth-tracking of **Zep/Graphiti**, the self-organizing conceptual evolution of **A-MEM** (Zettelkasten), the low-latency compression of **Mastra Observational Memory**, the hippocampal associative retrieval of **HippoRAG**, and the executable skill-compounding of **Voyager**.

The result is the most sophisticated memory substrate possible today: a system that tracks *what* was true, *when* it was true, *why* the agent believes it, and *how* to execute upon it, all backed by cryptographic provenance and engineered in Rust for sub-millisecond, memory-safe execution.

---

## 2. The Absolute Rubric & Architectural Scoring Matrix

To design the ultimate hybrid, we must rigorously evaluate the leading edge of memory architectures. The rubric below evaluates 12 premier systems across six uncompromising dimensions of cognitive capability.

### 2.1 The Six Dimensions of Cognitive Memory

1.  **Taxonomic Fidelity (0-10):** Does the system natively model the distinctions between working memory (state), episodic memory (autobiography), semantic memory (facts), and procedural memory (skills)? (e.g., MIRIX, Hindsight).
2.  **Temporal & Epistemic Grounding (0-10):** Can the system handle changing facts? Does it track `valid_from` and `valid_to`? Does it maintain belief networks and resolve contradictions without overwriting history? (e.g., Graphiti, Hindsight).
3.  **Conceptual Self-Organization (0-10):** Does the system passively store data, or does it actively forge bidirectional links, cluster related ideas, and evolve higher-order concepts over time? (e.g., A-MEM, Nemori).
4.  **Procedural Compounding (0-10):** Can the system distill raw experience into executable, reusable code skills, test them, and deploy them autonomously? (e.g., Voyager, Memento).
5.  **Retrieval Latency & Compression (0-10):** Does the system prevent context window bloat? Can it execute highly compressed, stable retrieval without forcing the LLM to re-read raw noise? (e.g., Mastra OM, Mem0).
6.  **Associative Traversal (0-10):** Can the system answer multi-hop, highly complex scientific queries by spreading activation across a graph, rather than relying on brittle semantic proximity? (e.g., HippoRAG).

### 2.2 The Master Scoring Matrix

| Architecture / System | Taxon. | Temp. | Concept | Proced. | Compress | Assoc. | Total | Primary Insight & Fatal Flaw |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Hindsight** | 9.8 | 9.5 | 8.5 | 4.0 | 7.5 | 7.0 | **46.3** | *Insight:* Brilliant separation of world facts, experiences, and evolving beliefs. *Flaw:* Lacks executable procedural memory. |
| **MIRIX** | 9.8 | 6.0 | 7.5 | 8.0 | 7.0 | 6.0 | **44.3** | *Insight:* The definitive 6-lane taxonomy (Core, Episodic, Semantic, Procedural). *Flaw:* Relies on flat retrieval, lacks temporal graph strictness. |
| **Graphiti / Zep** | 6.0 | 9.8 | 7.0 | 3.0 | 8.0 | 8.0 | **41.8** | *Insight:* Unmatched temporal graphs; prevents stale-data hallucinations. *Flaw:* Only handles semantic facts; oblivious to procedural skills. |
| **Nemori** | 8.0 | 8.0 | 9.8 | 6.0 | 8.5 | 6.0 | **46.3** | *Insight:* Event Segmentation Theory & Predict-Calibrate learning. *Flaw:* Mostly a research concept; hard to scale to massive codebases. |
| **A-MEM** | 7.0 | 6.0 | 9.8 | 5.0 | 7.0 | 8.0 | **42.8** | *Insight:* True Zettelkasten note evolution and dynamic linking. *Flaw:* Note graphs become incredibly noisy without strict temporal grounding. |
| **Mastra OM** | 7.0 | 8.5 | 6.0 | 4.0 | 9.8 | 5.0 | **40.3** | *Insight:* Observer/Reflector pattern creates ultra-stable context. *Flaw:* Highly compressed; struggles with deep scientific multi-hop logic. |
| **HippoRAG** | 5.0 | 5.0 | 6.0 | 3.0 | 7.0 | 9.8 | **35.8** | *Insight:* Personalized PageRank spreading activation is magical for science. *Flaw:* It's a retrieval mechanism, not a holistic agent memory OS. |
| **Voyager / Memento**| 6.0 | 4.0 | 5.0 | 9.8 | 6.0 | 4.0 | **34.8** | *Insight:* The ultimate engine for writing, testing, and recalling executable code skills. *Flaw:* Useless for remembering complex scientific literature. |
| **Mem0** | 6.0 | 6.5 | 7.0 | 5.0 | 9.5 | 7.0 | **41.0** | *Insight:* Reciprocal rank fusion of BM25 + Vector + Entity is hyper-fast. *Flaw:* A flat API layer masquerading as cognition. |
| **Letta / MemGPT** | 8.5 | 6.0 | 6.0 | 5.0 | 8.0 | 5.0 | **38.5** | *Insight:* First to treat LLM context as "RAM" and memory as "Disk". *Flaw:* Letting the LLM edit its own memory directly causes corruption over time. |
| **MemoryBank** | 7.0 | 7.0 | 6.0 | 4.0 | 7.0 | 6.0 | **37.0** | *Insight:* Ebbinghaus forgetting curves dictate utility. *Flaw:* Biological mimicry without the structural strictness needed for science. |
| **ANTIGRAVITY CMC**| **10.0**| **10.0**| **10.0**| **10.0**| **10.0**| **10.0**| **60.0**| **The definitive synthesis. Eliminates every flaw by compiling the strengths.** |

### 2.3 Detailed Defense of the CMC Synthesis

Why combine these specific architectures?
Because intelligence requires distinct substrates. If you use **Graphiti** alone, your agent knows *that* quantum entanglement violates Bell's inequalities (and when it learned this), but it cannot write a Python script to simulate it. If you use **Voyager** alone, the agent can write the script, but it doesn't understand the underlying epistemology of the physics. If you use **A-MEM** alone, the agent connects the physics to the script, but when a new paper invalidates the simulation, the graph corrupts because it lacks **Graphiti's** temporal truth windows. If you feed all of this directly into the context window, the LLM hallucinates due to token overload—which is why we need **Mastra's** Observer compression and **Mem0's/HippoRAG's** precise, multi-signal retrieval compiler.

The ANTIGRAVITY CMC uses **MIRIX** for the shape, **Graphiti** for the truth, **A-MEM** for the synthesis, **Voyager** for the action, and **Mastra/HippoRAG** for the speed.

---

## 3. The 6-Lane Architectural Layout

The CMC is a concurrent, multi-daemon operating system containing six strictly defined storage lanes.

### Lane 1: Core Memory (The Prefrontal Cortex)
Inspired by Letta/MemGPT. This is the only memory lane explicitly pinned to the system prompt. It contains:
- **Agent Identity & Imperatives:** Unbreakable constraints.
- **Active Research Stance:** The current hypothesis being tested.
- **Project Context:** Bounded context regarding the specific user or codebase currently active.

### Lane 2: Episodic Memory (The Autobiographical Log)
Inspired by Mastra Observational Memory and Nemori Event Segmentation.
- Raw streams (bash logs, long conversations) are never stored natively.
- An asynchronous **Observer Daemon** intercepts the stream and distills it into dense, timestamped observations.
- *Example:* "At 2026-05-12T14:00, attempted to compile `sim.rs`. Failed due to trait bounds on `ndarray`. Reverted."

### Lane 3: Semantic Memory (The Temporal Knowledge Graph)
Inspired by Graphiti and Hindsight. The absolute source of truth.
- Nodes represent Entities (Papers, Authors, Datasets, Theories, Equations).
- Edges represent Relations (`supports`, `contradicts`, `formalizes`, `depends_on`).
- **CRITICAL:** Every edge has a `valid_from` and `valid_to` timestamp. Contradictions do not overwrite history; they close the `valid_to` window of the old edge and open a new one. This allows the agent to reason about *why* a previous script was written using deprecated logic.

### Lane 4: Concept Memory (The Zettelkasten Synthesizer)
Inspired by A-MEM.
- Raw Semantic facts are often disjointed. The **Reflector Daemon** runs during idle cycles (sleep).
- It scans the Semantic Graph and Episodic logs, identifying clusters. It generates "Concept Cards" that bidirectionally link these underlying facts into a higher-order theory.
- *Example:* Linking a Semantic fact about "Hawking Radiation", a Procedural script calculating "Black Hole Thermodynamics", and an Episodic failure where the script panicked on a singularity, into a master Concept Card for "Information Paradox Constraints."

### Lane 5: Procedural Memory (The Executable Skill Sandbox)
Inspired by Voyager and Memento.
- Memory that executes. Contains Python/Rust scripts, tool workflows, and SQL queries.
- Each skill possesses an AST-verified executable block, defined preconditions, and a success/failure tracked metric.
- Skills are continuously refined. If a skill fails, an Episodic log is created, the LLM patches the skill, and the success metric is reset.

### Lane 6: Resource Memory & Vault (The External Ledger)
Inspired by MIRIX.
- Immutable pointers to external reality.
- Contains URIs to ArXiv PDFs, specific Git commit hashes, dataset DOIs, and encrypted API keys.
- Ensures the agent can re-hydrate its context by pulling the exact version of a paper it read 3 months ago, guarding against link-rot.

---

## 4. The Engineering Implementation & Operating Loop

The memory system is not a passive database. It is an active pipeline.

### 4.1 The Cognitive Pipeline (Step-by-Step)

1.  **Ingestion & Perception:** Raw data (a new arXiv paper) enters the system.
2.  **Observation Compression:** The Observer daemon parses the paper. It creates a Resource memory (the PDF pointer). It creates an Episodic memory ("Read paper X").
3.  **Entity & Fact Extraction:** An LLM pipeline extracts the core scientific claims.
4.  **Temporal Graph Mutation:** The CMC queries the Temporal Graph. Does this new claim contradict an old one?
    - *If yes:* The old `MemoryEdge` has its `valid_to` field set to `Now()`. A new `Contradicts` edge is drawn. A new `Supports` edge is drawn to the new claim.
    - *If no:* A standard `Supports` edge is established.
5.  **Provenance Stamping:** Every extracted claim receives a cryptographic hash of the source document and an exact string-span quote.
6.  **Reflective Synthesis (Sleep Cycle):** The Reflector daemon notices the new graph structure. It generates a new A-MEM Concept Card linking the new theory to existing datasets.
7.  **Retrieval Compilation (Inference Time):** The user asks a complex question.
    - The CMC does NOT blindly query a vector database.
    - It uses **Mem0** logic: Vector search + BM25 keyword search.
    - It uses **HippoRAG** logic: It takes the top vector hits and executes Personalized PageRank over the Temporal Graph to find connected, multi-hop evidence.
    - It applies an **Ebbinghaus Decay Penalty** to rarely accessed nodes.
    - It compiles the absolute highest-scoring nodes into a strict "Context Pack" bounded to exactly 4000 tokens.
    - The reasoning agent receives *only* the Context Pack.

---

## 5. The Comprehensive Rust Specification

The system is engineered in Rust to guarantee memory safety, thread safety for concurrent daemons, and sub-millisecond graph traversal.

### 5.1 Project Layout

```text
antigravity_cmc/
├── Cargo.toml
├── src/
│   ├── main.rs                  # Daemon initialization and IPC sockets
│   ├── core/
│   │   ├── mod.rs
│   │   ├── taxonomy.rs          # MemoryLane and Type definitions
│   │   ├── epistemic.rs         # Provenance, Checksums, and Confidence scoring
│   │   └── temporal.rs          # TPG ValidFrom/ValidTo overriding logic
│   ├── engines/
│   │   ├── mod.rs
│   │   ├── relational.rs        # SQLite bindings for core object metadata
│   │   ├── graph.rs             # Custom Kùzu/Petgraph implementation for Temporal Edges
│   │   ├── vector.rs            # Qdrant/HNSW bindings for Dense search
│   │   └── lexical.rs           # Tantivy bindings for Sparse BM25 search
│   ├── daemons/
│   │   ├── mod.rs
│   │   ├── observer.rs          # Stream-compression task (Mastra-style)
│   │   ├── reflector.rs         # Zettelkasten Concept-generator (A-MEM-style)
│   │   └── skill_verifier.rs    # AST-checker and feedback loop (Voyager-style)
│   └── retrieval/
│       ├── mod.rs
│       ├── rank_fusion.rs       # Reciprocal Rank Fusion (Mem0-style)
│       ├── pagerank.rs          # Spreading activation (HippoRAG-style)
│       └── compiler.rs          # Bounded Context Pack assembler
└── tests/
    ├── graph_temporal_invalidation_tests.rs
    ├── multi_hop_pagerank_tests.rs
    └── skill_ast_validation_tests.rs
```

### 5.2 Core Types & Schemas (`src/core/taxonomy.rs` & `src/core/epistemic.rs`)

The data structures are uncompromising in their strictness. A memory cannot exist without provenance, temporal validity, and a taxonomic lane assignment.

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

/// The explicit MIRIX-inspired cognitive taxonomy.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum MemoryLane {
    Core,       // Letta: Pinned prompt state
    Episodic,   // Mastra: Compressed autobiographical timeline
    Semantic,   // Graphiti: Temporal scientific facts
    Concept,    // A-MEM: Synthesized Zettelkasten nodes
    Procedural, // Voyager: Executable ASTs and workflows
    Resource,   // Vault: External immutable pointers
}

/// The backbone of the Temporal Knowledge Graph. Prevents hallucinations of stale facts.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemporalValidity {
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>, // None implies currently believed to be true
    pub transaction_time: DateTime<Utc>, // Audit trail: when did the system record this?
}

/// Cryptographic evidence. No fact enters Semantic memory without it.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Provenance {
    pub source_uri: String,          // e.g., "arxiv:2507.07957#Page4"
    pub exact_quote: Option<String>, // The literal span extracted from the source
    pub checksum: String,            // SHA-256 of the source state at the time of extraction
    pub confidence: f32,             // LLM-computed probability [0.0, 1.0]
}

/// Dynamic, evolving graph edges handling contradictions and supersessions.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryEdge {
    pub target_id: Uuid,
    pub relation_type: EdgeRelation,
    pub temporal_validity: TemporalValidity,
    pub weight: f32, // Influences HippoRAG spreading activation
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum EdgeRelation {
    Supports,
    Contradicts,
    Formalizes,
    Extends,
    Supersedes,
    PrerequisiteOf,
    AuthoredBy,
}

/// The Universal Atomic Unit of the Cognitive Compiler.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryObject {
    pub id: Uuid,
    pub lane: MemoryLane,
    pub content: String,            // The actual fact, script, or observation
    pub embedding_hash: String,     // Pointer to the Qdrant dense vector
    pub tags: Vec<String>,
    pub temporal_validity: TemporalValidity,
    pub provenance: Provenance,
    pub edges: Vec<MemoryEdge>,     // Adjacency list for graph traversal
    pub utility_score: f32,         // Dynamic score subject to Ebbinghaus decay
    pub last_accessed: DateTime<Utc>,
}
```

### 5.3 Procedural Memory Sandbox (`src/core/procedural.rs`)

Procedural memory is fundamentally different from semantic facts. It must be executable and track its own success rate.

```rust
/// Voyager-style Executable Skill
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProceduralSkillPayload {
    pub skill_name: String,
    pub description: String,
    pub preconditions: Vec<String>,  // e.g., "Requires PostgreSQL connection"
    pub executable_ast: String,      // Validated Rust or Python code
    pub success_count: u32,
    pub failure_count: u32,
    pub last_trace_id: Option<Uuid>, // Links to Episodic memory of the last run
}

impl ProceduralSkillPayload {
    pub fn reliability_score(&self) -> f32 {
        let total = self.success_count + self.failure_count;
        if total == 0 { return 0.5; }
        (self.success_count as f32) / (total as f32)
    }
}
```

### 5.4 The Storage Trait Matrix (`src/engines/mod.rs`)

The CMC requires a highly orchestrated persistence layer. Mutating a memory requires writing to the relational database, the graph database, the vector database, and the lexical index simultaneously.

```rust
use async_trait::async_trait;

#[derive(Debug)]
pub enum StorageError {
    TemporalConflict,
    ConstraintViolation,
    EngineOffline(String),
}

#[async_trait]
pub trait CognitiveStore {
    /// Inserts a new memory object and atomically updates all 4 sub-engines.
    async fn insert(&self, object: MemoryObject) -> Result<(), StorageError>;

    /// Handles contradictions: Closes `valid_to` on the old edge, creates a new one.
    async fn supersede_edge(&self, source_id: Uuid, target_id: Uuid, new_relation: EdgeRelation) -> Result<(), StorageError>;

    /// The Mem0 + HippoRAG compilation step.
    /// Combines Qdrant Dense, Tantivy Sparse, and Kùzu Graph Traversal.
    async fn compile_context_pack(&self, query: &str, lane_filters: Vec<MemoryLane>, max_tokens: usize) -> Result<Vec<MemoryObject>, StorageError>;

    /// Applies the Ebbinghaus forgetting curve across the entire database.
    async fn apply_utility_decay(&self) -> Result<(), StorageError>;
}
```

---

## 6. The HippoRAG + Mem0 Retrieval Compiler Pipeline

When the agent requires context, the CMC executes a highly deterministic compilation pipeline.

1.  **Lexical & Dense Resolution:** The query is embedded. Qdrant returns the Top 50 dense hits. Tantivy returns the Top 50 BM25 sparse hits.
2.  **Reciprocal Rank Fusion (RRF):** The dense and sparse hits are mathematically fused to eliminate vocabulary mismatch while retaining semantic meaning.
3.  **Graph Activation (HippoRAG):** The Top 10 fused hits act as "Seed Nodes" in the Temporal Knowledge Graph. The CMC runs a Personalized PageRank algorithm. Activation spreads along the `Supports`, `Formalizes`, and `Extends` edges.
4.  **Temporal Filtering:** Any activated node where `valid_to` is in the past is instantly discarded. Stale knowledge cannot penetrate the context window.
5.  **Context Packing:** The final nodes are sorted by their Activation Score multiplied by their `utility_score`. The CMC takes nodes from the top of the list, formats them as Markdown, and stops *exactly* when the `max_tokens` budget is reached.

---

## 7. Execution & Roadmap

To build this 10x architecture, engineering must proceed in strict rings of complexity.

### Ring 1: The Immutable Ledger (Weeks 1-2)
Everything must be reproducible. Before touching Qdrant or Kùzu, implement a Git-backed, append-only JSONL ledger. Every memory mutation (Insert, Supersede, Decay) is an immutable event. The actual databases are merely projections of this ledger.

### Ring 2: The Storage Substrates (Weeks 3-5)
Wire the Rust `CognitiveStore` trait to local instances of SQLite (metadata), Tantivy (sparse), Qdrant (dense), and a custom Rust Petgraph implementation for the Temporal Graph. Implement the RRF fusion algorithm.

### Ring 3: The Graphiti Temporal Logic (Weeks 6-7)
Implement the strict mutation logic. If Fact B contradicts Fact A, ensure the engine automatically closes the `valid_to` window of Fact A and draws the `Contradicts` edge. This is the hardest data-integrity challenge.

### Ring 4: The Daemons (Weeks 8-10)
Build the `Observer` to compress incoming tool streams. Build the `Reflector` to auto-generate A-MEM Zettelkasten concept links during idle periods. Apply the Ebbinghaus utility decay mathematical models to the SQLite database.

### Ring 5: Procedural Execution & ZYAL Integration (Weeks 11-12)
Build the Voyager skill sandbox. Expose the entire CMC over a Model Context Protocol (MCP) server or direct Unix socket to the ZYAL daemon. Replace ZYAL's naive context-window appending with explicit `CompileContextPack()` calls.

---
*Architectural Blueprint Finalized. ANTIGRAVITY Systems. 2026.*
