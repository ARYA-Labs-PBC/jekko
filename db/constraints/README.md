# Constraints

Declare durable database truth here as small verification queries for checks,
foreign keys, and referential integrity.

Current invariant coverage:

- `event_sequence.owner_id` must be either `NULL` or a non-empty owner token.
- `part.session_id` must reference an existing `session.id`.
- `session.workspace_id` must reference an existing `workspace.id` when set.
- `session.parent_id` must reference an existing `session.id` when set.

Workflow notes:

- Use these scripts to verify database invariants after schema changes.
- Keep application code on the transaction side of the boundary; do not duplicate these invariants in product logic.
