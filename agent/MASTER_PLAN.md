# Master Plan

This file is the canonical phase log referenced by `AGENTS.md`. It tracks the in-flight macro plan for the repo and the rolling status of each phase.

## Phase index

For explicit MASTER_PLAN / phase-mode work, the local phase workflow lives under `tips/phases/`. See `tips/phases/00-phase-index.md` for the per-phase reading order. Plans the user supplies in-conversation (paper, release, implementation, or handoff plans) are the controlling plan and bypass this index — see `CLAUDE.md` for the routing rule.

## Active macro plan: foamy-koala

Plan file: `/home/ubuntu/.claude/plans/please-study-this-the-foamy-koala.md`

Centralization + super-agent kernel work toward driving long-running multi-stage workloads (mega-runs) through a unified ZYAL backbone with multi-user credential routing, parallel tool-enabled reasoning, semantic memory, and live observability.

### Phase log

| Phase | Subject | Status | Commit(s) |
|---|---|---|---|
| A | Unblock both build trees (EventKind variant + jnoccio-fusion field scaffolding) | done | 5378ce11e |
| B1 | Scaffold zyal-core leaf crate (6 modules) | done | 8b1c045ac |
| B2.1 | Migrate CredentialSourcePolicy → zyal-core | done | d7d61ece0 |
| B2.2 | Migrate forbidden-content patterns → zyal-core (split: shape vs credential) | done | cb2a9de71 |
| B2.3 | Alias SuperReasoningArtifactContract → zyal_core::ArtifactContract | done | (this branch) |
| B2.4 | Alias ReasoningArtifactKind → zyal_core::ArtifactKind (+8 super-agent variants available) | done | (this branch) |
| B2.5 | Split MemoryCapsule write-gate into named helpers | done | (this branch) |
| B2.6 | LaneId/RunId/ArtifactRef newtypes | scaffolded in B1; first usage lands in Phase F | — |
| C1 | Scaffold zyal-key-pool crate (pool / balancer / budget) | done | (this branch) |
| C2 | Wire zyal-key-pool into jnoccio-fusion + jankurai-runner | pending | — |
| C3 | Create this MASTER_PLAN.md (closes `missing-agent-readable-docs` cap) | done | (this branch) |
| D | Parallel + tool-enabled reasoning lanes with reducer fence | pending | — |
| E1 | Structured memory + promotion lifecycle | pending | — |
| E2 | Semantic retrieval via embeddings | pending | — |
| F | Compile → Seed → Execute super-agent kernel (12-stage blueprint) | pending; adoption plan ready at `tips/ZYAL/helper/` | — |
| G | Live observability + auto-remediation (Watcher, WorkerLease + quarantine) | pending | — |
| H | Live execution against MiniRedis (smoke + heavy + chaos + audit) | pending | — |

### Naming conventions

- **`jankurai`** is the external code auditor binary (https://github.com/neverhuman/jankurai/). Pre-existing `crates/jankurai*` keep their names; NEW crates introduced by this plan use `zyal-*` (for backbone types) or `jekko-*` (for general infra).
- **`jekko`** is this repo / workspace.
- **`ZYAL`** is the orchestration / runbook system inside jekko.

### Pre-existing repo caps

Tracked but NOT introduced by this plan; address as part of relevant later phases or a dedicated cleanup pass:

- `release-readiness-gap` — needs release artifact docs/staging
- `no-agent-friendly-exception-pattern` — repo-wide pattern refactor
- `missing-agent-readable-docs` — partially addressed by this file; full closure when more agent-readable docs land
- `ci-local-parity` — `just ci-local` is missing CI steps the GitHub workflows run

The pre-commit hook at `tools/jankurai-hooks/pre-commit` is informational only — it computes pass/fail but does not exit non-zero on score regressions. Tightening it is a follow-up.
