# jankurai Repo Score

- Standard: `jankurai`
- Auditor: `1.3.0`
- Schema: `1.7.0`
- Paper edition: `2026.05-ed8`
- Target stack ID: `rust-ts-vite-react-postgres-bounded-python`
- Target stack: `Rust core + TypeScript/React/Vite + PostgreSQL + generated contracts + exception-only Python AI/data service`
- Repo: `.`
- Run ID: `1778774073`
- Started at: `1778774073`
- Elapsed: `5096` ms
- Scope: `full`
- Raw score: `76`
- Final score: `64`
- Decision: `advisory`
- Minimum score: `85`
- Caps applied: `vibe-placeholders-in-product-code, fallback-soup-in-product-code, future-hostile-dead-language-in-product-code, missing-web-e2e-lane, missing-rendered-ux-qa-lane, agent-tool-supply-chain-gap, typescript-bad-behavior`

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
| `vibe-placeholders-in-product-code` | 68 | yes |
| `fallback-soup-in-product-code` | 70 | yes |
| `future-hostile-dead-language-in-product-code` | 64 | yes |
| `severe-duplication-in-product-code` | 70 | no |
| `generated-zone-mutation-risk` | 76 | no |
| `direct-db-access-from-wrong-layer` | 66 | no |
| `missing-web-e2e-lane` | 82 | yes |
| `missing-rendered-ux-qa-lane` | 84 | yes |
| `prompt-injection-risk` | 78 | no |
| `overbroad-agent-agency` | 65 | no |
| `secret-like-content-detected` | 60 | no |
| `false-green-test-risk` | 76 | no |
| `destructive-migration-risk` | 70 | no |
| `authz-or-data-isolation-gap` | 78 | no |
| `input-boundary-gap` | 78 | no |
| `agent-tool-supply-chain-gap` | 78 | yes |
| `release-readiness-gap` | 80 | no |
| `missing-rust-property-or-integration-tests` | 82 | no |
| `no-agent-friendly-exception-pattern` | 76 | no |
| `missing-agent-readable-docs` | 80 | no |
| `streaming-runtime-drift` | 78 | no |
| `rust-bad-behavior` | 72 | no |
| `sql-bad-behavior` | 72 | no |
| `typescript-bad-behavior` | 72 | yes |
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

- Status: `review` hard=`0` warning=`18` files=`508`
- Policy: min-lines=`10` min-tokens=`100` max-findings=`50` include-tests=`false` strict=`false`
- Duplicate volume: lines=`72` tokens=`213` bytes=`2110`

- Notes:
  - hard classes are limited to exact active-source file matches and substantial exact same-name units
  - warning classes include same-body different-name units and token/block duplication
  - tests, fixtures, stories, config, Docker, and migrations are omitted unless --include-tests is set

| Kind | Severity | Language | Lines | Tokens | Instances | Reason |
| --- | --- | --- | ---: | ---: | --- | --- |
| `ExactUnitDifferentName` | `Warning` | `rust` | 17 | 54 | `crates/memory-benchmark/src/corpus/real_papers/model.rs:216-233, crates/qbank-builder/src/lib.rs:352-369` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `typescript` | 13 | 31 | `packages/jekko/src/cli/cmd/tui/context/capability.ts:232-245, packages/jekko/src/cli/cmd/tui/context/jankurai-baseline.ts:99-112` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `rust` | 7 | 29 | `crates/memory-benchmark/src/corpus/real_papers/score.rs:270-277, crates/memory-benchmark/src/corpus/real_papers/validation.rs:752-759` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `rust` | 7 | 17 | `crates/cogcore/src/hash.rs:11-18, crates/memory-benchmark/src/hash.rs:12-19` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `typescript` | 2 | 9 | `packages/jekko/src/cli/cmd/debug/agent.ts:116-118, packages/jekko/src/cli/cmd/tui/context/capability.ts:82-84, packages/jekko/src/cli/cmd/tui/context/jankurai-baseline.ts:40-42, packages/jekko/src/cli/cmd/tui/context/jankurai-history.ts:65-67` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 5 | 11 | `crates/jankurai-runner/src/locks.rs:53-58, crates/jankurai-runner/src/locks.rs:66-71` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `typescript` | 4 | 20 | `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-capability.tsx:40-44, packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-history.tsx:83-87` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `typescript` | 2 | 9 | `packages/jekko/src/cli/cmd/tui/context/capability.ts:82-84, packages/jekko/src/cli/cmd/tui/context/jankurai-baseline.ts:40-42, packages/jekko/src/cli/cmd/tui/context/jankurai-history.ts:65-67` | `same-name semantic unit copied across multiple files` |
| `ExactUnitSameName` | `Warning` | `typescript` | 2 | 10 | `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/empty-hero.tsx:65-67, packages/jekko/src/cli/ui.ts:57-59` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 2 | 6 | `crates/qbank-builder/src/full_text_import.rs:556-558, crates/qbank-builder/src/main.rs:265-267` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `rust` | 2 | 5 | `crates/cogcore/src/hash.rs:21-23, crates/memory-benchmark/src/hash.rs:29-31` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 2 | 4 | `crates/memory-benchmark/src/corpus/real_papers/model.rs:212-214, crates/qbank-builder/src/lib.rs:322-324` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 2 | `crates/memory-benchmark/src/corpus/real_papers/json_helpers.rs:68-69, crates/memory-benchmark/src/corpus/real_papers/json_helpers.rs:89-90, crates/memory-benchmark/src/corpus/real_papers/json_helpers.rs:96-97` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 2 | 1 | `crates/qbank-builder/src/fixture.rs:308-310, crates/sandboxctl/src/spec_types.rs:169-171` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `rust` | 1 | 2 | `crates/memory-benchmark/src/types.rs:263-264, crates/memory-benchmark/src/types.rs:302-303` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 1 | `crates/qbank-builder/src/paper_tournament.rs:2692-2693, crates/qbank-builder/src/paper_tournament.rs:2702-2703` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 1 | `crates/qbank-builder/src/paper_tournament.rs:361-362, crates/qbank-builder/src/paper_tournament.rs:2803-2804` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 1 | `crates/qbank-builder/src/paper_tournament.rs:492-493, crates/qbank-builder/src/paper_tournament.rs:1262-1263` | `same body appears under different names across files` |

## Dimensions

| Dimension | Weight | Score | Weighted | Evidence |
| --- | ---: | ---: | ---: | --- |
| Ownership and navigation surface | 13 | 100 | 13.00 | root `AGENTS.md` present; `CODEOWNERS` present |
| Contract and boundary integrity | 13 | 98 | 12.74 | contract surface found; generated contract artifacts found |
| Proof lanes and test routing | 12 | 80 | 9.60 | one-command setup/validation lane found; deterministic fast lane found |
| Security and supply-chain posture | 12 | 86 | 10.32 | lockfile present; secret or dependency scan tooling found |
| Code shape and semantic surface | 12 | 0 | 0.00 | largest authored code file: crates/qbank-builder/src/paper_tournament.rs (2919 LOC); code file exceeds 500 LOC |
| Data truth and workflow safety | 8 | 95 | 7.60 | database surface present; structured db boundary manifest present |
| Observability and repair evidence | 8 | 88 | 7.04 | observability libraries or patterns found; ops/observability directory present |
| Context economy and agent instructions | 7 | 100 | 7.00 | root `AGENTS.md` present; root `AGENTS.md` stays short |
| Jankurai tool adoption and CI replacement | 7 | 10 | 0.70 | control-plane files present; applicable=17 |
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

- Web surface: `true`
- Layered UX lane: `false`
- Missing: `design token discipline`
- Tuiwright TUI flows: `1` flow(s) across `1` file(s); assertions=`1` actions=`3` artifacts=`screenshot=1`

## Tool Adoption

- Control plane present: `true`
- Applicable tools: `17`
- Configured: `0`
- CI evidence: `0`
- Artifact verified: `0`
- Replaced count: `0`
- Missing CI evidence: `audit-ci, proof-routing, proofbind, proofmark-rust, copy-code, security, ci-bad-behavior, git-bad-behavior, release-bad-behavior, ux-qa, contract-drift, rust-witness, authz-matrix, input-boundary, agent-tool-supply, release-readiness, cost-budget`

| Tool | Category | Mode | Status | Replaced | Artifacts |
| --- | --- | --- | --- | --- | --- |
| `audit-ci` | `audit` | `auto` | `missing` | `manual repo scoring, ad hoc score gates` | `agent/repo-score.json, agent/repo-score.md` |
| `proof-routing` | `proof` | `auto` | `missing` | `ad hoc proof lane selection, manual proof receipts` | `agent/repo-score.json, agent/repo-score.md, target/jankurai/repair-queue.jsonl` |
| `proofbind` | `proof` | `auto` | `missing` | `manual changed-surface routing, ad hoc proof obligation lists` | `target/jankurai/proofbind/surface-witness.json, target/jankurai/proofbind/obligations.json` |
| `proofmark-rust` | `proof` | `auto` | `missing` | `line-only coverage review, manual in-diff mutation review` | `target/jankurai/proofmark/proofmark-receipt.json, target/jankurai/proofmark/proof-receipt.json` |
| `copy-code` | `audit` | `auto` | `missing` | `ad hoc copy-code review, manual duplication triage` | `target/jankurai/copy-code.json, target/jankurai/copy-code.md` |
| `security` | `security` | `auto` | `missing` | `gitleaks, dependency review, SBOM/provenance` | `target/jankurai/security/evidence.json` |
| `ci-bad-behavior` | `security` | `auto` | `missing` | `mutable workflow refs, secret echo/debug workflow checks, non-blocking security scans` | `target/jankurai/language-bad-behavior.log` |
| `git-bad-behavior` | `audit` | `auto` | `missing` | `destructive git automation, force-push release scripts, hidden stash-based state` | `target/jankurai/language-bad-behavior.log` |
| `release-bad-behavior` | `release` | `auto` | `missing` | `manual release checklist, ad hoc tag and artifact review, manual provenance review` | `target/jankurai/language-bad-behavior.log` |
| `ux-qa` | `ux` | `auto` | `missing` | `playwright, axe-core, visual baselines` | `target/jankurai/ux-qa.json` |
| `db-migration-analyze` | `db` | `auto` | `not_applicable` | `manual migration review` | `target/jankurai/migration-report.json` |
| `contract-drift` | `contract` | `auto` | `missing` | `handwritten contract drift checks, openapi diff` | `agent/repo-score.json, agent/repo-score.md` |
| `rust-witness` | `rust` | `auto` | `missing` | `manual witness graphing` | `target/jankurai/rust/witness-graph.json` |
| `vibe-coverage` | `audit` | `auto` | `not_applicable` | `manual vibe-coding coverage spreadsheet` | `target/jankurai/vibe-coverage.json, target/jankurai/vibe-coverage.md` |
| `coverage-evidence` | `proof` | `auto` | `not_applicable` | `manual coverage report review, ad hoc mutation survivor review` | `target/jankurai/coverage/coverage-audit.json, target/jankurai/coverage/coverage-audit.md` |
| `authz-matrix` | `security` | `auto` | `missing` | `manual authz matrix review` | `agent/repo-score.json, agent/repo-score.md` |
| `input-boundary` | `security` | `auto` | `missing` | `manual unsafe sink review` | `agent/repo-score.json, agent/repo-score.md` |
| `agent-tool-supply` | `security` | `auto` | `missing` | `manual MCP/tool trust review` | `agent/repo-score.json, agent/repo-score.md` |
| `release-readiness` | `release` | `auto` | `missing` | `manual launch checklist` | `agent/repo-score.json, agent/repo-score.md` |
| `cost-budget` | `release` | `auto` | `missing` | `manual spend review` | `agent/repo-score.json, agent/repo-score.md` |

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
   Reason: `Code shape and semantic surface` scored 0 below the standard floor of 85
   Fix: split large or ambiguous authored code into smaller semantic modules with focused tests
   Rerun: `just fast`
   Fingerprint: `sha256:408afe7569bff7459e65389654968cba17f98e294b8bee81a2af80cdb6f1170b`
   Evidence: largest authored code file: crates/qbank-builder/src/paper_tournament.rs (2919 LOC), code file exceeds 500 LOC, code file exceeds 1000 LOC, most code files stay under 300 LOC
2. `high` `security` `.jekko/daemons/jnoccio-route-metadata-smoke.zyal:1`
   Rule: `HLT-024-AGENT-TOOL-SUPPLY-GAP`
   Check: `HLT-024-AGENT-TOOL-SUPPLY-GAP:security` `hard` confidence `0.88`
   Route: TLR `Security, secrets, agency`, lane `security`, owner `agent`
   Docs: `docs/audit-rubric.md#top-level-risk-mapping`
   Matched term: `agent/zyal`
   Reason: canonical ZYAL repository root
   Fix: move the runbook to `agent/zyal/` and rename it to `*.zyal`
   Rerun: `just security`
   Fingerprint: `sha256:ada5b6630bce1ba6ed34b0b825b7c275b78189ca29ef59cccee7feb3898e4c00`
   Evidence: path=.jekko/daemons/jnoccio-route-metadata-smoke.zyal, supported_contract_version=2.4.0, release_tag=v1.0.0
3. `high` `security` `.jekko/daemons/jnoccio-route-metadata-smoke.zyal:1`
   Rule: `HLT-024-AGENT-TOOL-SUPPLY-GAP`
   Check: `HLT-024-AGENT-TOOL-SUPPLY-GAP:security` `hard` confidence `0.88`
   Route: TLR `Security, secrets, agency`, lane `security`, owner `agent`
   Docs: `docs/audit-rubric.md#top-level-risk-mapping`
   Matched term: `<<<ZYAL`
   Reason: non-open sentinel
   Fix: keep the runbook envelope at the top of the file after optional comments
   Rerun: `just security`
   Fingerprint: `sha256:7cb1b17dd05114fb97056bf6c5edf5eff624bcd590b0b4dfa889dafff1b2b632`
   Evidence: supported_contract_version=2.4.0, runtime_sentinel_version=v1
4. `high` `context` `agent/owner-map.json`
   Rule: `HLT-003-OWNERLESS-PATH`
   Check: `HLT-003-OWNERLESS-PATH:context` `hard` confidence `0.88`
   Route: TLR `Context/setup`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#ownership-boundaries`
   Reason: path `TUI_UPGRADE.md` has no owner-map route
   Fix: add the narrowest stable prefix for this path to `agent/owner-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:1d613b88943a7c7ef28f2aedecc38decdd75b52403ea251a3b586413041d1067`
   Evidence: TUI_UPGRADE.md
5. `medium` `proof` `agent/test-map.json`
   Rule: `HLT-004-UNMAPPED-PROOF`
   Check: `HLT-004-UNMAPPED-PROOF:proof` `soft` confidence `0.76`
   Route: TLR `Verification`, lane `fast`, owner `workspace`
   Docs: `agent/JANKURAI_STANDARD.md#proof-lanes`
   Reason: `Proof lanes and test routing` scored 80 below the standard floor of 85
   Fix: route each owned path to a deterministic proof command and make the lane executable in CI
   Rerun: `just fast`
   Fingerprint: `sha256:4c32cd60475fd4c803e64a568543f5c081a156b33b97fa688f83b5a578024262`
   Evidence: one-command setup/validation lane found, deterministic fast lane found, test runner present in automation surface, GitHub workflow files present
6. `high` `proof` `agent/test-map.json`
   Rule: `HLT-004-UNMAPPED-PROOF`
   Check: `HLT-004-UNMAPPED-PROOF:proof` `hard` confidence `0.88`
   Route: TLR `Verification`, lane `fast`, owner `workspace`
   Docs: `agent/JANKURAI_STANDARD.md#proof-lanes`
   Reason: path `TUI_UPGRADE.md` has no test-map proof route
   Fix: add the narrowest stable prefix and runnable proof command to `agent/test-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:de292aadb247e6711a56a7b3bbf8b0322dc0fe815b9d3de3316d2adfd145c07d`
   Evidence: TUI_UPGRADE.md
7. `high` `test` `apps/web`
   Rule: `HLT-013-RENDERED-UX-GAP`
   Check: `HLT-013-RENDERED-UX-GAP:test` `hard` confidence `0.88`
   Route: TLR `Verification and rendered UX`, lane `web`, owner `apps`
   Docs: `docs/testing.md`
   Reason: web surface lacks a Playwright/Cypress e2e lane
   Fix: add Playwright e2e tests for critical user flows and wire them into the fast or CI proof map
   Rerun: `just ux-qa`
   Fingerprint: `sha256:baba171f944c5384a440bd31dbe0783c2a295323e516450d3b28505654cb406d`
   Evidence: web surface detected
8. `high` `ux-qa` `apps/web`
   Rule: `HLT-013-RENDERED-UX-GAP`
   Check: `HLT-013-RENDERED-UX-GAP:ux-qa` `hard` confidence `0.88`
   Route: TLR `Verification and rendered UX`, lane `web`, owner `apps`
   Docs: `docs/testing.md`
   Reason: web surface lacks layered rendered UX QA evidence
   Fix: add Storybook state coverage, Playwright screenshots, visual review or `@jankurai/ux-qa`, accessibility scans, CLS checks, generated mocks, and design tokens
   Rerun: `just ux-qa`
   Fingerprint: `sha256:571d35c2e730a393b782bac14825b197c0543920bb21967079d264ac602ea5b1`
   Evidence: rendered UX QA lane missing
9. `high` `vibe` `crates/cogcore/src/ingest/mod.rs:23`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `stub` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:3d7c81e719bce88920cddcb443507e6d7f4ea4120d3153ef4dacc9c185d95160`
   Evidence: crates/cogcore/src/ingest/mod.rs:23, future-hostile/dead-language term `stub` appears
10. `high` `vibe` `crates/cogcore/src/ingest/mod.rs:23`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: product code contains TODO/stub/unimplemented/unreachable placeholder markers
   Fix: replace placeholders with implemented behavior, typed unsupported-state errors, or a tracked exception record with docs
   Rerun: `just fast`
   Fingerprint: `sha256:ebbd726c0c33a22d6738fad69f9f0e70395ed60a567c5f7224f34d3f2f0792f2`
   Evidence: crates/cogcore/src/ingest/mod.rs:23 /// stub with a fixture backend. ZYAL-mediated LLM backends sit at this
11. `high` `vibe` `crates/cogcore/src/ingest/paper.rs:83`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: fallback soup detected in product code
   Fix: collapse fallback chains into explicit typed states with bounded retry policy, telemetry, and documented repair guidance
   Rerun: `just fast`
   Fingerprint: `sha256:f492f74c0adec1c685d3567d716b4c0ed8078efff96d0e72565bd7e8253e8b51`
   Evidence: crates/cogcore/src/ingest/paper.rs:83 .unwrap_or_else(|| "2026-01-01T00:00:00Z".to_string());
12. `medium` `proof` `crates/memory-benchmark/data/real-paper-bank/papers/290e6358b80d1c67be2b42f01f73f532be663cd2d01f2fff8c4f85339b31623d.json:33`
   Rule: `HLT-027-HUMAN-REVIEW-EVIDENCE-GAP`
   Check: `HLT-027-HUMAN-REVIEW-EVIDENCE-GAP:proof` `soft` confidence `0.88`
   Route: TLR `Repair`, lane `audit`, owner `tools`
   Docs: `docs/testing.md`
   Matched term: `review evidence`
   Reason: proof and review claims need receipts
   Fix: attach raw CI logs, review receipts, and replayable commands instead of accepting claims or summaries
   Rerun: `just score`
   Fingerprint: `sha256:b5b7dcb0b2f45475f1da57b588d6f41b3e5075d19378ac25432a91154ec43a8d`
   Evidence: "text": "1. Introduction In recent years, global climate change has resulted in more frequent, hot, and humid weather during the summer. These environmental con
13. `high` `vibe` `packages/jekko/src/cli/cmd/tui/app-view.tsx:255`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `fallback` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:794e51ff5825e73765f81997f6121a0a324fe72a87b8eb22270ab7f035f4a0d7`
   Evidence: packages/jekko/src/cli/cmd/tui/app-view.tsx:255, future-hostile/dead-language term `fallback` appears
14. `high` `vibe` `packages/jekko/src/cli/cmd/tui/context/capability.ts:18`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `fallback` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:ad8c2fc259d8f4f895164b869d0b24f55956bb0631cde1b2f00b515e61f7abf3`
   Evidence: packages/jekko/src/cli/cmd/tui/context/capability.ts:18, future-hostile/dead-language term `fallback` appears
15. `high` `vibe` `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/activity-feed.tsx:62`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `fallback` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:c2ee4250d0854ca16d1cde78379fd4098c91df132d0e821813dd37692907935f`
   Evidence: packages/jekko/src/cli/cmd/tui/feature-plugins/shell/activity-feed.tsx:62, future-hostile/dead-language term `fallback` appears
16. `high` `vibe` `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-capability.tsx:15`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `fallback` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:ce7695c2d55ff264f761eefa6892c3a8530dd77d874c885bb0202de175a9737a`
   Evidence: packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-capability.tsx:15, future-hostile/dead-language term `fallback` appears
17. `high` `vibe` `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-capability.tsx:179`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `fallback` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:c585e60578d9c5ff1d4f39822f7c0dd81bb4b59401a9f6fcd186897c2fe8debe`
   Evidence: packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-capability.tsx:179, future-hostile/dead-language term `fallback` appears
18. `high` `vibe` `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-capability.tsx:244`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `fallback` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:064ee40c19641ff9bd50e1722a24962938a7a72f75eeab0a38b640a71b460271`
   Evidence: packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-capability.tsx:244, future-hostile/dead-language term `fallback` appears
19. `high` `vibe` `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-history.tsx:23`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `todo` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:019f78b96d3f83cc7f851f642bb67bcadced4a5921a7a1756e60e0be140841f0`
   Evidence: packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-history.tsx:23, future-hostile/dead-language term `todo` appears
20. `high` `vibe` `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-history.tsx:95`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `unused` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:a1e52feddc4a59b99c0dcc984b30a7b3edcb8e2db5dba906e9d9b04ac4afe0d9`
   Evidence: packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-history.tsx:95, future-hostile/dead-language term `unused` appears
21. `high` `vibe` `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-history.tsx:105`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `fallback` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:cbd4899f88d6417a860ce1499c018976d7cb182548730bebbb8a7c3a4a223846`
   Evidence: packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-history.tsx:105, future-hostile/dead-language term `fallback` appears
22. `high` `vibe` `packages/jekko/src/cli/cmd/tui/routes/session/session-body-core.tsx:48`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `legacy` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:8908cd6dc2ca60234cc14086489bff2530ba1fa3b004ec3f10bcb8756f1ad721`
   Evidence: packages/jekko/src/cli/cmd/tui/routes/session/session-body-core.tsx:48, future-hostile/dead-language term `legacy` appears
23. `high` `boundary` `packages/jekko/src/cli/headless.ts:508`
   Rule: `HLT-031-TYPESCRIPT-BAD-BEHAVIOR`
   Check: `HLT-031-TYPESCRIPT-BAD-BEHAVIOR:boundary` `hard` confidence `0.95`
   Route: TLR `Contracts/data`, lane `fast`, owner `tools`
   Docs: `docs/testing.md`
   Matched term: `typescript.types.any-boundary`
   Reason: value shape is not proven before the cast
   Fix: validate the value first, then narrow it with a proof-aware decoder
   Rerun: `just fast`
   Fingerprint: `sha256:19a96b03f65dd1e0df8ecdfdc6bdba8cf97e52ce7a6df42ce3ebe92da0d64fa2`
   Evidence: detector=typescript.types.any-boundary, path=packages/jekko/src/cli/headless.ts, line=508, snippet=const event = JSON.parse(line) as Record<string, any>
24. `high` `vibe` `packages/jekko/src/config/keybinds.ts:134`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `legacy` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:a27694d28c702a03f74cb8615dd730817d47d52b5c214553f709137b36c7bdd3`
   Evidence: packages/jekko/src/config/keybinds.ts:134, future-hostile/dead-language term `legacy` appears
25. `high` `boundary` `packages/jekko/src/provider/jnoccio-route-metadata.ts:76`
   Rule: `HLT-031-TYPESCRIPT-BAD-BEHAVIOR`
   Check: `HLT-031-TYPESCRIPT-BAD-BEHAVIOR:boundary` `hard` confidence `0.95`
   Route: TLR `Contracts/data`, lane `fast`, owner `tools`
   Docs: `docs/testing.md`
   Matched term: `typescript.types.any-boundary`
   Reason: value shape is not proven before the cast
   Fix: validate the value first, then narrow it with a proof-aware decoder
   Rerun: `just fast`
   Fingerprint: `sha256:e459a01e2f46e5b9de67f532bde73b9af77506438dec644e55df6428eaef88b0`
   Evidence: detector=typescript.types.any-boundary, path=packages/jekko/src/provider/jnoccio-route-metadata.ts, line=76, snippet=const parsed = JSON.parse(value) as unknown

## Policy

- Policy file: `./agent/audit-policy.toml`
- Minimum score: `85`
- Fail on: `critical, high`

## Agent Fix Queue

1. `high` `HLT-031-TYPESCRIPT-BAD-BEHAVIOR` `packages/jekko/src/cli/headless.ts` - validate the value first, then narrow it with a proof-aware decoder
   Route: `Contracts/data`/`fast`
2. `high` `HLT-031-TYPESCRIPT-BAD-BEHAVIOR` `packages/jekko/src/provider/jnoccio-route-metadata.ts` - validate the value first, then narrow it with a proof-aware decoder
   Route: `Contracts/data`/`fast`
3. `high` `HLT-004-UNMAPPED-PROOF` `agent/test-map.json` - add the narrowest stable prefix and runnable proof command to `agent/test-map.json`
   Route: `Verification`/`fast`
4. `medium` `HLT-004-UNMAPPED-PROOF` `agent/test-map.json` - route each owned path to a deterministic proof command and make the lane executable in CI
   Route: `Verification`/`fast`
5. `medium` `HLT-027-HUMAN-REVIEW-EVIDENCE-GAP` `crates/memory-benchmark/data/real-paper-bank/papers/290e6358b80d1c67be2b42f01f73f532be663cd2d01f2fff8c4f85339b31623d.json` - attach raw CI logs, review receipts, and replayable commands instead of accepting claims or summaries
   Route: `Repair`/`audit`
6. `high` `HLT-003-OWNERLESS-PATH` `agent/owner-map.json` - add the narrowest stable prefix for this path to `agent/owner-map.json`
   Route: `Context/setup`/`fast`
7. `high` `HLT-024-AGENT-TOOL-SUPPLY-GAP` `.jekko/daemons/jnoccio-route-metadata-smoke.zyal` - move the runbook to `agent/zyal/` and rename it to `*.zyal`
   Route: `Security, secrets, agency`/`security`
8. `high` `HLT-024-AGENT-TOOL-SUPPLY-GAP` `.jekko/daemons/jnoccio-route-metadata-smoke.zyal` - keep the runbook envelope at the top of the file after optional comments
   Route: `Security, secrets, agency`/`security`
9. `high` `HLT-013-RENDERED-UX-GAP` `apps/web` - add Playwright e2e tests for critical user flows and wire them into the fast or CI proof map
   Route: `Verification and rendered UX`/`web`
10. `high` `HLT-013-RENDERED-UX-GAP` `apps/web` - add Storybook state coverage, Playwright screenshots, visual review or `@jankurai/ux-qa`, accessibility scans, CLS checks, generated mocks, and design tokens
   Route: `Verification and rendered UX`/`web`
11. `high` `HLT-001-DEAD-MARKER` `crates/cogcore/src/ingest/mod.rs` - remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Route: `Entropy`/`fast`
12. `high` `HLT-001-DEAD-MARKER` `crates/cogcore/src/ingest/mod.rs` - replace placeholders with implemented behavior, typed unsupported-state errors, or a tracked exception record with docs
   Route: `Entropy`/`fast`
13. `high` `HLT-001-DEAD-MARKER` `crates/cogcore/src/ingest/paper.rs` - collapse fallback chains into explicit typed states with bounded retry policy, telemetry, and documented repair guidance
   Route: `Entropy`/`fast`
14. `high` `HLT-001-DEAD-MARKER` `packages/jekko/src/cli/cmd/tui/app-view.tsx` - remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Route: `Entropy`/`fast`
15. `high` `HLT-001-DEAD-MARKER` `packages/jekko/src/cli/cmd/tui/context/capability.ts` - remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Route: `Entropy`/`fast`
16. `high` `HLT-001-DEAD-MARKER` `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/activity-feed.tsx` - remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Route: `Entropy`/`fast`
17. `high` `HLT-001-DEAD-MARKER` `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-capability.tsx` - remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Route: `Entropy`/`fast`
18. `high` `HLT-001-DEAD-MARKER` `packages/jekko/src/cli/cmd/tui/feature-plugins/shell/pane-history.tsx` - remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Route: `Entropy`/`fast`
19. `high` `HLT-001-DEAD-MARKER` `packages/jekko/src/cli/cmd/tui/routes/session/session-body-core.tsx` - remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Route: `Entropy`/`fast`
20. `high` `HLT-001-DEAD-MARKER` `packages/jekko/src/config/keybinds.ts` - remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Route: `Entropy`/`fast`
21. `medium` `HLT-001-DEAD-MARKER` `.` - split large or ambiguous authored code into smaller semantic modules with focused tests
   Route: `Entropy`/`fast`
