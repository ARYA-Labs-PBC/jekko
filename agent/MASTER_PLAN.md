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
| B2.3 | Alias SuperReasoningArtifactContract → zyal_core::ArtifactContract | done | 0c8a9c5f5 |
| B2.4 | Alias ReasoningArtifactKind → zyal_core::ArtifactKind (+8 super-agent variants available) | done | 6a4bdeead |
| B2.5 | Split MemoryCapsule write-gate into named helpers | done | d06c90d35 |
| B2.6 | LaneId/RunId/ArtifactRef newtypes | scaffolded in B1; first usage lands in Phase F | — |
| C1 | Scaffold zyal-key-pool crate (pool / balancer / budget) | done | b288ca619 |
| C2 | Wire zyal-key-pool into jnoccio-fusion + jankurai-runner (UsersPool fanout + PolicyHook gate) | done | (next hash) |
| C3 | Create this MASTER_PLAN.md (closes `missing-agent-readable-docs` cap) | done | f081adf28 |
| D1 | ToolMode { Off, ReadOnly, Full } policy + per-role mapping | done | 830c1ee15 |
| D2 | Lift hardcoded JEKKO_RUN_DISABLE_TOOLS, add JEKKO_RUN_TOOL_ALLOWLIST | done | 830c1ee15 (same commit as D1) |
| D3-D5 | Parallel brainstorm via JoinSet + reducer fence + tests | deferred (needs complete_structured refactor for Send-safety) | — |
| E1 | Structured memory + promotion lifecycle (memory_kind, promotion_status, claim_text, approved_by_role) | done | (this branch) |
| E2 | Semantic retrieval via embeddings (depends on E1) | pending | — |
| F1 | jekko-runtime daemon: SuperReasoningPlan registration API + canonical_phases() 12-stage builder | done | fb0adf07cf |
| F2 | jankurai-runner: SuperReasoningConfig + draft module | pending (module-name collision with existing `superreasoning/` to resolve) | — |
| F3 | 12-stage blueprint + super_reasoning_stage_blueprint() | partially landed via F1's `canonical_phases()`; jankurai-runner-side wiring is F2/F3 | — |
| F4 | New zyal-supervisor crate with SQLite schema | in flight (subagent) | — |
| F5 | zyalc Profile::SuperWorkflow + ambitious-superworkflow.zyal example | pending | — |
| G1 | Watcher metrics + remediation engine + 5 new EventKind variants | done | (this branch) |
| G2 | Ratatui dashboard surface | pending | — |
| G3 | jekko-cli watch subcommand + notify-based tail loop | pending | — |
| G4 | jnoccio-fusion /metrics Prometheus endpoint | pending | — |
| H | Live execution against MiniRedis (smoke + heavy + chaos + audit) | pending; requires F1-F5 land first | — |

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
