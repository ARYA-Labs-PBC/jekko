# 05 — Formulas (Closed Forms for cogcore)

All the math the MEMSPECs handwaved. Every formula here is deterministic, stdlib-only, and tested via property-tests in `tests/`.

## 1. Topic-strength formula

The center-of-mass of "topic hardening." Recomputed on every cell append and during consolidation.

### Inputs (per topic)

| Symbol | Meaning |
|---|---|
| `topic.strength` | previous strength ∈ [0, 1] |
| `topic.last_update_tx` | ISO timestamp of last update |
| `topic.half_life_hours` | FSRS-derived per-topic half-life |
| `topic.contradiction_pressure` | weighted count of unresolved contradictions |
| `stats.recent_observes_30d` | u32 — observes in last 30d (wall-window via BENCH_NOW) |
| `stats.distinct_subjects` | u32 — distinct subjects in topic |
| `stats.new_concepts_7d` | u32 — concepts joined in last 7d |
| `stats.success_count` | u32 — recall→feedback success |
| `stats.failure_count` | u32 — recall→feedback failure |
| `stats.avg_source_quality` | f32 ∈ [0, 1] |
| `stats.recall_count` | u32 |
| `stats.superseded_fraction` | f32 ∈ [0, 1] — superseded cells / total cells |

### Closed form

```
dt_h = (BENCH_NOW − topic.last_update_tx) / 3600                      # hours since last update
decay = exp(−dt_h / topic.half_life_hours)                            # ∈ (0, 1]
decayed_base = topic.strength · decay                                 # carries prior

recency    = stats.recent_observes_30d / 30.0                         # ~ events per day
recurrence = ln(1.0 + stats.distinct_subjects)                        # log scale on diversity
utility    = stats.success_count / (stats.success_count + stats.failure_count + 1.0)
novelty    = stats.new_concepts_7d                                    # raw count
src_q      = stats.avg_source_quality                                 # ∈ [0, 1]
retr_succ  = stats.recall_count / (stats.recall_count + 1.0)          # smoothed [0, 1)

pressure = 0.30 · topic.contradiction_pressure + 0.10 · stats.superseded_fraction

topic.strength = clamp(
    decayed_base
    + 0.20 · recency
    + 0.18 · recurrence
    + 0.12 · utility
    + 0.08 · novelty
    + 0.10 · src_q
    + 0.20 · retr_succ
    − pressure,
    0.0, 1.0)
```

Sum of weights `0.20 + 0.18 + 0.12 + 0.08 + 0.10 + 0.20 = 0.88` — deliberately sub-1.0 because `decayed_base` already carries the prior.

### Update half-life

```
topic.half_life_hours = fsrs_topic_half_life(topic.strength, &stats)
topic.last_update_tx = BENCH_NOW
```

See §3 below.

### Tuning

Initial weights (α_r=0.20, α_c=0.18, α_u=0.12, α_n=0.08, α_q=0.10, α_h=0.20) are tuned on T0 offline. They become hyperparameters T1 (`tools/autoresearch`) can sweep — the loop self-tunes them.

## 2. Cell-strength formula (per-cell FSRS)

Same shape as topic, but per-cell:

```
dt_h = (BENCH_NOW − cell.last_recall_tx) / 3600
decay = exp(−dt_h / cell.half_life_hours)
decayed_base = cell.strength · decay

utility    = cell.utility                                              # EMA from feedback
src_q      = cell.source_quality
recall_n   = cell.recall_count
success_n  = cell.success_count
success_r  = success_n / max(1, recall_n)

bump = 0.25 · success_r + 0.10 · utility + 0.10 · src_q

cell.strength = clamp(decayed_base + bump, 0.0, 1.0)
cell.half_life_hours = fsrs_cell_half_life(cell.strength, success_r, recall_n)
cell.last_recall_tx = BENCH_NOW
```

Cell strength is invoked on every successful recall via `WalOp::RecallTouch`.

## 3. FSRS half-life

Anki's FSRS-5 variant, simplified for stdlib (no `f64::powf` is fine, but minimize fp ops):

```
fn fsrs_topic_half_life(strength: f32, stats: &TopicStats) -> f32 {
    let base = 24.0_f32;                                // 1 day base
    let success_factor = (1.0 + 4.0 * (stats.success_count as f32
        / (stats.success_count + stats.failure_count + 1) as f32));
    let recall_factor = 1.0 + 0.5 * (stats.recall_count as f32).ln_1p();
    let strength_factor = 1.0 + 2.0 * strength;
    base * success_factor * recall_factor * strength_factor
}

fn fsrs_cell_half_life(strength: f32, success_rate: f32, recall_count: u32) -> f32 {
    let base = 24.0_f32;
    let s_factor = 1.0 + 3.0 * success_rate;
    let r_factor = 1.0 + 0.4 * (recall_count as f32).ln_1p();
    let strength_factor = 1.0 + 1.5 * strength;
    base * s_factor * r_factor * strength_factor
}
```

A topic with 90% retrieval success doubles half-life on each successful recall.

## 4. Hebbian update rules

Sparse co-activation matrix `coact: BTreeMap<(CellId, CellId), f32>` where `a < b` enforces order. All `cell_id` references are `(min(a,b), max(a,b))` for symmetry.

### On recall (hot path, deterministic, WAL-recorded as `RecallTouch`)

```
for (a, b) in pairs(used_ids.iter().take(64)):           # cap 64 pairs/recall
    let key = (min(a,b), max(a,b))
    let prev = coact.get(&key).copied().unwrap_or(0.0)
    coact.insert(key, prev + 0.05 · (1.0 − prev))         # η_recall = 0.05
```

### On feedback `TaskSuccess` or `Verified`

```
for (a, b) in pairs(feedback.used.iter()):
    let key = (min(a,b), max(a,b))
    let prev = coact.get(&key).copied().unwrap_or(0.0)
    coact.insert(key, prev + 0.15 · (1.0 − prev))         # η_success = 0.15
```

### On feedback `Falsified`

```
for (a, b) in pairs(feedback.used.iter()):
    let key = (min(a,b), max(a,b))
    if let Some(prev) = coact.get(&key) {
        coact.insert(key, prev − 0.20 · prev)             # η_falsify = 0.20
    }
```

### On feedback `TaskFailure`

Smaller penalty than `Falsified` (failure doesn't mean wrong, just unhelpful):
```
for (a, b) in pairs(feedback.used.iter()):
    let key = (min(a,b), max(a,b))
    if let Some(prev) = coact.get(&key) {
        coact.insert(key, prev − 0.05 · prev)
    }
```

### On feedback `Ignored`

```
for (a, b) in pairs(feedback.used.iter()):
    let key = (min(a,b), max(a,b))
    if let Some(prev) = coact.get(&key) {
        coact.insert(key, prev − 0.02 · prev)
    }
```

### Decay (offline only, in `consolidate.rs`)

```
let half_life_h = 720.0;                                   # 30 days
for (key, w) in coact.iter_mut():
    let dt_h = (BENCH_NOW − last_decay_tx) / 3600
    *w = *w · exp(−dt_h / half_life_h)
coact.retain(|_, w| *w >= 0.02)                            # sparsifier
```

Total coact storage bound: ≤ 2 · M · log²(M) for M cells. Offline pruning triggers when memory > 256 MB.

## 5. Concept emergence — MinHash + Jaccard

### Sketch construction (per cell, on observe)

```
fn minhash_sketch(tokens: &[TokenId], n: usize = 8) -> [u32; 8] {
    let bigrams = generate_bigrams(tokens);
    let mut sketch = [u32::MAX; 8];
    for (i, seed) in MINHASH_SEEDS.iter().enumerate() {        # 8 deterministic seeds
        for &bg in bigrams.iter() {
            let h = fnv1a_seq(&[seed.to_le_bytes(), bg.to_le_bytes()]);
            sketch[i] = sketch[i].min(h);
        }
    }
    sketch
}

const MINHASH_SEEDS: [u32; 8] = [
    0xdeadbeef, 0xfeedface, 0xcafebabe, 0xbaadf00d,
    0x12345678, 0x87654321, 0xa5a5a5a5, 0x5a5a5a5a,
];
```

### Jaccard estimate

```
fn jaccard_minhash(a: &[u32; 8], b: &[u32; 8]) -> f32 {
    let matches = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count();
    matches as f32 / 8.0
}
```

8-hash MinHash has expected error ≈ 0.18 (standard result `√(p(1-p)/k)` for `p=0.5, k=8`). Cheap and good enough for concept clustering.

### Concept promotion algorithm (`consolidate.rs::promote_concepts`)

```
fn promote_concepts(unprocessed: &[CellId], cells: &Cells, concepts: &mut Concepts) {
    let mut buckets: BTreeMap<u32, Vec<CellId>> = BTreeMap::new();
    for &id in unprocessed.iter() {
        let key = cells[id].minhash_sketch[0];               # bucket by first hash
        buckets.entry(key).or_default().push(id);
    }
    for (_key, bucket) in buckets {
        if bucket.len() < 3 { continue; }
        let mut group = Vec::new();
        for &a in bucket.iter() {
            for &b in group.iter() {
                let j = jaccard_minhash(&cells[a].minhash, &cells[b].minhash);
                if j < 0.55 { continue 'outer; }
            }
            group.push(a);
        }
        if group.len() >= 3 {
            let kernel = top_15_tfidf_intersection(&group, cells);
            let label = most_frequent_subject(&group, cells);     # ASCII-lex tiebreak
            concepts.open(Concept { kernel_tokens: kernel.into(), member_cells: group.into_iter().collect(), ... });
        }
    }
}
```

Tiebreak rule: when two subjects are tied for "most frequent," pick ASCII-lex smallest. Never use insertion order.

### Topic emergence (community detection on concept graph)

```
fn promote_topics(concepts: &Concepts, graph: &Graph, topics: &mut Topics) {
    let mut edges: Vec<(ConceptId, ConceptId, f32)> = Vec::new();
    for (a, b, coact_weight) in graph.coact_concept_pairs() {
        if coact_weight >= 0.40 {
            edges.push((a, b, coact_weight));
        }
    }
    let components = greedy_modularity_components(&edges, &concepts);
    for component in components {
        if component.len() >= 4 {
            let label = most_shared_kernel_token(&component, concepts);
            topics.open(Topic { concepts: component, ... });
        }
    }
}
```

Greedy modularity: union-find with `Δmodularity > 0.1` merge threshold. Reference impl: pure stdlib, no `petgraph`.

### Topic merge

If two topics overlap by more than 60% of concept set, merge into the older (by `formed_at`):

```
overlap(t1, t2) = |t1.concepts ∩ t2.concepts| / min(|t1.concepts|, |t2.concepts|)
if overlap(t1, t2) > 0.6:
    merge t1 into older(t1, t2)
```

## 6. Fusion score (hot path)

For each candidate cell during `recall`:

```
score(cell, query) =
    1.00 · bm25_z(cell, query)                                       # normalized BM25
  + 0.60 · (1 if subject_substring(cell.subject, query.text) else 0)
  + 0.40 · (|query_tokens ∩ kernel_tokens(cell)| / |kernel_tokens(cell)|)
  + 0.30 · exp(-Δt_h(cell) / topic.half_life_hours)
  + 0.50 · cell.strength
  + 0.40 · cell.utility
  + 0.30 · cell.source_quality
  + 0.70 · max(topic.strength : topic ∈ cell.topics)
  − 0.50 · 1[CellFlags::SUPERSEDED]
  − 0.50 · 1[CellFlags::CONTRADICTED]
```

`bm25_z` = BM25 score z-scored over the candidate pool to bound it in roughly `[-2, +3]`.

These weights are constants — never mutated in the hot path. They are config in `cogcore/config.toml` and so are T1-sweepable by AutoResearch.

## 7. Graph rerank boost

For each cell in top-K (K=32) survivors:

```
boost(cell) = 0.15 · Σ_{other ∈ top32 \ cell} coact[(min(cell, other), max(cell, other))]
score(cell) += boost(cell)
```

Bounded cost: 32² = 1024 lookups per recall.

## 8. Compounding hop-depth weights

Per-case score for compounding fixtures:
```
case_score = Σ_q (depth_weight[q.hop_depth] · q_score) / Σ_q depth_weight[q.hop_depth]
```

Where `depth_weight = [1.0, 1.5, 2.25, 3.4]` for hops 1-4. Each hop ~1.5× rarer to land than the previous one.

## 9. Hardening composite score

Per-case `hardening_score`:
```
hardening_score = 0.4 · support_concentration
                + 0.3 · confidence_growth
                + 0.2 · token_reduction
                + 0.1 · determinism
```

Where:
```
support_concentration = clamp((|used_ids@t0| − |used_ids@t4|) / max(1, |used_ids@t0|), 0, 1)
confidence_growth     = clamp(confidence[t4] − confidence[t0], 0, 1)
token_reduction       = clamp((tokens[t0] − tokens[t4]) / max(1, tokens[t0]), 0, 1)
determinism           = 1.0 if pack_hash[t4] matches fresh-rerun pack_hash else 0.0
```

**Gate-to-zero**: if correctness fails at any timestep (any of t0..t4), the entire case scores 0. Prevents degenerate strategies.

## 10. North-star composite

Final number reported to the user / used by the AutoResearch reducer:

```
N = (T0·0.10 + T1·0.30 + compounding·0.20 + hardening·0.15 + qbank·0.20) / 1.00
N ← apply_hard_gates(N, gates)
```

Sum of weights = 1.00. Each input is the suite's own gate-capped total. `score_mix.rs` divides by `Σ weights`, so adding more inputs later only requires extending the `--input` list.

## 11. Hard-gate caps

| Condition | Cap |
|---|---:|
| `unsafe_tool_exec > 0` | 50 |
| `privacy_leaks > 0` | 60 |
| `citation_issues > 0` | 70 |
| `future_leaks > 0` | 75 |
| `!deterministic` | 80 |
| `compounding_regression ≤ -3.0` | 85 |
| `hardening_regression ≤ -3.0` | 85 |
| `!knowledge_non_degradation` | 80 |

Strictest cap wins (chained `score = score.min(cap)`).

## 12. Determinism contracts

These properties MUST hold or the system is broken:

1. `bench --candidate X` run twice with same args produces byte-identical JSON.
2. After a stream of N observes + M recalls, `export_state_hash()` is deterministic in the input sequence.
3. After `rebuild()` from the ledger, `export_state_hash()` matches the live state.
4. `recall_as_of(q, t)` and `recall_at(q, t)` MUST NOT mutate state (no `WalOp::RecallTouch`).
5. The order of WAL ops applied during replay yields the same projections regardless of any insertion-order quirks (sorted BTreeMap iteration, ASCII-lex tiebreaks everywhere).
6. Concept names break ties via ASCII-lex of subject strings.
7. Topic names break ties via ASCII-lex of "most-shared kernel token."

Property tests live in `crates/cogcore/tests/ledger_replay.rs` and `tests/determinism_property.rs`.

## 13. Token-interning canonicalization

After `rebuild()`, the token interner must reassign IDs in canonical order (sorted by first-seen bytes) so the projection hashes remain stable. The naive "first-observed-gets-id-0" rule is fragile because WAL replay order may differ from live insertion order. Solution:

```
fn canonicalize_after_replay(&mut self) {
    let mut all_tokens: BTreeSet<&[u8]> = BTreeSet::new();
    for cell in self.cells.values() {
        for tok in cell.tokens.iter() {
            all_tokens.insert(self.interner.bytes_of(tok));
        }
    }
    // Reassign IDs in BTreeSet iteration order (= sorted bytes)
    let new_interner = Interner::from_sorted(&all_tokens);
    self.interner = new_interner;
    // Rewrite cell.tokens with new IDs
    for cell in self.cells.values_mut() {
        cell.tokens = cell.tokens.iter().map(|t| new_interner.id_of_bytes(self.interner.bytes_of(t))).collect();
    }
}
```

Required for `tests/ledger_replay.rs` to pass.

## 14. Glossary of weights and constants

| Name | Value | Where used |
|---|---:|---|
| `η_recall` | 0.05 | Hebbian update on recall |
| `η_success` | 0.15 | Hebbian update on feedback success |
| `η_falsify` | 0.20 | Hebbian update on feedback falsified |
| `α_r` (recency) | 0.20 | Topic strength |
| `α_c` (recurrence) | 0.18 | Topic strength |
| `α_u` (utility) | 0.12 | Topic strength |
| `α_n` (novelty) | 0.08 | Topic strength |
| `α_q` (source quality) | 0.10 | Topic strength |
| `α_h` (retrieval success) | 0.20 | Topic strength |
| `w_contra` | 0.30 | Contradiction pressure |
| `w_stale` | 0.10 | Superseded fraction |
| `τ_attach` (concept attachment) | 0.45 | Hot path |
| `τ_form` (concept emergence) | 0.55 | Consolidation |
| `τ_topic_coact` | 0.40 | Topic emergence |
| `λ_graph_boost` | 0.15 | Recall rerank |
| `top_K` (graph rerank window) | 32 | Recall |
| `pair_cap` | 64 | Hebbian per recall |
| `coact_decay_half_life_h` | 720 (30d) | Offline decay |
| `coact_prune_threshold` | 0.02 | Sparsifier |
| `bm25_k1` | 1.5 | BM25 |
| `bm25_b` | 0.75 | BM25 |
| `bm25_avg_doc_len_init` | 200 | Bootstrapped on first 1000 cells |
| `BENCH_NOW` | "2026-05-12T00:00:00Z" | Deterministic clock |
| `MINHASH_SEEDS` | 8 × u32 constants | MinHash sketch |
| `FSRS_BASE_HOURS` | 24 | FSRS half-life base |
| `consolidation_interval_observes` | 512 | Default daemon trigger |
| `Budget::ZERO` | `{0, 0, 0}` | Benchmark default |

All values are config (`cogcore/config.toml`) for T1 sweeps. AutoResearch can tune these and observe northstar improvements.
