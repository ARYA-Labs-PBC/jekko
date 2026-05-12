# contracts/events/AGENTS.md

<!-- jankurai generated adapter -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
Owns `contracts/events/`.
Boundary: event contract sources (JSON Schema, AsyncAPI, or protobuf) for cross-cell event communication.
Forbidden: handwritten event consumers, generated bindings (those live in `contracts/generated/`), product truth in events.
Proof lane: contract drift detection via `jankurai audit` — generated bindings must match contract sources.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
