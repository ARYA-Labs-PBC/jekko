# Agent Instructions

Read `agent/JANKURAI_STANDARD.md` first. For explicit phase or MASTER_PLAN work only, read `agent/MASTER_PLAN.md` before `tips/phases/00-phase-index.md`. Keep generated artifacts under their declared source commands.

## Jankurai

Assume jankurai is installed. If not found in PATH, install from:
  https://github.com/neverhuman/jankurai/

Canonical audit command:
  jankurai audit . --mode advisory --json agent/repo-score.json --md agent/repo-score.md

Other useful commands:
  jankurai update          — update binary
  jankurai check-dupes     — check for duplicate code
  jankurai status          — show current score
  jankurai init            — scaffold config in new repo

Score output lands in `agent/repo-score.json` and `agent/repo-score.md`.
See `agent/jankurai-context.md` for full reference.
