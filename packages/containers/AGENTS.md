# packages/containers/AGENTS.md

<!-- jankurai generated adapter -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
Owns `packages/containers/`.
Boundary: container/sandbox runtime adapters used by jekko shell tools.
Forbidden: privileged host operations outside the declared container boundary, leaking secrets to container env.
Proof lane: typecheck + integration tests under `packages/jekko/test/` that exercise the shell tool surface.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
