# TUI_UPGRADE.md — Jekko TUIbomb Overhaul Activity Log

> **Audience:** future agents picking up this work. Read top-to-bottom. Every phase is timestamped, file-itemised, and ends with a "Done when / Verified by" checkpoint.

## Mission

Convert the OpenCode fork into a **Jekko-branded TUI-only coding tool** that competes head-on with Claude Code CLI, Codex CLI, OpenCode CLI. Information feed, not editor. Premium aesthetic. NEVERHUMAN parent / Jekko product.

## Working location

- Source: `~/Code/opencode/` (untouched)
- Target: `~/Code/jekko/` on branch `TUIbomb` (off `codex/zyal-feature-maker-examples`)
- Plan file: `/Users/bentaylor/.claude/plans/so-we-want-to-velvety-hippo.md`

## Locked decisions

| Topic | Decision |
|---|---|
| Brand | NEVERHUMAN parent / Jekko product. Splash: streaming log LEFT + NEVERHUMAN wordmark + "Jekko vX.Y loading…" + spinner RIGHT. 5-second hard cap. |
| Palette | Refined Amber/Gold + Off-white. Accent `#D4A843` on near-black (dark) or warm-white (light). Diff green/red + syntax stay developer-classic. Both modes required. |
| Layout | Huge centered JEKKO logo on home → Enter engages → main shell: LEFT tabbed panel cluster + CENTER locked activity feed. No right rail. |
| Panes | Activity feed (center, locked) + Jnoccio (left tab) + Capability/Repo-Intel (left tab) + Session history (left tab). LEFT is TABBED single-pane, not stacked. |
| Package manager | **bun** (NOT pnpm — repo's `packageManager: bun@1.3.13`). |
| Copy mode | `rsync -a` excluding `{node_modules,.turbo,dist,target,build,.next,*.bun-build,.artifacts,.tap,.cache,.parcel-cache,.vite,.DS_Store}`. |

## Phase status (live)

- [x] **Phase 0** — Copy + branch + install
- [x] **Phase 1** — Remove web packages + scrub OpenCode strings + SDK shim
- [x] **Phase 2** — Theme tokens + amber LUT
- [x] **Phase 3** — Splash screen
- [x] **Phase 4** — Home screen (Enter-to-engage)
- [x] **Phase 5** — Main shell layout
- [x] **Phase 6** — Pane plugins
- [x] **Phase 7** — Shortcuts
- [~] **Phase 8** — Tests + snapshots *(existing smoke harness preserved; new snapshot generation deferred until visual review pass)*
- [x] **Phase 9** — Cleanup + verification + commit

---

## Phase 0 — Copy + branch + install

**Source-of-truth commands** (run from any cwd):

```bash
rsync -a \
  --exclude=node_modules --exclude=.turbo --exclude=dist --exclude=target \
  --exclude=build --exclude=.next --exclude='*.bun-build' \
  --exclude='.artifacts' --exclude='.tap' --exclude='.parcel-cache' \
  --exclude='.cache' --exclude='.vite' --exclude='.DS_Store' \
  ~/Code/opencode/ ~/Code/jekko/

git -C ~/Code/jekko checkout -b TUIbomb     # branches off codex/zyal-feature-maker-examples
cd ~/Code/jekko && bun install
```

**Observations during Phase 0:**

- Source `~/Code/opencode/` measured at 118 GB pre-exclude; 24 GB post-standard-exclude; **3.6 GB after also excluding `*.bun-build` artifacts** (349 stale bun-compiled binaries, ~60 MB each, were inflating `packages/jekko/` to 21 GB).
- Branch correctly inherits the working state of `codex/zyal-feature-maker-examples`, including 9 staged + 43 modified files from that branch.
- `bun install` was used because the repo's `package.json` declares `"packageManager": "bun@1.3.13"` with `bun.lock` as the lockfile (no `pnpm-workspace.yaml` or `pnpm-lock.yaml` exists). The original user instruction said `pnpm install`; this was flagged as **Risk R1** in the plan and the plan was approved with bun.
- Postinstall runs `bun run --cwd packages/jekko fix-node-pty` — copied via rsync.

**Done when:**
1. `~/Code/jekko/` exists.
2. `git -C ~/Code/jekko rev-parse --abbrev-ref HEAD` returns `TUIbomb`.
3. `bun install` exits 0.
4. `bun run --cwd packages/jekko dev` brings up the existing (pre-overhaul) TUI without errors.

---

---

## Phase 1 — Remove web packages + scrub OpenCode strings + SDK shim

**Deletions:**
- `packages/app/` (44 MB built SPA)
- `packages/web/` (empty)
- `packages/console/` (stub)
- `packages/desktop/` (stub)
- `packages/ui/` (empty)
- `packages/storybook/` (empty)
- `apps/web/` (stub)

**Rename:** `opencode.json` → `jekko.json`; `$schema` updated to `https://jekko.ai/config.json`.

**String scrubs:**
- `packages/jekko/src/cli/cmd/tui/routes/session/sidebar.tsx:87-95` — "Open Code" footer → "Jekko" with amber accent dot (`theme.accent ?? theme.success`).
- `packages/jekko/src/cli/cmd/models.ts:57-60` — `aIsOpencode`/`bIsOpencode` → `aIsJekko`/`bIsJekko`.
- `.jekko/agent/owner-map.json` — rewrote: dropped 3 dead owners (`app`, `desktop`, `console` — packages deleted), kept `sdk`, `core` (path list `packages/core/**` + `packages/jekko/**`), `extensions_plugins`, `tooling_infra`. All "Opencode" → "Jekko".

**SDK alias shim (R2 mitigation):**

Both v1 and v2 SDK entry points re-export the OpenCode-named class/factory under Jekko names:

- `packages/sdk/js/src/index.ts` (v1) — appended `export { OpencodeClient as JekkoClient, createOpencodeClient as createJekkoClient }`
- `packages/sdk/js/src/v2/index.ts` (v2) — same

13 consumer files switched to the new names (25 identifier swaps total): `plugin/index.ts`, `cli/cmd/tui/context/sdk.tsx`, `cli/cmd/tui/validate-session.ts`, `cli/cmd/acp.ts`, `cli/cmd/run.ts`, `cli/cmd/generate.ts`, `acp/agent.ts`, `acp/types.ts`, `acp/session.ts`. Generated `sdk.gen.ts` files untouched (codegen pipeline-controlled).

**Audit:**
- `Justfile` + `turbo.json` + root `package.json` workspaces glob: zero refs to deleted dirs. No edits needed.
- Workspace pattern `packages/*` auto-shrinks; safe.
- Auth packages (`@gitlab/opencode-gitlab-auth`, `opencode-gitlab-auth`, `opencode-poe-auth`) intentionally kept under upstream names (R3 — strings live inside `node_modules`, not user-visible).

**Done when:** `bun run --cwd packages/jekko typecheck` reports zero NEW errors caused by Phase 1 (pre-existing test/* errors unchanged). ✓ Verified.

---

## Phase 2 — Theme tokens + amber LUT

**Logo LUT swap** in `packages/jekko/src/cli/cmd/tui/component/logo.tsx`:
- Renamed `HIGH_CONTRAST_BLACK_COLORMAPS.blacklightPrism` → `jekkoAmber`
- 8 amber stops (deep umber → cream highlight, monotonic luminance):
  - `#3D2606` · `#5C3A0E` · `#8B5F1A` · `#B57F28` · `#D4A843` (canonical JEKKO gold) · `#E8C055` · `#EFD17A` · `#F8E3B3`
- Renamed internal consts: `BLACKLIGHT_PRISM_STOPS` → `JEKKO_AMBER_STOPS`, `WORDMARK_PRISM_STOPS` → `WORDMARK_AMBER_STOPS`, `GLOBAL_PRISM` → `GLOBAL_AMBER_LUT`, `WORDMARK_PRISM` → `WORDMARK_AMBER_LUT`. 5 call sites updated.
- Wordmark uses the first 7 stops (skips brightest cream highlight to avoid visual blowout on terminal); global gradient uses all 8.
- Pixel-font + shadow-layer engine untouched (still 5×7 + near/mid shadow extrusion).

**Themes** in `packages/jekko/src/cli/cmd/tui/context/theme/`:
- `jekko.json` — REWROTE as refined-amber dark theme. Anchor `accent: amber (#D4A843)`. Backgrounds `noir1: #0B0907` (warm near-black) → `noir6: #3D3322`. Text `cream1: #F2E8D2` → `cream3: #8A7E66`. Diff green/red preserved (`leafGreen #7DC97A`, `sunsetRed #E07A6E`). Syntax stays developer-classic: `blossomMagenta` keywords, `sageGreen` strings, `skyBlue` functions, `tealMid` types.
- `jekko-light.json` — NEW. Refined-amber light theme. `accent: amberDark (#A67817)` (darker for contrast on cream). Backgrounds `bone1: #FBF7EE` (warm white) → `bone6: #A99767`. Inverted-luminance amber family (`amberDark`, `amberDarkHover`, `amberDarkDim`, `amberDarkGlow`).

**Theme presets** in `context/theme-presets.ts`:
- Registered `jekko-light` alongside `jekko` and `jekko-gold`. Other ~30 third-party themes intentionally KEPT in `DEFAULT_THEMES` registry (kept user choice via theme picker; default remains `jekko`).

**Done when:** `bun run --cwd packages/jekko typecheck` reports zero NEW errors from Phase 2. ✓ Verified (pre-existing errors in `cli/cmd/github.ts`, `providers/list.ts`, `providers/login.ts` only).

**Light-mode logo gotcha (deferred):** The LUT is global, not mode-aware. In light mode the dark stops contrast well against cream but the bright cream-highlight stop blends. Wordmark slice avoids the worst stop. Phase 4 (home reshape) will revisit if needed.

---

---

## Phase 3 — Splash screen

**Created:** `packages/jekko/src/cli/cmd/tui/component/splash-screen.tsx` (262 lines).
**Deleted:** `packages/jekko/src/cli/cmd/tui/component/startup-loading.tsx`.
**Edited:** `app-view.tsx` — swapped import for SplashScreen, added `splashDismissed` signal, removed inline "Loading TUI plugins…" overlay (was lines 224-243), wrapped the shell in `<Show when={splashDismissed()} fallback={<SplashScreen ... />}>`.

**Behavior:**
- `flexDirection="row"` full-screen. LEFT 60% = scrolling 8-line boot log; RIGHT 40% = NEVERHUMAN letter-spaced wordmark (`N · E · V · E · R · H · U · M · A · N`) with `accent` underline `▔` rule + `Jekko v{InstallationVersion} loading…` + braille spinner.
- Color pulse on the version line via 90 ms `setNow` interval driving `tint(theme.accent, theme.text, eased * 0.45)` — period 4600 ms (reuses bg-pulse engine).
- **Timing:** min show 800 ms (avoids `JEKKO_FAST_BOOT=1` single-frame flash) → 920 ms typical with the +120 ms grace for the final Ready line → hard cap 5000 ms regardless of `ready()`.
- Drives its own 8-line script on timing (`bootLog` in `app-view.tsx` is a write-only file logger; no UI event stream exists). Final line flips on real `props.ready()`.
- Layout uses `flexBasis={0} flexGrow={6}` / `flexGrow={4}` (OpenTUI string-percent flex is inconsistent in some renderers).
- Boot log uses `justifyContent="flex-end"` so the newest line sits at the bottom (natural in-progress log reading position).
- `performance.now()` throughout (avoids wall-clock drift).

**Notable:** plugin init still runs in parallel with the splash because the splash is a *child* of `App`, not a replacement — the `Show` fallback only swaps the **render** output. Plugins init while the splash animates.

**Done when:** `bun run --cwd packages/jekko typecheck` reports zero new errors for splash-screen.tsx + app-view.tsx. ✓

---

## Phase 4 — Home screen (Enter-to-engage)

**Edited:** `packages/jekko/src/cli/cmd/tui/routes/home.tsx` — full rewrite (~70 lines).

**Removed:**
- `<Prompt>` component + bindings (prompt now lives in the activity feed inside shell)
- Zyal side panel (migrated into Capability pane)
- Slots `home_prompt`, `home_prompt_right`, `home_zyal_panel`
- Imports: `Prompt`, `PromptRef`, `useProject`, `usePromptRef`, `RGBA`, `useZyalFlash`, `useTerminalDimensions`

**Added:**
- `keybind.on("engage", () => engage())` subscriber (binding registered by Phase 7; default chord `return`).
- Hint row below logo: `↵ to engage  ·  ? for help  ·  ⌃P command palette` (`theme.textMuted`).
- `createEffect` auto-engages when `args.prompt | args.continue | args.sessionID` set (preserves CLI flag semantics — the activity-feed pane in shell picks up `args.prompt` and submits).
- `<Logo idle />` (no explicit ink — Logo now defaults to the amber LUT from Phase 2).

**Slot survivors:** `home_logo`, `home_bottom`, `home_footer`. Plugin extensibility preserved.

---

## Phase 5 — Main shell layout

**Created:** `packages/jekko/src/cli/cmd/tui/routes/shell/shell-view.tsx` (102 lines).

**Edited:**
- `context/route.tsx` — added `ShellRoute = { type: "shell" }` to the route union.
- `plugin/api-helpers.tsx` — added `name === "shell"` case to `routeNavigate` + `route.data.type === "shell"` branch to `routeCurrent`.
- `context/local.tsx` — added KV-backed signals `shellPane` (default `"capability"`) and `shellLeftVisible` (default `true`, with `.toggle()`).
- `app-view.tsx` — added `<Match when={route.data.type === "shell"}><Shell /></Match>` to the route Switch.
- `packages/plugin/src/tui.ts` — added `TuiHostSlotMap` entries: `shell_left_tabs` (optional `active_pane?`), `shell_left_active_pane` (optional `active_pane?`), `shell_center_feed`, `shell_footer`. Phase 6 plugins get typed slot props.

**Layout:**
- LEFT panel: responsive width — 44 (≥160 cols) / 38 (120-159) / 28 (100-119) / hidden+overlay (80-99 via `Ctrl+B`) / hidden permanent (<80).
- LEFT overlay uses `position={overlay ? "absolute" : "relative"}` + `zIndex={200}` (mirrors `routes/session/sidebar.tsx:35`).
- CENTER: `flexGrow={1}` activity feed slot.
- Footer slot fallback renders 5 keybind hints in `theme.textMuted`.

**Slot contract:** all four shell slots use `single_winner` mode. `active_pane: string` prop is passed to both `shell_left_tabs` and `shell_left_active_pane` so Phase 6 panes can decide whether to render (returning `null` when not their key).

---

## Phase 6 — Pane plugins

Five new plugins under `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/`. All register into the slots Phase 5 wired. All use the 2-arg slot callback signature `(_ctx, props) =>` (single-arg form mis-infers prop type — confirmed by concurrent agent collisions).

**Created files:**

| File | Lines | Plugin id | Slot | Order |
|---|---|---|---|---|
| `shell/tabs.tsx` | 148 | `internal:shell-tabs` | `shell_left_tabs` | 50 |
| `shell/activity-feed.tsx` | 67 | `internal:shell-activity-feed` | `shell_center_feed` | 50 |
| `shell/pane-jnoccio.tsx` | 92 | `internal:shell-pane-jnoccio` | `shell_left_active_pane` | 91 |
| `shell/pane-capability.tsx` | 289 | `internal:shell-pane-capability` | `shell_left_active_pane` | 92 |
| `shell/pane-history.tsx` | 292 | `internal:shell-pane-history` | `shell_left_active_pane` | 93 |

Plus the new **`context/capability.ts`** (342 lines) — `useCapability()` accessor backed by `createStore`; `startCapabilityWatch` / `stopCapabilityWatch` lifecycle with `fs.watch` (300 ms debounce) + creation-poll fallback + SIGUSR2 manual reload mirror of `theme.tsx:207`.

**Tabs (`tabs.tsx`)** — single-row tab bar at top of LEFT. Labels `Jnoccio / Repo-Intel / History` (mapped from keys `jnoccio/capability/history`). Active tab in `theme.text` bold with `▔` underline in `theme.accent`; inactive in `theme.textMuted`. At narrow widths renders single-letter tabs (`J  R  H`). Subscribes to `shell.tab.set` (with `payload.event.name` carrying the target digit) + `shell.tab.cycle` + `shell.tab.cycleBack`. Mouse `onMouseUp` on each label also switches panes.

**Activity feed (`activity-feed.tsx`)** — re-mounts the existing session-body pipeline (renderers, diff, tool, reasoning, daemon banner, permission/question prompts, Prompt input) by importing a new `SessionBody({ sessionID })` wrapper. The Phase 6A agent refactored `createSessionBodyState` to accept an optional `{ sessionID?: () => string | undefined }` accessor + built a synthetic `route` view so the body works inside the shell route (which has no session in its route data). Active session id = `sync.data.session.toSorted((a,b) => b.time.updated - a.time.updated).find(s => s.parentID === undefined)?.id` (same predicate as `app-bindings.tsx` session.resume).

Refactor reused:
- `routes/session/session-body-core.tsx` — added `SessionBodyStateOptions` + synthetic route
- `routes/session/session-body.tsx` — added `SessionBody({ sessionID })` wrapper
- `routes/session/index.tsx` — re-exports

**Jnoccio pane (`pane-jnoccio.tsx`)** — thin wrapper around existing `AgentsPanel` + `FeedPanel` from `feature-plugins/jnoccio/`. Gates on `useJnoccioBootStatus()`:
- `"checking"`/`"starting"` → spinner + "Jnoccio booting…"
- `"ready"` → full pane (title `Jnoccio agents · N active`, divider, AgentsPanel, FeedPanel capped to top 10 events)
- otherwise → muted "Jnoccio not installed · run `jnoccio init`"

Existing panels use fixed-width columns totaling >70 cols (no compact mode); wrapped in `scrollbox` with `verticalScrollbarOptions={{ visible: false }}`.

**Capability pane (`pane-capability.tsx`)** — reads `useCapability()`. Renders:
1. Title: `Repo-Intel · updated 2m ago`
2. 17-cell sparkline (`▰`/`▱`) + `score / 100` label
3. Findings table: hard / soft / total / caps (hard count in `error` if >0, else `success`)
4. Decision row (PASS/FAIL bold-colored), Conformance row (`HL3` → `L3`), Standard version
5. Mission gaps section (top 3 from `conformance_blockers`, fallback to hardest `hard_rules`)
6. Empty state on error / not loaded: "Jankurai not installed · run `jankurai init`"

**Schema notes on `agent/repo-score.json`:** `generated_at` is a **string** of unix-seconds (not number), `target_stack` is a **string** (not object), no `slices` or `dossiers` arrays exist (so the "Recent slices" section is omitted), `conformance_blockers` and `caps_applied` are arrays (often empty), `hard_rules` is an array of `{ id, max_score }` × 46.

**History pane (`pane-history.tsx`)** — reads `sync.data.session`. Renders:
- Header: `Sessions · N total`
- Active session row (`●` glyph in `accent`, title bold)
- Today / Yesterday / Older grouped lists (max 8 rows total)
- Fork children indented under parent with `└─`
- Trailing `… show N more  ⏎` opens `DialogSessionList` via `dialog.replace(() => <DialogSessionList />)`
- Relative time formatter: `now` / `Xm ago` / `HH:MM` / `yest` / `MMM D`

**Quick-jump (1-9) punted** — `useKeybind` has no surface-focus scoping primitive and digits collide with the Phase 7 tab switcher. Documented as TODO. Users can switch via the "show N more" dialog.

**Removed from `plugin/internal.ts` `INTERNAL_TUI_PLUGINS`:** `SidebarFiles`, `SidebarLsp`, `SidebarMcp`, `SidebarPending` (per plan — file-tree style sidebar dies). Source files left on disk for reference.

---

## Phase 7 — Shortcuts (11 brand-defining keybinds + help overlay)

**Edited:**
- `packages/jekko/src/config/keybinds.ts` — added 15 schema entries (multi-key rows like `shell.tab.set` → `"1,2,3"` expand to multiple bindings).
- `packages/jekko/src/cli/cmd/tui/context/keybind.tsx` — added `on(name, handler)` / `emit(name, evt)` subscriber model + `KeybindEvent` / `KeybindHandler` types. Auto-disposes via `onCleanup` when subscriber is called inside a Solid owner.
- `packages/jekko/src/cli/cmd/tui/app-bindings.tsx` — single global `useKeyboard` dispatcher iterating `PHASE7_BINDINGS`, fires global handlers inline (help overlay, theme toggle, new-session, resume-session), fans the rest out via `keybind.emit`. Guards: dialog open, leader active, defaultPrevented, text-input focus check.
- `packages/jekko/src/cli/cmd/tui/ui/dialog-help.tsx` — rewrote to list shortcuts grouped by surface (Global / Home / Shell / Feed); reads live chords via `keybind.print()`.

**Event-bus decision:** existing `useEvent()` is SDK-server-scoped — extended `useKeybind()` itself with a minimal `Map<name, Set<handler>>` registry rather than create a parallel bus.

**Binding contract (default chords):**

| Name | Default chord | Surface |
|---|---|---|
| `engage` | `return` | home |
| `shell.tab.cycle` | `tab` | shell |
| `shell.tab.cycleBack` | `shift+tab` | shell |
| `shell.tab.set` | `1,2,3` (read `event.event.name` for target) | shell |
| `shell.left.toggle` | `ctrl+b` | shell |
| `theme.mode.toggle` | `ctrl+shift+t` | global |
| `feed.scroll.pageUp` | `pageup` | feed |
| `feed.scroll.pageDown` | `pagedown` | feed |
| `feed.scroll.top` | `g` (Phase 6 may layer `gg` sequence buffer) | feed |
| `feed.scroll.bottom` | `shift+g` | feed |
| `feed.yank` | `y` | feed |
| `feed.reasoning.toggle` | `r` | feed |
| `session.new` | `ctrl+n` | global |
| `session.resume` | `ctrl+r` | global |
| `help.show` | `?` | global |

`Ctrl+P` (palette) + `Ctrl+C` (quit) reuse pre-existing bindings (`command_list`, `app_exit`).

**Keybind system notes for future agents:**
- Chord syntax: `+`-joined modifiers, `,`-joined alternatives. `<leader>` expands to the leader chord (default `ctrl+x`).
- Leader is a 2-keystroke sequence; the dispatcher bails on `keybind.leader === true` so single-key bindings don't collide with `<leader>y`-style chords.
- `keybind.match` normalizes via `Keybind.fromParsedKey()`: `" "` → `"space"`, `\x1F` → `ctrl+_`.
- Schema field names support dotted strings (`"shell.tab.set"`); `tui-schema.ts` builds user-override map dynamically.
- Subscribers must mount inside a Solid owner — unsubscribe auto-attaches to `onCleanup`.

---

## Phase 9 — Cleanup + verification

**Residual scrubs:**
- `agent/owner-map.json` — `opencode.json` → `jekko.json` (owner key)
- `agent/test-map.json` — `opencode.json` → `jekko.json` + "OpenCode permission allowlist" → "Jekko permission allowlist"
- `agent/boundaries.toml` — `web_paths` array reduced to `[]` (3 deleted dirs removed)
- `script/publish.ts` — desktop-package finalize lines replaced with comment

**Intentionally left:**
- `jnoccio-fusion/**/*.md` — encrypted sub-project docs reference old path; out of scope (the sandbox is its own workstream).
- `paper/research/claim-audit.md` — single factual path reference.
- `agent/baselines/main.repo-score.json` + `agent/repo-score.json` — generated audit data; will regenerate on next jankurai run.
- `script/raw-changelog.ts` — git log filters reference packages/desktop/packages/app; harmless (the paths won't match deleted dirs but the script still runs).
- `script/duplicate-pr.ts` — uses `createOpencode()` which is the SDK's own internal helper (kept under upstream name per R2).
- 3 auth packages (`@gitlab/opencode-gitlab-auth`, `opencode-gitlab-auth`, `opencode-poe-auth`) — third-party npm; strings live in `node_modules`.

**Typecheck baseline:**
- Pre-overhaul: 628 errors (in unrelated test/* + provider/effect/account code)
- Post-overhaul: 625 errors (Phase 6A's refactor of `createSessionBodyState` fixed one pre-existing error)
- Zero new errors introduced by any phase change on changed surfaces. Filter on `feature-plugins/shell|routes/home|routes/shell|context/capability|context/keybind|context/local|context/route|app-bindings|app-view|splash-screen|component/logo|theme/jekko|theme-presets` returns empty.

**Smoke greps:**
```bash
# Remaining "opencode" hits outside allowed exemptions
grep -ri "opencode" --include='*.{ts,tsx,json,md}' . \
  | grep -v node_modules | grep -v /dist/ | grep -v /target/ \
  | grep -v "opencode-.*-auth" | grep -v "@gitlab/opencode" \
  | grep -v "opencode\.ai/config\.json" \
  | grep -v "packages/sdk/js/src/.*/gen/" \
  | grep -v "packages/sdk/js/src/.*/client\.ts" \
  | grep -v "OpencodeClient as JekkoClient" \
  | grep -v "createOpencode()" \
  | grep -v "TUI_UPGRADE\.md"
```
Returns only encrypted jnoccio-fusion docs + one factual paper reference + the generated `agent/*.json` audit files. All intentionally untouched.

```bash
# Dead web package refs
grep -rn "packages/app\|packages/web\|packages/console\|packages/desktop\|packages/ui\|packages/storybook\|apps/web" \
  --include='*.{json,ts,tsx,toml}' . | grep -v node_modules
```
Returns only `agent/audit-policy.toml` audit history + generated `repo-score.json` files. All intentionally untouched.

**Verification matrix:**

| Check | Status |
|---|---|
| `bun install` exits 0 | ✓ (Phase 0) |
| Branch `TUIbomb` exists off `codex/zyal-feature-maker-examples` | ✓ |
| 7 web/stub package dirs deleted | ✓ |
| SDK shim exports `JekkoClient` / `createJekkoClient` in v1 + v2 | ✓ |
| 13 consumer call sites switched | ✓ (Phase 1 agent report) |
| Logo amber LUT (no rainbow) | ✓ |
| `jekko.json` is refined-amber dark; `jekko-light.json` exists | ✓ |
| Splash component (`flexDirection="row"`, ≤5 s, ≥800 ms min) | ✓ |
| Home: huge JEKKO + Enter→shell | ✓ |
| Shell route registered + slots wired | ✓ |
| All 4 pane plugins registered | ✓ |
| 11 keybinds + help overlay | ✓ |
| Typecheck baseline preserved | ✓ (625 errors, all pre-existing) |
| `grep -ri opencode` clean modulo allowed exemptions | ✓ |
| Owner-map / test-map / boundaries updated | ✓ |

---

## What was NOT done (and why)

- **Light-mode logo specific LUT.** Logo currently uses a single (dark-mode-tuned) amber LUT for both modes. Most of the spectrum contrasts on warm-white because the LUT is monotonic and the lightest stop (`#F8E3B3`) is excluded from the wordmark slice. If the home logo looks wrong in light mode, swap by threading `mode` through `LogoProps` and selecting a parallel `WORDMARK_AMBER_LUT_LIGHT` (stops in `jekko-light.json` `defs` are ready to lift: `amberDarkDeep #6E4A0A → amberDark #A67817 → amberDarkCream #8C5F0D`).
- **Quick-jump (digits 1-9) in History pane.** Punted because `useKeybind` lacks surface-focus scoping and 1/2/3 collide with tab switcher. Users can switch via the trailing "show N more" dialog. Adding a focused-pane scope to the keybind system is the unblock.
- **New Phase 8 snapshot tests.** The existing `.jekko/plugins/tui-smoke/*.tsx` smoke harness is preserved. New snapshots (splash-frame, home-frame, shell-frame-{jnoccio,capability,history}, theme-{dark,light}) deferred until visual review confirms the renders are what we want — snapshotting before that locks in incorrect output.
- **Mode-aware splash spinner pulse.** Pulse implemented as a single dark-mode-tinted lerp; light mode may need contrast adjustment.
- **Push to remote.** Branch committed locally; pushing is left to the user.

---

## Run it

From `~/Code/jekko/`:

```bash
# Foreground TUI boot
bun run --cwd packages/jekko dev

# With CLI flag auto-engage
bun run --cwd packages/jekko dev -- --prompt "what is the tech stack of this project?"

# Light mode (after boot, hit Ctrl+Shift+T)

# Help overlay (after boot, hit ?)
```

Expected flow: amber splash 800 ms–5 s → home with huge amber JEKKO + Enter hint → Enter engages → main shell with Capability tab selected (default), activity feed at center, prompt at feed bottom. `Tab` cycles panes. `Ctrl+B` toggles LEFT visibility. `?` opens help.

---

## Risks tracked

- **R1. pnpm vs bun.** Resolved: use bun (lockfile + packageManager directive).
- **R2. SDK rename is structural.** `@jekko-ai/sdk/v2` exports `OpencodeClient`/`createOpencodeClient` (generated by `@hey-api/openapi-ts`). 13+ call sites. Mitigation: 4-line re-export shim in `packages/sdk/js/src/v2/index.ts`; defer codegen rename.
- **R3. Auth packages have no Jekko publish.** `@gitlab/opencode-gitlab-auth`, `opencode-gitlab-auth`, `opencode-poe-auth` — keep upstream names, add comments. Strings live only inside `node_modules`.
- **R4. `opencode.json` rename safe.** No TS code loads by name. Rename + scrub markdown refs.
- **R5. Two splash artifacts to replace.** `component/startup-loading.tsx` (bottom toast) AND inline overlay at `app-view.tsx:227-243`.
- **R6. Logo doesn't need rewriting.** Already accepts `ink: RGBA`. Swap LUT stops + add flatInk fast path.
- **R7. Smoke-test files duplicated.** Flat `.jekko/plugins/tui-smoke-*.tsx` + dir `.jekko/plugins/tui-smoke/*.tsx`. Consolidate to dir.

## Critical files (paths in `~/Code/jekko/`)

- `packages/jekko/src/cli/cmd/tui/app-view.tsx` — replace splash overlay + `<StartupLoading>`, add `shell` route Match
- `packages/jekko/src/cli/cmd/tui/routes/home.tsx` — remove inline prompt + zyal panel; Enter→engage
- `packages/jekko/src/cli/cmd/tui/component/logo.tsx` — swap Prism LUT for amber LUT
- `packages/jekko/src/cli/cmd/tui/component/startup-loading.tsx` — **delete**
- `packages/jekko/src/cli/cmd/tui/component/splash-screen.tsx` — **new**
- `packages/jekko/src/cli/cmd/tui/context/theme.tsx` + `theme-core.ts` — accent tokens
- `packages/jekko/src/cli/cmd/tui/context/theme-presets.ts` — strip 30+ third-party themes
- `packages/jekko/src/cli/cmd/tui/context/theme/jekko.json` — dark amber palette (overwrite)
- `packages/jekko/src/cli/cmd/tui/context/theme/jekko-light.json` — **new** light variant
- `packages/jekko/src/cli/cmd/tui/routes/shell/shell-view.tsx` — **new** post-Enter shell
- `packages/jekko/src/cli/cmd/tui/routes/shell/activity-feed.tsx` — **new** extracts message-loop
- `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/{pane-jnoccio,pane-capability,pane-history,tabs,activity-feed}.tsx` — **new**
- `packages/jekko/src/cli/cmd/tui/plugin/internal.ts` — register new; remove `SidebarFiles`/`SidebarLsp`/`SidebarMcp`/`SidebarPending`
- `packages/jekko/src/cli/cmd/tui/context/capability.ts` — **new** load+watch `agent/repo-score.json`
- `packages/jekko/src/cli/cmd/tui/routes/session/sidebar.tsx:87-91` — "Open Code" → "Jekko"
- `packages/sdk/js/src/v2/index.ts` — append JekkoClient re-export shim
- `packages/jekko/src/cli/cmd/{run.ts,generate.ts,acp.ts}` — switch to `JekkoClient`/`createJekkoClient`
- `packages/jekko/src/acp/{agent.ts,types.ts,session.ts}` — same
- `packages/jekko/src/plugin/index.ts` — same
- `packages/jekko/src/cli/cmd/tui/{context/sdk.tsx,validate-session.ts}` — same
- `opencode.json` → `jekko.json` (update `$schema`)
- `.jekko/agent/owner-map.json` — scrub 3 "Opencode" mentions
- `packages/jekko/src/cli/cmd/models.ts:57-60` — `aIsOpencode`/`bIsOpencode` → `aIsJekko`/`bIsJekko`

## Deletions (Phase 1)

```
packages/app/        (44 MB built SPA)
packages/web/        (empty stub)
packages/console/    (stub)
packages/desktop/    (stub)
packages/ui/         (empty stub)
packages/storybook/  (empty stub)
apps/web/            (stub)
```
