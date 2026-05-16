# Migrations

> **Historical**: This document references the pre-Rust-port stack (Bun/OpenTUI/Solid/etc.). The current implementation is Rust-native (Ratatui/Crossterm). See [docs/archive/historical/](../../docs/archive/historical/) for the migration record.

Add versioned SQL migrations here.

Guidance:

- Each migration folder should be a single change with a timestamped name.
- Keep migration SQL deterministic and reversible where possible.
- Treat `db/migrations/` as the source of durable schema truth for the repo.

Generation:

- Create migrations through the Jekko Drizzle workflow, not by hand.
- Regenerate any derived artifacts with the recorded command in `agent/generated-zones.toml`.
