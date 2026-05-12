# TUI testing

Jekko is TUI-only. Browser UI and Playwright web lanes are intentionally out of scope.

## CI-safe lane

Run the no-secret TUI lane before merging product UI changes:

```sh
just tui-ci
```

This builds the host binary, verifies `jekko --version` and `jekko --help`, checks that no `web` command is exposed, runs rendered TUI component tests, compiles TUIwright tests, and runs the CI-safe PTY boot regression with `JEKKO_BIN` set.

The boot smoke regression is:

```sh
JEKKO_BIN="$(bun --cwd packages/jekko ./script/host-binary-path.ts)" \
  cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml \
  default_tui_paints_first_frame -- --nocapture
```

It launches isolated offline PTYs at `80x24`, `120x30`, and `200x60`. A run fails if the screen is still blank after 5 seconds or if the home prompt sentinel does not appear within 10 seconds.

Artifacts are written under `target/tuiwright-jekko/`:

- `boot/*.png` for boot-smoke screenshots.
- `traces/*.trace.jsonl` for Tuiwright spawn/action traces.
- `logs/*.log` for copied Jekko boot logs from the isolated XDG data directory.

For local diagnosis, run the built binary with visible stderr logging:

```sh
JEKKO_BIN="$(bun --cwd packages/jekko ./script/host-binary-path.ts)"
"$JEKKO_BIN" --pure --print-logs --log-level DEBUG
```

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

To copy approved Jekko/Jnoccio keys from home-level env files without printing values:

```sh
just tui-live-prod-init
```

Then run:

```sh
just tui-live-prod
```

The live lane refuses to run when `CI=true`, redacts key values in output, and writes screenshots under `target/tuiwright-jekko/`.
