# packages/plugin/AGENTS.md

<!-- jankurai generated adapter -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
Owns `packages/plugin/`.
Boundary: plugin authoring SDK + runtime contract surface for jekko plugins.
Forbidden: bundling product features, runtime IO outside the plugin contract, hidden state.
Proof lane: `plugin-fast` (typecheck + build) via `just plugin-fast`.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
