# ZYAL Powerful Examples

Seventeen flagship runbooks that show what ZYAL v2.6 makes possible. Each is a complete `<<<ZYAL v1:daemon>>>` block — paste any of them into a trusted Jekko prompt to preview the runtime plan, then explicitly arm to execute.

| File | Demonstrates | Power blocks |
|---|---|---|
| [`01-fix-until-green.zyal`](01-fix-until-green.zyal) | Bug-fix discipline with anti-vibe gates, capability leases, rollback, done | `quality`, `capabilities`, `rollback`, `done`, `models`, `budgets` |
| [`02-hypothesis-tournament.zyal`](02-hypothesis-tournament.zyal) | Disjoint experiments in isolated worktrees, blind cross-provider judging, negative-memory learning | `experiments`, `models`, `memory`, `budgets`, `evidence` |
| [`03-billion-loc-monorepo.zyal`](03-billion-loc-monorepo.zyal) | Repo intelligence + scope control + blast radius for massive codebases | `repo_intelligence`, `capabilities`, `experiments`, `quality`, `evidence` |
| [`04-fleet-portfolio.zyal`](04-fleet-portfolio.zyal) | Multi-issue dispatcher with cron, GitHub triggers, Jnoccio live metrics, budgets, anti-recursion, leases | `triggers`, `fleet.jnoccio`, `budgets`, `models`, `done`, `observability` |
| [`05-secure-mcp-lockdown.zyal`](05-secure-mcp-lockdown.zyal) | Capability leases, command floor, brokered secrets, sandbox isolation, MCP rug-pull defense | `capabilities`, `sandbox`, `security`, `mcp`, `arming` |
| [`06-evidence-graph-merge.zyal`](06-evidence-graph-merge.zyal) | Proof lanes + merge witness + rollback verify + workflow-driven approvals | `workflow`, `evidence`, `approvals`, `rollback`, `done` |
| [`07-self-improving-skills.zyal`](07-self-improving-skills.zyal) | Governed skill quarantine → human review → repo registry promotion | `skills`, `memory`, `approvals`, `quality`, `observability` |
| [`08-full-power-runbook.zyal`](08-full-power-runbook.zyal) | Every v2.1 power block in one runbook | All |
| [`09-control-plane-preview.zyal`](09-control-plane-preview.zyal) | Preview-only control-plane contract with interop, runtime, trust, taint, and release metadata | `interop`, `runtime`, `capability_negotiation`, `memory_kernel`, `evidence_graph`, `trust`, `taint`, `requirements`, `evaluation`, `release`, `roles`, `channels`, `imports`, `reasoning_privacy`, `unsupported_feature_policy` |
| [`10-jankurai-master-loop.zyal`](10-jankurai-master-loop.zyal) | Advanced host-enforced jankurai repair and porting loop with repair-plan ingestion, incubator routing, rollback, branch/main regression, taint, memory, evidence, and metrics | `jankurai`, `taint`, `fleet.jnoccio`, `incubator`, `experiments`, `memory`, `evidence`, `rollback`, `done`, `observability` |
| [`12-jankurai-min-loop.zyal`](12-jankurai-min-loop.zyal) | Minimal host-enforced jankurai continuous repair loop for ten low-risk workers, verification, checkpoint commit, and push | `jankurai`, `fleet`, `checkpoint`, `observability` |
| [`13-advanced-research-loop.zyal`](13-advanced-research-loop.zyal) | Evidence-first external research loop with parallel cited search and extraction controls; powers the README demo asset | `research`, `permissions`, `evidence`, `budgets` |
| [`memory-benchmark/prompt-scoring.zyal`](memory-benchmark/prompt-scoring.zyal) | Judge-population prompt scoring for the tracked memory benchmark corpus | `fan_out`, `memory`, `evidence`, `security`, `observability` |
| [`memory-benchmark/autoresearch-basic.zyal`](memory-benchmark/autoresearch-basic.zyal) | Basic sandboxed AutoResearch memory benchmark with four workers and best-verified-patch reduction | `fan_out`, `experiments`, `memory`, `sandbox`, `observability` |
| [`memory-benchmark/executable-benchmark.zyal`](memory-benchmark/executable-benchmark.zyal) | Deterministic Rust harness scoring for baseline, candidate, and reference memory systems | `fan_out`, `experiments`, `evidence`, `sandbox`, `observability` |
| [`memory-benchmark/autoresearch-chase.zyal`](memory-benchmark/autoresearch-chase.zyal) | Advanced sandboxed AutoResearch tournament with 20 lanes, persistent best-state promotion, and reducer state | `fan_out`, `experiments`, `memory`, `research`, `sandbox`, `evidence`, `observability` |
| [`memory-benchmark/generated-challenge.zyal`](memory-benchmark/generated-challenge.zyal) | Seeded generated public/private memory benchmark challenge with seed commitments | `evidence`, `permissions`, `done`, `security` |

## Running an example

1. Copy the YAML block (including sentinels) into the Jekko prompt.
2. The TUI flips to the **jekko-gold** theme, signalling daemon-arm mode.
3. The host renders the **Run Card**: capabilities granted, budgets, fleet/Jnoccio, taint summary, risk score, unsupported features, and preview-only limitations.
4. Type `ZYAL_ARM RUN_FOREVER id=<run-id>` only after reviewing the Run Card.
5. Watch the run stream events; pause/abort/inspect via `jekko daemon ...` CLI.

## Safety reminder

These examples grant real capabilities. Read the `capabilities`, `permissions`, `rollback`, `jankurai`, and `done` blocks before arming. The current runtime parses and previews `arming` policies, but the shipped start path still accepts the simple `ZYAL_ARM RUN_FOREVER id=<run-id>` sentinel; host nonce/hash arming is a preview contract until the start API is extended. The `jankurai` block is host-enforced when enabled.
The new `research` block is parsed and previewed as a host contract for current external evidence gathering; it is summarized in the Run Card and daemon prompt, while the hosted research backend remains optional and receipt-driven.
