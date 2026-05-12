# ZYAL Claim Audit

Local repository: `/Users/bentaylor/Code/opencode`.

Status labels:

- Shipped: parser/runtime/test path exists and is wired into the daemon loop or public surface.
- Partial: helper modules and tests exist, or preview/run-card support exists, but full start-loop enforcement is incomplete.
- Preview: strict parser/preview support exists, but daemon execution must not be claimed as enforcing it.

| Claim | Status | Evidence |
|---|---|---|
| ZYAL blocks use strict sentinels and reject code fences, duplicate blocks, mismatched IDs, unknown top-level keys, and unknown nested keys. | Shipped | `packages/jekko/src/agent-script/parser.ts` (`extractZyalBlock`, `parseZyalSync`, `assertZyalNestedKeys`); `packages/jekko/src/agent-script/parser.test.ts`. |
| `ZYAL_ARM RUN_FOREVER id=<id>` is required by daemon start. | Shipped for simple arming | `packages/jekko/src/session/daemon.ts` throws when `parsed.arm` is missing; parser validates trailing arm ID. |
| Hash-bound, nonce-bound, origin-bound arming is parsed and shown in preview but not enforced by the start API. | Preview | `ZyalArmingPolicy` and preview summary in `schema.ts`; TUI run card explicitly says hash/nonce/origin policies are preview-only. |
| Schema covers core, safety, evidence, security, power, fleet, taint, and control-plane preview blocks. | Shipped parser/preview | `ZyalSpec` in `schema.ts`; `SUPPORTED_FEATURE_KEYS` in `parser.ts`; full docs examples parser test. |
| Current canonical example corpus uses `.zyal` examples. | Shipped docs/test | `docs/ZYAL/examples/README.md`; `parser.test.ts` asserts the canonical filenames. |
| Core daemon loop is host-owned: run, checkpoint, stop evaluation, incubator tick, continuation prompt, sleep, and context rotation. | Shipped | `packages/jekko/src/session/daemon.ts` `runDaemon` loop. |
| Daemon state persists in SQLite tables for runs, iterations, events, tasks, passes, task memories, workers, and artifacts. | Shipped | `packages/jekko/src/session/daemon.sql.ts`; `daemon-store.ts`. |
| Stop checks support shell and git-clean conditions. | Shipped | `daemon.ts` `evaluateStop`; `daemon-checks.ts`; parser schema for `ZyalStopCondition`. |
| Checkpointing runs verification before git add/commit/push; `push: ask` pauses for approval. | Shipped | `daemon-checkpoint.ts`; `daemon-checkpoint.test.ts`. |
| Incubator enforces finite passes, scratch/worktree write scopes, MCP profile checks, readiness routing, promotion/exhaustion gates, and task memory. | Shipped core | `daemon-incubator.ts`, `daemon-task-router.ts`, `daemon-task-promote.ts`, `daemon-task-memory.ts`, `daemon-store.ts`, `daemon-incubator.test.ts`. |
| v1.1 `on`, hooks, retry, constraints, and guardrail helpers exist with tests. | Partial to shipped depending path | `daemon-on-handler.ts`, `daemon-hooks.ts`, `daemon-retry.ts`, `daemon-constraints.ts`, `daemon-guardrails.ts`, tests. The daemon loop records hook/on-handler events and uses retry for stop evaluation; input/output guardrail helpers are not comprehensively wired into every tool path. |
| Workflow, memory, evidence, approvals, skills, sandbox, security, and observability modules exist with unit tests. | Partial | `daemon-workflow.ts`, `daemon-memory.ts`, `daemon-evidence.ts`, `daemon-approvals.ts`, `daemon-skills.ts`, `daemon-sandbox.ts`, `daemon-security.ts`, `daemon-observability.ts`, corresponding tests. These modules are not all fully enforced inside the daemon start loop. |
| Capability leases, anti-vibe quality gates, model routing, nested budgets, triggers, rollback, done, repo intelligence are strict parser/preview contracts. | Preview | `schema.ts`, `parser.ts`, parser tests, examples; no complete tool-gate/diff-gate/start-loop enforcement yet. |
| Fleet worker caps are schema/semantic enforced and surfaced in TUI metrics. | Shipped parser/preview, partial runtime | `ZYAL_FLEET_MAX_WORKERS`, parser semantic cap checks, `zyal-flash.test.ts`; runtime worker orchestration remains limited. |
| Taint labels and prompt-injection regexes are parsed and previewed but not enforced during daemon execution. | Preview | `schema.ts` v2.3 `ZyalTaint`, parser tests, `docs/ZYAL/examples/README.md` safety note, TUI run card preview-only text. |
| HTTP API exposes preview/start/list/get/events/pause/resume/abort/compact/rotate/tasks/pass/memory/incubator/task actions. | Shipped | `server/routes/instance/httpapi/groups/daemon.ts`, `handlers/daemon.ts`, `httpapi-daemon.test.ts`. |
| TUI renders a Run Card and ZYAL mode sidebar with live loop/token/cost metrics. | Shipped | `component/prompt/index.tsx` `zyalRunCard`; `routes/session/zyal-sidebar.tsx`; `zyal-flash.test.ts`. |
| External literature shows agentic programming has open challenges in memory, safety, monitoring, and human collaboration. | External | `agenticProgrammingSurvey2025`; `failedAgenticPRs2026`; `codingAgentPromptInjection2026`. |

Paper wording guardrails:

- Say "host-enforced" only for shipped paths in the daemon loop or API.
- Say "strict parser/preview contract" for v2.1/v2.2/v2.3 blocks unless the runtime path is verified.
- Do not claim model output can never cause harm; claim model output cannot bypass the specific host checks that are actually wired.
- Do not cite social posts as empirical evidence.
