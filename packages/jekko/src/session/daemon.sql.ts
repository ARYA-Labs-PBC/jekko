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
