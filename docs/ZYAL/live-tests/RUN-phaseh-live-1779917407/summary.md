# Run summary — `phaseh-live-1779917407`

**Schema:** `zyal.run_summary.v1`
**Pipeline:** `zyal_hero_judge`
**Terminal status:** `run_finished`
**Duration:** 164 s

## Pipeline progress

- **deepest_stage:** `—`
- **stages reached (0):** 
- **stages completed (1):** final_signoff
- **artifacts produced:** 

## Model calls

- total_attempts: **22** / parsed: **14** / retryable_failures: 8 / final_blocks: 0 / empty_responses: 7
- latency p50: 3803 ms, p95: 20217 ms
- by_user: user_1=11, user_2=11
- by_provider: jnoccio=22
- by_kind: hero_generate=6, judge_patch=2, knowledge_curate=1, literature_synthesis=3, meta_judge=1, red_team=4, verifier=5
- by_state: parsed=14, retryable_failure=8

## Budget

- max_calls: —, used: 22, remaining: 10, exhausted: false

## Gates

none observed

## Signals

| id | name | count |
|---|---|---:|
| `judge_patch` | `judge_patch` | 2 |
| `promotion_decision` | `promotion_decision` | 1 |

## Artifact paths

- `claim_ledger_jsonl`: `/home/ubuntu/jekko/target/zyal/runs/phaseh-live-1779917407/claim_ledger.jsonl`
- `events_jsonl`: `/home/ubuntu/jekko/target/zyal/runs/phaseh-live-1779917407/events.jsonl`
- `headless_state_json`: `/home/ubuntu/jekko/target/zyal/runs/phaseh-live-1779917407/STATE.json`
- `headless_state_md`: `/home/ubuntu/jekko/target/zyal/runs/phaseh-live-1779917407/STATE.md`
- `model_receipts_jsonl`: `/home/ubuntu/jekko/target/zyal/runs/phaseh-live-1779917407/model_receipts.jsonl`
- `negative_memory_jsonl`: `/home/ubuntu/jekko/target/zyal/runs/phaseh-live-1779917407/negative_memory.jsonl`
- `replay_receipt_json`: `/home/ubuntu/jekko/target/zyal/runs/phaseh-live-1779917407/replay_receipt.json`
- `reviewer_packet_json`: `/home/ubuntu/jekko/target/zyal/runs/phaseh-live-1779917407/reviewer_packet.json`
- `superreasoning_packet_json`: `/home/ubuntu/jekko/target/zyal/runs/phaseh-live-1779917407/superreasoning_packet.json`
- `unsupported_claims_jsonl`: `/home/ubuntu/jekko/target/zyal/runs/phaseh-live-1779917407/unsupported_claims.jsonl`

## Links

- [observability](docs/ZYAL/OBSERVABILITY.md)
- [playbook](docs/ZYAL/AGENT_PLAYBOOK.md)
- [quality_band](docs/ZYAL/MODEL_QUALITY_BAND.md)
- [spec](docs/ZYAL/SPEC.md)
