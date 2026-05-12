# Architecture

This is the canonical agent-readable architecture note for repair work.

## Agent-friendly exception pattern

- Use a typed exception record instead of free-form logging:

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

- Include the failure boundary, the command to rerun, and the next action.

## Budget proof

- Treat paid or external work as budgeted work.
- Record `budget_usd`, `owner`, `stop_condition`, `approval_ref`, and `timestamp_utc` before starting a release lane.
- If the lane would exceed the budget, stop and mark the task blocked.

## Boundary evidence

- Keep receipts short, deterministic, and grepable.
- Pair each observed issue with the exact proof command.
- Route wider repairs back to the canonical standard instead of expanding the scope in place.
