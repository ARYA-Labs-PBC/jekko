# Migrations

Add versioned SQL migrations here.

Guidance:

- Each migration folder should be a single change with a timestamped name.
- Keep migration SQL deterministic and reversible where possible.
- Treat `db/migrations/` as the source of durable schema truth for the repo.

Generation:

- Create migrations through the Jekko Drizzle workflow, not by hand.
- Regenerate any derived artifacts with the recorded command in `agent/generated-zones.toml`.
