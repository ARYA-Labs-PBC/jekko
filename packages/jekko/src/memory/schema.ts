import { sqliteTable, text, integer, index, primaryKey } from "drizzle-orm/sqlite-core"
import { Timestamps } from "../storage/schema.sql"

import { ProjectTable } from "../project/project.sql"
import type { ProjectID } from "../project/schema"

export const MemoryEvidenceTable = sqliteTable(
  "memory_evidence",
  {
    id: integer().primaryKey({ autoIncrement: true }),
    project_id: text()
      .$type<ProjectID>()
      .notNull()
      .references(() => ProjectTable.id, { onDelete: "restrict" }),
    tier: text().notNull(),
    subject: text().notNull(),
    predicate: text().notNull(),
    object: text().notNull(),
    snippet: text().notNull(),
    search_text: text().notNull(),
    payload_json: text().notNull(),
    owner: text(),
    session_id: text(),
    evidence_hash: text(),
    ...Timestamps,
  },
  (table) => [
    index("memory_evidence_project_tier_idx").on(table.project_id, table.tier, table.time_updated),
    index("memory_evidence_search_idx").on(table.project_id, table.search_text),
    index("memory_evidence_subject_idx").on(table.project_id, table.subject, table.time_updated),
  ],
)

export const FailedAttemptTable = sqliteTable(
  "failed_attempt",
  {
    project_id: text()
      .$type<ProjectID>()
      .notNull()
      .references(() => ProjectTable.id, { onDelete: "restrict" }),
    signature: text().notNull(),
    failure_kind: text().notNull(),
    owner: text().notNull().default(""),
    attempted_fix_hash: text().notNull(),
    evidence_hash: text().notNull(),
    session_id: text(),
    seen_count: integer().notNull().default(1),
    ...Timestamps,
  },
  (table) => [
    primaryKey({ columns: [table.project_id, table.signature, table.failure_kind, table.owner, table.attempted_fix_hash, table.evidence_hash] }),
    index("failed_attempt_sig_idx").on(table.project_id, table.signature, table.failure_kind, table.time_updated),
  ],
)
