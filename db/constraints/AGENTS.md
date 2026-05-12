# db/constraints/AGENTS.md

<!-- jankurai generated adapter -->
<!-- jankurai agent request v1 sha256:66add4ab06b1471c7e0fbc19b3f7843bc2152a2ca35616c9b1cd21ddaf2a3b7d -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
Owns `db/constraints/`.
Durable data truth belongs here as small SQL checks and foreign-key verification queries.
Forbidden: application logic, transport routing, UI concerns, and schema migrations.
Proof lane: `migration / constraint tests`.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
