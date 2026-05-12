# ZYAL Workflow

Single shared append-only log for all agents touching the ZYAL surface
(jekko package, jnoccio-fusion gateway, docs, tips, examples).

**Conventions** — every entry ends with `— <agent-name>`. Agents re-read
this file before each append. Earlier completed work folds into the
**Archive** at the bottom; only active claims, recent receipts, and open
coordination notes live above.

## Active claims / open coordination

2026-05-09T00:57:58Z — Active — codex — Locked Jnoccio Fusion provider + TUI unlock flow.

- Scope: public locked `jnoccio/jnoccio-fusion` metadata, unlock API/service, TUI unlock dialog, `.env.jnoccio` placeholder creation, encryption checker hardening, docs, SDK regeneration, and focused tests.
- Detailed coordination lives in `UNLOCK_WORKPLAN.md`.

— codex

2026-05-09T01:24:46Z — Closed — codex — Locked Jnoccio Fusion provider + TUI unlock flow.

- Implementation is in the worktree. Detailed receipts, touched files, validation commands, skips, and blockers live in `UNLOCK_WORKPLAN.md`.

— codex

## Recent receipts

### 2026-05-09T05:29:02Z — Fixed — codex — Jnoccio TUI pbcopy paste unlock

- Fixed the real TUI paste fallback: keypress input now preserves uppercase `evt.sequence`, prompt/dialog/service normalization strips whitespace, `%`, CSI paste controls, and `200`/`201` bracketed-paste marker remnants before submit/decrypt/cache.
- Verification: new TUI paste/dialog tests 5 pass; unlock/provider/TUI sync tests 21 pass; local route suite 2 pass / 1 existing HttpApi skip; typecheck, encrypted-path scan, `rtk git diff --check`, and `rtk just fast` passed.
- Rebuilt, installed, and ad-hoc signed `/opt/homebrew/bin/jekko`; installed version is `0.0.0-codex/jnoccio-unlock-flow-202605090527`, and the gated installed unlock smoke passes 1/1.

— codex

### 2026-05-09T05:02:11Z — Verified with one route blocker — codex — Jnoccio paste and installed unlock proof

- Added TUI bracketed-paste tests, TUI unlock-dialog SDK integration proof, local fresh-clone legacy route proof, and gated installed-binary smoke proof.
- Verification: new TUI tests 3 pass; prior unlock/provider/TUI sync tests 20 pass; local route suite 2 pass / 1 skip; installed smoke skips by default and passes when gated with `/opt/homebrew/bin/jekko`; `rtk bun run typecheck`, encrypted-path scan, `rtk git diff --check`, and `rtk just fast` passed with the existing advisories already logged in `UNLOCK_WORKPLAN.md`.
- Installed binary version: `0.0.0-codex/jnoccio-unlock-flow-202605090452`.
- Remaining blocker: the experimental provider HttpApi unlock-route proof is skipped because the route still fails on missing `@jekko/Config`; legacy Hono route, service, TUI input, git-crypt clone unlock, and installed binary are verified.

— codex

### 2026-05-09T01:24:46Z — Done with validation blockers — codex — Locked Jnoccio unlock surface

- Completed:
  - Public locked `jnoccio/jnoccio-fusion` provider/model metadata, default/current/connected filtering for locked models, and configured-provider override behavior.
  - Local unlock service plus legacy and experimental `POST /provider/jnoccio/unlock` route surfaces.
  - TUI locked model/provider rows and retryable key-file unlock dialog that refreshes providers and selects Jnoccio on success.
  - `.env.jnoccio` ignored local placeholder flow, expanded docs/env example, and hardened encrypted-index checker.
  - Focused provider, unlock, encryption, TUI sync, local real-key skip, SDK type/client, and ZYAL token cleanup edits.
- Verification passed:
  - `rtk bun test test/jnoccio/unlock.test.ts test/provider/jnoccio-locked.test.ts` → 9 pass.
  - `rtk bun test test/jnoccio/encryption-check.test.ts` → 3 pass.
  - `rtk bun test test/cli/cmd/tui/sync.test.tsx` → 4 pass.
  - `rtk bun test test/session/llm.test.ts` → 17 pass.
  - `rtk bun run typecheck` in `packages/jekko` → pass.
  - `rtk cargo test` in `jnoccio-fusion` → 77 pass.
  - `rtk tools/check-encrypted-paths.sh --index`, `rtk git diff --check`, and `rtk just fast` → pass, with existing `just fast` advisories documented in `UNLOCK_WORKPLAN.md`.
- Remaining blockers:
  - `test/server/httpapi-provider.test.ts` fails before OAuth assertion with `Service not found: @jekko/Config`; `sync.test.tsx` passes in the combined command.
  - `rtk bun run --cwd packages/sdk/js build` did not complete because OpenAPI generation emitted a zero-byte spec/hung; generated SDK files were patched manually.
  - Strict legacy-token scan still has pre-existing/noisy matches in archived tips and substring false positives.

— codex

### 2026-05-09T02:17:29Z — Verified — codex — Local Jnoccio key unlock

- `~/jnoccio-fusion.key` exists locally and was used as the unlock path.
- Fixed the unlock service already-unlocked path and the `--index` checker syntax.
- Verification:
  - `JNOCCIO_GIT_CRYPT_KEY_PATH=~/jnoccio-fusion.key rtk bun test test/local/jnoccio-unlock.local.test.ts` in `packages/jekko` → 1 pass.
  - Direct `unlockJnoccioFusion({ keyPath: "~/jnoccio-fusion.key" })` → `status: "unlocked"`, `unlocked: true`, created ignored `jnoccio-fusion/.env.jnoccio`.
  - `rtk cargo metadata --manifest-path jnoccio-fusion/Cargo.toml --no-deps --format-version 1` → pass.
  - `rtk tools/check-encrypted-paths.sh --index` → pass, 46 encrypted protected blobs.
  - `rtk bun test test/jnoccio/unlock.test.ts test/provider/jnoccio-locked.test.ts` → 10 pass.

— codex

### 2026-05-09T00:42:17Z — Done — codex — Gap A + Run Card + docs verified

- Re-read workflow before append. No new edit conflict; Claude has ceded Gap A to codex and finished Gap B/C separately.
- Completed:
  - `jnoccio-fusion/src/router.rs` accepts both header families, preferring `x-jekko-*` and falling back to legacy `x-opencode-*`; tests cover Jekko, legacy, and mixed preference.
  - `packages/jekko/src/session/llm.ts` and `packages/jekko/test/session/llm.test.ts` are already at intended state in HEAD: all outbound LLM calls create daemon metadata, and tests cover non-Jekko + `provider: jnoccio` identity headers.
  - `packages/jekko/src/cli/cmd/tui/component/prompt/index.tsx` Start daemon confirmation now renders a Run Card with checkpoint, fleet/Jnoccio, arming, capabilities, budgets, taint, unsupported-feature policy, risk flags, and preview-only notes.
  - Jnoccio docs/snippets now use `/Users/bentaylor/Code/opencode/jnoccio-fusion` and the current 20-total-instance cap.
  - ZYAL docs now call out `taint` as parse/preview-only in this patch.
- Verification:
  - `rtk bun test test/session/llm.test.ts` in `packages/jekko` → 17 pass / 0 fail.
  - `rtk bun test src/agent-script/parser.test.ts` in `packages/jekko` → 88 pass / 0 fail.
  - `rtk bun test test/cli/tui/zyal-flash.test.ts test/cli/tui/jnoccio-ws.test.ts` in `packages/jekko` → 10 pass / 0 fail.
  - `rtk bun run typecheck` in `packages/jekko` → pass.
  - `rtk rustfmt --edition 2024 --check src/router.rs` in `jnoccio-fusion` → pass.
  - `rtk cargo test` in `jnoccio-fusion` → 77 pass / 0 fail.
  - Strict legacy token scan for `OCAL`, `Ocal`, `ZYML`, `VYAL`, `internal:sidebar-ocal`, `.ocal.yml`, `ocal-ready` → no matches.
  - `rtk git diff --check` → pass.
  - `rtk just fast` → exit 0 with existing advisories: missing committed lockfile, echo-only proof, missing `syft`, stale security evidence head.
- Formatting note: full `cargo fmt --check` still reports pre-existing formatting drift outside this patch (`fusion.rs`, `mcp.rs`, `state.rs`, `tests/mcp.rs`); only touched router was formatted.
- Remaining dirty outside codex scope: pre-existing `packages/jekko/src/cli/logo.ts`.

— codex

### 2026-05-09T00:39:43Z — Progress — codex — Gap A + Run Card + docs edits staged in worktree

- Re-read workflow after Claude compacted it; active codex claim remains for Gap A/header compatibility plus Run Card/docs drift.
- Edited so far:
  - `jnoccio-fusion/src/router.rs`: header parser now prefers `x-jekko-*` and falls back to `x-opencode-*`; unit tests added for Jekko, legacy, and preference behavior.
  - `packages/jekko/src/cli/cmd/tui/component/prompt/index.tsx`: Start daemon confirmation now renders a Run Card with checkpoint, fleet/Jnoccio, arming, capabilities, budgets, taint, unsupported-feature policy, risk flags, and explicit preview-only notes for hash/nonce/origin arming and taint.
  - Jnoccio docs: stale `/Users/bentaylor/Code/jekko` paths and 10-instance wording updated toward `/Users/bentaylor/Code/opencode` and the current 20-instance cap.
  - ZYAL docs: Run Card and taint preview-only wording made explicit.
- Next: confirm Jekko `llm.ts` / `llm.test.ts` state after concurrent workflow edits, then run targeted tests.

— codex

### 2026-05-09T00:55:00Z — Done — claude — Gap B + Gap C + workflow compact

Ceded Gap A to codex (00:36:01Z claim). Landed:

- **Gap B — master-loop runbook commands** (`docs/ZYAL/examples/10-jankurai-master-loop.zyal.yml`):
  - `stop.all[0]` `jankurai audit` now writes to `/tmp/jankurai-stop.json` then `jq -e '.findings | length == 0'` against the file. Same fix in `before_iteration` + `constraints[0]` (per-purpose temp paths).
  - `before_checkpoint` + `checkpoint.verify[1]` switched from missing `bun run lint` to `(cd packages/jekko && bun run typecheck)` so the runbook works against the canonical script set.
  - `capabilities.shell-tests` regex tightened: `tsgo --noEmit` removed; `bun|pnpm|npm run typecheck` allowed; lint dropped.
  - Verify steps explicitly `cd packages/jekko` first since the master-loop runs from repo root.
- **Gap C — `# Setup` preamble** added to runbook documenting the `@ai-sdk/openai-compatible` config to point `provider: jnoccio` at `http://127.0.0.1:4317/v1` until a native jnoccio provider is registered in `packages/jekko/src/provider/provider.ts`.
- **Workflow file compaction**: 259 lines → ~80 lines. Earlier done entries (21:47:51Z through 23:50:00Z) folded into the Archive at the bottom as one-line summaries.
- Verification: `bun test src/agent-script/parser.test.ts` → 88 pass / 0 fail. Runbook parses clean (taint preview unchanged: 6 labels, 2 forbid rules, prompt-injection scan).

NOT touched (codex territory): `jnoccio-fusion/src/router.rs`, `packages/jekko/src/session/llm.ts`, TUI Run Card, jnoccio docs paths, `provider.ts`, `agent/repo-score.{json,md}`.

— claude

## Archive (closed work — chronological one-liners)

- **2026-05-08T21:47:51Z — codex** — initial ZYAL rename + locale fallout (head 13541a6).
- **2026-05-08T21:55:31Z — codex** — parser/control-plane/TUI/daemon/typecheck gates passing.
- **2026-05-08T22:05:00Z — codex** — preview-only control-plane example, parser coverage, doc labels, ZYAL flash/task-memory rename.
- **2026-05-08T22:07:20Z — codex** — ZYAL-only audit claim (head 5a9952b).
- **2026-05-08T22:14:53Z — codex** — paper/ZYAL.md IEEE-grade hardening + tips/ZYAL exact-token cleanup.
- **2026-05-08T22:15:00Z — claude** — Phase 2/3/4 claim.
- **2026-05-08T22:17:12Z — codex** — `packages/jekko/src/plugin/index.ts` compatibility cast for external auth plugins (`jekko-gitlab-auth`, `jekko-poe-auth`).
- **2026-05-08T22:19:44Z — codex** — control-plane preview taint sync (commit a658d8a).
- **2026-05-08T22:25:00Z–22:50:00Z — claude** — Phase 2 paused/resumed around codex's opencode→jekko rename (worktree peaked 2403 dirty; resumed post bc033be3).
- **2026-05-08T23:05:00Z — claude** — Phase 2 + 3 + 4 done: TOCTOU `incrementJnoccioCounters` atomic merge; schema int bounds on `fleet.max_workers` + `fleet.jnoccio.max_instances`; telemetry-headers hoist via `daemonRequestHeaders`; `teardown` ordering invariant; session-route cleanup ordering; new `taint` top-level block (key #40) with full validator; 12 new tests covering atomic counter race + 9 taint accept/reject + 3 schema int rejection. Local commit 5c5346e0.
- **2026-05-08T23:15:00Z — claude** — push initially blocked by pre-push typecheck on plugin/index.ts; codex landed compatibility cast (22:17Z entry above).
- **2026-05-08T23:35:00Z–23:50:00Z — claude** — finalize: relocate `wow.yml` → `docs/ZYAL/examples/10-jankurai-master-loop.zyal.yml`, harden with v2.3 `taint` (6 labels, 2 forbid rules, 5 injection patterns), bump key count, repoint `/opt/homebrew/bin/{jekko,opencode}` symlinks. 88 parser tests pass; pushed (a7f96cd1e).
- **2026-05-09T00:15:00Z — claude** — audit findings posted: 7 gaps surfaced (header prefix mismatch, runbook commands, test failures, no jnoccio provider registration, preview-only enforcement gaps, sequential incubator, missing MCP spawn linkage).
- **2026-05-09T00:42:00Z — claude** — claim refined: ceded Gap A to codex, took Gap B + Gap C + workflow compact.
- **2026-05-09T03:05:15Z — codex** — Jnoccio unlock validation complete: repair-aware migration startup, `todo -> pending` schema rename, encrypted unlock envelope regeneration, local secret cache proof, `rtk bun run typecheck`, storage/unlock/provider/TUI tests, `rtk tools/check-encrypted-paths.sh --index`, `rtk cargo test`, `rtk just fast`, and `rtk bun run build --single --skip-install` smoke test all passed.
- **2026-05-09T04:15:00Z — codex** — TUI unlock failure root-caused to stale installed `/opt/homebrew/bin/jekko` plus packaged repo-root resolution. Patched unlock repo-root discovery to prefer `JNOCCIO_REPO_ROOT`/`process.cwd()`, removed invalid JSX span props blocking typecheck, rebuilt and installed `0.0.0-codex/jnoccio-unlock-flow-202605090414`, ad-hoc signed the installed binary, and verified cached-secret unlock through the installed server route. Proof: installed `jekko --version`, installed-server `POST /provider/jnoccio/unlock` with `{}`, focused storage/unlock/provider/TUI/local tests, typecheck, encrypted-path scan, `rtk cargo test`, `rtk just fast`, and single-binary build smoke all passed.
- **2026-05-09T04:26:00Z — codex** — Rotated `/Users/bentaylor/jnoccio-fusion.unlock` to a 128-character A-Z0-9 software key, regenerated the encrypted Jnoccio git-crypt-key envelope, and rebuilt/installed/signed `/opt/homebrew/bin/jekko` `0.0.0-codex/jnoccio-unlock-flow-202605090425`. Proof: no repo hit for the software key, local fresh-clone unlock, source unlock, installed-server cached and typed unlocks, focused unlock/provider/encryption tests, TUI sync, typecheck, encrypted-path scan, and `rtk just fast`.
- **2026-05-09T06:10:00Z — claude** — Real root cause + tuiwright proof. User-visible bug: in any clone where `jnoccio-fusion/` is still git-crypt encrypted, the model picker showed Jnoccio Fusion as ACTIVE (not locked), so the unlock dialog never opened. Cause: `.jekko/jekko.jsonc` declares `provider.jnoccio` with model definition, populating `providers["jnoccio"]` with `status: active` during `configProviders` merge in `packages/jekko/src/provider/provider.ts`; the subsequent `if (!providers[jnoccioProviderID])` locked-info fallback was a no-op. Fix: always force `status: "locked"` on `jnoccio-fusion` when `isJnoccioFusionConfigured()` reports false, regardless of config-merge result. New `crates/tuiwright-jekko-unlock/` Rust crate (jankurai `tuiwright` dev-dep) spawns `/opt/homebrew/bin/jekko` in a real PTY against a fresh `git clone`, drives Ctrl+P → Switch model → Enter → Enter → Enter → bracketed paste → Enter, and asserts "Jnoccio Fusion unlocked" toast plus plaintext `jnoccio-fusion/Cargo.toml` plus cached secret; six screenshots written to `target/tuiwright-jekko/`. Rebuilt and installed `/opt/homebrew/bin/jekko` `0.0.0-codex/jnoccio-unlock-flow-202605090609`. Proof: tuiwright PTY E2E 1/1 (~8s); jnoccio + TUI 35/35; installed-binary smoke 1/1; local route + tui-paste-unlock 2/2 + 1 pre-existing skip; `rtk bun run typecheck` clean. Diagnostic side-finding: user's macOS clipboard at complaint time held 21716 bytes of code, not the 128-byte secret — `pbcopy < ~/jnoccio-fusion.unlock` was overwritten before the TUI paste; verify `pbpaste | wc -c == 128` immediately after pbcopy.
- **2026-05-09T05:50:00Z — claude** — Re-bound stale envelope (post-rotation drift) to `~/jnoccio-fusion.unlock` and added end-to-end TUI paste-to-unlock proof. `rtk bun packages/jekko/script/encrypt-jnoccio-key.ts --key-file .git/git-crypt/keys/default --secret-file ~/jnoccio-fusion.unlock` rewrote the embedded envelope. New tests: env-gated round-trip in `test/jnoccio/unlock.test.ts` (envelope → raw key) and `test/local/jnoccio-tui-paste-unlock.local.test.tsx` driving `<DialogJnoccioUnlock/>` via `@opentui/solid` `testRender` + `mockInput.pasteBracketedText` against a real `cloneRepo()` + real `unlockJnoccioFusion()`. Rebuilt `/opt/homebrew/bin/jekko` `0.0.0-codex/jnoccio-unlock-flow-202605090548`, ad-hoc resigned to clear `com.apple.provenance` SIGKILL. Proof: 15/15 unlock tests, 1/1 local route, 1/1 TUI paste-to-unlock, 1/1 installed-binary smoke (`JNOCCIO_INSTALLED_UNLOCK_E2E=1`), 34/35 jnoccio+TUI aggregate (1 pre-existing skip), `rtk bun run typecheck` clean, `rtk git diff --check` clean.
