# jankurai Repo Score

- Standard: `jankurai`
- Auditor: `1.3.0`
- Schema: `1.7.0`
- Paper edition: `2026.05-ed8`
- Target stack ID: `rust-ts-vite-react-postgres-bounded-python`
- Target stack: `Rust core + TypeScript/React/Vite + PostgreSQL + generated contracts + exception-only Python AI/data service`
- Repo: `.`
- Run ID: `1778937248`
- Started at: `1778937248`
- Elapsed: `3291` ms
- Scope: `full`
- Raw score: `84`
- Final score: `64`
- Decision: `advisory`
- Minimum score: `85`
- Caps applied: `fallback-soup-in-product-code, future-hostile-dead-language-in-product-code`

## Hard Rule Caps

| Rule | Max Score | Applied |
| --- | ---: | --- |
| `no-root-agent-instructions` | 75 | no |
| `no-one-command-setup-or-validation` | 70 | no |
| `no-deterministic-fast-lane` | 65 | no |
| `no-security-lane-on-high-risk-repo` | 60 | no |
| `generated-contracts-or-public-api-drift-untested` | 80 | no |
| `python-direct-product-truth-or-db-ownership` | 72 | no |
| `no-secret-or-dependency-scanning-in-ci` | 78 | no |
| `no-jankurai-audit-lane-in-ci` | 82 | no |
| `jankurai-required-tool-ci-evidence-gap` | 88 | no |
| `non-optimal-product-language-found` | 74 | no |
| `too-much-python-in-product-surface` | 72 | no |
| `boundary-reclassification-evidence-gap` | 72 | no |
| `vibe-placeholders-in-product-code` | 68 | no |
| `fallback-soup-in-product-code` | 70 | yes |
| `future-hostile-dead-language-in-product-code` | 64 | yes |
| `severe-duplication-in-product-code` | 70 | no |
| `generated-zone-mutation-risk` | 76 | no |
| `direct-db-access-from-wrong-layer` | 66 | no |
| `missing-web-e2e-lane` | 82 | no |
| `missing-rendered-ux-qa-lane` | 84 | no |
| `prompt-injection-risk` | 78 | no |
| `overbroad-agent-agency` | 65 | no |
| `secret-like-content-detected` | 60 | no |
| `false-green-test-risk` | 76 | no |
| `destructive-migration-risk` | 70 | no |
| `authz-or-data-isolation-gap` | 78 | no |
| `input-boundary-gap` | 78 | no |
| `agent-tool-supply-chain-gap` | 78 | no |
| `release-readiness-gap` | 80 | no |
| `missing-rust-property-or-integration-tests` | 82 | no |
| `no-agent-friendly-exception-pattern` | 76 | no |
| `missing-agent-readable-docs` | 80 | no |
| `streaming-runtime-drift` | 78 | no |
| `rust-bad-behavior` | 72 | no |
| `sql-bad-behavior` | 72 | no |
| `typescript-bad-behavior` | 72 | no |
| `docker-bad-behavior` | 72 | no |
| `python-bad-behavior` | 72 | no |
| `ci-bad-behavior` | 70 | no |
| `git-bad-behavior` | 70 | no |
| `gittools-bad-behavior` | 70 | no |
| `release-bad-behavior` | 70 | no |
| `web-security-bad-behavior` | 68 | no |
| `repo-rot-bad-behavior` | 88 | no |
| `comment-hygiene-dangerous-residue` | 72 | no |
| `ci-local-parity` | 70 | no |

## Copy-Code Redundancy

- Status: `review` hard=`0` warning=`42` files=`448`
- Policy: min-lines=`10` min-tokens=`100` max-findings=`50` include-tests=`false` strict=`false`
- Duplicate volume: lines=`123` tokens=`372` bytes=`3729`

- Notes:
  - hard classes are limited to exact active-source file matches and substantial exact same-name units
  - warning classes include same-body different-name units and token/block duplication
  - tests, fixtures, stories, config, Docker, and migrations are omitted unless --include-tests is set

| Kind | Severity | Language | Lines | Tokens | Instances | Reason |
| --- | --- | --- | ---: | ---: | --- | --- |
| `ExactUnitDifferentName` | `Warning` | `rust` | 17 | 54 | `crates/memory-benchmark/src/corpus/real_papers/model.rs:219-236, crates/qbank-builder/src/core_types.rs:80-97` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `rust` | 4 | 8 | `crates/jekko-runtime/src/tool/edit/mod.rs:62-66, crates/jekko-runtime/src/tool/read.rs:55-59, crates/jekko-runtime/src/tool/task.rs:52-56, crates/jekko-runtime/src/tool/webfetch.rs:94-98, crates/jekko-runtime/src/tool/write.rs:48-52` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `rust` | 6 | 18 | `crates/xtask/src/publish_release.rs:105-111, crates/xtask/src/publish_release_package.rs:183-189, crates/xtask/src/publish_release_registry.rs:229-235` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `rust` | 10 | 31 | `crates/xtask/src/publish_npm_package.rs:44-54, crates/xtask/src/publish_release_package.rs:142-152` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `rust` | 10 | 28 | `crates/xtask/src/compliance_close.rs:171-181, crates/xtask/src/pr_standards.rs:241-251` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `rust` | 4 | 12 | `crates/xtask/src/close_issues.rs:127-131, crates/xtask/src/compliance_close.rs:183-187, crates/xtask/src/pr_standards.rs:143-147` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `rust` | 7 | 18 | `crates/xtask/src/pr_compliance.rs:110-117, crates/xtask/src/pr_standards.rs:157-164` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `rust` | 6 | 23 | `crates/xtask/src/publish_npm_package.rs:56-62, crates/xtask/src/publish_release_package.rs:154-160` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `rust` | 2 | 5 | `crates/jekko-tui/src/feature_plugins/jankurai/tests.rs:12-14, crates/jekko-tui/src/feature_plugins/jnoccio/tests.rs:14-16, crates/jekko-tui/src/feature_plugins/plugin_manager/tests.rs:7-9, crates/jekko-tui/src/transcript/question/tests.rs:6-8` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `rust` | 6 | 13 | `crates/xtask/src/pr_compliance.rs:102-108, crates/xtask/src/pr_standards.rs:149-155` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `rust` | 1 | 4 | `crates/jekko-runtime/src/tool/bash.rs:45-46, crates/jekko-runtime/src/tool/edit/mod.rs:58-59, crates/jekko-runtime/src/tool/read.rs:51-52, crates/jekko-runtime/src/tool/task.rs:48-49, crates/jekko-runtime/src/tool/webfetch.rs:90-91, crates/jekko-runtime/src/tool/write.rs:44-45` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 4 | `crates/jekko-runtime/src/tool/edit/mod.rs:105-106, crates/jekko-runtime/src/tool/edit/mod.rs:122-123, crates/jekko-runtime/src/tool/edit/mod.rs:137-138, crates/jekko-runtime/src/tool/read.rs:105-106, crates/jekko-runtime/src/tool/read.rs:122-123, crates/jekko-runtime/src/tool/write.rs:71-72` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 5 | 11 | `crates/jankurai-runner/src/locks.rs:53-58, crates/jankurai-runner/src/locks.rs:66-71` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `rust` | 4 | 19 | `crates/xtask/src/close_issues.rs:138-142, crates/xtask/src/compliance_close.rs:194-198` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `rust` | 4 | 18 | `crates/xtask/src/pr_compliance.rs:119-123, crates/xtask/src/pr_management.rs:95-99` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `rust` | 4 | 12 | `crates/xtask/src/pr_compliance.rs:96-100, crates/xtask/src/pr_standards.rs:137-141` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 5 | `crates/xtask/src/commands/package.rs:252-253, crates/xtask/src/commands/package.rs:266-267, crates/xtask/src/commands/package.rs:276-277, crates/xtask/src/commands/package.rs:294-295` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 5 | `crates/jekko-tui/src/components/splash/state.rs:415-416, crates/jekko-tui/src/components/splash/state.rs:424-425, crates/jekko-tui/src/components/splash/state.rs:438-439, crates/jekko-tui/src/components/splash/state.rs:447-448` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 4 | `crates/jekko-tui/src/feature_plugins/sidebar.rs:316-317, crates/jekko-tui/src/feature_plugins/sidebar.rs:338-339, crates/jekko-tui/src/feature_plugins/sidebar.rs:349-350, crates/jekko-tui/src/feature_plugins/sidebar.rs:360-361` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 2 | `crates/jekko-tui/src/feature_plugins/jnoccio/model.rs:97-98, crates/jekko-tui/src/feature_plugins/jnoccio/model.rs:106-107, crates/memory-benchmark/src/types.rs:263-264, crates/memory-benchmark/src/types.rs:302-303` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 2 | 6 | `crates/qbank-builder/src/cli/discover.rs:224-226, crates/qbank-builder/src/full_text_import_detail_support.rs:159-161` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 2 | 5 | `crates/xtask/src/pr_compliance.rs:129-131, crates/xtask/src/pr_standards.rs:172-174` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `rust` | 2 | 5 | `crates/jekko-tui/src/feature_plugins/jnoccio/tests.rs:18-20, crates/jekko-tui/src/feature_plugins/plugin_manager/tests.rs:11-13` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 2 | 4 | `crates/memory-benchmark/src/corpus/real_papers/model.rs:215-217, crates/qbank-builder/src/core_types.rs:50-52` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 2 | `crates/memory-benchmark/src/corpus/real_papers/json_helpers.rs:68-69, crates/memory-benchmark/src/corpus/real_papers/json_helpers.rs:89-90, crates/memory-benchmark/src/corpus/real_papers/json_helpers.rs:96-97` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `rust` | 2 | 3 | `crates/jekko-core/src/keybind/chord.rs:106-108, crates/jekko-core/src/keybind/set.rs:62-64` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 2 | 1 | `crates/qbank-builder/src/fixture.rs:307-309, crates/sandboxctl/src/spec_types.rs:169-171` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 6 | `crates/jekko-tui/src/components/splash/state.rs:261-262, crates/jekko-tui/src/components/splash/state.rs:278-279` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 5 | `crates/jekko-tui/src/components/splash/state.rs:298-299, crates/jekko-tui/src/components/splash/state.rs:351-352` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `rust` | 1 | 5 | `crates/jekko-tui/src/feature_plugins/mod.rs:172-173, crates/jekko-tui/src/feature_plugins/sidebar.rs:298-299` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 5 | `crates/xtask/src/close_stale_prs.rs:244-245, crates/xtask/src/pr_compliance.rs:196-197` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 5 | `crates/xtask/src/publish_build_plan.rs:157-158, crates/xtask/src/publish_build_plan.rs:191-192` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 5 | `crates/jekko-provider/src/transform/variants/efforts.rs:65-66, crates/jekko-provider/src/transform/variants/efforts.rs:90-91` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 4 | `crates/qbank-builder/src/paper_tournament/provenance.rs:202-203, crates/qbank-builder/src/paper_tournament/summary.rs:114-115` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `rust` | 1 | 4 | `crates/xtask/src/close_stale_prs.rs:225-226, crates/xtask/src/pr_compliance.rs:208-209` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 4 | `crates/jekko-tui/src/components/splash/state.rs:408-409, crates/jekko-tui/src/components/splash/state.rs:431-432` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 4 | `crates/jekko-tui/src/components/splash/state.rs:182-183, crates/jekko-tui/src/components/splash/state.rs:204-205` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `rust` | 1 | 2 | `crates/memory-benchmark/src/types.rs:263-264, crates/memory-benchmark/src/types.rs:302-303` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 1 | `crates/jekko-tui/src/feature_plugins/mod.rs:249-250, crates/xtask/src/commands/package.rs:309-310` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 1 | `crates/jekko-provider/src/transform/variants/efforts.rs:49-50, crates/jekko-provider/src/transform/variants/efforts.rs:78-79` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 1 | `crates/xtask/src/close_stale_prs.rs:146-147, crates/xtask/src/pr_compliance.rs:190-191` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 0 | `crates/xtask/src/close_stale_prs.rs:231-232, crates/xtask/src/shared.rs:173-174` | `same body appears under different names across files` |

## Dimensions

| Dimension | Weight | Score | Weighted | Evidence |
| --- | ---: | ---: | ---: | --- |
| Ownership and navigation surface | 13 | 100 | 13.00 | root `AGENTS.md` present; `CODEOWNERS` present |
| Contract and boundary integrity | 13 | 98 | 12.74 | contract surface found; generated contract artifacts found |
| Proof lanes and test routing | 12 | 100 | 12.00 | one-command setup/validation lane found; deterministic fast lane found |
| Security and supply-chain posture | 12 | 86 | 10.32 | lockfile present; secret or dependency scan tooling found |
| Code shape and semantic surface | 12 | 38 | 4.56 | largest authored code file: crates/jekko-tui/src/components/splash/state.rs (491 LOC); most code files stay under 300 LOC |
| Data truth and workflow safety | 8 | 95 | 7.60 | database surface present; structured db boundary manifest present |
| Observability and repair evidence | 8 | 88 | 7.04 | observability libraries or patterns found; ops/observability directory present |
| Context economy and agent instructions | 7 | 100 | 7.00 | root `AGENTS.md` present; root `AGENTS.md` stays short |
| Jankurai tool adoption and CI replacement | 7 | 29 | 2.03 | control-plane files present; applicable=15 |
| Python containment and polyglot hygiene | 4 | 100 | 4.00 | no Python files in scope |
| Build speed signals | 4 | 95 | 3.80 | build acceleration markers found; targeted test/build commands found |

## Reference Profile Structure

- Applicable cells: `8` canonical=`8` noncanonical=`0` guidance missing=`0`

| Cell | Status | Canonical | Detected | Aliases | Guidance | Owner | Proof lane | Agent fix |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `web` | `not_applicable` | `apps/web/` | `-` | `frontend/, ui/, packages/web/, packages/ui/` | `not_required` | `apps/web` | `rendered UX / Playwright` | `no action` |
| `api` | `not_applicable` | `apps/api/` | `-` | `api/, server/, backend/` | `not_required` | `apps/api` | `edge handler / contract tests` | `no action` |
| `domain` | `canonical` | `crates/domain/` | `crates/domain` | `domain/, core/` | `present` | `crates/domain` | `unit / property tests` | `keep `crates/domain/AGENTS.md` aligned with owns / forbidden / proof lane guidance` |
| `application` | `canonical` | `crates/application/` | `crates/application` | `application/, usecases/, use-cases/` | `present` | `crates/application` | `use-case / authz tests` | `keep `crates/application/AGENTS.md` aligned with owns / forbidden / proof lane guidance` |
| `adapters` | `canonical` | `crates/adapters/` | `crates/adapters` | `adapters/, infra/, integrations/` | `present` | `crates/adapters` | `adapter integration tests` | `keep `crates/adapters/AGENTS.md` aligned with owns / forbidden / proof lane guidance` |
| `workers` | `canonical` | `crates/workers/` | `crates/workers` | `workers/, jobs/, scheduler/, queue/` | `present` | `crates/workers` | `workflow / replay tests` | `keep `crates/workers/AGENTS.md` aligned with owns / forbidden / proof lane guidance` |
| `contracts` | `canonical` | `contracts/` | `contracts` | `openapi/, protobuf/, json-schema/, generated/` | `present` | `contracts` | `generation / drift checks` | `keep `contracts/AGENTS.md` aligned with owns / forbidden / proof lane guidance` |
| `db` | `canonical` | `db/` | `db` | `migrations/, constraints/, sql/` | `present` | `db` | `migration / constraint tests` | `keep `db/AGENTS.md` aligned with owns / forbidden / proof lane guidance` |
| `python-ai` | `canonical` | `python/ai-service/` | `python, python/ai-service` | `python/, ai-service/, evals/, embeddings/, model/` | `present` | `python/ai-service` | `eval / contract tests` | `keep `python/ai-service/AGENTS.md` aligned with owns / forbidden / proof lane guidance` |
| `ops` | `canonical` | `ops/` | `.github, .github/workflows, ops` | `.github/, .github/workflows/, ci/, release/, observability/, security/` | `present` | `ops` | `security lane / workflow lint` | `keep `ops/AGENTS.md` aligned with owns / forbidden / proof lane guidance` |

## Rendered UX QA

- Web surface: `false`
- Layered UX lane: `true`
- Missing: `none`
- Tuiwright TUI flows: `2` flow(s) across `2` file(s); assertions=`2` actions=`3` artifacts=`screenshot=2, trace_path=1`

## Tool Adoption

- Control plane present: `true`
- Applicable tools: `15`
- Configured: `14`
- CI evidence: `0`
- Artifact verified: `0`
- Replaced count: `0`
- Missing CI evidence: `audit-ci, proof-routing, proofbind, proofmark-rust, copy-code, security, ci-bad-behavior, git-bad-behavior, release-bad-behavior, contract-drift, rust-witness, authz-matrix, agent-tool-supply, release-readiness, cost-budget`

| Tool | Category | Mode | Status | Replaced | Artifacts |
| --- | --- | --- | --- | --- | --- |
| `audit-ci` | `audit` | `auto` | `configured` | `manual repo scoring, ad hoc score gates` | `agent/repo-score.json, agent/repo-score.md` |
| `proof-routing` | `proof` | `auto` | `configured` | `ad hoc proof lane selection, manual proof receipts` | `agent/repo-score.json, agent/repo-score.md, target/jankurai/repair-queue.jsonl` |
| `proofbind` | `proof` | `auto` | `configured` | `manual changed-surface routing, ad hoc proof obligation lists` | `target/jankurai/proofbind/surface-witness.json, target/jankurai/proofbind/obligations.json` |
| `proofmark-rust` | `proof` | `auto` | `configured` | `line-only coverage review, manual in-diff mutation review` | `target/jankurai/proofmark/proofmark-receipt.json, target/jankurai/proofmark/proof-receipt.json` |
| `copy-code` | `audit` | `auto` | `missing` | `ad hoc copy-code review, manual duplication triage` | `target/jankurai/copy-code.json, target/jankurai/copy-code.md` |
| `security` | `security` | `auto` | `configured` | `gitleaks, dependency review, SBOM/provenance` | `target/jankurai/security/evidence.json` |
| `ci-bad-behavior` | `security` | `auto` | `configured` | `mutable workflow refs, secret echo/debug workflow checks, non-blocking security scans` | `target/jankurai/language-bad-behavior.log` |
| `git-bad-behavior` | `audit` | `auto` | `configured` | `destructive git automation, force-push release scripts, hidden stash-based state` | `target/jankurai/language-bad-behavior.log` |
| `release-bad-behavior` | `release` | `auto` | `configured` | `manual release checklist, ad hoc tag and artifact review, manual provenance review` | `target/jankurai/language-bad-behavior.log` |
| `ux-qa` | `ux` | `auto` | `not_applicable` | `playwright, axe-core, visual baselines` | `target/jankurai/ux-qa.json` |
| `db-migration-analyze` | `db` | `auto` | `not_applicable` | `manual migration review` | `target/jankurai/migration-report.json` |
| `contract-drift` | `contract` | `auto` | `configured` | `handwritten contract drift checks, openapi diff` | `agent/repo-score.json, agent/repo-score.md` |
| `rust-witness` | `rust` | `auto` | `configured` | `manual witness graphing` | `target/jankurai/rust/witness-graph.json` |
| `vibe-coverage` | `audit` | `auto` | `not_applicable` | `manual vibe-coding coverage spreadsheet` | `target/jankurai/vibe-coverage.json, target/jankurai/vibe-coverage.md` |
| `coverage-evidence` | `proof` | `auto` | `not_applicable` | `manual coverage report review, ad hoc mutation survivor review` | `target/jankurai/coverage/coverage-audit.json, target/jankurai/coverage/coverage-audit.md` |
| `authz-matrix` | `security` | `auto` | `configured` | `manual authz matrix review` | `agent/repo-score.json, agent/repo-score.md` |
| `input-boundary` | `security` | `auto` | `not_applicable` | `manual unsafe sink review` | `agent/repo-score.json, agent/repo-score.md` |
| `agent-tool-supply` | `security` | `auto` | `configured` | `manual MCP/tool trust review` | `agent/repo-score.json, agent/repo-score.md` |
| `release-readiness` | `release` | `auto` | `configured` | `manual launch checklist` | `agent/repo-score.json, agent/repo-score.md` |
| `cost-budget` | `release` | `auto` | `configured` | `manual spend review` | `agent/repo-score.json, agent/repo-score.md` |

## Boundary manifest (ingested)

- Path: `agent/boundaries.toml`
- Stack: `rust-ts-postgres-bounded-python` · version: `0.4.0`
- Queue path counts — adapter: `2`, event_contract: `1`, generated_type: `1`, client_marker: `7`, streaming_exception: `1`
- Content fingerprint: `sha256:65fe11e0be72e3ce25bed8fa55e239acc39c55520cd41c0344c7aab23eb0573d`

## Boundary Reclassifications

No audited runtime boundary reclassifications declared.

## Findings

1. `medium` `shape` `.`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:shape` `soft` confidence `0.76`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: `Code shape and semantic surface` scored 38 below the standard floor of 85
   Fix: split large or ambiguous authored code into smaller semantic modules with focused tests
   Rerun: `just fast`
   Fingerprint: `sha256:fd993662fc1b84fb6f5bdfac4701012a2da720c2c0f3adc0e6c41eb926c8c3bf`
   Evidence: largest authored code file: crates/jekko-tui/src/components/splash/state.rs (491 LOC), most code files stay under 300 LOC, copy-code advisory classes found: 42 (advisory only, no score impact), fallback soup marker found
2. `high` `vibe` `crates/jankurai-runner/src/classifier.rs:81`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: fallback soup detected in product code
   Fix: collapse fallback chains into explicit typed states with bounded retry policy, telemetry, and documented repair guidance
   Rerun: `just fast`
   Fingerprint: `sha256:bcdd63ac524e27ecb31e119f8647eb0e5c39ec06253f83e9d0a32372abaf7a98`
   Evidence: crates/jankurai-runner/src/classifier.rs:81 let raw_findings: Vec<RawFinding> = parsed.findings.unwrap_or_default();
3. `high` `vibe` `crates/jekko-tui/src/lib.rs:401`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `legacy` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:70dd663df90aac13d9b38b816c11e82e47ab99a24a00adb1d6520df0fe8c0c63`
   Evidence: crates/jekko-tui/src/lib.rs:401, future-hostile/dead-language term `legacy` appears

## Policy

- Policy file: `./agent/audit-policy.toml`
- Minimum score: `85`
- Fail on: `critical, high`

## Agent Fix Queue

1. `high` `HLT-001-DEAD-MARKER` `crates/jankurai-runner/src/classifier.rs` - collapse fallback chains into explicit typed states with bounded retry policy, telemetry, and documented repair guidance
   Route: `Entropy`/`fast`
2. `high` `HLT-001-DEAD-MARKER` `crates/jekko-tui/src/lib.rs` - remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Route: `Entropy`/`fast`
3. `medium` `HLT-001-DEAD-MARKER` `.` - split large or ambiguous authored code into smaller semantic modules with focused tests
   Route: `Entropy`/`fast`
