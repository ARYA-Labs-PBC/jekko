# Run summary — `heavy-mini-qb-R-1779923213`

**Schema:** `zyal.run_summary.v1`
**Pipeline:** `zyal_advanced_port`
**Terminal status:** `halted`
**Duration:** 68 s

## Pipeline progress

- **deepest_stage:** `finalize_master_plan`
- **stages reached (6):** capture_target, frame_request, retrieve_context, brainstorm_stages, critique_stages, finalize_master_plan
- **stages completed (1):** finalize_master_plan
- **artifacts produced:** task_contract, evidence, context_pack, stage_proposal, critique, master_plan

## Model calls

- total_attempts: **10** / parsed: **5** / retryable_failures: 5 / final_blocks: 0 / empty_responses: 1
- latency p50: 4604 ms, p95: 25321 ms
- by_user: user_1=5, user_2=5
- by_provider: jnoccio=10
- by_kind: frame=1, stage_brainstorm=4, stage_critique=1, stage_reduce=1, verifier=3
- by_state: parsed=5, retryable_failure=5
- by_quality_band: top10=3, top20=2

## Budget

- max_calls: —, used: 10, remaining: 2, exhausted: false

## Gates

none observed

## Signals

none fired

## Artifact paths

- `events_jsonl`: `/home/ubuntu/jekko/target/zyal/runs/heavy-mini-qb-R-1779923213/events.jsonl`

## Links

- [observability](docs/ZYAL/OBSERVABILITY.md)
- [playbook](docs/ZYAL/AGENT_PLAYBOOK.md)
- [quality_band](docs/ZYAL/MODEL_QUALITY_BAND.md)
- [spec](docs/ZYAL/SPEC.md)
