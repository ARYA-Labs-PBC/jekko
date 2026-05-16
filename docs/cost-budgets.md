# Cost & spend-risk budgets

This document is the canonical budget proof for every Jekko surface that
could plausibly incur real spend. It is the artifact `jankurai`'s
`HLT-026-COST-BUDGET-GAP` rule looks for, and it is the document operators
must consult before running anything that hits a paid provider.

Every paid or otherwise unbounded operation in this repo is enumerated
below with: (1) a declared budget ceiling, (2) the concrete enforcement
mechanism that holds the ceiling, and (3) a verification recipe a reviewer
can run from a clean checkout to confirm the enforcement is still wired up.

If you add a new surface that can spend money, network bandwidth, or CI
minutes, add a row to the surface table in the same PR. An undeclared
spend surface is treated as a release-blocking gap by the
`HLT-026-COST-BUDGET-GAP` audit rule.

The release lane reads the machine-readable budget declaration at
[`docs/testing.md#release-budget-gate`](./testing.md#release-budget-gate)
and validates it against the per-surface inventory at
[`docs/testing.md#cost-budget-proof`](./testing.md#cost-budget-proof). The
two documents must agree on every `cost_usd`, `stop_condition`,
`kill_switch`, and `approval_ref`.

## Surface inventory

| Surface | Budget | Enforcement | Verification |
|---------|--------|-------------|--------------|
| Default unit + integration suite (`cargo test --workspace --locked`) | $0 / run | No real provider HTTP. Adapter tests in `crates/jekko-provider/tests/` use canned SSE bytes from `tests/fixtures/*.sse` served over a one-shot `tokio::net::TcpListener` bound to `127.0.0.1:0`. | `rg "reqwest\|\.send\(\)\.await" crates/jekko-provider/tests/` returns zero hits. `rg "#\[ignore\]" crates/jekko-provider/` returns zero hits because there are no paid tests in this crate to gate. |
| Live-provider TUI proof (`crates/tuiwright-jekko-unlock/tests/live_prod_tui.rs::live_jekko_prompt_round_trips_through_tui`) | $0 in CI; bounded by operator wallet locally | `#[ignore]` on the test. `enabled()` requires `JEKKO_TUI_LIVE_PROD=1`. `require_env("JEKKO_API_KEY")` aborts if the key is missing. The test explicitly returns an error if `CI=true`, so a CI runner cannot ever execute it even with `--ignored`. Default model pinned to `jekko/gpt-5-nano` (cheap nano-class) unless overridden via `JEKKO_LIVE_MODEL`. Prompt is a single fixed-string round-trip. Per-test wall timeout 180 s (`LIVE_TIMEOUT`). | `rg "JEKKO_TUI_LIVE_PROD\|require_env\|CI.*true" crates/tuiwright-jekko-unlock/tests/live_prod_tui.rs`. Default `cargo test --workspace --locked` skips the test because it is `#[ignore]`d. |
| CI unit job | 60 min wall clock; single-runner | `timeout-minutes: 60` on the `unit` job in `.github/workflows/test.yml`; matrix has one entry (`linux`), so no parallel fan-out spend. Concurrency group cancels stale runs in-progress to avoid duplicated burn. | `grep -E "timeout-minutes\|matrix:\|max-parallel" .github/workflows/test.yml`. |
| CI TUI job | 60 min wall clock; single-runner | `timeout-minutes: 60` on the `tui` job in `.github/workflows/test.yml`. No matrix; runs on a single `ubuntu-latest`. | `grep -E "timeout-minutes" .github/workflows/test.yml`. |
| Other CI workflows (`generate`, `stats`, `parity`, `nix-hashes`, `security`, `jankurai`, `deploy`) | 10-60 min wall clock per job (see workflow file) | `timeout-minutes:` declared on every job. No workflow runs paid LLM traffic. | `grep -E "timeout-minutes" .github/workflows/*.yml`. |

## Why the default test pass costs $0

`cargo test --workspace --locked` (the lane every developer and every CI
runner uses) does not transmit a single byte to a real LLM provider.

- The HTTP-shaped tests in `crates/jekko-provider/tests/stream_e2e.rs` and
  `sse_parity.rs` bind a one-shot `tokio::net::TcpListener` to
  `127.0.0.1:0`, hand the adapter a synthesised base URL pointing at the
  loopback port, and serve a canned `text/event-stream` body loaded from
  `crates/jekko-provider/tests/fixtures/*.sse`.
- No production code path in `crates/jekko-provider/src/` is exercised
  against a real provider endpoint during the default test run.
- The only tests that hit a real provider are in
  `crates/tuiwright-jekko-unlock/tests/live_prod_tui.rs`, and they are
  `#[ignore]`d *and* refuse to start when `CI=true` *and* require both
  `JEKKO_TUI_LIVE_PROD=1` and `JEKKO_API_KEY` to be set.

## Current gaps and proposed work

A per-run dollar cap (the `JEKKO_PROVIDER_BUDGET_CENTS`-style guard
mentioned in earlier drafts of this document) does *not* exist in the
codebase today. The current spend ceiling on the local live-provider lane
is therefore operator-side and policy-based:

- The model defaults to `jekko/gpt-5-nano`, a nano-tier model whose
  per-token cost is small enough that a single fixed-prompt round-trip is
  pocket change. An operator who overrides `JEKKO_LIVE_MODEL` to a
  premium-tier model is consciously taking on the bill.
- The prompt is a fixed string (`"Reply exactly with JEKKO_TUI_LIVE_OK
  and no other text."`); there is no input fan-out.
- The test is single-shot; there is no retry storm.
- The kill switch is the operator's `Ctrl-C` plus the 180 s
  `LIVE_TIMEOUT` wall clock.

Proposed (not yet implemented) follow-ups:

1. Add a `JEKKO_PROVIDER_BUDGET_CENTS` env var honoured by the
   live-provider lane that aborts the test if the predicted spend (based
   on the response token count) exceeds the cap.
2. Emit a structured cost receipt (provider, model, input/output tokens,
   estimated dollar cost) at the end of every `live_jekko_*` test run,
   appended to `target/jankurai/cost-receipts.jsonl`.
3. Wire the receipt into the release-gate evidence bundle described in
   `docs/release.md`.

Until those land, treat the live-provider lane as a manual, attended
operation that is never invoked from CI.

## Running the live-provider smoke (paid by the operator)

1. Build the host binary: `cargo build -p jekko-cli --locked`.
2. Export the env vars:

   ```sh
   export JEKKO_API_KEY="…"
   export JEKKO_BIN="$(cargo run -p xtask -- host-binary-path)"
   export JEKKO_TUI_LIVE_PROD=1
   # Optional: pin a cheaper model.
   export JEKKO_LIVE_MODEL="jekko/gpt-5-nano"
   ```

3. Run the gated test:

   ```sh
   cargo test -p tuiwright-jekko-unlock --locked -- --ignored \
       live_jekko_prompt_round_trips_through_tui
   ```

4. Inspect the artifacts under
   `target/tuiwright-jekko/artifacts/live-prod/` (PNG screenshots of the
   boot frame and the model-reply frame).

## Verification recipe

A reviewer can confirm the budget claims above with the following one-shot
check from the repo root:

```sh
# 1. Default test pass makes zero outbound HTTP.
rg "reqwest|\.send\(\)\.await" crates/jekko-provider/tests/

# 2. Live-provider tests are gated.
rg "#\[ignore\]|JEKKO_TUI_LIVE_PROD|CI.*true" \
    crates/tuiwright-jekko-unlock/tests/live_prod_tui.rs

# 3. Every CI job has a wall-clock cap.
grep -E "timeout-minutes" .github/workflows/*.yml

# 4. No accidental parallel CI fan-out.
grep -E "matrix:|max-parallel" .github/workflows/test.yml
```

All four checks should match the table in [Surface inventory](#surface-inventory).
