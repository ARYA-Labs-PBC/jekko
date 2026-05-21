# TUI testing

Jekko is TUI-only. Browser UI and Playwright web lanes are intentionally out of scope.

The TUI lanes drive the actual built host binary through a PTY using the
`crates/tuiwright-jekko-unlock` harness. Tests assert real terminal frames,
not mocked output.

## CI-safe lane

Run the no-secret TUI lane before merging product UI changes:

```sh
just tui-ci
```

This builds the host binary, verifies `jekko --version` and `jekko --help`,
runs the Rust TUI smoke lane, compiles the tuiwright integration tests, and
runs the CI-safe PTY first-frame regression with `JEKKO_BIN` pointed at the
built host binary.

The boot regression command is:

```sh
JEKKO_BIN="$(cargo run -p xtask -- host-binary-path)" \
  cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml \
  default_tui_paints_first_frame -- --nocapture
```

It launches the actual built host binary in an isolated offline PTY. It fails
fast if the screen is still blank after 5 seconds or if the home prompt
sentinel does not appear within 10 seconds.

The quick startup smoke is the fastest gate for plugin-loader and boot-hang
regressions. Run it first after touching startup, plugin loading, or
`.jekko/plugins`:

```sh
just tui-startup-smoke
```

## Artifacts

All TUI tests write under `target/tuiwright-jekko/`:

- `boot/*.png` — boot-smoke screenshots.
- `traces/*.trace.jsonl` — tuiwright spawn/action traces.
- `logs/*.log` — Jekko boot logs copied from the isolated XDG data directory.
- `baseline/<screen>/<WxH>.{png,txt}` — checked-in reference snapshots.
- `rust/<screen>/<WxH>.{png,txt}` — Rust render output for baseline diffing.

For local diagnosis, run the host binary directly with visible stderr logging:

```sh
JEKKO_BIN="$(cargo run -p xtask -- host-binary-path)"
"$JEKKO_BIN" --pure --print-logs --log-level DEBUG
```

## Baseline matrix

The baseline matrix is the parity contract between the Rust render and the
captured reference render. The matrix is **11 screens × 5 resolutions = 55
PNG snapshots and 55 text snapshots**, captured under
`target/tuiwright-jekko/baseline/`.

Screens currently covered:

- Clean (9): `home`, `command-dialog`, `model-dialog`, `provider-dialog`,
  `theme-dialog`, `session-empty`, `shell`, `splash`, `prompt-autocomplete`.
- Advisory pre-trigger (2): `jnoccio-panel`, `zyal-panel`. These require
  explicit opt-in env to surface and are tracked as advisory until the Rust
  trigger paths land.

Deferred screens (`permission-prompt`, `question-prompt`, `jankurai-panel`)
need LLM mock fixtures or a discoverable trigger keybind and are not yet in
the matrix.

To compare a Rust render against the baseline:

1. Run the Rust render capture lane (it writes under
   `target/tuiwright-jekko/rust/`).
2. Diff against the baseline:

```sh
cargo run -p xtask -- baseline-diff \
  --baseline target/tuiwright-jekko/baseline \
  --rust target/tuiwright-jekko/rust \
  --format text \
  --threshold 1.0
```

- `--threshold` is a percent mismatch ceiling per pair (omit for advisory).
- `--format json` switches to machine-readable output for CI.

The lane prints a table with status, byte diff, and percent mismatch for each
pair, and exits non-zero if any pair exceeds the threshold.

## Local live production lane

Live tests are opt-in and must stay local. CI must not provide production keys.

The canonical local env file is outside the repo:

```sh
~/.config/jekko/live-prod.env
```

Supported keys:

```sh
JEKKO_API_KEY=...
JEKKO_LIVE_MODEL=jekko/gpt-5-nano
JNOCCIO_UNLOCK_SECRET_PATH=$HOME/jnoccio-fusion.unlock
JNOCCIO_TUIWRIGHT_E2E=1
JNOCCIO_TUI_TEST=1
```

To copy approved Jekko/Jnoccio keys from home-level env files without
printing values:

```sh
just tui-live-prod-init
```

That helper routes through `cargo run -p xtask -- live-prod-init`.

Then run the live lane:

```sh
just tui-live-prod
```

That lane routes through `cargo run -p xtask -- live-prod` and reuses the
`crates/tuiwright-jekko-unlock` PTY proof tests.

The live lane refuses to run when `CI=true`, redacts key values in output,
and writes screenshots under `target/tuiwright-jekko/`.

## Local live balancer lane

This lane is also local-only and refuses to run in CI. It proves the real
`jekko run` path selects every provider-specific candidate under the active
`~/.jekko/users/<user>/llm.env` tree for the chosen provider.

The smoke reads the current candidate set, stages those exact `llm.env`
files into a disposable temp `HOME` / `JEKKO_HOME`, then loops until every
candidate user has been observed once or the ceiling expires.

The source of truth for per-user credentials is:

```sh
~/.jekko/users/<user>/llm.env
```

Supported overrides:

- `JEKKO_LIVE_BALANCER_PROVIDER`
- `JEKKO_LIVE_BALANCER_MODEL`
- `JEKKO_LIVE_BALANCER_COUNT`
- `JEKKO_BIN`

Unlike the live-prod lane, this smoke is not pinned to a sample pair such as
`user_1` / `user_2`. It proves whatever provider-specific candidates are
currently eligible on the machine.

Run it through the wrapper script:

```sh
JEKKO_LIVE_BALANCER=1 scripts/live-balancer-smoke.sh
```

The wrapper builds and installs the host binary, then runs the ignored
tuiwright smoke with an exact test filter so helper tests in the same file do
not accidentally run.

## Background

The baseline matrix was captured from the pre-Rust product build to freeze a
behavioral contract for the Rust TUI. Treat the baseline PNGs and text
snapshots as the parity target until the Rust render matches under the default
threshold; do not re-capture against the Rust build.
