# GitLab CI/CD — parity transfer from `.github/workflows/`

This directory contains a hand-written, full-parity translation of all
sixteen GitHub Actions workflows under `.github/workflows/` into GitLab CI
YAML. The entry point is the repo-root `.gitlab-ci.yml`, which uses
`include:` to compose the workflow-specific files below.

## Parity matrix

| GitHub workflow | GitHub jobs | GitLab file | GitLab jobs | Note |
|---|---|---|---|---|
| `check-encrypted-paths.yml` | `verify-encryption` | `static.yml` | `check-encrypted-paths` | 1:1 |
| `close-issues.yml` | `close` | `scheduled.yml` | `scheduled:close-issues` | Cron set in GitLab Schedules UI; `SCHEDULED_JOB=close-issues` |
| `close-stale-prs.yml` | `close-stale-prs` | `scheduled.yml` | `scheduled:close-stale-prs` | Cron `0 6 * * *`; `SCHEDULED_JOB=close-stale-prs` |
| `compliance-close.yml` | `close-non-compliant` | `scheduled.yml` | `scheduled:compliance-close` | Cron `*/30 * * * *`; `SCHEDULED_JOB=compliance-close` |
| `deploy.yml` | `noop` | `deploy.yml` | `deploy` | Triggered via Run Pipeline UI (manual) |
| `generate.yml` | `generate` | `generate.yml` | `generate` | Push to `dev` only |
| `jankurai.yml` | `audit`, `sandbox-backends` (3×matrix) | `jankurai.yml` | `jankurai:audit`, `jankurai:sandbox-backends` (3×parallel) | SARIF surfaced via `reports.sast` |
| `nix-eval.yml` | `nix-eval` | `nix.yml` | `nix-eval` | Push to `dev` + MR to `dev` |
| `nix-hashes.yml` | `compute-hash` (4×matrix), `update-hashes` | `nix.yml` | `nix-hashes:compute:{linux-x64,linux-arm,macos-intel,macos-arm}`, `nix-hashes:update` | Update job consumes the four compute-job artifacts via `needs:` |
| `notify-discord.yml` | `notify` | `notify.yml` | `notify-discord` | Runs on tag pipeline matching `^v\d+\.\d+\.\d+` |
| `parity.yml` | `rust-parity-gates`, `guard-advisory` | `parity.yml` | `parity:rust-parity-gates`, `parity:guard-advisory` | Both lanes preserve `RUSTFLAGS=-D warnings` |
| `pr-standards.yml` | `check-standards`, `check-compliance` | `pr-standards.yml` | `pr-standards:check-standards`, `pr-standards:check-compliance` | GitHub `pull_request_target` → GitLab `merge_request_event`; event JSON synthesised in `before_script` |
| `security.yml` | `scan` | `security.yml` | `security:scan` | TruffleHog skipped on manual-dispatch (parity with `if: !workflow_dispatch`) |
| `stats.yml` | `stats` | `scheduled.yml` | `scheduled:stats` | Cron `0 12 * * *`; `SCHEDULED_JOB=stats`; gated on `$CI_PROJECT_PATH == "neverhuman/jekko" && $JEKKO_AUTOMATION_ENABLED == "1"` |
| `test.yml` | `unit` (1×matrix linux), `tui` | `test.yml` | `test:unit`, `test:tui` | JUnit surfaced via `reports.junit` (parity with `mikepenz/action-junit-report`) |
| `typecheck.yml` | `typecheck` | `static.yml` | `typecheck` | 1:1 |

**Total jobs** transferred: 24 (counting matrix expansions).

## Trigger mapping reference

| GitHub event | GitLab equivalent |
|---|---|
| `on: pull_request` | `rules: - if: $CI_PIPELINE_SOURCE == "merge_request_event"` |
| `on: pull_request_target` | Same as `pull_request` — GitLab's MR pipelines already run in the target project's context |
| `on: push: branches: [main]` | `rules: - if: $CI_COMMIT_BRANCH == "main" && $CI_PIPELINE_SOURCE == "push"` |
| `on: push: branches: [dev]` | `rules: - if: $CI_COMMIT_BRANCH == "dev" && $CI_PIPELINE_SOURCE == "push"` |
| `on: schedule: cron: "..."` | `rules: - if: $CI_PIPELINE_SOURCE == "schedule"`; cron set in Settings → CI/CD → Schedules |
| `on: workflow_dispatch` | `rules: - if: $CI_PIPELINE_SOURCE == "web" \|\| $CI_PIPELINE_SOURCE == "api"` |
| `on: release: types: [released]` | `rules: - if: $CI_COMMIT_TAG =~ /^v\d+\.\d+\.\d+/` (paired with the project's tag-on-release flow) |
| `concurrency:` group + `cancel-in-progress` | `interruptible: true` (set at `default:` level in `.gitlab-ci.yml`) + per-job `resource_group:` |

## Third-party action substitutions

| GitHub action | GitLab substitution |
|---|---|
| `actions/checkout@v4` / `@v4.2.2` | Implicit GitLab clone. `GIT_DEPTH` and `GIT_STRATEGY` configured per job; full-history jobs set `GIT_DEPTH: "0"` |
| `dtolnay/rust-toolchain@stable` | `rustup default stable` in the `.rust_setup` block in `_shared.yml` |
| `Swatinem/rust-cache@v2.8.2` | GitLab `cache:` keyed on `Cargo.lock` + `rust-toolchain.toml`, paths `.cargo/` + `target/` |
| `actions/upload-artifact@v4` / `@v7` | `artifacts:` block with `paths:` + `expire_in:` |
| `actions/download-artifact@v8` | `needs: - job: <upstream>: artifacts: true` |
| `actions/create-github-app-token@v2` | `ops/ci/setup-git-committer.sh` (mints an installation token from `JEKKO_APP_ID`+`JEKKO_APP_SECRET`) |
| `mikepenz/action-junit-report@v6` | `artifacts.reports.junit:` |
| `github/codeql-action/upload-sarif@v4.35.4` | `artifacts.reports.sast:` |
| `nixbuild/nix-quick-install-action@v34` | Inline `curl -sSfL https://install.determinate.systems/nix \| sh ...` in `before_script` |
| `trufflesecurity/trufflehog@main` | Inline install via the upstream `install.sh` |
| `anchore/sbom-action@v0.24.0` | `syft` inline install |
| `anchore/scan-action@v3` | `grype` inline install |

## Composite action substitution

`.github/actions/setup-git-committer/action.yml` (creates GitHub App token,
configures committer) is replaced by `ops/ci/setup-git-committer.sh`. The
script reads `JEKKO_APP_ID` + `JEKKO_APP_SECRET` from CI/CD variables and
exports `APP_TOKEN` + `APP_SLUG` for downstream steps. When the variables
are absent, it falls back to a generic `gitlab-ci-bot` identity (no GitHub
push capability) so MRs from forks still pass.

## CI/CD variables required

Set under **Settings → CI/CD → Variables**:

| Variable | Type | Workflow(s) that need it |
|---|---|---|
| `JEKKO_APP_ID` | Variable | `generate.yml`, `jankurai.yml` (badge refresh), `nix-hashes.yml` (update), `stats.yml` |
| `JEKKO_APP_SECRET` | Masked (multiline PEM) | Same as above |
| `DISCORD_WEBHOOK` | Masked | `notify.yml` |
| `JEKKO_AUTOMATION_ENABLED` | Variable | `scheduled.yml` (stats gate) |
| `GITHUB_TOKEN` | Masked | All scheduled housekeeping jobs that use `gh` CLI against the GitHub mirror |

Defaults provided by GitLab (no setup needed): `CI_JOB_TOKEN`,
`CI_COMMIT_BRANCH`, `CI_COMMIT_TAG`, `CI_PIPELINE_SOURCE`,
`CI_MERGE_REQUEST_*`, `CI_PROJECT_PATH`, `CI_PROJECT_NAMESPACE`.

## Schedules to configure

Under **Settings → CI/CD → Schedules**, create four entries — each with the
corresponding `SCHEDULED_JOB` variable set:

| Schedule | Cron | `SCHEDULED_JOB` |
|---|---|---|
| close-issues | `0 2 * * *` | `close-issues` |
| close-stale-prs | `0 6 * * *` | `close-stale-prs` |
| compliance-close | `*/30 * * * *` | `compliance-close` |
| stats | `0 12 * * *` | `stats` |

## Differences vs GitHub Actions

These behaviours have no direct GitLab equivalent and are handled by
adjacent project configuration:

- **Per-job `permissions:`** — GitLab uses project-level permissions instead
  of job-level permission tokens. The jankurai badge-refresh job in
  `jankurai.yml` pushes via `CI_PUSH_TOKEN` (set up as a Project Access
  Token with `write_repository` scope) instead of GitHub's per-job
  `contents: write`.
- **Composite actions** — replaced with shell helpers under `ops/ci/`.
- **Matrix on different runners** — GitLab can't compute a runner tag from
  a matrix entry. The `jankurai:sandbox-backends` macOS row guards against
  running on a linux runner; configure a `macos` tagged GitLab Runner if
  full parity is needed.
- **GitHub App tokens** — `JEKKO_APP_SECRET` is a multiline PEM private key.
  Store it in GitLab as a **File** variable (not a regular masked string)
  so `\n` separators survive the round-trip.

## Validating locally

```bash
# Static YAML lint
python3 - <<'PY'
import yaml
class L(yaml.SafeLoader): pass
def keep(loader, tag_suffix, node):
    if isinstance(node, yaml.ScalarNode): return loader.construct_scalar(node)
    if isinstance(node, yaml.SequenceNode): return loader.construct_sequence(node)
    return loader.construct_mapping(node)
L.add_multi_constructor('!', keep)
import glob
for p in [".gitlab-ci.yml"] + sorted(glob.glob(".gitlab/ci/*.yml")):
    with open(p) as f: yaml.load(f, Loader=L)
    print(p, "OK")
PY

# GitLab-side lint (requires gitlab-ci-local or GitLab CI/CD Lint API)
gitlab-ci-local --validate
```
