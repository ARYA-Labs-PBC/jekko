# Testing

Run the narrowest proof lane that matches the change:

- `just fast` for local shape, ownership, and small documentation updates.
- `just score` for repo-score and documentation-dimension checks.
- `just security` for security-lane fixes that are not blocked by external evidence requirements.
- `just ux-qa` for rendered UI evidence.

Keep the proof receipt with the task entry:

- command
- exit result
- UTC timestamp
- touched files

Release lanes must record both budget proof and launch-gate evidence before any paid or unbounded work starts.
This is the canonical release gate: no paid or unbounded work starts without both receipts.

## Release budget gate

Release lanes are paid or otherwise spend-risky operations. Before any paid or unbounded lane starts, record all of the following in the task receipt:

- `budget_usd`
- `cost_usd`
- `quota_requests`
- `owner`
- `stop_condition`
- `kill_switch`
- `approval_ref`
- `timestamp_utc`

The receipt must make the stop condition and kill switch machine-readable. If either one is missing, or if the stop condition is already met, stop and mark the task blocked.

If a proof lane is blocked, record the block in the task and do not mark the task complete.

## Budget / stop-condition policy

- Treat paid or external work as budgeted work.
- Record `budget_usd`, `cost_usd`, `quota_requests`, `owner`, `stop_condition`, `approval_ref`, and `timestamp_utc` before starting a release lane.
- If the lane would exceed the budget or requires unbounded paid work, stop and mark the task blocked instead of continuing.
- Keep any `kill_switch` explicit and lane-scoped.

## Cost budget proof

- Record `budget_usd`, `cost_usd`, `owner`, `quota_requests`, `stop_condition`, `kill_switch`, `approval_ref`, `timestamp_utc`, and `proof_command` before starting a release lane.
- Keep the proof short, machine-readable, and attached to the release task before the lane begins.

```json
{
  "budget_usd": 12.5,
  "cost_usd": 12.5,
  "owner": "standard",
  "stop_condition": "block paid or unbounded operations when remaining budget is below 5% or approval_ref is missing",
  "quota_requests": 250,
  "kill_switch": "COST_GUARD_OFF=true",
  "approval_ref": "docs/testing.md#cost-budget-proof",
  "timestamp_utc": "2026-05-07T00:00:00Z"
}
```

- Do not start a release lane without this proof.

## Launch gate evidence

- Record machine-readable launch-gate evidence before any release lane.
- Do not call a release lane complete without security, backups, monitoring, rollback, and abuse controls evidence.

```json
{
  "security": true,
  "backups": true,
  "monitoring": true,
  "rollback": true,
  "abuse_controls": true,
  "proof_command": "just check",
  "timestamp_utc": "2026-05-07T00:00:00Z"
}
```

## Observability and repair evidence

- Use typed repair surfaces in docs and code instead of ad-hoc text.

## Agent-friendly exception pattern

- Prefer an agent-friendly exception pattern that includes:

```json
{
  "domain": "observability",
  "code": "OBS-001",
  "boundary": "verification",
  "retryable": true,
  "command": "just score",
  "retry_hint": "rerun after the scoped doc change",
  "timestamp_utc": "2026-05-07T00:00:00Z"
}
```

- Keep trace and receipt records machine-readable and bounded.

```json
{
  "domain": "observability",
  "code": "OBS-001",
  "command": "just score",
  "retry_hint": "rerun after the scoped doc change",
  "timestamp_utc": "2026-05-07T00:00:00Z"
}
```

- Log JSON repair traces per task with:
  - `task_id`
  - `lane`
  - `result_code`
  - `proof_command`
  - `evidence_path`
- Include a replay path in every repair entry: command, args, failure mode, and rerun instruction.

## Human-visible repair receipt template

For every lane execution, add a machine-readable receipt that includes:

- command
- exit code
- UTC timestamp
- command output summary or artifact path
- touched paths

## Boundary evidence format

- Keep a separate section per failure boundary (`verification`, `runtime`, `security`, `observability`, `release`) and keep receipts short, deterministic, and grepable.
- Avoid prose-only evidence; pair each observed issue with the exact command needed to reproduce it.

## Repair receipt schema

```yaml
repair_ticket:
  task_id:
  lane:
  command:
  exit_code:
  timestamp_utc:
  evidence_path:
  purpose:
  reason:
  common_fixes:
    -
  docs_url: docs/testing.md
  repair_hint:
```

Use this schema to preserve repairability and tell the next agent where to rerun proof:

- `exit_code` and raw artifact path in every lane claim.
- `repair_hint` should state the next rerun command and expected file scope.
- Include `purpose` and `reason` so the next agent can reproduce without tribal context.

### Telemetry trace contract

- Emit structured traces for repair attempts with fields:
  - `task_id`
  - `boundary`
  - `status` (`pass` / `fail`)
  - `proof_command`
  - `error_code`
  - `rerun_hint`
