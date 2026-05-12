# packages/core/AGENTS.md

<!-- jankurai generated adapter -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
Owns `packages/core/`.
Boundary: shared TypeScript runtime utilities consumed by `packages/jekko/`, `packages/plugin/`, and `packages/sdk/`.
Forbidden: product domain truth, network/IO side effects without explicit boundary, direct DB access.
Proof lane: `core-fast` (typecheck + build + test) via `just core-fast`.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
