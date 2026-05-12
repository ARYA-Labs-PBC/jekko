# packages/script/AGENTS.md

<!-- jankurai generated adapter -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
Owns `packages/script/`.
Boundary: helper scripts shared across packages — generators, codegen, build glue.
Forbidden: long-running daemons, product code, anything that ships in the user-facing binary.
Proof lane: typecheck + targeted unit tests when behavior is testable.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
