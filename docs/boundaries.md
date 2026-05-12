# Boundaries

Use this repo's canonical ownership maps and proof lanes when changing code:

- Read `AGENTS.md` first for the current operating rules.
- Read `agent/JANKURAI_STANDARD.md` before making task-scoped edits.
- Use `agent/owner-map.json` and `agent/test-map.json` to find the right owner and proof lane.
- Treat `agent/generated-zones.toml` as read-only unless the task explicitly says to regenerate output.
- Stop if a change would expand permissions, touch secrets, or require external service mutation.

When a task needs broader scope than the listed paths, mark it blocked and hand it off rather than widening the edit.
