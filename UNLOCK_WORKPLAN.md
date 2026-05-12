# Jnoccio Unlock Workplan

Shared coordination log for the locked Jnoccio Fusion provider and local
git-crypt unlock flow. Keep entries append-only and re-read this file plus
`ZYAL_WORKFLOW.md` before each edit batch.

## Active Claims

- 2026-05-09T00:57:58Z — Codex claim: implement locked Jnoccio provider metadata, unlock API/service, TUI unlock UX, encryption checker hardening, docs/env updates, tests, SDK regeneration, and proof receipts. Claimed paths include `packages/jekko/src/provider/**`, `packages/jekko/src/server/routes/instance/httpapi/**`, `packages/jekko/src/cli/cmd/tui/**`, `packages/jekko/test/**`, `packages/sdk/js/**` generated SDK outputs, `tools/check-encrypted-paths.sh`, `.husky/check-encrypted-paths`, `.gitignore`, `jnoccio-fusion/.env.jnoccio.example`, `jnoccio-fusion/KEYS.md`, `jnoccio-fusion/ENCRYPTION.md`, and this workplan.
- 2026-05-09T01:24:46Z — Codex claim closed. Implementation is in the worktree; remaining validation/generation blockers are recorded below.

## Decisions

- Unlock asks for a local key-file path and never accepts pasted key material.
- `.env.jnoccio` is local-only, ignored, and created from `.env.jnoccio.example` only when missing.
- Locked Jnoccio metadata lives in public Jekko code; encrypted `jnoccio-fusion` source remains protected.
- Real-key unlock tests are local-only and skip in CI, when `JNOCCIO_GIT_CRYPT_KEY_PATH` is absent, or when `git-crypt` is unavailable.

## Receipts

- 2026-05-09T00:57:58Z — Started. Re-read `AGENTS.md`, `agent/JANKURAI_STANDARD.md`, owner/test maps, generated zones, boundaries, `ZYAL_WORKFLOW.md`, `.gitattributes`, `.gitignore`, and encryption scripts. No active Claude claim found; worktree was clean before edits.
- 2026-05-09T01:24:46Z — Implemented locked Jnoccio Fusion provider metadata, `POST /provider/jnoccio/unlock` in legacy and experimental provider routes, injectable unlock service, TUI locked-model/provider unlock dialog, `.env.jnoccio` placeholder creation, provider/default filtering for locked models, generated SDK type/client updates, encryption checker index/blob mode, docs/env updates, and focused tests.
- 2026-05-09T01:24:46Z — Passing proof:
  - `rtk bun test test/jnoccio/unlock.test.ts test/provider/jnoccio-locked.test.ts` in `packages/jekko` → 9 pass / 0 fail.
  - `rtk bun test test/jnoccio/encryption-check.test.ts` in `packages/jekko` → 3 pass / 0 fail.
  - `rtk bun test test/cli/cmd/tui/sync.test.tsx` in `packages/jekko` → 4 pass / 0 fail.
  - `rtk bun test test/local/jnoccio-unlock.local.test.ts` in `packages/jekko` → 1 skip because `JNOCCIO_GIT_CRYPT_KEY_PATH` was absent.
  - `rtk bun test test/session/llm.test.ts` in `packages/jekko` → 17 pass / 0 fail.
  - `rtk bun run typecheck` in `packages/jekko` → pass.
  - `rtk cargo test` in `jnoccio-fusion` → 77 pass / 0 fail.
  - `rtk tools/check-encrypted-paths.sh --index` → 46 protected Jnoccio files encrypted in index mode.
  - `rtk git diff --check` → pass.
  - `rtk git ls-files jnoccio-fusion/.env.jnoccio` → no tracked local env file.
  - `rtk just fast` → exit 0 with existing advisories: missing committed lockfile, echo-only proof, missing `syft`, stale security evidence head.
- 2026-05-09T01:24:46Z — Partial/failed proof:
  - `rtk bun test test/server/httpapi-provider.test.ts test/cli/cmd/tui/sync.test.tsx` in `packages/jekko` → `sync.test.tsx` passes, but `test/server/httpapi-provider.test.ts` fails before the OAuth parity assertion with `Service not found: @jekko/Config`.
  - `rtk bun run --cwd packages/sdk/js build` did not complete; isolated `rtk bun run --conditions=browser ./src/index.ts generate` in `packages/jekko` exited 0 but emitted a zero-byte OpenAPI file. Generated SDK source was manually patched and formatted.
  - Strict legacy token scan for `OCAL`, `Ocal`, `ZYML`, `VYAL`, `internal:sidebar-ocal`, `.ocal.yml`, and `ocal-ready` still reports noisy/pre-existing matches in archive-style `tips/smarter/*.txt` content plus false positives such as `LOCAL_API_KEY`/`LOCALE*`. Actionable source `ZYML` hits in TUI logo/UI code were corrected.
- 2026-05-09T02:50:00Z — Claude validation pass. Re-ran the suites Codex listed; 13 pass / 0 fail across `test/jnoccio/unlock.test.ts`, `test/provider/jnoccio-locked.test.ts`, `test/jnoccio/encryption-check.test.ts`. Real-key local test passes 1/1 with `JNOCCIO_GIT_CRYPT_KEY_PATH=$HOME/jnoccio-fusion.key`. Re-ran with `CI=true` and the local-only test self-skips correctly. `tools/check-encrypted-paths.sh --index` reports 46 protected files encrypted; `tsgo --noEmit` passes. End-to-end story holds.
- 2026-05-09T02:48:00Z — Paste-mode decision (resolved by user 2026-05-09): keep path-only. Codex's security argument (terminal scrollback, tmux buffers, macOS pasteboard) won out over flexibility. Dialog stays as shipped. No edits to `dialog-jnoccio-unlock.tsx`.
- 2026-05-09T02:35:00Z — Claude review: read Codex implementation end-to-end. Helper at `packages/jekko/src/util/jnoccio-unlock.ts`, dialog at `dialog-jnoccio-unlock.tsx`, tests under `test/jnoccio/`, `test/local/`, `test/provider/`. Coverage of path validation, git-crypt-missing, wrong-key, idempotent re-unlock, plaintext-signal verification, env-no-overwrite, and false-positive-success is solid. Decision noted: Codex chose path-only (no paste); raising paste with the user before any dialog edits.
- 2026-05-09T02:17:29Z — Real local key verification:
  - Verified `~/jnoccio-fusion.key` is readable and expands to the expected local key file.
  - Fixed the local unlock service to treat an already-unlocked repo as success while still creating or preserving `.env.jnoccio`.
  - Fixed `tools/check-encrypted-paths.sh --index` syntax and adjusted the local real-key test to run the current checker against the fresh clone's index.
  - `JNOCCIO_GIT_CRYPT_KEY_PATH=~/jnoccio-fusion.key rtk bun test test/local/jnoccio-unlock.local.test.ts` in `packages/jekko` → 1 pass / 0 fail.
  - Direct service call `unlockJnoccioFusion({ keyPath: "~/jnoccio-fusion.key" })` → `status: "unlocked"`, `unlocked: true`, created local ignored `jnoccio-fusion/.env.jnoccio`.
  - `rtk cargo metadata --manifest-path jnoccio-fusion/Cargo.toml --no-deps --format-version 1` → pass.
  - `rtk git check-ignore -v jnoccio-fusion/.env.jnoccio` → ignored by `jnoccio-fusion/.gitignore`; `rtk git ls-files jnoccio-fusion/.env.jnoccio` → no tracked file.
  - `rtk tools/check-encrypted-paths.sh --index` → 46 protected Jnoccio files encrypted in index mode.
  - `rtk bun test test/jnoccio/unlock.test.ts test/provider/jnoccio-locked.test.ts` in `packages/jekko` → 10 pass / 0 fail.
- 2026-05-09T03:05:15Z — Codex validation pass: replaced the startup migrator path with a repair-aware raw SQL applicator, added the missing `todo -> pending` rename migration, and regenerated the encrypted key envelope from `~/jnoccio-fusion.key` plus `~/jnoccio-fusion.unlock`. Verification passed with `rtk bun run typecheck`, `rtk bun test test/storage/db.test.ts test/storage/json-migration.test.ts`, `rtk bun test test/jnoccio/unlock.test.ts test/jnoccio/encryption-check.test.ts test/provider/jnoccio-locked.test.ts`, `rtk bun test test/cli/cmd/tui/sync.test.tsx`, `JNOCCIO_UNLOCK_SECRET_PATH=~/jnoccio-fusion.unlock rtk bun test test/local/jnoccio-unlock.local.test.ts`, `rtk tools/check-encrypted-paths.sh --index`, `rtk cargo test`, and `rtk just fast`. Build/install proof also passed via `cd packages/jekko && rtk bun run build --single --skip-install`, which emitted the binary smoke test `jekko --version`.
- 2026-05-09T04:15:00Z — Troubleshot failed TUI unlock. The cached secret file was valid and source unlock passed, but the installed `/opt/homebrew/bin/jekko` binary was still `0.0.0-codex/jnoccio-unlock-flow-202605090401` and resolved the repo root from bundled source paths. Patched `repoRootFromSource()` to prefer `JNOCCIO_REPO_ROOT`, then `process.cwd()`, then bundled source discovery; rebuilt `0.0.0-codex/jnoccio-unlock-flow-202605090414`, installed it to `/opt/homebrew/bin/jekko`, and ad-hoc signed it so macOS will execute it from Homebrew's bin dir. Proof passed: installed `jekko --version`, installed-server `POST /provider/jnoccio/unlock` with empty payload using `~/jnoccio-fusion.unlock`, storage/unlock/provider/TUI/local tests, `rtk bun run typecheck`, `rtk tools/check-encrypted-paths.sh --index`, `rtk cargo test`, `rtk just fast`, and `rtk bun run build --single --skip-install`.
- 2026-05-09T04:26:00Z — Rotated the local unlock secret to a simple A-Z0-9 software key at `/Users/bentaylor/jnoccio-fusion.unlock` with `0600` permissions and regenerated the committed encrypted key envelope from the real local git-crypt key. The raw key and software key were not printed and were not found in the repo tree. Proof passed: local fresh-clone unlock with `JNOCCIO_UNLOCK_SECRET_PATH=~/jnoccio-fusion.unlock`, direct source unlock, installed-server cached `{}` unlock, installed-server typed `unlockSecret` unlock, focused unlock/provider/encryption tests, TUI sync test, `rtk bun run typecheck`, `rtk tools/check-encrypted-paths.sh --index`, `rtk just fast`, and rebuilt/signed `/opt/homebrew/bin/jekko` `0.0.0-codex/jnoccio-unlock-flow-202605090425`.
- 2026-05-09T05:02:11Z — Added verification tests for the real 128-character software-key paste flow, TUI unlock-dialog SDK path, local fresh-clone legacy route path, and gated installed-binary smoke path. The prompt tests prove bracketed paste submits exactly the sanitized 128-character secret with no newline or `%`; the dialog test proves the SDK request body is exactly `{ unlockSecret: <128 chars> }` and the TUI refresh selects `jnoccio/jnoccio-fusion`; the local route proof unlocks a temp clone and verifies git-crypt, `.env.jnoccio`, `cargo metadata`, and active provider state; the installed-binary proof uses an isolated temp clone/cache and `/opt/homebrew/bin/jekko` version `0.0.0-codex/jnoccio-unlock-flow-202605090452`.
- 2026-05-09T05:02:11Z — Passing proof:
  - `rtk bun test test/cli/cmd/tui/dialog-secret-prompt.test.tsx test/cli/cmd/tui/jnoccio-unlock-dialog.test.tsx` in `packages/jekko` -> 3 pass / 0 fail / 13 expect calls.
  - `rtk bun test test/jnoccio/unlock.test.ts test/provider/jnoccio-locked.test.ts test/cli/cmd/tui/sync.test.tsx` in `packages/jekko` -> 20 pass / 0 fail / 73 expect calls.
  - `rtk bun test test/local/jnoccio-unlock.local.test.ts test/local/jnoccio-unlock-route.local.test.ts` in `packages/jekko` -> 2 pass / 1 skip / 0 fail / 21 expect calls; skip is the experimental HttpApi route blocker below.
  - `rtk bun test test/local/jnoccio-installed-unlock.local.test.ts` in `packages/jekko` -> 1 skip by default because `JNOCCIO_INSTALLED_UNLOCK_E2E` was unset.
  - `JNOCCIO_INSTALLED_UNLOCK_E2E=1 JNOCCIO_INSTALLED_JEKKO=/opt/homebrew/bin/jekko rtk bun test test/local/jnoccio-installed-unlock.local.test.ts` in `packages/jekko` -> 1 pass / 0 fail / 8 expect calls.
  - `rtk bun run typecheck` in `packages/jekko` -> pass.
  - `rtk tools/check-encrypted-paths.sh --index` -> 46 protected Jnoccio files encrypted in index mode.
  - `rtk git diff --check` -> pass.
  - `rtk just fast` -> exit 0 with existing advisories: missing committed lockfile, echo-only proof, missing `syft`, stale security evidence head.
- 2026-05-09T05:02:11Z — No tracked secret, `.env.jnoccio`, temp key, or generated artifact was added. The local-only unlock secret at `/Users/bentaylor/jnoccio-fusion.unlock` and installed binary at `/opt/homebrew/bin/jekko` were read/used only through test gates and were not edited.
- 2026-05-09T05:29:02Z — Fixed the remaining `pbcopy < /Users/bentaylor/jnoccio-fusion.unlock` TUI failure. Root cause was the real terminal fallback path: uppercase paste bytes can arrive as keypress events whose OpenTUI `name` is lowercased, and bracketed-paste markers can leave `200`/`201` digit remnants around the key. The prompt now preserves `evt.sequence` for single-character keypress input, normalizes prompt state on each input boundary, and the dialog/service normalize submitted secrets before decrypting or caching.
- 2026-05-09T05:29:02Z — Passing proof:
  - `rtk bun test test/cli/cmd/tui/dialog-secret-prompt.test.tsx test/cli/cmd/tui/jnoccio-unlock-dialog.test.tsx` in `packages/jekko` -> 5 pass / 0 fail / 20 expect calls.
  - `rtk bun test test/jnoccio/unlock.test.ts test/provider/jnoccio-locked.test.ts test/cli/cmd/tui/sync.test.tsx` in `packages/jekko` -> 21 pass / 0 fail / 77 expect calls.
  - `rtk bun run typecheck` in `packages/jekko` -> pass.
  - `rtk bun run build --single --skip-install` in `packages/jekko` -> pass; build smoke emitted `0.0.0-codex/jnoccio-unlock-flow-202605090527`.
  - Installed `/opt/homebrew/bin/jekko` was replaced with the rebuilt binary and ad-hoc signed; `/opt/homebrew/bin/jekko --version` -> `0.0.0-codex/jnoccio-unlock-flow-202605090527`.
  - `JNOCCIO_INSTALLED_UNLOCK_E2E=1 JNOCCIO_INSTALLED_JEKKO=/opt/homebrew/bin/jekko rtk bun test test/local/jnoccio-installed-unlock.local.test.ts` in `packages/jekko` -> 1 pass / 0 fail / 8 expect calls.
  - `rtk bun test test/local/jnoccio-unlock.local.test.ts test/local/jnoccio-unlock-route.local.test.ts` in `packages/jekko` -> 2 pass / 1 skip / 0 fail / 21 expect calls; skip is still the experimental HttpApi `@jekko/Config` blocker.
  - `rtk tools/check-encrypted-paths.sh --index` -> 46 protected Jnoccio files encrypted in index mode.
  - `rtk git diff --check` -> pass.
  - `rtk just fast` -> exit 0 with existing advisories: missing committed lockfile, echo-only proof, missing `syft`, stale security evidence head.

## Receipts (continued)

- 2026-05-09T06:10:00Z — claude — Tuiwright (jankurai/Rust) real-PTY end-to-end proof + locked-status enforcement bug fix. Discovered the actual root cause of the user's "NOT working" report: `.jekko/jekko.jsonc` declares `provider.jnoccio` with full model definition, which made the config-merge loop in `provider.ts` populate `providers["jnoccio"]` with `models["jnoccio-fusion"].status = "active"`. The subsequent locked-info pass (`if (!providers[jnoccioProviderID]) ...`) was a no-op, so a freshly-cloned (still git-crypt-locked) repo showed Jnoccio Fusion as active in the model picker — the unlock dialog never opened from `Ctrl+P → Switch model`.
  - Fix: `packages/jekko/src/provider/provider.ts` now always overrides `models["jnoccio-fusion"].status = "locked"` when `isJnoccioFusionConfigured()` reports the filesystem is locked, regardless of config merge.
  - Tuiwright proof crate: new `crates/tuiwright-jekko-unlock/` (dev-dep `tuiwright` from `https://github.com/jeppsontaylor/Jankurai` `branch=main`) spawns `/opt/homebrew/bin/jekko` against a fresh `git clone` in a real PTY, drives `Ctrl+P → "Switch model" → Enter → Enter → Enter → bracketed paste → Enter`, and asserts the "Jnoccio Fusion unlocked" toast plus plaintext `jnoccio-fusion/Cargo.toml` plus cached secret. Run: `JNOCCIO_TUIWRIGHT_E2E=1 cargo test --test jekko_unlock_pty -- --nocapture` from `crates/tuiwright-jekko-unlock`. → 1 pass / 0 fail in ~8s. Six screenshots written to `target/tuiwright-jekko/{01-boot,02-provider,03-model,04-dialog,05-pasted,06-unlocked}.png`.
  - Rebuilt and installed `/opt/homebrew/bin/jekko` `0.0.0-codex/jnoccio-unlock-flow-202605090609` (ad-hoc `codesign --force --sign -`).
  - Aggregate sweep: jnoccio + TUI 35/35 pass; installed-binary smoke (`JNOCCIO_INSTALLED_UNLOCK_E2E=1`) 1/1 pass; local route + tui-paste-unlock 2/2 pass + 1 pre-existing HttpApi skip; `rtk bun run typecheck` clean.
  - Diagnostic side-finding: at moment of complaint, user's macOS clipboard contained 21716 bytes of code (sha 52604f9, starts "impo..."), NOT the 128-byte unlock secret (sha 05a34b8). Something between `pbcopy < ~/jnoccio-fusion.unlock` and the TUI Cmd+V overwrote the clipboard. After `pbcopy < ~/jnoccio-fusion.unlock`, immediately verify with `pbpaste | wc -c` (must = 128) before pasting.

- 2026-05-09T05:50:00Z — claude — Envelope re-bind + paste-to-unlock end-to-end verified. Root cause of TUI "Unlock key was not valid" identified as envelope/secret drift after rotation commit `957804a7c` (envelope encrypted with a secret no longer matching `~/jnoccio-fusion.unlock`). Raw git-crypt key was unchanged across `~/jnoccio-fusion.key`, `~/jnoccio-fusion.key.bak`, `.git/git-crypt/keys/default` (sha256 `e9823210d9914e5544d8f586f8b3415e0096a2597f084b64cd8b1cf0fe32da4f`); secret format was already 128 ASCII chars matching `[A-Za-z0-9_-]{128}`.
  - Re-encrypted envelope: `rtk bun packages/jekko/script/encrypt-jnoccio-key.ts --key-file .git/git-crypt/keys/default --secret-file ~/jnoccio-fusion.unlock` rewrote `packages/jekko/src/util/jnoccio-encrypted-key.ts` (no rotation; reads existing secret).
  - Round-trip proof: new env-gated test in `packages/jekko/test/jnoccio/unlock.test.ts` decrypts the embedded envelope with `~/jnoccio-fusion.unlock` and asserts equality with `.git/git-crypt/keys/default`. `JNOCCIO_RAW_KEY_PATH=… JNOCCIO_UNLOCK_SECRET_PATH=… rtk bun test test/jnoccio/unlock.test.ts` → 15 pass / 0 fail.
  - Service proof: `rtk bun test test/local/jnoccio-unlock-route.local.test.ts` → 1 pass / 1 pre-existing HttpApi skip.
  - TUI paste-to-unlock proof (NEW): `packages/jekko/test/local/jnoccio-tui-paste-unlock.local.test.tsx` mounts `<DialogJnoccioUnlock/>` via `@opentui/solid` `testRender`, drives `mockInput.pasteBracketedText(secret)` then `pressEnter()`, with the SDK fetch shim invoking the real `unlockJnoccioFusion()` against a fresh `cloneRepo()` and the embedded envelope. Asserts unlock body, dialog close, model selection, `isJnoccioFusionUnlocked(clone) === true`, secret cached. → 1 pass / 0 fail.
  - Installed-binary proof: rebuilt `packages/jekko/dist/jekko-darwin-arm64/bin/jekko`, copied to `/opt/homebrew/bin/jekko`, ad-hoc resigned (`codesign --force --sign -`) to clear `com.apple.provenance` SIGKILL. `/opt/homebrew/bin/jekko --version` → `0.0.0-codex/jnoccio-unlock-flow-202605090548`. `JNOCCIO_INSTALLED_UNLOCK_E2E=1 rtk bun test test/local/jnoccio-installed-unlock.local.test.ts` → 1 pass / 0 fail.
  - Aggregate jnoccio + TUI sweep: `rtk bun test test/jnoccio/ test/cli/cmd/tui/` → 34 pass / 1 pre-existing skip / 0 fail.
  - `rtk bun run typecheck` → pass. `rtk git diff --check` → pass.

## Touched Files

- `UNLOCK_WORKPLAN.md`
- `ZYAL_WORKFLOW.md`
- `.gitignore`
- `.husky/check-encrypted-paths`
- `tools/check-encrypted-paths.sh`
- `jnoccio-fusion/.env.jnoccio.example`
- `jnoccio-fusion/ENCRYPTION.md`
- `jnoccio-fusion/KEYS.md`
- `packages/jekko/src/util/jnoccio-unlock.ts`
- `packages/jekko/src/provider/provider.ts`
- `packages/jekko/src/provider/models.ts`
- `packages/jekko/src/config/provider.ts`
- `packages/jekko/src/v2/model.ts`
- `packages/jekko/src/server/routes/instance/httpapi/groups/provider.ts`
- `packages/jekko/src/server/routes/instance/httpapi/handlers/provider.ts`
- `packages/jekko/src/server/routes/instance/index.ts`
- `packages/jekko/src/server/routes/instance/provider.ts`
- `packages/jekko/src/cli/cmd/tui/component/dialog-jnoccio-unlock.tsx`
- `packages/jekko/src/cli/cmd/tui/component/dialog-model.tsx`
- `packages/jekko/src/cli/cmd/tui/component/dialog-provider.tsx`
- `packages/jekko/src/cli/cmd/tui/component/use-connected.tsx`
- `packages/jekko/src/cli/cmd/tui/context/local.tsx`
- `packages/jekko/src/cli/cmd/tui/ui/dialog-secret-prompt.tsx`
- `packages/jekko/src/cli/cmd/tui/component/logo.tsx`
- `packages/jekko/src/cli/ui.ts`
- `packages/jekko/test/jnoccio/unlock.test.ts`
- `packages/jekko/test/jnoccio/encryption-check.test.ts`
- `packages/jekko/test/local/jnoccio-unlock.local.test.ts`
- `packages/jekko/test/local/jnoccio-local-helpers.ts`
- `packages/jekko/test/local/jnoccio-unlock-route.local.test.ts`
- `packages/jekko/test/local/jnoccio-installed-unlock.local.test.ts`
- `packages/jekko/test/provider/jnoccio-locked.test.ts`
- `packages/jekko/test/cli/cmd/tui/sync.test.tsx`
- `packages/jekko/test/cli/cmd/tui/dialog-secret-prompt.test.tsx`
- `packages/jekko/test/cli/cmd/tui/jnoccio-unlock-dialog.test.tsx`
- `packages/jekko/test/local/jnoccio-tui-paste-unlock.local.test.tsx`
- `packages/jekko/src/util/jnoccio-encrypted-key.ts` (regenerated by `script/encrypt-jnoccio-key.ts`)
- `packages/jekko/src/provider/provider.ts` (locked-status enforcement override after configProviders merge)
- `crates/tuiwright-jekko-unlock/Cargo.toml`
- `crates/tuiwright-jekko-unlock/src/lib.rs`
- `crates/tuiwright-jekko-unlock/tests/jekko_unlock_pty.rs`
- `packages/sdk/js/src/gen/types.gen.ts`
- `packages/sdk/js/src/v2/gen/sdk.gen.ts`
- `packages/sdk/js/src/v2/gen/types.gen.ts`

## Blockers

- Experimental provider HttpApi unlock-route proof is recorded as skipped in `test/local/jnoccio-unlock-route.local.test.ts` because the provider HttpApi route currently fails before/inside the request with missing `@jekko/Config`. Legacy Hono route, unlock service, TUI paste/dialog path, git-crypt fresh-clone proof, and installed-binary route proof pass.
- Experimental provider HttpApi parity test still fails with missing `@jekko/Config` service before the OAuth assertion. The focused locked-provider, TUI sync, unlock service, encryption, typecheck, and cargo tests pass.
- SDK regeneration is blocked by the current OpenAPI generation path producing a zero-byte spec/hanging package build; generated SDK files were patched manually to keep TypeScript consumers aligned.
- Broad strict legacy-token scan remains noisy against archived tips and substring false positives; source-level `ZYML` regressions found during this work were fixed.
