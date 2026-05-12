# Database

This directory owns durable database truth for the repo.

Layout:

- `db/migrations/` holds versioned SQL migrations.
- `db/constraints/` holds small verification queries for key invariants.

Rules:

- Put durable schema changes in migrations, not ad hoc SQL in product code.
- Put invariant checks in constraints, then enforce workflow behavior through application-owned transactions.
- Keep transport, UI, and feature logic out of this directory.

Proof:

- Use the migration / constraint test lane for DB changes.
- Prefer the smallest change that preserves existing rows and migrations.
