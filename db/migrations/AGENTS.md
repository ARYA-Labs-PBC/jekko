# db/migrations/AGENTS.md

<!-- jankurai generated adapter -->
<!-- jankurai agent request v1 sha256:66add4ab06b1471c7e0fbc19b3f7843bc2152a2ca35616c9b1cd21ddaf2a3b7d -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
Owns `db/migrations/`.
Durable schema truth belongs here as versioned SQL migrations only.
Forbidden: application logic, transport routing, UI concerns, and hand-edited generated artifacts.
Proof lane: `migration / constraint tests`.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
