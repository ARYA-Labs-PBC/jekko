# Running CI locally

Every CI job has a local equivalent so failures surface before they reach
GitHub. In this repo, `scripts/ci-local.sh` is the local entrypoint and
`just` recipes wrap it. All lanes route through `just` + `cargo` (or
`cargo run -p xtask`); there is no Node or Bun on the local CI path.

## Quick reference

| Recipe                   | Mirrors                            | Lane class    | Notes |
| ------------------------ | ---------------------------------- | ------------- | ----- |
| `just ci-doctor`         | local prerequisite check           | quick         | reports missing tools and install hints |
| `just ci-quick`          | fast workspace lane                | quick         | same checks as `just fast` |
| `just fast`              | typecheck + narrow tests           | quick         | smallest feedback loop |
| `just tui-startup-smoke` | local host-binary TUI smoke        | quick         | first-frame PTY regression on the built binary |
| `just tui-ci`            | CI-safe TUI lane                   | comprehensive | host binary smoke, TUI crate tests, tuiwright compile + first-frame |
| `just ci-local-pr-dry-run` | PR policy parity                   | comprehensive | uses a clean-room worktree, requires an authenticated `gh` CLI, and an open PR branch |
| `cargo test --workspace --locked --no-fail-fast` | full unit + integration suite | comprehensive | run before any release lane |
| `cargo run -p xtask -- ci-fast` | xtask-driven quick gate     | quick         | scaffold for the consolidated quick lane |
| `just ci-audit`          | Jankurai audit lane                | comprehensive | runs the audit and zero-caps gate |
| `just ci`                | full local CI parity               | comprehensive | runs the full local CI sequence |

Pick a quick lane during iteration. Run the comprehensive lanes before
pushing for review and before any release lane.

## First-time setup

```sh
just ci-doctor
```

Typical tools the doctor expects:

- `cargo`, `rustc`, `just`, `gh`, `jq`, `rg`, `awk`, `python3`
- `gitleaks`, `cargo-audit`, `zizmor`, `syft`
- `latexmk`
- `jankurai`
- authenticated `gh` CLI access so `gh auth token` succeeds for PR-policy parity

## Lane selection

- **Editing docs only:** `just fast` (and `just tui-startup-smoke` if the
  change touches TUI docs).
- **Editing one crate:** `cargo test -p <crate> --locked --no-fail-fast`
  during iteration, then `just fast` before pushing.
- **Editing TUI / plugin loader / startup:** start with
  `just tui-startup-smoke`, then `just tui-ci` before pushing.
- **Editing schemas, fixtures, OpenAPI, tools:** run the matching xtask
  parity gate (`baseline-diff`, `openapi-check`, `tool-schema-parity`,
  `session-fixture-parity`, `httpapi-parity`, `db-migration-smoke`).
- **Editing PR policy / pull_request_target workflows:** run
  `just ci-local-pr-dry-run` after `gh auth token` succeeds. It synthesizes a
  `pull_request_target` event in a temporary worktree, exports the explicit
  PR-policy contract, and invokes `ops/ci/pr-policy.sh` for standards and
  compliance. Then run `just ci` before pushing.
- **Pre-release:** `cargo test --workspace --locked --no-fail-fast` plus
  every applicable xtask parity gate. See `docs/release.md`.

## Editing the lane

Keep `scripts/ci-local.sh` as the source of truth for local CI flow
changes. When a workflow changes, update the corresponding local command
there so the local and GitHub paths stay aligned. For PR policy, keep the
wrapper contract and the clean-room worktree lane in sync with
`ops/ci/pr-policy.sh` and `Justfile`.

For TUI boot regressions, keep `docs/testing-tui.md`, `ops/ci/test-tui.sh`,
`just tui-ci`, and `just tui-startup-smoke` aligned.
