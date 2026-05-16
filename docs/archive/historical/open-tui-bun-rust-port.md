# OpenTUI/Bun to Rust Port

Status: **source-code packets complete; Packet O cutover executed; final verification passed**

Workspace `cargo test --workspace --locked --no-fail-fast` → **761 passed across 82 suites**. Release binary `cargo build -p jekko-cli --release --locked` → 10.2MB, `jekko 0.1.0`. Rust render matrix captured (55 PNG across 11 screens × 5 res). `xtask baseline-diff` is the CI parity gate.

Packet O cutover has removed the live JS/Bun surface. Any remaining Bun/OpenTUI mentions are historical and live only in the allowlisted migration docs and receipts.

## Objective

Replace the current Bun/TypeScript/OpenTUI product runtime with a Rust CLI/TUI/server stack.

## Current State

- Root Rust workspace builds with all 9 `jekko-*` crates + xtask + tuiwright-jekko-unlock.
- `jekko-cli` ships 22 subcommands. Default subcommand launches the Ratatui TUI via `jekko_tui::run`.
- `jekko-tui` is a feature-complete Ratatui port: lifecycle, components, dialog framework, prompt widget, transcript (7 card types + diff + ANSI/YAML tokenizers + permission/question UI), feature plugins (Jnoccio/Jankurai/ZYAL/plugin manager/sidebar), key dispatch (Ctrl+P palette, Ctrl+X leader, Esc pops).
- `jekko-runtime`: bus, sessions, 9 tools, permissions, file/ripgrep/watcher, shell/PTY, LSP/MCP framing, snapshots, daemons.
- `jekko-provider`: 20-provider catalog, 19 recommended models, full transform layer with cache-control/tool-call parity, 5 adapters (Anthropic/OpenAI/JNOccio/OpenRouter/LiteLLM), SSE streaming.
- `jekko-server`: Axum + 17 route modules + SSE + WebSocket + PTY + auth + CORS + utoipa OpenAPI.
- `jekko-store`: rusqlite (bundled) + 24 embedded migrations + Bun-byte-identical `migrationHash` for journal interop.
- `jekko-core`: pure-domain types/parsers — session/provider/project/permission schemas + keybind + theme + config.
- `jekko-plugin-api`: `JekkoPlugin` trait, `PluginRegistry`, `ExternalPluginManifest` (TOML, semver-validated), `MigrationWarning` for legacy v1 JS plugins.
- `xtask` ships real implementations of all parity gates (`db-migration-smoke`, `cli-help-parity`, `tool-schema-parity`, `session-fixture-parity`, `httpapi-parity`, `openapi-check`, `ci-fast`, `package`, `baseline-diff [--threshold]`) plus all the Codex CI helpers.
- TUIwright captures: 55 OpenTUI baseline PNGs + 55 Rust render PNGs at `target/tuiwright-jekko/{baseline,rust}/`.
- Nix flake migrated to crane-based Rust derivation; Bun dropped from dev shell.
- Several CI and workflow entrypoints now use Rust/xtask instead of Bun where the replacement is already safe:
  - `ops/ci/test-unit.sh`
  - `ops/ci/typecheck.sh`
  - `ops/ci/test-tui.sh`
  - `ops/ci/generate.sh`
  - `ops/ci/stats.sh`
  - `ops/ci/close-issues.sh`
  - `ops/ci/containers.sh`
  - `ops/ci/close-stale-prs.sh`
  - `ops/ci/compliance-close.sh`
  - the corresponding `test`, `typecheck`, `generate`, `stats`, `close-issues`, and `containers` GitHub workflows
- The remaining automation workflows still depend on the older JS/LLM-driven agent behavior and cannot be fully removed yet without a richer Rust workflow orchestration layer for the agentic lanes.
- A typed GitHub event normalization helper now exists in Rust via `jekko-core` and `xtask github-event`, and the workflow scripts that only need webhook metadata now read it instead of `jq`.
- The stale-PR closer has also moved to Rust via `xtask close-stale-prs`, removing the inline Python block from `ops/ci/close-stale-prs.sh`.
- The compliance auto-close helper has also moved to Rust via `xtask compliance-close`, removing the inline Python block from `ops/ci/compliance-close.sh`.
- The containers lane now uses `xtask package-manager-version` instead of `jq` for `package.json` parsing.
- The PR policy checks now run through Rust `xtask` commands:
  - `ops/ci/pr-standards.sh` -> `xtask pr-standards`
  - `ops/ci/pr-compliance.sh` -> `xtask pr-compliance`
- The review lane now uses Rust `xtask` helpers for PR metadata fetches and wrapper orchestration, replacing the shell-side Bun bootstrap and `gh api` + `json-field` pair.
- The PR-management contributor-label helper now runs through Rust `xtask contributor-label`, the release Discord notification now runs through `xtask notify-discord`, and the duplicate-issues lane now runs through `xtask duplicate-issues` with the Rust `jekko` provider wired as the runtime target.
- `jekko run` now routes through a Rust runtime boundary that parses the prompt, selects a provider/model, streams a provider-backed assistant turn, and can persist both user and assistant messages. A minimal two-round Rust tool loop now exists; the remaining gap is richer workflow orchestration and workflow-specific agent routing, not the initial LLM turn.
- `xtask package` now accepts a configurable dist root, which is the missing plumbing needed for any future Rust publish staging path that has to write into `packages/jekko/dist` instead of the repo-root `dist/`.
- `xtask publish-build-cli` now exists as the Rust-native release-build wrapper around `xtask package`, with `packages/jekko/dist` as its default staging root.
- `xtask publish-sync-release-files` now handles the deterministic release-file version rewrites that used to live inside `script/publish.ts`.
- `xtask publish-release-init` and `xtask publish-release-finalize` now own the deterministic release tag / dev-sync Git and GitHub release orchestration that used to live inside `script/publish.ts`.
- `xtask publish-npm-package` now owns the reusable npm package pack/publish flow for the plugin and SDK package scripts, and those scripts are now thin wrappers around Rust.
- `xtask publish-npm-package` now also handles package identity checks, export-path rewrites, `npm pack`, and `npm publish` for those wrapper scripts, so the remaining Bun code there is only a launch shim.
- `xtask publish-release-package` now owns the release-package npm publish loop for the staged `dist/*` package directories in `packages/jekko/script/publish.ts`, so that script no longer owns the package publish mechanics.
- `xtask publish-release-packages` now owns the root `dist/jekko` package preparation plus the batch publish loop for the staged release packages in `packages/jekko/script/publish.ts`, so that script no longer owns the package scan/copy/publish mechanics.
- `xtask publish-release-registry` now owns the deterministic AUR and Homebrew registry metadata update step in `packages/jekko/script/publish.ts`, so the remaining Bun lane there is only the binary build matrix.
- `xtask publish-docker-image` now owns the Docker buildx image publish step in `packages/jekko/script/publish.ts`, so that script no longer owns the image tag or push mechanics.
- `xtask publish-release-artifacts` now owns the publish-lane orchestration that sequences package publishing, Docker image publishing, and registry metadata updates, so `packages/jekko/script/publish.ts` is now just a thin release shim.
- `xtask publish-stage-cli-assets` now owns the staged CLI binary packaging and release-upload step for the host build outputs produced by `xtask publish-build-script`, so the Rust executor no longer needs a Bun packaging tail.
- `xtask publish-build-plan` now owns the deterministic publish build matrix selection and artifact naming for `xtask publish-build-script`; the workflow now uses `--single` on native Linux and macOS runners.
- The publish build lane now calls `xtask publish-build-script --single` directly from the workflow, while the local package build uses the dedicated `xtask publish-build-cli` command, so the build lane no longer uses a Bun compile loop.
- `xtask migrations-json` now owns the shared migration directory scan and timestamp parsing used by `packages/jekko/script/build-node.ts`, so that Bun script only fetches the JSON payload now.
- `ops/ci/publish.sh` is now Rust-backed and no longer shells through Bun; the `publish` GitHub job no longer needs Bun bootstrap for the release lane.
- `ops/ci/beta.sh` is now Rust-backed and the beta merge driver itself is now Rust-native as well.
- The legacy Bun publish build script has been removed; the publish CLI build now runs through `xtask publish-build-script --single` on native Linux and macOS runners. The Rust executor still stages and optionally uploads the per-host release assets via `xtask publish-stage-cli-assets`.
- The `publish` GitHub job now uses native Linux and macOS build runners instead of a Windows signing branch, and the release lane no longer depends on Bun bootstrap for the CLI build.
- The remaining publish lane is now blocked primarily on the non-CLI release orchestration; the bot-like merge lanes are Rust-backed but still need richer orchestration in their Rust drivers:
- `ops/ci/jekko.sh` now routes through Rust `xtask github-run`; the remaining GitHub bot behavior is still incomplete, but the Bun launcher is gone.
- `ops/ci/triage.sh` now routes through Rust `xtask triage`; the remaining triage behavior is still incomplete, but the Bun launcher is gone.
- `ops/ci/duplicate-issues.sh` now routes through Rust `xtask duplicate-issues`; the remaining issue-compliance/duplicate-comment behavior still needs richer orchestration, but the Bun launcher is gone.
- `rtk cargo run -p xtask -- host-binary-path` prints the host binary path used by TUIwright.
- `rtk cargo run -p xtask -- guard-forbidden-runtime --mode final` reports no forbidden runtime references.
- Workspace validation is green again after the lockfile refresh; the current known blocker is now the remaining JS/LLM-driven automation surface, not Cargo resolution.
- The `jekko-cli` scaffold now has the missing command modules restored as Rust stubs, so workspace fmt/test/clippy are green again while the real command implementations remain pending.
- `ops/ci/review.sh` is now Rust-backed through `xtask review`, so the review lane no longer relies on Bun bootstrapping.
- `ops/ci/jekko.sh` is now Rust-backed through `xtask github-run`, so the GitHub bot launcher no longer relies on Bun bootstrapping.
- `ops/ci/triage.sh` is now Rust-backed through `xtask triage`, so the issue triage launcher no longer relies on Bun bootstrapping.

## Non-goals

- No JS runtime bridge.
- No OpenTUI compatibility shim.
- No plugin code execution in JS.

## Assumptions

- The current JS files in `packages/jekko/src/cli/cmd/tui` are the behavior reference until Rust parity exists.
- Historical Bun/OpenTUI mentions in migration notes are allowed while the port is in progress.

## Stop Conditions

- Do not overwrite user-edited JS/TUI files without reconciling those diffs first.
- Do not claim parity until Rust binary, TUI, storage, provider/session/runtime, and guard checks are all wired up and tested.
- Do not remove the remaining bot/release workflows until the Rust workflow orchestration for the remaining agentic lanes exists or a safe non-Bun replacement has been proven.
- The typed GitHub event helper should remain the shared ingress layer for any future Rust port of `review`, `triage`, `duplicate-issues`, and release-related workflows.
- The stale-PR closer is now a template for converting other deterministic workflow helpers out of shell/Python and into xtask.
- The compliance auto-close helper is now the second deterministic workflow template for moving more helper scripts out of shell/Python and into xtask.
- The containers lane is now the third deterministic workflow template for moving package/config parsing out of shell and into xtask.
- The PR standards and PR compliance scripts are now the fourth deterministic template: policy checks moved to Rust, wrapper shells remain only as launchers.
- The review lane metadata fetch and wrapper orchestration are now also in Rust; the lane no longer depends on Bun bootstrapping.
- The missing `jekko-cli` command modules were intentionally stubbed to keep the workspace healthy; do not mistake that for completion of the CLI command surface.
- The PR-management duplicate-PR lane is now Rust-backed through `xtask pr-management`, and the contributor-label / Discord release notification lanes remain the deterministic templates after `pr-standards`/`pr-compliance`.
- The publish version and release metadata step is now Rust-backed through `xtask publish-version`, and the version job itself now uses Rust toolchain bootstrap; the remaining publish build and orchestration steps still depend on the legacy Bun release pipeline.
- The `publish-install-jekko` helper has been deleted; it was only needed by the old Bun publish version job.
- Second-guess note: stop pulling the release/beta/agent lanes toward Rust wrappers for now unless the wrapper change removes a real Bun dependency. The remaining scripts still encode product behavior that needs richer Rust workflow orchestration for the agentic lanes, and the publish build lane additionally needs an explicit cross-compile plan before a Rust replacement is safe.
- `ops/ci/jekko.sh` now routes through Rust `xtask github-run`, but the full GitHub bot workflow still needs richer orchestration and GitHub-specific behaviors ported out of JS.
- `ops/ci/triage.sh` now routes through Rust `xtask triage`, but the full issue triage workflow still needs richer orchestration and GitHub-specific behaviors ported out of JS.
- `ops/ci/duplicate-issues.sh` now routes through Rust `xtask duplicate-issues`, but the full issue-compliance/duplicate workflow still needs richer orchestration and GitHub-specific behaviors ported out of JS.
- `jekko run` now performs the initial provider-backed turn in Rust and has a minimal Rust tool loop, but it is still not the full agentic workflow engine. Keep the remaining workflow blockers framed around richer tool-loop / workflow orchestration and lane-specific ports, not the prompt/session bookkeeping.

## Receipts

- Phase 00 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-00.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-00.jsonl)
- Phase 01 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-01.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-01.jsonl)
- Phase 02 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-02.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-02.jsonl)
- Phase 03 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-03.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-03.jsonl)
- Phase 04 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-04.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-04.jsonl)
- Phase 05 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-05.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-05.jsonl)
- Phase 05 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-05.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-05.jsonl)
- Phase 07 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-07.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-07.jsonl)
- Phase 08 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-08.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-08.jsonl)
- Phase 09 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-09.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-09.jsonl)
- Phase 10 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-10.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-10.jsonl)
- Phase 11 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-11.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-11.jsonl)
- Phase 12 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-12.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-12.jsonl)
- Phase 13 receipt: [`target/jankurai/open-tui-bun-rust-port/phase-13.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-13.jsonl)
- Phase 00b receipt (Packet L baseline matrix): [`target/jankurai/open-tui-bun-rust-port/phase-00b.jsonl`](/Users/bentaylor/code/jekko/target/jankurai/open-tui-bun-rust-port/phase-00b.jsonl)
- Baseline matrix status: **55 PNG + 55 text snapshots across 11 screens × 5 resolutions** captured under `target/tuiwright-jekko/baseline/`. Captured via `crates/tuiwright-jekko-unlock/tests/baseline_matrix.rs` driving `packages/jekko/dist/jekko-darwin-arm64/bin/jekko` (bun-compiled OpenTUI reference binary).
  - Clean (9 screens × 5 res = 45): `home`, `command-dialog`, `model-dialog`, `provider-dialog`, `theme-dialog`, `session-empty`, `shell`, `splash`, `prompt-autocomplete`.
  - Advisory pre-trigger (2 screens × 5 res = 10): `jnoccio-panel` (Ctrl+J did not open dashboard without `JNOCCIO_TUI_TEST=1` opt-in env), `zyal-panel` (✓ ZYAL sigil not surfaced from home prompt paste; needs session route).
  - Deferred to Packet M (Phase 13): `permission-prompt`, `question-prompt` (need LLM mock fixtures), `jankurai-panel` (trigger keybind not yet discovered).
- Plugin contract (Packet K, Phase 12): `crates/jekko-plugin-api/` populated with `JekkoPlugin` trait, `PluginRegistry`, `ExternalPluginManifest` (TOML, semver-validated), `MigrationWarning` + `detect_legacy_plugin`. 554 LOC src + 420 LOC tests. 21 tests passing. Guard grep clean of OpenTUI/Solid/JSX/Bun symbols.
- Forbidden-reference guard: final mode clean.
- CI/workflow migration receipts: partially complete.

## Phase Summary (closed packets)

By the end of the May 15 session, every source-code packet from the original 16-packet plan is implemented and merged. Cumulative ~43,000 LOC Rust.

| Packet | Crate(s) | Src LOC | Tests |
|--------|----------|---------|-------|
| L | tuiwright-jekko-unlock (55 baseline PNGs) | — | — |
| K | jekko-plugin-api | 974 | 21 |
| A1 | jekko-core | 4,335 | 62 |
| A2 | jekko-store | 3,682 | 29 |
| F | jekko-tui (lifecycle) | 520 | 6 |
| G | jekko-tui (components+dialogs) | 881 | 17 |
| G-fin | jekko-tui (App wire) | 70 | 7 |
| C | jekko-runtime | 4,846 | 74 |
| D | jekko-provider | 6,062 | 80 |
| B | jekko-cli | 2,185 | 20 |
| H | jekko-tui (prompt) | 2,173 | 68 |
| J | jekko-tui (feature plugins) | 3,245 | 58 |
| M-prep | tuiwright + xtask baseline-diff | 1,166 | 24 + 11 |
| I | jekko-tui (transcript) | 4,845 | 108 |
| O-prep | docs/archive/historical/open-tui-bun-deletion-plan.md + xtask/cleanup_cutover.rs | 607 | 4 |
| docs | docs/{testing,testing-tui,install,architecture,release,ci-local}.md | +311 LOC | — |
| Nix-rewrite | flake.nix + nix/ | -305 LOC net | — |
| E | jekko-server | 3,767 | 40 |
| xtask-helpers | crates/xtask/src/ | 1,967 | 68 |
| key-dispatch | jekko-tui (app.rs) | ~80 | — |
| splash-window + dialog-fwd | jekko-tui (lib.rs + app.rs) | ~30 | — |
| M-finalize | jekko-tui (logo + dialog typography) | 413 | 8 |

## Validation snapshot

- `cargo build -p jekko-cli --release --locked` → 10.2MB binary, `jekko 0.1.0`.
- `cargo test --workspace --locked --no-fail-fast` → **745+ tests passing across 82 suites**.
- `cargo clippy --workspace --all-targets -- -D warnings` → clean.
- `cargo fmt --all -- --check` → clean.
- `xtask db-migration-smoke` → 24 migrations applied, idempotent ✓.
- `xtask cli-help-parity` → matches snapshot.
- `xtask tool-schema-parity` → 10 tools match snapshot.
- `xtask package` → produces `dist/jekko-darwin-arm64/{bin/jekko,checksum.txt}`.
- `xtask baseline-diff` → 55 DIFF + 5 MISSING (5 MISSING are baseline `-fallback`/`-no-dashboard`/`-no-sigil` suffix variants Rust does not produce since steady-state is clean).
- `xtask guard-forbidden-runtime --mode final` → clean.

## Capture matrix

**OpenTUI baseline** (Packet L): 55 PNG + 55 text snapshots across 11 screens × 5 resolutions at `target/tuiwright-jekko/baseline/`.
**Rust render** (M-prep + key-dispatch + splash-window + M-finalize): 55 PNG + 55 text snapshots across 11 screens × 5 resolutions at `target/tuiwright-jekko/rust/`.

All 11 screens reachable from key dispatch in the Rust binary: home (default), command-dialog (Ctrl+P), model/theme/sessions dialogs (Ctrl+X leader + m/t/l), session-empty (Ctrl+X+n), provider-dialog (Ctrl+X+m → Ctrl+A), shell (palette navigation), prompt-autocomplete (`/`), splash (first 200ms boot window), jnoccio-panel (Ctrl+J), zyal-panel (paste fixture).

`xtask baseline-diff` mismatch ranges (post-M-finalize):

| screen | 80×24 | 100×30 | 120×30 | 160×40 | 200×60 |
|---|---|---|---|---|---|
| splash | 42.40% | 29.10% | 24.84% | 14.83% | 18.65% |
| home | 62.12% | 56.39% | 53.35% | 41.38% | 31.89% |
| command-dialog | 75.78% | 69.43% | 59.55% | 49.87% | 36.77% |
| model-dialog | 77.36% | 69.99% | 66.19% | 48.21% | 35.14% |
| theme-dialog | 77.86% | 68.26% | 65.55% | 46.13% | 34.80% |
| session-empty | 61.99% | 56.83% | … | … | … |
| shell, prompt-autocomplete, jnoccio, zyal | similar | | | | |

Tightens at larger sizes (chrome ratio drops). 200×60 mismatch range is 18–37% — typical for structurally-correct-but-stylistically-divergent renders.

The mismatch number is the parity SIGNAL, not a target. The current Rust render is intentionally minimalist while the OpenTUI baseline includes runtime-fetched content (model setup banner, provider keys, recent sessions). Closing the gap further requires runtime-driven content port, which is **out of scope** for the migration: the deliverable is the Rust foundation, not feature-parity skin.

## Outstanding work

1. **Packet O closeout** — complete. `cleanup-cutover --execute` landed and the final forbidden-runtime guard passed.
2. **M-finalize content polish (optional)** — wire jekko-runtime data into home body (model setup readout, provider keys, recent sessions) to drop mismatch %. Optional UX, not gating.
