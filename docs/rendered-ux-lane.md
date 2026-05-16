# Rendered-UX QA lane

The Rust TUI's rendered-UX QA lives in `crates/tuiwright-jekko-unlock/`.

## What it covers
- TUI baseline screen matrix across 5 resolutions (80x24, 100x30, 120x30, 160x40, 200x60)
- Dialog interaction PTY scenarios (theme, model, prompt-quit confirmation, etc.)
- Full session boot + new-user-setup flows

## Test suites
- `tests/rust_baseline_matrix.rs` — captures + diffs PNG snapshots per screen per resolution
- `tests/baseline_matrix.rs` — checked-in reference baseline matrix
- `tests/rust_dialog_keys.rs` — dialog open/close and option-select via PTY
- `tests/tui_boot.rs`, `tests/new_user_setup.rs`, `tests/readme_demo.rs` — boot/onboarding flows
- `tests/jnoccio_tui_dashboard.rs`, `tests/live_prod_tui.rs` — extended dashboard/live coverage
- `tests/binary_smoke.rs`, `tests/jekko_unlock_pty.rs` — host-binary PTY smoke

## Gate command
`cargo run -p xtask -- baseline-diff --threshold 80` — exit 0 means rendered-UX QA passing.

The composed CI-safe entrypoint is `just tui-ci`.

## Updating baselines
Capture fresh PNGs via the matching `_baseline` test and commit them into
`target/tuiwright-jekko/baseline/<screen>/<WxH>.{png,txt}`. The Rust rerun
output lands under `target/tuiwright-jekko/rust/` for diff comparison.

To re-baseline from scratch:

```bash
rm -rf target/tuiwright-jekko/baseline/
rm -rf target/tuiwright-jekko/rust/
install -m 0755 target/release/jekko ~/.local/bin/jekko
JEKKO_BIN=/Users/bentaylor/.local/bin/jekko cargo test \
  -p tuiwright-jekko-unlock --test baseline_matrix --locked --no-fail-fast
JEKKO_BIN=/Users/bentaylor/.local/bin/jekko JEKKO_RUST_MATRIX=1 cargo test \
  -p tuiwright-jekko-unlock --test rust_baseline_matrix --locked --no-fail-fast
cargo run -p xtask --quiet -- baseline-diff --threshold 80
```

## Coverage matrix (55 baselines)

11 screens × 5 resolutions:
- home, splash, shell, session-empty, prompt-autocomplete
- command-dialog, model-dialog, provider-dialog, theme-dialog
- jnoccio-panel, zyal-panel

## Recipe failure semantics

`crates/tuiwright-jekko-unlock/tests/common/mod.rs` recipes hard-fail
(`bail!`) when sentinel text is missing within the recipe's timeout, instead
of silently capturing whatever frame is on screen. On failure, a forensic
`<screen>-<res>-recipe-timeout.png` is written next to the canonical baseline
so the regressed state can be inspected.

## Chat-Enter integration test

`tests/tui_chat_enter_mock.rs` exercises the full chat-Enter loop end-to-end:
1. Launch jekko with `JEKKO_TUI_TEST_MOCK_LLM=1` + `JEKKO_TUI_TEST_MOCK_RESPONSE="..."`
2. Type text on Home — first printable char auto-engages Shell route and forwards to prompt
3. Press Enter — UserCard appears in activity feed
4. Mock LLM hook synthesises an AssistantCard from the env-var response
5. Assert both cards render in the transcript

The hook short-circuits the real provider call in
`crates/jekko-runtime/src/agent/executor.rs` so the test runs offline with no
API keys or network. Test guards: any change that breaks the chat-Enter loop
fails this test immediately — no more silent regressions.
