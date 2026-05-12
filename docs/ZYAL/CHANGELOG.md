# ZYAL Changelog

## 2.6.0 - 2026-05-12

- Added first-class `jankurai:` syntax for host-enforced audit, repair-plan
  ingestion, task routing, verification, rollback, checkpoint, and branch/main
  regression checks.
- Added bundled examples `jankurai-continuous-repair` and
  `jankurai-porting-advanced`, both requiring `unsupported_feature_policy:
  required: [jankurai]`.
- Kept the runtime sentinel at `v1`; the contract and preview metadata move
  to `2.6.0`.

## 2.5.0 - 2026-05-11

- Renamed canonical extension from `.zyal.yml` to bare `.zyal`. Docs examples
  and paper listings migrate in-place; existing sentinel-wrapped syntax is
  unchanged.
- Introduced two new declarative profiles disambiguated by top-of-file
  pragma:
  - Profile B (`target=toml`) — compiles to TOML, first user is
    `agent/sandbox-lanes.zyal` → `agent/sandbox-lanes.toml`.
  - Profile C (`target=github-workflow`) — compiles to
    `.github/workflows/*.yml` so GitHub Actions can still find them.
- Shipped the `zyalc` compiler (`crates/zyalc/`) with idempotent
  `compile --all --check` drift detection.
- Added the sandbox-loop declarative function (`agent/sandbox-lanes.toml`)
  with three backends (worktree / bubblewrap / docker) and the
  `sandboxctl` runtime (`crates/sandboxctl/`).
- New jankurai rules: `HLT-032-ZYAL-COMPILE-DRIFT`,
  `HLT-033-UNDECLARED-SANDBOX-LOOP`.

## 2.4.0 - 2026-05-11

- Added the `research` block contract and preview surface for cited external evidence gathering.
- Finalized the contract/version split for ZYAL docs and preview metadata.
- Documented the runtime coverage limits, receipts, and preview-only boundaries for the current research path.
- Kept `.zyal.yml` compatibility strict: existing blocks remain valid unless they introduce unknown keys.
