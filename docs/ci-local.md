# Running CI locally

Every CI job should have a local equivalent so failures surface before they
reach GitHub. In this repo, `scripts/ci-local.sh` is the local entrypoint and
`just` recipes wrap it.

## Quick reference

| Recipe           | Mirrors | Notes |
| ---------------- | ------- | ----- |
| `just ci-doctor` | local prerequisite check | reports missing tools and install hints |
| `just ci-quick`  | fast workspace lane | same checks as `just fast` |
| `just ci-audit`  | Jankurai audit lane | runs the audit and zero-caps gate |
| `just ci`        | full local CI parity | runs the full local CI sequence |

## First-time setup

```bash
just ci-doctor
```

Typical tools the doctor expects:

- `cargo`, `rustc`, `npm`, `node`, `just`, `gh`, `jq`, `rg`, `awk`, `python3`
- `gitleaks`, `cargo-audit`, `zizmor`, `syft`
- `latexmk`
- `jankurai`

## Editing the lane

Keep `scripts/ci-local.sh` as the source of truth for local CI flow changes.
When a workflow changes, update the corresponding local command there so the
local and GitHub paths stay aligned.
