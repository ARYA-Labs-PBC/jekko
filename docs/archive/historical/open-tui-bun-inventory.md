# OpenTUI and Bun Inventory

This snapshot is **TypeScript/Solid/OpenTUI**, not Rust.

I checked the tree for a second UI stack and for a production Vite app. There is no Rust TUI source tree here, and there is no real `vite.config.ts` app in this snapshot. The only Vite hits are incidental doc/test strings.

## Objective

Document the live OpenTUI + Bun surface that would need replacement pressure in a Ratatui/Crossterm/Tokio port, while keeping the current implementation intact.

## What Is Live

### TUI runtime entrypoint

- `packages/jekko/src/index.ts`
  - CLI entrypoint that registers the TUI-related commands and routes execution into the TUI host.
  - Replacement pressure: the command bootstrap stays in Node/Bun-style CLI code today, while terminal rendering, event dispatch, and lifecycle management would move to a Rust terminal runtime.

- `packages/jekko/src/cli/cmd/tui/app.tsx`
  - Exports `tui()` and builds the actual OpenTUI renderer, Solid tree, startup fallback, and terminal configuration.
  - Core symbols:
    - `tui()`
    - `rendererConfig()`
    - `installFirstFrameWatchdog()`
  - Replacement pressure: this is the heart of the port. Renderer creation, first-frame watchdogs, alternate screen handling, mouse/kitty keyboard options, and startup error recovery are all terminal-runtime concerns that map to Ratatui/Crossterm/Tokio.

- `packages/jekko/src/cli/cmd/tui/app-bindings.tsx`
  - Exports `setupAppBindings()` and owns the global key dispatcher, dialog shortcuts, theme toggle, session navigation, and selection/copy behavior.
  - Core symbols:
    - `setupAppBindings()`
    - `PHASE7_BINDINGS`
  - Replacement pressure: this is where keyboard routing and input focus rules would need a Rust equivalent, including global key handling versus component-scoped binding emission.

- `packages/jekko/src/cli/cmd/tui/app-view.tsx`
  - Main view composition for the app after startup; it routes the user into shell/session/home/plugin surfaces.
  - Replacement pressure: all route-level widget composition will need a Rust view tree and event-driven redraw model.

- `packages/jekko/src/cli/cmd/tui/component/**`
  - OpenTUI widgets for prompt editing, dialogs, navigation, headers, toasts, spinners, copy/paste, and terminal affordances.
  - Replacement pressure: these are the direct widget translations from OpenTUI components to Ratatui widgets and layout primitives.

- `packages/jekko/src/cli/cmd/tui/context/**`
  - Solid providers and stores for route, project, sync, editor, theme, keybind, prompt, local state, and feature flags.
  - Replacement pressure: state ownership and reactive subscriptions will need a Rust-side store/event model instead of Solid context.

- `packages/jekko/src/cli/cmd/tui/routes/**`
  - Route implementations for `home`, `shell`, and `session`, including daemon status, transcript rendering, question flows, and subagent/session panes.
  - Replacement pressure: route-specific screen composition and live data subscriptions become Rust view modules and async tasks.

- `packages/jekko/src/cli/cmd/tui/feature-plugins/**`
  - Live feature surfaces such as `shell`, `home`, `sidebar`, `jnoccio`, `jankurai`, and `system` panels.
  - Replacement pressure: these are the highest-density widget clusters and are the first place a Rust port would need parity for layout, scrolling, tables, tabs, and gauges.

- `packages/jekko/src/cli/cmd/tui/plugin/**`
  - Runtime layer for plugin loading, scopes, theme syncing, slot registration, and host/plugin API bridging.
  - Replacement pressure: plugin lifecycle and slot mounting are the hardest part of the port because they couple async loading, cleanup, and UI injection.

- `packages/jekko/src/cli/cmd/tui/util/**`
  - Terminal helpers, selection helpers, scroll logic, clipboard helpers, transcript formatting, sound playback, and parser/tokenization support.
  - Replacement pressure: these helpers are the non-visual behavior that will have to be reimplemented around Rust I/O, process spawning, and terminal abstraction.

- `packages/jekko/src/cli/cmd/tui/asset/**`
  - Startup and interaction sounds used by the TUI.
  - Replacement pressure: assets are portable, but playback hooks are coupled to the current terminal runtime.

### Plugin API and host bridge

- `packages/jekko/src/plugin/index.ts`
  - Host plugin service and loader integration.
  - Core symbols:
    - `applyPlugin()`
    - `PluginInput.$`
  - Replacement pressure: `PluginInput.$` is explicit Bun shell compatibility in the plugin path. Any Rust/Tokio port needs a replacement process/shell API and a compatibility story for plugin execution.

- `packages/jekko/src/cli/cmd/tui/plugin/runtime-core.ts`
  - Core runtime state for TUI plugins, including plugin loading metadata, cleanup behavior, and scoped lifecycle management.
  - Core symbols:
    - `createPluginScope()`
    - `runCleanup()`
    - `resolveRoot()`
  - Replacement pressure: lifecycle and cleanup semantics map to Rust task scopes, cancellation, and shutdown ordering.

- `packages/jekko/src/cli/cmd/tui/plugin/slots.tsx`
  - Slot registry setup and host/plugin slot bridging.
  - Core symbols:
    - `setupSlots()`
    - `Slot`
  - Replacement pressure: this is the content-injection surface that will need a Rust widget registry or equivalent.

- `packages/plugin/src/tui.ts`
  - Shared plugin-facing TUI types for routes, dialogs, keybinds, prompts, toast, theme, and host API shape.
  - Replacement pressure: this is the contract file that should anchor a port so plugin authors keep the same structural API surface where possible.

- `packages/jekko/src/util/keybind.ts`
  - Key normalization, comparison, formatting, and parser helpers.
  - Replacement pressure: keyboard handling and chord parsing need to be preserved exactly because the TUI depends on stable bindings and text-input exclusion rules.

- `packages/jekko/parsers-config.ts`
  - Parser configuration for TUI syntax highlighting and embedded language support.
  - Replacement pressure: syntax highlighting or tokenization support will need a Rust-side equivalent if the same editor-like behavior is retained.

### Current symbol map to keep in view

- `tui()` in `packages/jekko/src/cli/cmd/tui/app.tsx`
- `setupAppBindings()` in `packages/jekko/src/cli/cmd/tui/app-bindings.tsx`
- `createPluginScope()` in `packages/jekko/src/cli/cmd/tui/plugin/runtime-core.ts`
- `setupSlots()` in `packages/jekko/src/cli/cmd/tui/plugin/slots.tsx`
- `applyPlugin()` in `packages/jekko/src/plugin/index.ts`
- `PluginInput.$` in `packages/jekko/src/plugin/index.ts`
- `createTuiPluginApi()` in `packages/jekko/test/fixture/tui-plugin.ts`

## Tests and Fixtures

- `packages/jekko/test/cli/tui/**`
  - Rendered TUI component and integration tests for startup, dialogs, plugin loading, keybinds, layout pieces, route flows, and live snapshot behavior.
  - Replacement pressure: these are the regression set for any port. If the Rust implementation changes widget or event semantics, these tests are the current behavior reference.

- `packages/jekko/test/cli/cmd/tui/**`
  - Command-level TUI tests that sit closer to the CLI entrypoints and the focused command behavior.
  - Replacement pressure: these prove command semantics, dialog flows, and route wiring that a port must preserve.

- `packages/jekko/test/local/jnoccio-tui-paste-unlock.local.test.tsx`
  - Local-only live proof for the Jnoccio paste-to-unlock flow.
  - Replacement pressure: this is an integration proof for clipboard, unlock flow, and live provider state. It should remain outside CI-only assumptions.

- `packages/jekko/test/fixture/tui-plugin.ts`
  - Fixture host API generator for TUI plugin tests.
  - Core symbol:
    - `createTuiPluginApi()`
  - Replacement pressure: this fixture is the contract emulator for plugin tests; if the host API changes, the fixture and the real host bridge must move together.

## Bun Surface

### Package manager and runtime declarations

- `package.json`
  - Root package manager pin, workspace scripts, and Bun-oriented developer flows.
  - Replacement pressure: this is the top-level runtime declaration that would need to change if the repo stopped using Bun as its package/runtime driver.

- `bunfig.toml`
  - Root Bun configuration.
  - Replacement pressure: this is direct Bun bootstrapping, so any runtime migration will need a new equivalent or removal.

- `bun.lock`
  - Workspace lockfile.
  - Replacement pressure: lockfile generation and dependency resolution are tied to Bun today.

- `packages/jekko/package.json`
  - Package-local Bun scripts, workspace exports, conditional imports, and OpenTUI dependencies.
  - Replacement pressure: this file ties the TUI package to Bun commands, Bun-only binaries, and OpenTUI-specific compiler conditions.

- `packages/jekko/bunfig.toml`
  - Package-local Bun preload config for OpenTUI tests and TUI fixtures.
  - Replacement pressure: it directly configures the current OpenTUI test runtime.

- `packages/jekko/tsconfig.json`
  - TypeScript config that extends Bun’s tsconfig and pins `jsxImportSource` to `@opentui/solid`.
  - Replacement pressure: this is the compile-time bridge that makes the TUI compile as Solid/OpenTUI in Bun.

### Package-local scripts and helpers

- `packages/jekko/script/**`
  - Package-local Bun scripts for build, smoke testing, binary path discovery, OpenTUI upgrade, postinstall, live TUI lanes, schema generation, publish, and support tasks.
  - Replacement pressure: this directory is the operational layer around the TUI build and smoke pipeline; it is one of the most direct Bun dependencies in the repo.

- `packages/jekko/scripts/**`
  - Package-local shell helpers that still call Bun for workspace and SDK comparison flows.
  - Replacement pressure: these helpers indicate Bun is part of the package-local tooling contract, not just root-level automation.

- `script/**`
  - Repo-root Bun scripts for formatting, changelog/version automation, release helpers, publish flows, and GitHub-facing utilities.
  - Replacement pressure: these are global maintenance tools and should be treated as Bun-bound operational scripts.

- `script/hooks`
  - Hook bootstrap that runs Bun-based install and typecheck behavior.
  - Replacement pressure: hook execution depends on Bun being available.

- `script/sign-windows.ps1`
  - Windows-side release helper.
  - Replacement pressure: executable bit preservation and cross-platform release tooling matter if the script surface is reworked.

- `script/release`
  - Release entrypoint shell helper.
  - Replacement pressure: this is part of the release automation path and should be included in any tooling migration inventory.

### CI, bootstrap, and platform setup

- `.github/actions/setup-bun/action.yml`
  - Shared GitHub Action that installs Bun, caches dependencies, and handles Windows linker quirks.
  - Replacement pressure: this is the canonical bootstrap for CI and release workflows.

- `.github/publish-python-sdk.yml`
  - Disabled/placeholder workflow file that still documents Bun-based setup for SDK publishing.
  - Replacement pressure: even though it is not active, it preserves a Bun bootstrap assumption in the repo history.

- `.github/workflows/**`
  - CI workflows that call `setup-bun`, run Bun install steps, or rely on Bun-driven package scripts.
  - Replacement pressure: workflow migration would need to replace Bun setup across build, test, publish, triage, and maintenance lanes.

- `.husky/pre-push`
  - Pre-push hook that verifies Bun version compatibility and runs typecheck.
  - Replacement pressure: this is a developer-facing Bun gate.

- `ops/ci/**`
  - Local and remote CI helpers, including TUI smoke, typecheck, publish, beta, PR management, and operational scripts.
  - Replacement pressure: Bun is embedded in the operational command surface, not just the package manifests.

- `nix/**`
  - Nix packaging, node_modules normalization, Bun lock handling, and Nix build helpers.
  - Replacement pressure: the Nix build path encodes Bun install/build assumptions and should be treated as part of the migration surface.

## Docs and Runbooks

- `docs/testing-tui.md`
  - Canonical TUI testing runbook for CI-safe lanes, startup smoke, and live local proof.
  - Replacement pressure: it defines how the current OpenTUI runtime is verified.

- `docs/ci-local.md`
  - Local CI parity guide with explicit references to TUI smoke lanes.
  - Replacement pressure: a port should keep the same proof lanes or update this runbook at the same time.

- `docs/testing.md`
  - General proof-lane policy and receipt guidance.
  - Replacement pressure: any new Rust TUI proof lane should fit this receipt/verification model.

- `docs/install.md`
  - High-level install guidance for the repo’s bootstrap flow.
  - Replacement pressure: if Bun stops being the bootstrap runtime, this document must be updated alongside the tooling.

- `docs/architecture.md`
  - Canonical architecture note for agent work.
  - Replacement pressure: this is where new port-specific evidence patterns or boundary guidance would be recorded.

- `docs/boundaries.md`
  - Boundary and ownership guidance for edits.
  - Replacement pressure: a port should keep its own generated/read-only boundaries explicit.

- `packages/jekko/BUN_SHELL_MIGRATION_PLAN.md`
  - Bun shell migration planning note for the package.
  - Replacement pressure: this is a direct record of the remaining Bun shell surface and should be read as part of any runtime migration.

- `packages/jekko/specs/tui-plugins.md`
  - TUI plugin API spec and behavior reference.
  - Replacement pressure: this is the clearest contract file for plugin compatibility and should be kept aligned with host/runtime changes.

## What Is Not Here

- No Rust TUI source tree.
- No production Vite app.
- No generated output directories.
- No `dist/`, `target/`, `.turbo/`, `node_modules/`, `packages/sdk/js/src/gen/**`, or `packages/*/sst-env.d.ts`.

## Appendix A: Frozen Archive Manifest

The archive is built from the live working tree by expanding the following roots and then de-duplicating the resulting file list before tar creation:

```text
packages/jekko/src/cli/cmd/tui/**
packages/plugin/src/tui.ts
packages/jekko/src/plugin/index.ts
packages/jekko/src/util/keybind.ts
packages/jekko/parsers-config.ts
packages/jekko/test/cli/tui/**
packages/jekko/test/cli/cmd/tui/**
packages/jekko/test/local/jnoccio-tui-paste-unlock.local.test.tsx
packages/jekko/test/fixture/tui-plugin.ts
packages/jekko/package.json
packages/jekko/bunfig.toml
packages/jekko/tsconfig.json
packages/jekko/script/**
packages/jekko/scripts/**
packages/jekko/BUN_SHELL_MIGRATION_PLAN.md
packages/jekko/specs/tui-plugins.md
package.json
bunfig.toml
bun.lock
script/**
.github/actions/setup-bun/action.yml
.github/publish-python-sdk.yml
.github/workflows/**
.husky/pre-push
ops/ci/**
nix/**
docs/testing-tui.md
docs/ci-local.md
docs/testing.md
docs/install.md
docs/architecture.md
docs/boundaries.md
```

Generated or read-only zones remain excluded from the archive:

- `packages/sdk/js/src/gen/**`
- `packages/*/sst-env.d.ts`
- `dist/`
- `target/`
- `.turbo/`
- `node_modules/`
- any other generated artifact directory listed in `agent/generated-zones.toml`

