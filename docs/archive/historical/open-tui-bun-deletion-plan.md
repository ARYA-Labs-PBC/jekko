# Packet O - Final JS Surface Deletion Plan

Generated: 2026-05-15T15:17:00Z
Status: inventory complete; Packet O cutover executed and final guard clean
Last verified against commit: 4c0247e6c0264b2c92e01a7fb2545111ad97ac13

This document is the staging plan for Packet O (Phase 15) - the cutover that
removes the live JS/Bun/OpenTUI/Solid surface from the repo once the Rust
port has reached parity. Doing the deletion before in-flight Packets (E, G,
H, I, J, K, L, M, N) have landed would break the build, so this packet only
inventories what to delete and what to keep.

Migration tracking lives in `docs/archive/historical/open-tui-bun-rust-port.md`; the original
behavior inventory lives in `docs/archive/historical/open-tui-bun-inventory.md`.

## Files to delete (one-line each)

### Root manifests, config, and lockfiles

- bun.lock (root workspace lockfile, 304 KB)
- bunfig.toml (root Bun bootstrap config)
- package.json (root workspace declaration, Bun-pinned)
- package-lock.json (vestigial 67 B stub)
- turbo.json (Turborepo task graph)
- tsconfig.json (root TS compiler config)
- .oxlintrc.json (TS lint rules)
- .prettierignore (TS formatter ignores)
- check-zyal.mjs (root-level Node script)

### Husky and JS hook bootstrap

- .husky/pre-push (verifies Bun version, runs Bun typecheck)
- .husky/check-encrypted-paths (chained from JS hook surface)
- .husky/_/ (husky internal scripts)

### packages/ JavaScript trees

- packages/jekko/ (entire dir, primary TS/Solid/OpenTUI surface)
- packages/core/ (entire dir, shared TS effect/core)
- packages/plugin/ (entire dir, plugin TS contract)
- packages/script/ (entire dir, script TS surface)
- packages/sdk/js/ (JS SDK; keep packages/sdk/ for non-JS SDK material if any remains)
- packages/slack/ (entire dir, Slack TS integration)
- packages/ux-qa/ (entire dir, UX-QA harness)
- packages/containers/ (TS containers surface)
- packages/enterprise/ (TS enterprise surface)
- packages/function/ (TS function surface)

### Build, dist, and cache artifacts

- packages/jekko/dist/ (Bun-compiled binary distribution, ~98 MB)
- packages/jekko/.turbo/ (Turborepo cache)
- packages/jekko/node_modules/ (workspace symlink tree)
- packages/jekko/.18*.bun-build (88 bun-build snapshots, ~5.3 GB combined)
- packages/jekko/.jekko/ (per-package jekko cache)
- packages/core/.turbo/
- packages/plugin/dist/
- packages/plugin/.turbo/
- packages/script/.turbo/ (if present)
- packages/ux-qa/dist/
- packages/sdk/js/.turbo/
- node_modules/ (root Bun-installed dependency tree, ~1.7 GB)
- .turbo/ (root Turborepo cache, ~5 GB)
- .jekko/ (root per-repo jekko TS cache)
- .agents/ (agent JS state, if Bun-bound)

### Root-level JS scripts (script/)

- ~~script/beta.ts~~ (deleted; xtask beta is Rust-native)
- script/changelog.ts
- script/duplicate-pr.ts
- script/format.ts
- script/generate.ts
- script/memory-benchmark-seed-commit.ts
- script/publish.ts
- script/raw-changelog.ts
- script/stats.ts
- script/sync-zed.ts
- script/version.ts
- script/github/close-issues.ts
- script/sign-windows.ps1 (cross-platform release helper, port to xtask)
- script/release (release entrypoint shell, port to xtask)
- script/record-readme-demo.sh (TUI demo recorder, port to Rust harness)
- script/hooks (JS hook bootstrap)

### Root-level mjs scripts (scripts/)

- scripts/jankurai-dispatch-classifier.mjs
- scripts/persist-concept.mjs
- scripts/regression-sentinel.mjs
- scripts/ci-local.sh (kept by shell, but JS deps removed when packages/ goes)
- scripts/ci-doctor.sh (kept by shell, but JS deps removed when packages/ goes)

### tools/ JS scripts

- tools/jankurai-audit-gate.mjs

### Patch files for JS deps

- patches/@npmcli%2Fagent@4.0.0.patch
- patches/@standard-community%2Fstandard-openapi@0.2.9.patch

### Nix files to replace

- nix/node_modules.nix -> replace with Rust-only derivation (currently encodes Bun lock normalization)
- nix/jekko.nix -> rewrite for `cargo build -p jekko-cli` (currently invokes Bun build)
- nix/hashes.json -> regenerate against Rust-only inputs
- nix/scripts/ (audit; keep helpers, drop Bun-specific entries)
- flake.nix -> drop Bun, keep Rust toolchain
- flake.lock -> regenerate without Bun inputs

### GitHub workflows still on Bun (delete or convert)

These workflows still call `setup-bun` and `bun install`. They block Bun
removal until each is either converted to a Rust/xtask pathway or has an
explicit gating comment.

- .github/workflows/beta.yml (release beta lane)
- .github/workflows/duplicate-issues.yml (LLM agent lane)
- .github/workflows/jekko.yml (jekko run github run entry)
- .github/workflows/pr-management.yml (LLM PR management lane)
- .github/workflows/publish.yml (full release pipeline)
- .github/workflows/review.yml (LLM PR review lane)
- .github/workflows/triage.yml (LLM triage lane)
- .github/publish-python-sdk.yml (placeholder; still references Bun bootstrap)

### GitHub composite actions

- .github/actions/setup-bun/ (entire dir; canonical Bun bootstrap action)

### ops/ci scripts still on Bun (delete or convert)

- ops/ci/beta.sh (release beta)
- ops/ci/duplicate-issues.sh (LLM agent)
- ops/ci/jekko.sh (jekko run github run; KEEP exception until Rust agent lands - tagged `# TODO(O): keep until Rust agent landed`)
- ops/ci/pr-management.sh (LLM PR management)
- ops/ci/publish-build-cli.sh (Bun build of jekko-cli binary)
- ops/ci/publish-install-jekko.sh (Bun install step)
- ops/ci/publish-version.sh (Bun-driven version stamp)
- ops/ci/publish.sh (release orchestration)
- ops/ci/review.sh (LLM PR review)
- ops/ci/triage.sh (LLM triage)
- ops/ci/sandbox-backends.sh (audit for Bun usage; keep if pure shell)

### Justfile recipes

- Justfile (audit and rewrite: drop Bun-driven recipes after their workflow equivalents move)
- ops/ci/lib.sh (audit; remove Bun helper functions when no caller remains)

### Mini fleet smoke harness

- mini-fleet-smoke/ (entire dir, currently TS-based smoke harness)
- mini-fleet-smoke/package.json

### Internal Bun migration plan note

- packages/jekko/BUN_SHELL_MIGRATION_PLAN.md (planning artifact for now-completed shell migration)

### Docs to clean up

The following docs reference Bun/OpenTUI/Solid in non-historical ways and
need rewriting once the runtime is removed. Move historical-only content
into an `archive/` subdir rather than deleting outright.

- docs/archive/historical/open-tui-bun-inventory.md (historical, archived)
- docs/testing-tui.md (rewrite for Ratatui/Crossterm + TUIwright Rust harness)
- docs/ci-local.md (drop Bun smoke lanes, point to xtask equivalents)
- docs/install.md (drop Bun bootstrap, point to `cargo install` / nix flake)
- docs/testing.md (drop Bun receipts, point to cargo test + xtask proof lanes)
- docs/architecture.md (drop Bun/OpenTUI runtime mentions)
- docs/boundaries.md (drop JS generated-zone callouts)
- docs/ZYAL/SPEC.md (rewrite Bun examples)
- docs/ZYAL/CHANGELOG.md (mark Bun migration entries historical)
- docs/ZYAL/sandbox-loops.md (drop Bun loop examples)

### Top-level docs

- ZYAL_WORKFLOW.md (audit for Bun references)
- ZYAL_MISSION.md (audit for Bun references)
- UNLOCK_WORKPLAN.md (audit; references current Bun TUI baseline)
- SANDBOX_WORKPLAN.md (audit; references Bun sandbox harness)
- BABYSIT_WORK.md (audit for Bun mentions)
- MEMORY_SYSTEM_LEVELUP.md (audit for Bun mentions)
- README.md (rewrite install/build sections to Rust-only)
- CONTRIBUTING.md (drop Bun bootstrap, point to cargo)
- CHANGELOG.md (append cutover entry)

### Specs files

- packages/jekko/specs/tui-plugins.md (move into Rust plugin spec)
- packages/jekko/specs/effect/* (drop entire Effect framework specs)
- packages/jekko/specs/v2/* (audit; v2 message shapes belong in jekko-core docs)

## Files to KEEP (despite Bun/OpenTUI mention)

These have historical migration mentions only - they MUST NOT be deleted by
the cutover. The cleanup-cutover plan flags them explicitly so the executor
will skip them.

- `docs/archive/historical/open-tui-bun-rust-port.md` (the migration tracking doc; close out and archive after cutover)
- `docs/archive/historical/open-tui-bun-deletion-plan.md` (this file; archive after cutover)
- `tips/goodbye_OpenTUIBun/` (the original handoff plan - archive material)
- migration receipts under `target/jankurai/open-tui-bun-rust-port/`
- `achat.md` references (coord chat is sacrosanct)
- `agent/JANKURAI_STANDARD.md` mentions of Bun (process-level guidance, not code)
- `agent/MASTER_PLAN.md` mentions (phase planning doc)
- `tips/phases/` history (phase logs, historical only)
- `crates/tuiwright-jekko-unlock/baseline/` PNG/text snapshots (kept as the reference baseline; only retire after Ratatui parity is signed off)

## Pre-flight checks (must all pass before cutover)

- [ ] All UI packets done: G wired, H wired, I wired, J wired into `crates/jekko-tui/src/lib.rs`
- [ ] All server packets done: E wired into `crates/jekko-server/`
- [ ] Workspace clean:
      `cargo fmt --all -- --check`
      `cargo clippy --workspace --all-targets --all-features -- -D warnings`
      `cargo test --workspace --locked --no-fail-fast`
- [ ] Release build: `cargo build -p jekko-cli --release --locked` -> produces working binary
- [ ] TUIwright Rust capture: `JEKKO_BIN=$(cargo run -p xtask -- host-binary-path) cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test rust_baseline_matrix` -> green
- [ ] Baseline diff: `cargo run -p xtask -- baseline-diff --threshold 5` -> green (Rust capture within 5% of the frozen Bun baseline)
- [ ] Codex's N-cont. workflows: `review`, `triage`, `duplicate-issues`, `pr-management`, `beta`, `publish` plus `ops/ci/jekko.sh` are either converted to Rust or carry an explicit `# TODO(O): keep until Rust agent landed` comment
- [ ] DB migration parity: `cargo run -p xtask -- db-migration-smoke` -> green
- [ ] HTTPAPI parity: `cargo run -p xtask -- httpapi-parity` -> green (Rust httpapi matches the prior Hono surface)
- [ ] OpenAPI snapshot: `cargo run -p xtask -- openapi-check` -> green
- [ ] Plugin contract spec: `crates/jekko-plugin-api` covers every public surface previously exported from `packages/plugin/src/tui.ts`
- [ ] Forbidden-runtime guard runs in advisory mode with zero JS-only hits remaining:
      `cargo run -p xtask -- guard-forbidden-runtime --mode advisory` reports `0 references`

## Cutover commands (run in order)

```bash
# 1. Snapshot the JS tree for git history
git tag pre-js-cutover

# 2. Preview, then execute
cargo run -p xtask -- cleanup-cutover --dry-run
cargo run -p xtask -- cleanup-cutover --execute

# 3. Update agent maps
# (edit agent/test-map.json, agent/owner-map.json, etc. - manual)

# 4. Update README + CHANGELOG
# (manual)

# 5. Final guard
cargo run -p xtask -- guard-forbidden-runtime --mode final

# 6. Workspace must still be green
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --locked --no-fail-fast
cargo build -p jekko-cli --release --locked
```

## Post-cutover audit

Run from a clean checkout of `main` after the cutover lands:

- `find packages -type f | wc -l` -> 0
- `find . -maxdepth 2 -name 'package.json' -not -path './target/*'` -> empty
- `find . -name 'bunfig.toml' -not -path './target/*'` -> empty
- `find . -name 'bun.lock' -not -path './target/*'` -> empty
- `grep -r 'setup-bun' .github/` -> empty
- `cargo run -p xtask -- guard-forbidden-runtime --mode final` -> exits 0

## Estimated post-cutover state

- packages/ disk size: ~98 MB (dist) + ~1.7 GB (node_modules) -> ~0 KB
- workflow count: 23 -> ~16 (drop 7 Bun-dependent lanes)
- guard-forbidden-runtime: advisory mode -> final mode (zero hits expected)
- ops/ci script count: 37 -> ~27 (drop 10 Bun-dependent scripts, keep the
  Rust-backed remainder)
- Top-level manifest count: 9 JS-related files -> 0
- Bun-build cache size: ~5.3 GB -> 0

## Counts (gathered during inventory)

- Total JS/TS files in `packages/`: 1393 (.ts + .tsx + .js + .mjs + .cjs + .json)
- Total `.ts` files in `packages/`: 1177
- Total `.tsx` files in `packages/`: 151
- Total LOC in `packages/` `.ts` files: 231,045
- Total LOC in `packages/` `.tsx` files: 25,585
- Combined TS/TSX LOC removed: ~256,630
- Root `bun.lock` size: 304 KB
- Root `package.json` size: 3.3 KB
- Root `bunfig.toml` size: 70 B
- Bun-build snapshots under `packages/jekko/.18*.bun-build`: 88 files
- `packages/jekko/dist/` size: ~98 MB
- Root `node_modules/` size: ~1.7 GB
- `.turbo/` root cache size: ~5 GB across `tar.zst` tarballs
- GitHub workflows referencing Bun setup: 7
- ops/ci scripts referencing Bun directly: 10
- GitHub composite action with Bun bootstrap: 1 (`.github/actions/setup-bun/`)
- Husky JS hooks: 2 (`pre-push`, `check-encrypted-paths`)
- Root `.ts` scripts under `script/`: 12 (plus 1 `.ps1`, 1 shell helper)
- Root `.mjs` scripts under `scripts/`: 3
- Docs files referencing Bun/OpenTUI: 6 in `docs/` + 3 in `docs/ZYAL/` = 9
- Patches for JS deps: 2

## Open exceptions (must be resolved before final mode)

- `ops/ci/jekko.sh` is intentionally kept on Bun until the Rust agent
  runtime lands. Codex's tracking note: this is a known exception. The
  cleanup-cutover module flags this file as `edit_files` (insert
  `# TODO(O): keep until Rust agent landed`) rather than deleting it.
- `.github/workflows/jekko.yml` follows the same gating rule and is held
  in `edit_files` for the same reason.
- The release/beta/publish lanes (`ops/ci/beta.sh`, `ops/ci/publish*.sh`,
  `ops/ci/pr-management.sh`, `ops/ci/review.sh`, `ops/ci/triage.sh`,
  `ops/ci/duplicate-issues.sh`) are held in `edit_files` until their
  Rust replacements land. Each one carries the `# TODO(O): ...` comment
  rather than being deleted in this packet.
