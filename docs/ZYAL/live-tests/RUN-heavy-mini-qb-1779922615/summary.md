# Run summary — `heavy-mini-qb-1779922615`

**Schema:** `zyal.run_summary.v1`
**Pipeline:** `zyal_advanced_port`
**Terminal status:** `halted`
**Duration:** 76 s

## Pipeline progress

- **deepest_stage:** `finalize_master_plan`
- **stages reached (6):** capture_target, frame_request, retrieve_context, brainstorm_stages, critique_stages, finalize_master_plan
- **stages completed (1):** finalize_master_plan
- **artifacts produced:** task_contract, evidence, context_pack, stage_proposal, critique, master_plan

## Model calls

- total_attempts: **11** / parsed: **5** / retryable_failures: 6 / final_blocks: 0 / empty_responses: 2
- latency p50: 2302 ms, p95: 36287 ms
- by_user: user_1=5, user_2=6
- by_provider: jnoccio=11
- by_kind: frame=1, stage_brainstorm=4, stage_critique=1, stage_reduce=2, verifier=3
- by_state: parsed=5, retryable_failure=6

## Budget

- max_calls: —, used: 11, remaining: 1, exhausted: false

## Gates

none observed

## Signals

none fired

## Artifact paths

- `events_jsonl`: `/home/ubuntu/jekko/target/zyal/runs/heavy-mini-qb-1779922615/events.jsonl`

## Links

- [observability](docs/ZYAL/OBSERVABILITY.md)
- [playbook](docs/ZYAL/AGENT_PLAYBOOK.md)
- [quality_band](docs/ZYAL/MODEL_QUALITY_BAND.md)
- [spec](docs/ZYAL/SPEC.md)
