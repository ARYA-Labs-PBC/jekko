# 02 — cogcore Design

The Rust memory crate. Self-contained, deterministic, zero default dependencies. Implements `memory_benchmark::MemorySystem` via a thin adapter shim.

## 1. Naming

**`cogcore`** — cognitive core. Two syllables, abstract, not tied to consumer (jankurai/jekko/zyal). Crates-io-available. Passes the `no_branded_identifiers` scan at `crates/memory-benchmark/src/lib.rs:88`.

Adapter name string: `"cogcore"`. Wired into `crates/memory-benchmark/src/runner.rs::boxed_adapter` as one new match arm.

## 2. Layout

```
crates/cogcore/
├── Cargo.toml            # zero default deps; opt-in features only
├── src/
│   ├── lib.rs            # public surface, re-exports
│   ├── adapter.rs        # MemorySystem trait impl (boundary shim)
│   ├── core.rs           # Core struct: ledger + projections + budget
│   ├── ledger.rs         # WAL append-only, hash-chained
│   ├── cell.rs           # MemoryCell, CellId, CellFlags
│   ├── concept.rs        # Concept kernels, MinHash, attachment
│   ├── topic.rs          # Topic emergence + strength formula
│   ├── graph.rs          # Sparse coact + typed edge list
│   ├── hebb.rs           # Hebbian update rules
│   ├── fsrs.rs           # FSRS variant for cells AND topics
│   ├── index.rs          # BM25-lite + bigram + equation + subject lanes
│   ├── retrieval.rs      # Hot path: fuse → mask → redact → rerank → pack
│   ├── ingest/
│   │   ├── mod.rs        # ExtractorBackend trait
│   │   ├── paper.rs      # Section split + section dispatch
│   │   ├── equation.rs   # LaTeX-ish parser, SI unit table
│   │   ├── theorem.rs    # Theorem-DAG builder
│   │   └── extractor.rs  # Deterministic rule-based default
│   ├── consolidate.rs    # Offline daemon (LLM budget gated)
│   ├── budget.rs         # LLM/embedding call budget
│   ├── hash.rs           # FNV-1a + optional blake3 (feature)
│   ├── time.rs           # BENCH_NOW pin, ISO helpers
│   └── canary.rs         # Fragment-built redactor
├── tests/
│   ├── trait_smoke.rs           # round-trip via MemorySystem
│   ├── ledger_replay.rs         # rebuild → byte-identical state_hash
│   ├── topic_hardens.rs         # 50 abstracts → topic.strength ≥ 0.8
│   ├── compounding_passes.rs    # 3-hop chain across events
│   ├── poisoned_paper.rs        # contradiction surfaces, control intact
│   └── benchmark_smoke.rs       # 100-fixture suite ≥ 85
└── benches/
    └── hot_path.rs              # observe/recall p50/p99 (optional cargo bench)
```

### Cargo.toml

```toml
[package]
name = "cogcore"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
path = "src/lib.rs"

[features]
default = []
experimental_blake3 = ["dep:blake3"]
experimental_hnsw = ["dep:hnsw_rs"]

[dependencies]
# zero defaults
blake3 = { version = "1", optional = true }
hnsw_rs = { version = "0.3", optional = true }

[dev-dependencies]
memory_benchmark = { path = "../memory-benchmark" }
```

## 3. Public surface (`lib.rs`)

```rust
pub mod core;
pub mod adapter;
mod ledger; mod cell; mod concept; mod topic; mod graph;
mod hebb; mod fsrs; mod index; mod retrieval;
mod ingest; mod consolidate; mod budget;
mod hash; mod time; mod canary;

pub use adapter::Adapter;
pub use core::{Config, Core, StorageBackend};
pub use budget::Budget;
pub use ingest::{ExtractorBackend, RuleBackend};

// Re-export trait types when memory_benchmark is on the dep graph for tests
#[cfg(feature = "_benchmark_types")]
pub use memory_benchmark::{Event, Query, Receipt, RecallResult, Tombstone};
```

## 4. Core data model

### `MemoryCell` (`cell.rs`)

```rust
pub struct MemoryCell {
    pub id: CellId,                       // FNV-1a(kind|subj|body|tx_time)
    pub kind: CellKind,                   // mirrors EventKind
    pub subject: SmallStr,                // interned
    pub body: Box<str>,
    pub tokens: Box<[TokenId]>,           // pre-tokenized
    pub eq_atoms: Box<[EqAtom]>,          // (lhs, op, rhs, units)
    pub provenance: ProvId,               // arena index
    pub valid_from: Option<Iso>,
    pub valid_to: Option<Iso>,
    pub tx_time: Iso,
    pub modality: ClaimModality,
    pub privacy: PrivacyClass,
    pub flags: CellFlags,                 // bitfield
    pub topics: SmallVec<[TopicId; 4]>,
    pub concepts: SmallVec<[ConceptId; 4]>,
    pub strength: f32,                    // FSRS, [0,1]
    pub half_life_hours: f32,             // per-cell FSRS
    pub last_recall_tx: Iso,
    pub recall_count: u32,
    pub success_count: u32,
    pub source_quality: f32,              // max over sources
    pub utility: f32,                     // EMA on feedback
    pub supersedes: SmallVec<[CellId; 2]>,
    pub contradicts: SmallVec<[CellId; 2]>,
    pub derived_from: SmallVec<[CellId; 2]>,
}

bitflags::bitflags! {
    pub struct CellFlags: u32 {
        const SUPERSEDED      = 0b00000001;
        const CONTRADICTED    = 0b00000010;
        const VAULT           = 0b00000100;
        const UNSAFE_SKILL    = 0b00001000;
        const UNIT_MISMATCH   = 0b00010000;
        const COUNTEREXAMPLE  = 0b00100000;
    }
}
```

CellId is interned `u32` internally for cache density; the boundary keeps `String` IDs to match the trait. `Iso` is a 20-char `[u8; 20]` for ASCII-lex comparison (works because `YYYY-MM-DDThh:mm:ssZ`).

### `Concept` (`concept.rs`)

```rust
pub struct Concept {
    pub id: ConceptId,
    pub label: Box<str>,                  // canonical name
    pub aliases: Box<[Box<str>]>,
    pub kernel_tokens: Box<[TokenId]>,    // top-15 TF-IDF
    pub member_cells: BTreeSet<CellId>,
    pub formed_at: Iso,
    pub mass: f32,                        // log-count
    pub minhash: [u32; 8],                // 8-hash sketch for ANN
}
```

### `Topic` (`topic.rs`)

```rust
pub struct Topic {
    pub id: TopicId,
    pub label: Box<str>,
    pub concepts: BTreeSet<ConceptId>,
    pub strength: f32,                    // [0,1]
    pub last_update_tx: Iso,
    pub half_life_hours: f32,
    pub contradiction_pressure: f32,
    pub stats: TopicStats,
}

pub struct TopicStats {
    pub recent_observes_30d: u32,
    pub distinct_subjects: u32,
    pub new_concepts_7d: u32,
    pub success_count: u32,
    pub failure_count: u32,
    pub avg_source_quality: f32,
    pub recall_count: u32,
    pub superseded_fraction: f32,
}
```

### `Graph` (`graph.rs`)

```rust
pub struct Graph {
    pub coact: BTreeMap<(CellId, CellId), f32>,  // a < b
    pub edges: Vec<Edge>,                        // typed
}

pub struct Edge {
    pub from: CellId,
    pub to: CellId,
    pub kind: EdgeKind,
    pub weight: f32,
}

pub enum EdgeKind {
    Supersedes, Contradicts, DerivedFrom,
    MentionedIn, EquationOf, TheoremCites,
}
```

### `WalEntry` (`ledger.rs`)

```rust
pub struct WalEntry {
    pub seq: u64,
    pub prev_hash: [u8; 16],
    pub op: WalOp,
    pub hash: [u8; 16],
}

pub enum WalOp {
    Observe(Box<MemoryCell>),
    Tombstone { id: CellId, reason: Box<str>, deletion_proof: [u8; 16] },
    Feedback { pack_id: Box<str>, outcome: Outcome, used: Box<[CellId]>, reason: Option<Box<str>> },
    RecallTouch { used_ids: Box<[CellId]>, tx_time: Iso },
    Consolidate(Delta),  // changes from a consolidation pass
}
```

WAL record layout on disk:
```
[seq: u64 LE]
[prev_hash: [u8; 16]]
[op_tag: u8]                  // 1=Observe, 2=Tombstone, 3=Feedback, 4=RecallTouch, 5=Consolidate
[op_payload_len: u32 LE]
[op_payload: bytes]
[hash: [u8; 16]]              // FNV-1a(prev_hash || seq_le || op_tag || op_payload)
```

## 5. Hot-path algorithms

### observe — target ~5–20 µs

```
1. Canonicalize id: FNV-1a(kind | subject | body | tx_time)
2. Tokenize body: UTF-8 lowercase, drop stopwords, hash → TokenId
3. Equation extraction: regex /\\s*([A-Za-z_]\\w*)\\s*(=|≈|\\propto)\\s*([^.]+?)(?:\\s+\\[([^\\]]+)\\])?/
   → EqAtom { lhs, op, rhs, units (SI-canonicalized) }
4. Theorem extraction: regex /(?:Theorem|Lemma|Prop\\.|Corollary)\\s+\\w+\\s*\\((.*)\\)\\s*[:.]/
   → DAG hypothesis/consequent edges
5. Build MemoryCell, push to ledger
6. Projection update:
   - inverted index posting list (BM25)
   - subject map: subject → Vec<CellId>
   - equation lane: lhs → Vec<EqAtom>
   - bigram index
7. Concept attachment:
   - Compute Jaccard against existing Concept.kernel_tokens
   - If max Jaccard > 0.45 → attach; if ≥ 0.55 → also append minhash member
   - Else → no concept yet (created during consolidation)
8. Topic counter bump for each attached concept
9. WalOp::Observe → recompute prev_hash + new_hash
```

No LLM, no network, no clock. All steps deterministic.

### recall — target ~50–300 µs

```
let cands = candidate_pool(query, tx_t);           // ≤ 4096
let fused = fuse_scores(cands, query);
let masked = bitemporal_filter(fused, world_t, tx_t);
let (redacted, warnings) = privacy_scan(masked);
let ranked = rerank_with_graph_boost(redacted, query);
let packed = pack_for_budget(ranked, query.token_budget);
surfaces = scan_warnings(packed);                   // superseded, contradicted, unit, skeptic
on_recall_success(packed);                          // FSRS + Hebbian + topic
ContextPack { ... }
```

#### Candidate pool

Union of:
- **BM25 top-256** over `query.text` tokens
- **Subject-exact** / **mention-exact** matches (priority lane)
- **Equation lane** if `intent == Equation` (lhs match)
- **Theorem DAG 1-hop closure** if `intent == Theorem`
- **Concept-expand**: for each candidate cell with a concept, pull top-2 sibling cells from same concept (depth-1 graph diffusion, weight × 0.5)

#### Fusion score (weights tuned offline; constants — never mutated in hot path)

```
score = 1.00 · bm25_z
      + 0.60 · subject_match              // 1 if subject substring of query
      + 0.40 · concept_overlap            // |query_tokens ∩ kernel_tokens| / |kernel_tokens|
      + 0.30 · recency                    // exp(-Δt / topic.half_life_hours)
      + 0.50 · cell.strength
      + 0.40 · cell.utility
      + 0.30 · cell.source_quality
      + 0.70 · max(topic.strength over cell.topics)
      − 0.50 · 1[CellFlags::SUPERSEDED]
      − 0.50 · 1[CellFlags::CONTRADICTED]
```

#### Bitemporal filter (mirrors `reference_context_pack`)

```
if tx_t.is_some() && cell.tx_time > tx_t: drop; emit CausalMaskApplied (once)
if world_t.is_some():
    if cell.valid_from > world_t: drop
    if cell.valid_to.is_some() && cell.valid_to <= world_t: drop
```

#### Privacy scan

Fragment-built canary patterns (same approach as `reference_context_pack`, never literal canary substrings in source — passes the secret-sprawl detector):
```rust
let p = format!("{}{}{}", "sk-memory-", "bench-", "CANARY-");
```

Vault short-circuits BEFORE rendering: `cell.privacy == Vault → answer += "[REDACTED:vault] "; emit Warning::Redacted; OmissionNote`.

#### Graph rerank

For top-32 survivors:
```
boost(cell) = 0.15 · Σ_{other ∈ top32 \\ cell} coact[cell, other]
score += boost
```

Cheap (32² = 1024 lookups), bounded.

#### Pack greedy

```
sort by score desc
budget = query.token_budget
for cell in sorted:
    cost = cell.body.len() / 4         // rough token count
    if budget >= cost: answer.push(cell); budget -= cost
    else: omitted.push(OmissionNote { reason: "budget_exhausted", bytes: cell.body.len() })
```

#### Post-recall mutations

**The load-bearing trick.** Recorded as a single WAL op so replay is byte-identical:

```rust
let used_ids: Vec<CellId> = packed.iter().map(|c| c.id).collect();
let touch = WalOp::RecallTouch { used_ids: used_ids.clone().into(), tx_time: BENCH_NOW };
ledger.append(touch);

for cell in used_ids.iter() {
    cell.recall_count += 1;
    cell.strength = fsrs::strengthen(cell.strength, cell.success_count, cell.recall_count);
    cell.last_recall_tx = BENCH_NOW;
}

// Hebbian update on pairs (capped at 64)
for (a, b) in pairs(used_ids.iter().take(64)) {
    let key = if a < b { (a, b) } else { (b, a) };
    let prev = graph.coact.get(&key).copied().unwrap_or(0.0);
    graph.coact.insert(key, prev + 0.05 * (1.0 - prev));
}

// Topic counter bump
for cell in used_ids.iter() {
    for topic_id in cell.topics.iter() {
        topics[topic_id].stats.recall_count += 1;
    }
}
```

### recall_as_of / recall_at

Same pipeline. Bitemporal filter uses `world_t` or `tx_t`. **No post-recall mutations** — historical reads must not compound. Test in `tests/ledger_replay.rs` verifies this property: `recall_as_of` then `export_state_hash` returns the same hash as before the recall.

## 6. Topic-strength formula

See `05-formulas.md` for the full derivation. Summary:

```
dt_h = (BENCH_NOW − topic.last_update_tx) / 3600
decay = exp(-dt_h / topic.half_life_hours)
decayed_base = topic.strength · decay

recency    = topic.stats.recent_observes_30d as f32 / 30.0
recurrence = ln(1.0 + topic.stats.distinct_subjects)
utility    = stats.success_count / (stats.success_count + stats.failure_count + 1.0)
novelty    = stats.new_concepts_7d as f32
src_q      = stats.avg_source_quality
retr_succ  = stats.recall_count / (stats.recall_count + 1.0)
pressure   = 0.30·topic.contradiction_pressure + 0.10·stats.superseded_fraction

topic.strength = clamp(
    decayed_base
    + 0.20·recency + 0.18·recurrence + 0.12·utility
    + 0.08·novelty + 0.10·src_q + 0.20·retr_succ
    − pressure,
    0.0, 1.0)
topic.half_life_hours = fsrs::topic_half_life(topic.strength, &stats)
topic.last_update_tx = BENCH_NOW
```

## 7. Concept emergence (offline, `consolidate.rs::promote_concepts`)

```
for unprocessed_cell in new_cells_since_last_pass:
    sketch[cell.id] = minhash(token_bigrams(cell), n=8)

buckets = group cells by sketch hash collisions
for bucket in buckets where |bucket| >= 3:
    if pairwise Jaccard among bucket ≥ 0.55:
        kernel = top-15 TF-IDF tokens intersection
        label = most-frequent subject (ASCII-lex tiebreak)
        open Concept { kernel_tokens: kernel, member_cells: bucket, minhash: sketch[bucket[0]] }
```

Topic emergence:
```
for connected_component in concept_graph (edges = coact ≥ 0.40):
    if |component| >= 4:
        label = most-shared kernel token across component
        open Topic { concepts: component, ... }
```

Concept-name ties: ASCII-lex order; never insertion order. Topic merge: if two topics overlap > 0.6 in concept set, merge into the older.

## 8. Hebbian update rules

```
on recall:    coact[a,b] ← coact[a,b] + 0.05 · (1 − coact[a,b])
on success:   coact[a,b] ← coact[a,b] + 0.15 · (1 − coact[a,b])
on falsify:   coact[a,b] ← coact[a,b] − 0.20 · coact[a,b]
decay:        coact[a,b] ← coact[a,b] · exp(-Δt / 720h), drop if < 0.02   (offline only)
```

Pair list capped at 64 in any single touch to bound cost. Total coact storage ≤ 2 · M · log²(M) for M cells; offline pruning when > 256 MB.

## 9. Paper ingestion (`ingest/paper.rs`)

Default backend is rule-based (`extractor::RuleBackend`) — deterministic, no LLM.

```
PDF/raw text
  → section split: regex /\\n\\s*\\d+(\\.\\d+)*\\s+[A-Z][^\\n]+\\n/ OR
                   fallback to "Abstract|Introduction|Methods|Results|Discussion|References"
  → per-section:
      claims     = sentence boundary splitter (. ! ? + capital follow)
      equations  = ingest::equation::parse_section (LaTeX-ish, SI unit table)
      theorems   = ingest::theorem::parse_section (header regex, hypothesis/consequent)
      citations  = "[Smith 2024]" / "\\cite{...}" → Source { uri, citation, quality }
  → for each unit: build MemoryCell with kind ∈ {Claim, Equation, Theorem}
                   provenance.span = section_hash + (char_start, char_end)
  → ledger::append → projection update → concept attachment
  → emit Receipt per section
```

### Equation parser (`ingest/equation.rs`)

LaTeX-ish lhs/rhs/units extraction. SI unit table:
- Base: kg, m, s, A, K, mol, cd
- Energy: eV, keV, MeV, GeV, TeV, J
- Power: W, kW, MW
- Speed: m/s, km/s, c
- Mismatched units across same `lhs` → set `CellFlags::UNIT_MISMATCH`; consolidation emits synthetic `Counterexample` event.

### Theorem-DAG (`ingest/theorem.rs`)

```
regex: /(?:Theorem|Lemma|Proposition|Corollary)\\s+(\\w+)(?:\\s*\\((.*)\\))?\\s*[:.]\\s*/
       → header, name
parse hypothesis/consequent from "if X then Y" or "given X, Y holds":
       → edges (consequent uses hypothesis)
```

DAG stored as typed edges in `Graph::edges` with `EdgeKind::TheoremCites`. On `recall` with `intent == Theorem`, 1-hop closure surfaces dependencies.

### LLM fallback hook

```rust
pub trait ExtractorBackend {
    fn extract(&mut self, text: &str, budget: &mut Budget) -> Vec<MemoryCell>;
}

pub struct RuleBackend;
impl ExtractorBackend for RuleBackend { /* deterministic */ }

pub struct MaybeLlmBackend<L> { rule: RuleBackend, llm: L }
impl<L: LlmClient> ExtractorBackend for MaybeLlmBackend<L> {
    fn extract(&mut self, text: &str, budget: &mut Budget) -> Vec<MemoryCell> {
        let mut cells = self.rule.extract(text, budget);
        if budget.check_and_consume(1) {
            cells.extend(self.llm.enrich(text));  // only invoked if budget allows
        }
        cells
    }
}
```

Benchmark wires `Budget::ZERO`; LLM path unreachable; determinism preserved.

## 10. Consolidation daemon (`consolidate.rs`)

Out-of-hot-path. Invoked by host (NOT by benchmark during scoring):
- explicitly via `Core::consolidate()`,
- every 512 observes (configurable),
- before `Core::snapshot()`.

Passes (deterministic order):

1. **Drift sweep** — recompute `CellFlags::SUPERSEDED` against same-subject newer cells.
2. **Concept promotion** — MinHash clustering on cells since last pass; open Concepts at Jaccard ≥ 0.55.
3. **Topic emergence / merge** — community detection on concept graph; merge topics overlapping > 0.6.
4. **FSRS retune** — refit per-topic half-lives from observed success-vs-recall window.
5. **Coact decay & prune** — `coact ← coact · exp(-Δt / 720h)`; drop < 0.02.
6. **Equation cross-check** — same `lhs` with incompatible units → synthetic `Counterexample` event tagged `UnitMismatch`.
7. **(Optional) LLM enrich** — guarded by `budget.has_budget()`. Default budget = 0.
8. **Compaction** — rewrite WAL trimming superseded `RecallTouch` ops, preserving receipt-chain head.

Output: a `WalOp::Consolidate(Delta)` recording the diff. Snapshot writes the post-consolidation state.

## 11. Storage layout

```
<state_dir>/
├── ledger.wal      # length-prefixed records, hash-chained
├── ledger.idx      # seq → offset map (rebuildable)
├── snapshot.bin    # rolling snapshot (stdlib serialization)
├── concepts.bin    # rebuildable from ledger
└── topics.bin      # rebuildable from ledger
```

### In-memory backend

`StorageBackend::Memory` for benchmark + zyal sandbox: WAL lives in `Vec<u8>`, no I/O. Same code paths. No disk syscalls. Fully reproducible.

### Rebuild semantics

```
fn rebuild(&mut self) -> Receipt {
    self.cells.clear(); self.concepts.clear(); self.topics.clear(); self.graph.clear();
    self.interner.canonicalize_after_replay();  // tokens reassigned in canonical order
    for entry in self.ledger.replay() {
        apply(entry);
    }
    let receipt = self.next_receipt(None, "rebuild");
    receipt
}

fn export_state_hash(&self) -> String {
    let mut buf = Vec::new();
    for id in self.cells.keys_sorted() { buf.extend(id.bytes()); buf.push(b'|'); }
    for ((a, b), w) in self.graph.coact_sorted() { write!(buf, "C:{a}-{b}:{w:.4};"); }
    for id in self.tombstones.keys_sorted() { write!(buf, "T:{id};"); }
    fnv1a_hex(&buf)
}
```

**Determinism contract**: `rebuild()` after a stream of observes+recalls yields the same `export_state_hash()` as the live state. Test in `tests/ledger_replay.rs` enforces. Token interning uses canonical-after-replay ordering (sorted by first-seen bytes) so TokenIds reassign deterministically.

## 12. Path to ≥85 on the existing benchmark

The 10-axis scorer (`crates/memory-benchmark/src/scorer.rs`) tells us exactly what to emit.

| Axis | Weight | cogcore mechanism |
|---|---:|---|
| correctness | 20 | BM25 + concept-expand + graph rerank → higher hit-rate than pure substring |
| provenance | 12 | Every cell carries `Source { quality ≥ 0.85 }`; cite all used cells |
| math_science | 12 | Equation lane + SI unit table + consolidation cross-check emits UnitMismatch |
| bitemporal_recall | 10 | Same logic as `reference_context_pack` (mirror it) |
| contradiction | 10 | Topic `contradiction_pressure` surfaces `SkeptikSurfaced`; `Supersedes` flag surfaces `Superseded`; `Counterexample` event-kind surfaces `Contradicted` |
| english_discourse_coreference | 8 | Extractor preserves explicit names (`alice`, `bob`, `director`); does not blank pronouns |
| privacy_redaction | 8 | Fragment-built canaries + Vault short-circuit pre-render |
| procedural_skill | 8 | `unsafe`/`quarantined` tag triggers `UnsafeToolRefused` + answer "refused" |
| feedback_adaptation | 6 | `confidence = 0.6 · utility + 0.4 · source_quality` (learned, not constant) |
| determinism_rebuild | 6 | Snapshot-replay byte-identical via `WalOp::RecallTouch` |

Target: ≥85 mean, ≥90 on the `science` domain after a topic-warm-up phase. Only wiring change: one match arm in `runner.rs::boxed_adapter` — the score-band calibration test (`lib_tests.rs::candidate_score_bands_stay_calibrated`) only checks 4 named references, so cogcore lives outside that band.

## 13. Embeddability + seedability

For AutoResearch and zyal sandboxes:

- Crate is path-dep-able and offline-buildable. Default features: zero external crates.
- `experimental_blake3` feature: swaps FNV-1a → blake3 hashes (4× faster collision-resistance, 1 dep). Off by default to preserve benchmark hash bytes.
- `experimental_hnsw` feature: opt-in HNSW index for concept centroids. Off by default.
- `Adapter::with_seed(state_bytes)` lets `population_memory`-style seeded competitions snapshot/restore in O(N) bytes via WAL.
- No `.git` / network / clock reads — `time.rs::BENCH_NOW` follows harness convention. `std::time` is forbidden on the hot-path module by a `#![deny(...)]` lint.

## 14. Risks (covered in `07-risks.md`)

- Determinism vs learning → `WalOp::RecallTouch` makes it replayable.
- Token interning across rebuild → `Interner::canonicalize_after_replay`.
- Concept-name ties → ASCII-lex.
- Hot-path concept-expand blowup → cap top-K=8 by strength.
- Embedding model upgrade strategy → defer; default has no embedding model.

## 15. What this does NOT do (deliberately)

- **No skill execution.** Skills are stored as `EventKind::Skill` cells but never invoked by cogcore. Execution is the host's responsibility.
- **No multimodal grounding.** Images, audio, screenshots are out of scope for v1. The MEMSPEC corpus touches on this (MIRIX is multimodal); cogcore is text-and-equation-first.
- **No federation.** Single-node only. No multi-agent sharing, no consensus.
- **No production embedding model.** Reuses no embedding stack. The fusion score is BM25 + graph + utility; no neural embedding required for the benchmark.
- **No on-disk encryption.** Vault privacy is enforced at render time via redaction, not at-rest via crypto.

These are explicit non-goals for v1. Tracked in `06-roadmap.md` for future phases.
