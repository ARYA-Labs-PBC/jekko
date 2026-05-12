// Persistent storage for the PR4 daemon-forever tables. Wraps Drizzle so the
// new modules (`daemon-finding-classifier`, `daemon-finding-dag`,
// `daemon-worker-pool`, `daemon-reviewer-pass`, `daemon-concept-memory`) can
// land their state into SQLite without pulling the big `daemon-store.ts`
// Effect Service apart. Keeping the boundary thin here makes PR4 reviewable
// and lets a future PR consolidate the surfaces.
//
// All functions take an explicit Drizzle handle so the same store can be
// exercised against test fixtures (better-sqlite3 in-memory) without needing
// to bootstrap the full Database layer.

import { and, asc, desc, eq } from "drizzle-orm"
import { ulid } from "ulid"

import {
  DaemonConceptLinkTable,
  DaemonConceptTable,
  DaemonFindingBatchTable,
  DaemonFindingEdgeTable,
  DaemonFindingTable,
  DaemonRegressionCycleTable,
} from "./daemon.sql"
import type { Finding, Severity } from "./daemon-finding-classifier"

// Minimal handle shape used here. The real Database.use() handle is
// structurally compatible; we keep this typed locally so tests can pass any
// Drizzle-compatible instance.
export type DrizzleHandle = {
  insert: (...args: unknown[]) => any
  update: (...args: unknown[]) => any
  delete: (...args: unknown[]) => any
  select: (...args: unknown[]) => any
}

// ─── findings ──────────────────────────────────────────────────────────────

export type FindingRow = typeof DaemonFindingTable.$inferSelect

export type InsertFindingInput = {
  runID: string
  iteration: number
  finding: Finding
}

export function insertFinding(db: any, input: InsertFindingInput): FindingRow {
  const now = Math.floor(Date.now() / 1000)
  const row: typeof DaemonFindingTable.$inferInsert = {
    id: ulid(),
    run_id: input.runID,
    iteration: input.iteration,
    rule_id: input.finding.ruleID,
    fingerprint: input.finding.fingerprint,
    severity: input.finding.severity,
    paths_json: input.finding.paths,
    cap: input.finding.cap ?? null,
    status: "queued",
    attempt_count: 0,
    time_created: now,
    time_updated: now,
  }
  return db.insert(DaemonFindingTable).values(row).returning().get()
}

export function listFindings(db: any, runID: string): FindingRow[] {
  return db
    .select()
    .from(DaemonFindingTable)
    .where(eq(DaemonFindingTable.run_id, runID))
    .orderBy(asc(DaemonFindingTable.time_created))
    .all()
}

export function updateFindingStatus(
  db: any,
  input: { findingID: string; status: string; batchID?: string; lastError?: string },
): FindingRow | undefined {
  const now = Math.floor(Date.now() / 1000)
  return db
    .update(DaemonFindingTable)
    .set({
      status: input.status,
      batch_id: input.batchID ?? null,
      last_error: input.lastError ?? null,
      time_updated: now,
    })
    .where(eq(DaemonFindingTable.id, input.findingID))
    .returning()
    .get()
}

export function bumpFindingAttempt(db: any, findingID: string): FindingRow | undefined {
  const existing = db.select().from(DaemonFindingTable).where(eq(DaemonFindingTable.id, findingID)).get()
  if (!existing) return undefined
  const now = Math.floor(Date.now() / 1000)
  return db
    .update(DaemonFindingTable)
    .set({ attempt_count: existing.attempt_count + 1, time_updated: now })
    .where(eq(DaemonFindingTable.id, findingID))
    .returning()
    .get()
}

// ─── batches ───────────────────────────────────────────────────────────────

export type BatchRow = typeof DaemonFindingBatchTable.$inferSelect

export function insertBatch(
  db: any,
  input: { runID: string; waveIndex: number; lane?: string; workerID?: string },
): BatchRow {
  const now = Math.floor(Date.now() / 1000)
  return db
    .insert(DaemonFindingBatchTable)
    .values({
      id: ulid(),
      run_id: input.runID,
      wave_index: input.waveIndex,
      lane: input.lane ?? "parallel",
      worker_id: input.workerID ?? null,
      status: "queued",
      time_created: now,
      time_updated: now,
    })
    .returning()
    .get()
}

export function listBatches(db: any, runID: string): BatchRow[] {
  return db
    .select()
    .from(DaemonFindingBatchTable)
    .where(eq(DaemonFindingBatchTable.run_id, runID))
    .orderBy(asc(DaemonFindingBatchTable.wave_index), asc(DaemonFindingBatchTable.time_created))
    .all()
}

export function updateBatchStatus(
  db: any,
  input: { batchID: string; status: string; startedAt?: number; endedAt?: number; result?: unknown },
): BatchRow | undefined {
  const now = Math.floor(Date.now() / 1000)
  return db
    .update(DaemonFindingBatchTable)
    .set({
      status: input.status,
      started_at: input.startedAt ?? null,
      ended_at: input.endedAt ?? null,
      result_json: (input.result ?? null) as any,
      time_updated: now,
    })
    .where(eq(DaemonFindingBatchTable.id, input.batchID))
    .returning()
    .get()
}

// ─── edges ─────────────────────────────────────────────────────────────────

export type EdgeRow = typeof DaemonFindingEdgeTable.$inferSelect

export function insertEdge(
  db: any,
  input: { runID: string; parentID: string; childID: string; kind?: string },
): void {
  const now = Math.floor(Date.now() / 1000)
  db.insert(DaemonFindingEdgeTable)
    .values({
      run_id: input.runID,
      parent_id: input.parentID,
      child_id: input.childID,
      kind: input.kind ?? "path_overlap",
      time_created: now,
    })
    .run()
}

export function listEdges(db: any, runID: string): EdgeRow[] {
  return db
    .select()
    .from(DaemonFindingEdgeTable)
    .where(eq(DaemonFindingEdgeTable.run_id, runID))
    .all()
}

// ─── concepts ──────────────────────────────────────────────────────────────

export type ConceptRow = typeof DaemonConceptTable.$inferSelect

export function upsertConcept(
  db: any,
  input: {
    runID: string
    conceptID: string
    definition: string
    derivedFrom?: string[]
    proofRefs?: string[]
    confidence?: number
  },
): ConceptRow {
  const now = Math.floor(Date.now() / 1000)
  const existing = db
    .select()
    .from(DaemonConceptTable)
    .where(and(eq(DaemonConceptTable.run_id, input.runID), eq(DaemonConceptTable.concept_id, input.conceptID)))
    .get()
  if (existing) {
    return db
      .update(DaemonConceptTable)
      .set({
        definition: input.definition,
        derived_from_json: (input.derivedFrom ?? null) as any,
        proof_refs_json: (input.proofRefs ?? null) as any,
        confidence: input.confidence ?? existing.confidence,
        invalidated_at: null,
        invalidated_reason: null,
        time_updated: now,
      })
      .where(eq(DaemonConceptTable.id, existing.id))
      .returning()
      .get()
  }
  return db
    .insert(DaemonConceptTable)
    .values({
      id: ulid(),
      run_id: input.runID,
      concept_id: input.conceptID,
      definition: input.definition,
      derived_from_json: (input.derivedFrom ?? null) as any,
      proof_refs_json: (input.proofRefs ?? null) as any,
      confidence: input.confidence ?? 0.5,
      time_created: now,
      time_updated: now,
    })
    .returning()
    .get()
}

export function recallConcept(db: any, runID: string, conceptID: string): ConceptRow | undefined {
  return db
    .select()
    .from(DaemonConceptTable)
    .where(and(eq(DaemonConceptTable.run_id, runID), eq(DaemonConceptTable.concept_id, conceptID)))
    .get()
}

export function listActiveConcepts(db: any, runID: string): ConceptRow[] {
  return db
    .select()
    .from(DaemonConceptTable)
    .where(and(eq(DaemonConceptTable.run_id, runID)))
    .orderBy(asc(DaemonConceptTable.concept_id))
    .all()
    .filter((row: ConceptRow) => row.invalidated_at === null)
}

export function invalidateConcept(
  db: any,
  input: { runID: string; conceptID: string; reason: string },
): ConceptRow | undefined {
  const existing = recallConcept(db, input.runID, input.conceptID)
  if (!existing) return undefined
  const now = Math.floor(Date.now() / 1000)
  return db
    .update(DaemonConceptTable)
    .set({
      invalidated_at: now,
      invalidated_reason: input.reason,
      time_updated: now,
    })
    .where(eq(DaemonConceptTable.id, existing.id))
    .returning()
    .get()
}

// ─── concept links ─────────────────────────────────────────────────────────

export function insertConceptLink(
  db: any,
  input: { runID: string; parentConcept: string; childConcept: string; relation?: string },
): void {
  db.insert(DaemonConceptLinkTable)
    .values({
      run_id: input.runID,
      parent_concept: input.parentConcept,
      child_concept: input.childConcept,
      relation: input.relation ?? "derived_from",
      time_created: Math.floor(Date.now() / 1000),
    })
    .run()
}

// ─── regression cycles ─────────────────────────────────────────────────────

export type RegressionRow = typeof DaemonRegressionCycleTable.$inferSelect

export function recordRegressionCycle(
  db: any,
  input: {
    runID: string
    iteration: number
    baselineScore?: number
    currentScore?: number
    hardDelta?: number
    softDelta?: number
    capsDelta?: number
    status?: "pass" | "regression" | "halted"
    result?: unknown
  },
): RegressionRow {
  const now = Math.floor(Date.now() / 1000)
  return db
    .insert(DaemonRegressionCycleTable)
    .values({
      id: ulid(),
      run_id: input.runID,
      iteration: input.iteration,
      baseline_score: input.baselineScore ?? null,
      current_score: input.currentScore ?? null,
      hard_delta: input.hardDelta ?? 0,
      soft_delta: input.softDelta ?? 0,
      caps_delta: input.capsDelta ?? 0,
      status: input.status ?? "pass",
      result_json: (input.result ?? null) as any,
      time_created: now,
      time_updated: now,
    })
    .returning()
    .get()
}

export function listRegressionCycles(db: any, runID: string, limit: number = 20): RegressionRow[] {
  return db
    .select()
    .from(DaemonRegressionCycleTable)
    .where(eq(DaemonRegressionCycleTable.run_id, runID))
    .orderBy(desc(DaemonRegressionCycleTable.iteration))
    .limit(limit)
    .all()
}

// ─── derived counters ─────────────────────────────────────────────────────

export function severityTotals(rows: readonly FindingRow[]): { caps: number; hard: number; soft: number } {
  let caps = 0
  let hard = 0
  let soft = 0
  for (const row of rows) {
    if (row.cap !== null && row.cap !== undefined) {
      caps += 1
      continue
    }
    const severity = row.severity as Severity
    if (severity === "critical" || severity === "high") hard += 1
    else soft += 1
  }
  return { caps, hard, soft }
}

export * as DaemonForeverStore from "./daemon-forever-store"
