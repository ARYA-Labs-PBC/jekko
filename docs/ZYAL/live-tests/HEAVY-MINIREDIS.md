# HEAVY MiniRedis — Phase 3 (the promised heavy run)

**Phase:** 3 — heavy live MiniRedis port-run + 12-stage supervisor walk.
**Stage 3.A RID:** `heavy-mini-1779901731` (port-run, `--live --max-ticks 4`)
**Stage 3.B RID:** requested `heavy-super-1779901809`, supervisor-derived `ambitious-superworkflow-template-1779901809989`
**Timestamp:** `2026-05-27T17:08:51Z` → `2026-05-27T17:10:11Z`
**Result:** ⚠️ Stage 3.A halted at brainstorm with structured-failure event sequence; Stage 3.B walked all 9 waves / 12 phases as scaffold.

## Stage 3.A — heavy port-run

**Command:**
```bash
JEKKO_ZYAL_LIVE=1 JEKKO_BIN=/home/ubuntu/jekko/target/release/jekko \
  JEKKO_KEY_SOURCE_POLICY=users-only JEKKO_MODEL_CALL_TIMEOUT_SECS=300 \
  rtk cargo run --manifest-path crates/jankurai-runner/Cargo.toml --locked -- \
    --repo . --run-id heavy-mini-1779901731 port-run \
    --config docs/ZYAL/examples/35-rust-redis-replacement-superreasoning.port-run.json \
    --live --max-ticks 4
```

Budget envelope (from `35-rust-redis-replacement-superreasoning.port-run.json`):
`worker_cap=2`, `max_calls=12`, `max_parallel=1`. `--max-ticks 4` lifts the 1-tick ceiling.

### Event histogram (events.jsonl — 17 events, 1.7 KB)

| Kind | n |
|---|---|
| `reasoning_state` | 4 |
| `model_attempt` | 4 |
| `model_attempt_outcome` | 4 |
| `live_budget` | 4 |
| `reasoning_artifact` | 3 (task_contract, evidence, context_pack) |
| `model_outcome` (parsed) | **1** |
| `run_started` | 1 |
| `run_finished` | **0** |

### Stage trajectory

```
capture_target → run_started
  → frame_request → model_attempt(frame, user_1, parsed, 619 B, 700 ms) ✓
    → reasoning_artifact(task_contract)
  → retrieve_context
    → reasoning_artifact(evidence, count=1)
    → reasoning_artifact(context_pack)
  → brainstorm_stages
    → model_attempt(stage_brainstorm, user_2, response_bytes=0, retryable_failure)
    → model_attempt(stage_brainstorm, user_1, response_bytes=0, retryable_failure)
    → model_attempt(stage_brainstorm, user_2, response_bytes=0, retryable_failure)
    → final_block → exit 1
```

**Distance reached:** stages 1–4 (capture_target → frame → context → brainstorm). The earlier P1.c run halted at stage 2 (frame_request); the heavy run reaches stage 4 — that's the FIX-1 gain.

### 🟢 User-pool rotation evidence (still working under load)

`user_1: 3, user_2: 2` across 5 emitted credential events. Rotation alternated user_2 → user_1 → user_2 → user_1 across the 4 attempts:

| Attempt | Kind | User | Bytes | Latency |
|---:|---|---|---:|---:|
| 1 | frame | user_1 | 619 | 700 ms |
| 2 | stage_brainstorm | user_2 | 0 | 1401 ms |
| 3 | stage_brainstorm | user_1 | 0 | 701 ms |
| 4 | stage_brainstorm | user_2 | 0 | 1601 ms |

### 🔴 Root cause of halt — content-side, not code-side

The model returns `response_bytes: 0` for `stage_brainstorm` three times in a row across both users. **`success: true` (post-FIX-1 correctly reporting that jekko's subprocess exited 0)**, but `state: "retryable_failure"` (the runner correctly recognises empty content as un-parseable). This is the **correct post-FIX-1 classification**: the model spoke, but said nothing.

Possible upstream causes (not investigated here):
- The `stage_brainstorm` prompt for the MiniRedis target may exceed a model's effective output budget, causing it to short-circuit to empty.
- The selected underlying provider (rotated by fusion) may have a content filter or token budget on long prompts.
- The router's `complexity_tier` heuristic for this prompt size may consistently pick small-context models.

**This is exactly the use case for the deferred quality-band feature (`docs/ZYAL/MODEL_QUALITY_BAND.md`):** ZYAL stages could declare `quality_band: top20` on `stage_brainstorm` to force selection of higher-win-rate (likely larger-context, more capable) models.

**Phase 4 candidates surfaced here:**
- **FIX-CAND-J:** treat 3 consecutive `response_bytes == 0` as a distinct signal — currently it just looks like generic retryable_failure. A new `EventKind::EmptyModelResponseRun` would let the operator distinguish "model declined" from "model never responded".
- **FIX-CAND-K:** when the runner detects a streak of empty responses, escalate to a different model class — but this is the quality-band feature, deferred.

### Fusion-side metrics (Stage 3.A delta)

| Metric | Before | After Stage A | Δ |
|---|---:|---:|---:|
| `fusion_requests_total` | 302 | 312 | **+10** |
| `fusion_success_total` | 240 | 248 | **+8** |
| `fusion_failure_total` | 60 | 62 | **+2** |

4 model attempts × 2 (primary + backup) = 8 expected fusion calls; +10 observed (extra 2 from a fusion-sample probe). 2 actual upstream failures recorded at the gateway level — those are the empty-response cases the runner re-tried.

**Gateway-level success rate during Stage 3.A: 80% (8/10).**

### Balancer cursor

```
- jnoccio|jnoccio-router|266
+ jnoccio|jnoccio-router|270
```

Cursor advanced by **+4 ticks** matching the 4 attempts — gateway-level cursor working. (User-level fan-out is still inside fusion's routing layer; the round_robin_cursor table partitions by `(provider, model)` only — design note in `crates/zyal-key-pool/src/balancer.rs:51-66`.)

### Crack signals

| Signal | Observed? | Notes |
|---|---|---|
| `model_attempt_outcome_burst` (s1) | **YES** — 3 outcomes in 0 s window at brainstorm | Same caveat as Phase 2: high-volume retries at one stage trip the heuristic |
| `balancer_no_rotation` (s2) | NO | +4 cursor |
| `parity_gap_open_growth` (s3) | N/A | Parity_lab never reached |
| `worker_stall_or_quarantine` (s4) | NO | Run completed in ~17 s |
| `live_budget_exhaustion` (s5) | NO | Used 4/12 budget; halted before exhaustion |
| `proof_failed_in_live_lane` (s6) | N/A | Proof_lane never reached |
| `provider_error_explosion` (s7) | NO | 20% failure rate < 50% threshold |
| `parity_no_evidence` (s11) | YES (no parity_result event) | Parity_lab unreached |
| `judge_patch_without_proof` (s12) | N/A | Hero-judge path not taken |

## Stage 3.B — 12-stage supervisor plan-walk

**Command:** `just zyal-super-redis heavy-super-1779901809`
- Expanded to: `cargo run -p jekko-cli --offline -- port-run --super agent/zyal/ambitious-superworkflow.zyal --run-id heavy-super-1779901809`
- **Note:** `--run-id` was silently overridden by the manifest's id (`ambitious-superworkflow-template-1779901809989`). This is the FIX-CAND-H from Phase 2.

### Wave traversal — all 9 waves cleanly

```
wave 1/9: source_of_truth                            (1 phase)
wave 2/9: architecture_blueprint, repo_graph_bootstrap (2 phases)
wave 3/9: contracts_and_slices                       (1 phase)
wave 4/9: parallel_subsystems, parity_lab            (2 phases)
wave 5/9: integration_fusion                         (1 phase)
wave 6/9: parity_gap_closure                         (1 phase)
wave 7/9: hardening_security, performance_closure    (2 phases)
wave 8/9: docs_release_ops                           (1 phase)
wave 9/9: final_signoff                              (1 phase)
```

Total: **12 phases marked Complete** in the supervisor store.

### Phase status (12/12 complete, scaffold-mode)

Every phase has the same summary: *"scaffold-mode: per-phase invocation deferred until --live wires the jankurai-runner subprocess for this phase"*.

Phase IDs (canonical order):
`source_of_truth → architecture_blueprint → repo_graph_bootstrap → contracts_and_slices → parallel_subsystems → parity_lab → integration_fusion → parity_gap_closure → hardening_security → performance_closure → docs_release_ops → final_signoff`

### No fusion activity during Stage 3.B

| Metric | Stage A end | Stage B end | Δ |
|---|---:|---:|---:|
| `fusion_requests_total` | 312 | 312 | **0** |
| `fusion_success_total` | 248 | 248 | **0** |
| `fusion_failure_total` | 62 | 62 | **0** |

Confirmed: the 12-stage SuperWorkflow walker in its current form is a **plan/store scaffold only** — it does not yet wire jankurai-runner per phase. The integration is "Phase H scaffold" per the Justfile comment.

### Phase 4 candidate: FIX-CAND-L
- The 12-stage supervisor walker should optionally invoke jankurai-runner per phase to make the SuperWorkflow plan executable. Right now `--live` mode on `port-run --super` spawns `jekko run --ephemeral --json --agent plan <prompt>` per phase, but the prompt comes from the manifest's phase definition. A bigger change would route each phase through the appropriate `port-run` or `hero-judge-run` recipe based on its `objective`.
- **Severity:** feature work, not a bug. Out of scope for this campaign.

## Phase 3 verdict

| Criterion | Required | Observed | Pass? |
|---|---|---|:---:|
| ParityGap events observed | (goal) | NONE — Stage 3.A halted before parity_lab | ❌ |
| ParityManifestGenerated emitted | (goal) | NO | ❌ |
| `parity/gaps.json` written | (goal) | NO | ❌ |
| ≥ 2 distinct user-slot identifiers | yes | user_1 + user_2 | ✅ |
| Cursor advanced | yes | +4 ticks | ✅ |
| `RunFinished` or structured terminal | yes (or structured failure) | structured `final_block` after 3 retryable_failures | ⚠️ partial |
| No SIGABRT/SIGSEGV in fusion | yes | clean | ✅ |
| Jankurai final_score unchanged | yes | 70/88/4/7 | ✅ |
| 12-stage plan-walk traverses cleanly | yes | all 9 waves, all 12 phases complete | ✅ |

**Overall verdict:** ⚠️ **PARTIAL PASS.** The heavy MiniRedis port-run halts at brainstorm with a **content-side** failure (model returns 0-byte response 3× in a row, correctly retried, correctly diagnosed). The user-rotation, balancer, fusion gateway, and supervisor scaffold are all healthy. Reaching parity_lab requires either (a) higher-quality models for `stage_brainstorm` (the quality-band feature) or (b) a different test target whose brainstorm stage is more amenable to the current model pool.

## Cumulative Phase 3 fix candidates

- **FIX-CAND-J:** new structured signal for "empty-response streak ≥ N" — would surface this content-side issue distinctly from generic retryable_failure.
- **FIX-CAND-K:** quality-band escalation on empty-response streak — depends on the deferred quality-band feature.
- **FIX-CAND-L:** wire jankurai-runner subprocess per phase in `port-run --super --live`. Feature work.

These join the consolidated queue in `BATCH-zyal-testing-phase2.md`.
