# packages/sdk/AGENTS.md

<!-- jankurai generated adapter -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
Owns `packages/sdk/`.
Boundary: public SDK surface for third-party integrations (TypeScript via `js/`, Rust via separate crates).
Forbidden: leaking internal jekko types, breaking semver without changelog entry, importing product internals.
Generated zones: `packages/sdk/js/src/gen/`, `packages/sdk/js/src/v2/gen/` — declared in `agent/generated-zones.toml`. Do not edit by hand.
Proof lane: `sdk-fast` (typecheck + build) via `just sdk-fast`.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
