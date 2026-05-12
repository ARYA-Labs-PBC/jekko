import { index, integer, primaryKey, real, sqliteTable, text } from "drizzle-orm/sqlite-core"
import { SessionTable } from "./session.sql"
import { Timestamps } from "@/storage/schema.sql"
import type { SessionID } from "./schema"

export const DaemonRunTable = sqliteTable(
  "daemon_run",
  {
    id: text().primaryKey(),
    root_session_id: text()
      .$type<SessionID>()
      .notNull()
      .references(() => SessionTable.id, { onDelete: "restrict" }),
    active_session_id: text()
      .$type<SessionID>()
      .notNull()
      .references(() => SessionTable.id, { onDelete: "restrict" }),
    status: text().notNull(),
    phase: text().notNull(),
    spec_json: text({ mode: "json" }).notNull(),
    spec_hash: text().notNull(),
    iteration: integer().notNull(),
    epoch: integer().notNull(),
    last_error: text(),
    last_exit_result_json: text({ mode: "json" }),
    stopped_at: integer(),
    ...Timestamps,
  },
  (table) => [
    index("daemon_run_root_idx").on(table.root_session_id),
    index("daemon_run_active_idx").on(table.active_session_id),
    index("daemon_run_status_idx").on(table.status),
  ],
)

export const DaemonIterationTable = sqliteTable(
  "daemon_iteration",
  {
    run_id: text()
      .notNull()
      .references(() => DaemonRunTable.id, { onDelete: "restrict" }),
    iteration: integer().notNull(),
    session_id: text()
      .$type<SessionID>()
      .notNull()
      .references(() => SessionTable.id, { onDelete: "restrict" }),
    terminal_reason: text().notNull(),
    result_json: text({ mode: "json" }).notNull(),
    token_usage_json: text({ mode: "json" }),
    cost: real(),
    checkpoint_sha: text(),
    ...Timestamps,
  },
  (table) => [primaryKey({ columns: [table.run_id, table.iteration] }), index("daemon_iteration_run_idx").on(table.run_id)],
)

export const DaemonEventTable = sqliteTable(
  "daemon_event",
  {
    id: text().primaryKey(),
    run_id: text()
      .notNull()
      .references(() => DaemonRunTable.id, { onDelete: "restrict" }),
    iteration: integer().notNull(),
    event_type: text().notNull(),
    payload_json: text({ mode: "json" }).notNull(),
    ...Timestamps,
  },
  (table) => [index("daemon_event_run_idx").on(table.run_id, table.time_created)],
)

export const DaemonTaskTable = sqliteTable(
  "daemon_task",
  {
    id: text().primaryKey(),
    run_id: text()
      .notNull()
      .references(() => DaemonRunTable.id, { onDelete: "restrict" }),
    external_id: text(),
    title: text().notNull(),
    body_json: text({ mode: "json" }).notNull(),
    status: text().notNull(),
    lane: text().notNull().default("normal"),
    phase: text().notNull().default("queued"),
    difficulty_score: real().notNull().default(0),
    risk_score: real().notNull().default(0),
    readiness_score: real().notNull().default(0),
    implementation_confidence: real().notNull().default(0),
    verification_confidence: real().notNull().default(0),
    attempt_count: integer().notNull().default(0),
    no_progress_count: integer().notNull().default(0),
    incubator_round: integer().notNull().default(0),
    incubator_status: text().notNull().default("none"),
    accepted_artifact_id: text(),
    last_assessment_json: text({ mode: "json" }),
    promotion_result_json: text({ mode: "json" }),
    blocked_reason: text(),
    priority: integer().notNull(),
    lease_worker_id: text(),
    lease_expires_at: integer(),
    locked_paths_json: text({ mode: "json" }),
    evidence_json: text({ mode: "json" }),
    ...Timestamps,
  },
  (table) => [
    index("daemon_task_run_status_idx").on(table.run_id, table.status, table.priority),
    index("daemon_task_lane_status_idx").on(table.run_id, table.lane, table.status, table.priority),
    index("daemon_task_lease_idx").on(table.lease_expires_at),
  ],
)

export const DaemonTaskPassTable = sqliteTable(
  "daemon_task_pass",
  {
    id: text().primaryKey(),
    run_id: text()
      .notNull()
      .references(() => DaemonRunTable.id, { onDelete: "restrict" }),
    task_id: text()
      .notNull()
      .references(() => DaemonTaskTable.id, { onDelete: "restrict" }),
    pass_number: integer().notNull(),
    pass_type: text().notNull(),
    context_mode: text().notNull(),
    agent: text(),
    session_id: text().$type<SessionID>().references(() => SessionTable.id, { onDelete: "set null" }),
    worker_id: text(),
    status: text().notNull(),
    started_at: integer(),
    ended_at: integer(),
    worktree_path: text(),
    worktree_branch: text(),
    cleanup_status: text().notNull().default("pending"),
    input_artifact_ids_json: text({ mode: "json" }),
    output_artifact_ids_json: text({ mode: "json" }),
    result_json: text({ mode: "json" }),
    score_json: text({ mode: "json" }),
    error_json: text({ mode: "json" }),
    ...Timestamps,
  },
  (table) => [
    index("daemon_task_pass_task_idx").on(table.run_id, table.task_id, table.pass_number),
    index("daemon_task_pass_status_idx").on(table.run_id, table.status),
  ],
)

export const DaemonTaskMemoryTable = sqliteTable(
  "daemon_task_memory",
  {
    id: text().primaryKey(),
    run_id: text()
      .notNull()
      .references(() => DaemonRunTable.id, { onDelete: "restrict" }),
    task_id: text()
      .notNull()
      .references(() => DaemonTaskTable.id, { onDelete: "restrict" }),
    kind: text().notNull(),
    title: text().notNull(),
    summary: text().notNull(),
    payload_json: text({ mode: "json" }),
    source_pass_id: text().references(() => DaemonTaskPassTable.id, { onDelete: "set null" }),
    importance: real().notNull().default(0.5),
    confidence: real().notNull().default(0.5),
    ...Timestamps,
  },
  (table) => [
    index("daemon_task_memory_task_idx").on(table.run_id, table.task_id, table.time_created),
    index("daemon_task_memory_kind_idx").on(table.run_id, table.task_id, table.kind),
  ],
)

export const DaemonWorkerTable = sqliteTable(
  "daemon_worker",
  {
    id: text().primaryKey(),
    run_id: text()
      .notNull()
      .references(() => DaemonRunTable.id, { onDelete: "restrict" }),
    role: text().notNull(),
    session_id: text().references(() => SessionTable.id, { onDelete: "set null" }),
    worktree_path: text(),
    branch: text(),
    status: text().notNull(),
    lease_task_id: text(),
    last_heartbeat_at: integer(),
    pool_id: text(),
    batch_id: text(),
    last_commit_sha: text(),
    ...Timestamps,
  },
  (table) => [index("daemon_worker_run_idx").on(table.run_id, table.status)],
)

export const DaemonArtifactTable = sqliteTable(
  "daemon_artifact",
  {
    id: text().primaryKey(),
    run_id: text()
      .notNull()
      .references(() => DaemonRunTable.id, { onDelete: "restrict" }),
    task_id: text().references(() => DaemonTaskTable.id, { onDelete: "restrict" }),
    pass_id: text().references(() => DaemonTaskPassTable.id, { onDelete: "set null" }),
    kind: text().notNull(),
    path_or_ref: text().notNull(),
    sha: text(),
    payload_json: text({ mode: "json" }),
    ...Timestamps,
  },
  (table) => [
    index("daemon_artifact_run_idx").on(table.run_id),
    index("daemon_artifact_task_idx").on(table.run_id, table.task_id),
    index("daemon_artifact_pass_idx").on(table.run_id, table.pass_id),
  ],
)

// ─── PR4: forever-runner tables ──────────────────────────────────────────────
// These mirror the SQLite schema written by `crates/jankurai-runner` (PR3) and
// give the daemon-TS bridge a typed view onto the same rows. Drizzle defs only;
// CRUD lives in `daemon-forever-store.ts`.

export const DaemonFindingTable = sqliteTable(
  "daemon_finding",
  {
    id: text().primaryKey(),
    run_id: text()
      .notNull()
      .references(() => DaemonRunTable.id, { onDelete: "restrict" }),
    iteration: integer().notNull().default(0),
    rule_id: text().notNull(),
    fingerprint: text().notNull(),
    severity: text().notNull(),
    paths_json: text({ mode: "json" }).notNull().default([]),
    cap: text(),
    status: text().notNull().default("queued"),
    attempt_count: integer().notNull().default(0),
    batch_id: text(),
    last_error: text(),
    ...Timestamps,
  },
  (table) => [
    index("daemon_finding_run_status_idx").on(table.run_id, table.status),
    index("daemon_finding_run_severity_idx").on(table.run_id, table.severity),
    index("daemon_finding_fp_idx").on(table.run_id, table.fingerprint),
  ],
)

export const DaemonFindingBatchTable = sqliteTable(
  "daemon_finding_batch",
  {
    id: text().primaryKey(),
    run_id: text()
      .notNull()
      .references(() => DaemonRunTable.id, { onDelete: "restrict" }),
    wave_index: integer().notNull(),
    lane: text().notNull().default("parallel"),
    worker_id: text(),
    status: text().notNull().default("queued"),
    started_at: integer(),
    ended_at: integer(),
    result_json: text({ mode: "json" }),
    ...Timestamps,
  },
  (table) => [
    index("daemon_finding_batch_run_wave_idx").on(table.run_id, table.wave_index),
    index("daemon_finding_batch_status_idx").on(table.run_id, table.status),
  ],
)

export const DaemonFindingEdgeTable = sqliteTable(
  "daemon_finding_edge",
  {
    run_id: text()
      .notNull()
      .references(() => DaemonRunTable.id, { onDelete: "restrict" }),
    parent_id: text().notNull(),
    child_id: text().notNull(),
    kind: text().notNull().default("path_overlap"),
    time_created: integer().notNull(),
  },
  (table) => [
    primaryKey({ columns: [table.run_id, table.parent_id, table.child_id] }),
    index("daemon_finding_edge_run_idx").on(table.run_id),
    index("daemon_finding_edge_child_idx").on(table.run_id, table.child_id),
  ],
)

export const DaemonConceptTable = sqliteTable(
  "daemon_concept",
  {
    id: text().primaryKey(),
    run_id: text()
      .notNull()
      .references(() => DaemonRunTable.id, { onDelete: "restrict" }),
    concept_id: text().notNull(),
    definition: text().notNull(),
    derived_from_json: text({ mode: "json" }),
    proof_refs_json: text({ mode: "json" }),
    confidence: real().notNull().default(0.5),
    invalidated_at: integer(),
    invalidated_reason: text(),
    ...Timestamps,
  },
  (table) => [
    index("daemon_concept_run_concept_idx").on(table.run_id, table.concept_id),
    index("daemon_concept_invalidated_idx").on(table.run_id, table.invalidated_at),
  ],
)

export const DaemonConceptLinkTable = sqliteTable(
  "daemon_concept_link",
  {
    run_id: text()
      .notNull()
      .references(() => DaemonRunTable.id, { onDelete: "restrict" }),
    parent_concept: text().notNull(),
    child_concept: text().notNull(),
    relation: text().notNull().default("derived_from"),
    time_created: integer().notNull(),
  },
  (table) => [
    primaryKey({ columns: [table.run_id, table.parent_concept, table.child_concept] }),
    index("daemon_concept_link_parent_idx").on(table.run_id, table.parent_concept),
    index("daemon_concept_link_child_idx").on(table.run_id, table.child_concept),
  ],
)

export const DaemonRegressionCycleTable = sqliteTable(
  "daemon_regression_cycle",
  {
    id: text().primaryKey(),
    run_id: text()
      .notNull()
      .references(() => DaemonRunTable.id, { onDelete: "restrict" }),
    iteration: integer().notNull(),
    baseline_score: real(),
    current_score: real(),
    hard_delta: integer().notNull().default(0),
    soft_delta: integer().notNull().default(0),
    caps_delta: integer().notNull().default(0),
    status: text().notNull().default("pass"),
    result_json: text({ mode: "json" }),
    ...Timestamps,
  },
  (table) => [
    index("daemon_regression_cycle_run_iter_idx").on(table.run_id, table.iteration),
    index("daemon_regression_cycle_status_idx").on(table.run_id, table.status),
  ],
)
