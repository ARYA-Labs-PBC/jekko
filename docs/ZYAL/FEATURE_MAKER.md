# Feature Maker Guide

Use the feature-maker runbooks when the goal is to recommend, implement, and
prove one feature at a time from repo evidence.

## Canonical File Taxonomy

- `README.md`
- `MISSION.md`
- `ROADMAP.md`
- `docs/adr/`
- `Cargo.toml`
- `package.json`
- `vite.config.ts`
- routes and API contracts
- migrations and database schemas
- tests and test fixtures
- `Dockerfile`
- CI and release workflows
- observability and proof outputs

## Project Atlas Artifacts

- `target/zyal/feature-maker/project-atlas.json`
- `target/zyal/feature-maker/capability-graph.json`
- `target/zyal/feature-maker/opportunity-bank.md`
- `target/zyal/feature-maker/feature-dossier.md`
- `target/zyal/feature-maker/critic-review.md`
- `target/zyal/feature-maker/proof-report.md`

## Feature Categories

- mission gap
- latent data
- API and platform
- UX workflow
- intelligence and ranking
- enterprise readiness
- developer platform
- core IP
- reliability and recovery
- integration and migration

## Runbook Shape

1. Build the atlas from canonical files and the current repo surface.
2. Derive a feature thesis and a capability gap from evidence.
3. Define the user workflow and the first vertical slice.
4. Add tests, integration validation, rollback notes, and proof commands.
5. Save the candidate patch before checkpointing in the insane path.
6. Open a draft PR only after the evidence, review, and rollback story are complete.

## Recommended Tooling

- Rust or TypeScript atlas builders for tree traversal and report generation
- tree-sitter, LSP, or SCIP for symbol and reference graphs
- TypeScript compiler API for route and API contract inspection
- SQL parsers for migrations and schema drift checks
- Playwright for UI workflow validation
- OpenTelemetry traces for proof and review receipts
- Postgres with `pgvector` for artifact and memory retrieval
- Kuzu or Neo4j for graph-backed opportunity analysis
- Qdrant or OpenSearch for semantic search over repo evidence
- CodeQL or Semgrep for security and migration risk checks

## Practical Standard

- Keep the feature small enough to ship behind a draft PR.
- Tie every recommendation to repo evidence and a user workflow.
- Keep tainted research quarantined until review.
- Require a rollback plan before promotion.
- Favor one feature, one PR, one proof lane.
