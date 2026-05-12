# ZYAL v1

- Contract version: `2.6.0`
- Release tag: `v1.0.0`
- Runtime sentinel version: `v1`
- Research block version: `v1`

`ZYAL` is the host-enforced daemon/runbook format for Jekko.

Rules:

1. The block may have only blank lines or `#` comment lines before `<<<ZYAL v1:daemon id=<id>>>`.
2. The body must be YAML.
3. `intent` must be `daemon`.
4. `confirm` must be `RUN_FOREVER`.
5. The block must end with `<<<END_ZYAL id=<id>>>` and `ZYAL_ARM RUN_FOREVER id=<id>`.
6. Top-level keys are strict. Unknown keys are rejected.
7. Code fences and duplicate ZYAL blocks are rejected.
8. Nested keys are strict inside every block, including `job`, `loop`, `stop`, `context`, `checkpoint`, `tasks`, `agents`, `mcp`, `permissions`, `ui`, `incubator`, v2, v2.1, `fleet`, `research`, and `jankurai`.

The shipped parser also accepts `memory` store scopes `task`, `run`, `global`, `agent`, and `repo`, plus skill trust values `builtin`, `verified`, `unverified`, `community`, and `local`.

Existing `.zyal.yml` runbooks remain valid unless a new unknown key appears.

The runtime owns continuation. The model may finish an iteration, but it does not get to declare the daemon complete.

## Incubator

ZYAL v1 supports an optional top-level `incubator` block for hard tasks. The incubator is host-enforced: each task gets finite typed passes, durable SQLite-backed artifacts, generated mirrors under `.jekko/daemon/<runID>/tasks/<taskID>/`, and a runtime-owned promotion gate.

Allowed pass types are `scout`, `idea`, `strengthen`, `critic`, `synthesize`, `prototype`, `promotion_review`, and `compress`.

Allowed context modes are:

- `blind`: objective and constraints only.
- `inherit`: current capsule plus selected memory summaries.
- `strengthen`: current best candidate plus an improvement rubric.
- `critic`: candidate or current best plus an adversarial checklist.
- `pool`: candidate summaries, objections, and score table.
- `promotion`: final packet plus required evidence checklist.
- `ledger_only`: compressed memory summary only.

Allowed write scopes are `scratch_only` and `isolated_worktree`. `main_worktree` is never accepted inside `incubator`; prototype passes may use only `isolated_worktree` or `scratch_only`.

The incubator also supports these optional hardening blocks:

- `exclude_when`: same condition schema as `route_when`, but matching tasks remain in the normal lane.
- `cleanup`: host-owned cleanup policy with `summarize_to_task_memory`, `archive_artifacts`, `delete_scratch`, and `delete_unmerged_worktrees`.
- `readiness`: optional thresholds used by the preview and runtime gate to explain or tune promotion readiness.

When `incubator.enabled` is true, these fields are required and must be finite positive values:

- `budget.max_passes_per_task`
- `budget.max_rounds_per_task`
- `promotion.promote_at`

Idea pass `count` must be finite and no greater than `budget.max_parallel_idea_passes`. Unknown incubator keys, unknown pass types, unknown context modes, unbounded budgets, negative counts, and pre-promotion main worktree writes are rejected during preview.

Model confidence is only a low-weight input. Promotion depends on host evidence such as a problem statement, current best plan, verification strategy, risk review, resolved critical objections, bounded scope, and prototype evidence when configured.

MCP profile rules are exact allow lists. A pass may name `mcp_profile`; daemon execution must verify required profiles before exposing tools. Unlisted MCP tools remain hidden.

Prototype passes run in an isolated git worktree with a child session and a recorded artifact path for the generated diff. The main worktree remains untouched until host promotion.

## Jankurai

ZYAL v1 supports an optional top-level `jankurai` block. When
`jankurai.enabled: true`, the block is host-enforced by Jekko rather than
preview-only.

The daemon can run `jankurai audit`, run `jankurai repair-plan`, ingest
repair packets into durable daemon tasks, lease conflict-free path-locked
tasks, route medium/high-risk packets into the incubator, block human-required
or never-auto findings, verify candidates with configured commands and
`agent/test-map.json`, roll back unverified primary-checkout patches, and
compare branch results against `origin/main` without checking out main in the
primary worktree.

Supported literals:

- `audit.mode`: `advisory`, `guarded`, `standard`, `ratchet`, `release`
- `task_source`: `repair_plan`, `findings`, `agent_fix_queue`, `repair_queue_jsonl`
- `selection.order`: `quick_wins_first`, `severity_first`, `random`
- risk values: `low`, `medium`, `high`, `critical`
- `verification.audit_delta`: `no_new_findings`, `no_score_drop`, `target_fingerprint_removed`, `none`

Default behavior is conservative: repair-plan ingestion, low-risk quick wins
first, randomized ties, skip human-review findings, and roll back unverified
work.
