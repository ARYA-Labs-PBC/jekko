# smartmemory/ — Design corpus for cogcore + 12-axis north-star + AutoResearch

This directory documents the design for a state-of-the-art memory system layered on top of the existing `crates/memory-benchmark/`. It is **documentation only** — no executable code lives here. Code lives in `crates/cogcore/`, `crates/memory-benchmark/src/generated/{compounding,hardening}.rs`, and `tools/autoresearch/`.

## What this is

Three coupled designs:

1. **cogcore** — a Rust memory crate that implements `memory_benchmark::MemorySystem` with a deterministic append-only ledger, FSRS-on-cells, Hebbian co-activation on retrieval, A-MEM concept kernels, paper-ingestion pipeline, and a topic-hardening formula. Zero default dependencies; LLMs only on offline consolidation behind a budget. Built to embed in `zyal` sandboxes as an AutoResearch seed.

2. **12-axis north-star benchmark** — extends the existing 10-axis scoring to 12 by adding `compounding` (weight 10) and `topic_hardening` (weight 8), with new fixture suites that exercise multi-hop chains, 5-timestep repeated queries, cross-domain transfer, and adversarial paper poisoning. A new `memory-benchmark-northstar` Justfile target produces a single composite score in <5min wall clock.

3. **4-tier AutoResearch loop** — orchestrator that runs aggressive search over memory designs in worktree-isolated workers, gated by shadow-suite divergence, calibration anti-tamper, and a receipt chain on a long-lived `autoresearch/chase-best` branch.

## Why this corpus exists

The user asked for a deep audit of the in-flight memory work plus an actionable design for what comes next. The MEMSPEC files in `tips/smart_memory/` (~30 specs across V1-V3 for CLAUDE/CODEX/GEMINI/ANTIGRAVITY + 23 deep tips) converged on a typed-lanes + append-only-ledger + multi-signal-retrieval pattern but never landed on a single implementation. The benchmark in `crates/memory-benchmark/` is mature but cannot measure the traits the user actually cares about (compounding, hardening, cross-domain transfer). This corpus is the synthesis: what to build, why, and in what order, with closed forms where the specs handwaved.

## Reading order

| File | Purpose |
|---|---|
| `00-audit.md` | What already works — benchmark core, ZYAL pipeline, MEMSPEC convergence |
| `01-gaps.md` | What's missing — no learning crate, benchmark blind spots, AutoResearch undefined behavior, spec handwaving |
| `02-cogcore-design.md` | Rust memory core: name, layout, data model, hot path, ingestion, consolidation, storage |
| `03-benchmark-12axis.md` | 12-axis weights, compounding/hardening suites, hard-gate extensions, <5min composite |
| `04-autoresearch-loop.md` | Trust zones, seeding, 4-tier mutation ladder, promotion contract, shadow suite, receipts |
| `05-formulas.md` | Closed forms (topic strength, Hebbian, FSRS, concept emergence, fusion score) |
| `06-roadmap.md` | 5-phase implementation plan |
| `07-risks.md` | 13 known risks + mitigations |
| `08-glossary.md` | Terms |
| `refs/` | Pointers into the repo and tip files (no copies) |

## Provenance

This corpus distills:
- The plan at `~/.claude/plans/can-you-please-do-curried-sparrow.md` (approved 2026-05-13).
- The existing repo: `crates/memory-benchmark/`, `crates/qbank-builder/`, `docs/ZYAL/`, `tips/smart_memory/`, `tips/smart_memory/v2/`.
- ~30 MEMSPEC documents across 4 AI tools and 3 generations.
- 36 tip files (13 top-level + 23 in `v2/`).

Naming follows the `feedback-naming-general-primitives` rule: pick abstract names, not consumer-tied names. `cogcore` is not "jankurai-memory" or "jekko-store"; it stands on its own.

## What is NOT in this corpus

- Code. cogcore source lives at `crates/cogcore/`. Generators at `crates/memory-benchmark/src/generated/`. Orchestrator at `tools/autoresearch/`.
- Experimental results. Benchmark numbers + autoresearch receipts are emitted to `target/memory-benchmark/` and `.jekko/daemon/memory-benchmark-chase/`, not here.
- Tip-file copies. The tips at `tips/smart_memory/{*.txt,*.md}` and `tips/smart_memory/v2/*.txt` are referenced by path, never duplicated.

## Status

**Phases 1-5 landed (snapshot 2026-05-13).** cogcore Phase 2 scores 90.65 on the 12-axis northstar (T0+T1+Compounding+Hardening+QBank). All four reference adapters stay in their [70, 90] calibration band. The AutoResearch orchestrator runs end-to-end with the T1 hyperparameter-sweep proposer; T2-T4 proposer ladders are deferred. See `refs/snapshot.md` for measured numbers and shipped file list, and `06-roadmap.md` for the per-phase milestones.
