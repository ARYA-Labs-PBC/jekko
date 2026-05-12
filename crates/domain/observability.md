# Domain Observability Contract

This file defines a typed, local-first repair surface for domain failures.

## Exception surface

```yaml
repair_ticket:
  domain: observability
  code: OBS-001
  boundary: repair
  retryable: true
  purpose: typed agent-friendly exception surface
  reason: opaque failures slow local debugging and reruns
  common_fixes:
    - rerun `just score`
    - inspect `docs/testing.md`
    - keep the repair scoped to `crates/domain`
  docs_url: docs/testing.md
  repair_hint: rerun `just score` after the scoped domain change
  timestamp_utc: 2026-05-07T10:42:25Z
```

## Trace contract

```json
{
  "task_id": "JK-0086",
  "lane": "observability",
  "result_code": "pass",
  "proof_command": "just score",
  "evidence_path": "crates/domain/observability.md",
  "purpose": "keep reruns local",
  "reason": "provide a typed repair hint for the next agent",
  "repair_hint": "rerun just score after the scoped domain change"
}
```
