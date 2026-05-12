# db/AGENTS.md

<!-- jankurai generated adapter -->
<!-- jankurai agent request v1 sha256:66add4ab06b1471c7e0fbc19b3f7843bc2152a2ca35616c9b1cd21ddaf2a3b7d -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
Owns `db/`.
Durable truth belongs in `db/migrations/` and `db/constraints/`; application-owned transactions keep workflow invariants, not ad hoc SQL in product code.
Forbidden: application logic, transport routing, and UI concerns.
Proof lane: `migration / constraint tests`.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
