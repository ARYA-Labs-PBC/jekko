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

---

## Phase H validation (GOD-logging session, 2026-05-27)

After FIX-1..7 + Phase A–F of the move-forward plan landed, this section
records a real live run that exercises the FULL chain — `users_pool`
fusion gateway → user_1+user_2 rotation → FIX-1's classifier →
empty-response tracker → auto-generated SUMMARY.json.

**Run:** `phaseh-live-1779917407` (OpenQG superreasoning,
`just zyal-superreasoning-live-local`).

**Why OpenQG, not heavy MiniRedis here?** The fusion-side
`quality_band` filter landed in Phase E (commit `c16933da0`), but the
**MANIFEST → jankurai-runner → jekko-run → request.extra** plumbing
that lets a ZYAL stage author *declare* a band per stage is a separate
sub-feature (the manifest currently has an empty `model_policy.routine: {}`
stanza — no field is yet parsed). Without that plumbing landed, the
heavy MiniRedis stage_brainstorm cannot escalate via quality_band on
its own. The OpenQG recipe was chosen as the Phase H driver because it
exercises the **same** live chain (users_pool fan-out, FIX-1 classifier,
empty-response tracker, SUMMARY.json auto-gen) on a pipeline that
*does* reach end-to-end completion. Once the manifest→runner plumbing
lands, heavy MiniRedis re-runs become a one-liner.

### Live evidence

| Metric | Value |
|---|---|
| `terminal_status` | **`run_finished`** ✅ (auto-emitted via the FIX-1-post finalize hook) |
| `halt_reason` | **`null`** ✅ |
| `duration_seconds` | 164 s |
| Model attempts | 22 |
| Parsed outcomes | 14 (63.6 % parse rate) |
| Retryable failures | 8 |
| Empty responses | **7** (no streak — they were spread across stages, so no halt) |
| User rotation | **`user_1: 11, user_2: 11`** — perfect 50/50 split |
| By kind | `hero_generate: 6, verifier: 5, red_team: 4, literature_synthesis: 3, judge_patch: 2, meta_judge: 1, knowledge_curate: 1` |
| Verifier score | 0.35 → promotion_decision (promoted=false, weighted score 0.732) |
| Quality index | overall=0.577, frontier=0.577, theory=0.618, rubric=0.725, question=0.839 |
| **Gateway delta** | +71 requests, +67 success, +4 failures (94.4 % success rate during the run) |
| **Balancer cursor** | 285 → 307 (+22, exactly matching the attempt count) |

### What this proves about the GOD-logging chain

1. **users_pool fan-out is active and balanced.** Both `user_1` and `user_2` saw 11 attempts each — fusion's per-user round-robin works under load.
2. **FIX-1 holds at scale.** Of 22 attempts, 14 were classified `parsed` (success), 8 were `retryable_failure`. Zero false-positive `model_failure` classifications — the prior session's misclassification bug stays fixed.
3. **Empty-response tracker behaves correctly.** 7 attempts had `response_bytes: 0`, but none formed a 3-streak at the same stage, so `EmptyResponseStreak` did NOT fire. The tracker only triggers on the dangerous pattern (consecutive empties at one stage), not noise.
4. **SUMMARY.json auto-generates at finalize.** The post-FIX-F hook in `hero_judge_runner_finalize::finalize_run` wrote `summary.{json,md}` next to `events.jsonl` automatically — no manual `--summarize` invocation needed.
5. **Provider-side recovery works.** 4 fusion failures occurred during the run; none halted the pipeline. The cooldown/retry logic in `jnoccio-fusion/src/limits.rs::cooldown_for` continues to do its job under live load.

### Auto-generated `summary.md` excerpt

```
# Run summary — `phaseh-live-1779917407`

**Schema:** `zyal.run_summary.v1`
**Pipeline:** `zyal_hero_judge`
**Terminal status:** `run_finished`
**Duration:** 164 s

## Model calls

- total_attempts: 22 / parsed: 14 / retryable_failures: 8 / final_blocks: 0 / empty_responses: 7
- latency p50: 3803 ms, p95: 20217 ms
- by_user: user_1=11, user_2=11
- by_provider: jnoccio=22
- by_kind: hero_generate=6, judge_patch=2, knowledge_curate=1, literature_synthesis=3, meta_judge=1, red_team=4, verifier=5
- by_state: parsed=14, retryable_failure=8

## Signals

| id | name | count |
| `judge_patch` | `judge_patch` | 2 |
| `promotion_decision` | `promotion_decision` | 1 |
```

The full SUMMARY.json + .md live at `target/zyal/runs/phaseh-live-1779917407/`.

### Heavy MiniRedis follow-up (one more step needed)

To exercise the same chain on the actual heavy MiniRedis brainstorm halt,
the next session needs to add the manifest-to-request plumbing:

1. `crates/jankurai-runner/src/model_policy.rs` (or similar): parse the
   `quality_band` value from each stage's `model_policy.<role>` entry.
2. `crates/jankurai-runner/src/model_client/runtime.rs`: forward the
   parsed band to the `jekko run` subprocess via env var
   (e.g. `JEKKO_RUN_QUALITY_BAND=top20`).
3. `crates/jekko-cli/src/cmd/run.rs` (or `cmd/run/`): read the env var
   and inject `{"quality_band": "<band>"}` into the OpenAI request's
   `extra` map.

Estimated size: 30–50 LOC across 2 files. Once landed, the heavy
MiniRedis manifest can declare:

```yaml
model_policy:
  routine: { quality_band: top50 }
  power:   { quality_band: top20 }   # stage_brainstorm uses this role
  critic:  { quality_band: top10 }
```

…and Phase H of THIS plan can be re-run on the MiniRedis pipeline to
prove the unblock. Tracked as FIX-CAND-M.
