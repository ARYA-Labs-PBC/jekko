import { Effect, Context, Layer } from "effect"
import { Database } from "../storage/db"
import { MemoryEvidenceTable, FailedAttemptTable } from "./schema"
import { sql, eq, and, desc } from "drizzle-orm"
import type { ProjectID } from "../project/schema"

export type MemoryTier = "working" | "episodic" | "semantic" | "procedural" | "negative" | "rule"

export interface FailureFingerprint {
  signature: string
  failureKind: string
  attemptedFixHash: string
  evidenceHash?: string
}

export type RetryDecision =
  | { type: "allow" }
  | { type: "block"; reason: string }

export interface FailedAttemptRecord {
  fingerprint: FailureFingerprint
  seenCount: number
  lastSeenAt: number
}

export interface MemoryHit {
  tier: MemoryTier
  snippet: string
  payloadJson: string
  score?: number
}

export type MemoryEvent =
  | { type: "record_working"; projectId: ProjectID; fact: any }
  | { type: "record_episodic"; projectId: ProjectID; fact: any }
  | { type: "record_semantic"; projectId: ProjectID; fact: any }
  | { type: "record_procedural"; projectId: ProjectID; fact: any }
  | { type: "record_negative"; projectId: ProjectID; fact: any }
  | { type: "upsert_rule"; projectId: ProjectID; rule: any }

export interface MemoryQuery {
  projectId: ProjectID
  tier?: MemoryTier
  queryText?: string
  limit?: number
}

export interface Interface {
  readonly record: (event: MemoryEvent) => Effect.Effect<void>
  readonly recall: (query: MemoryQuery) => Effect.Effect<MemoryHit[]>
  readonly retryDecision: (
    projectId: ProjectID,
    fingerprint: Omit<FailureFingerprint, "failureKind">,
  ) => Effect.Effect<RetryDecision>
  readonly recordFailedAttempt: (projectId: ProjectID, fingerprint: FailureFingerprint, sessionId?: string) => Effect.Effect<void>
  readonly failedAttemptsForSignature: (projectId: ProjectID, signature: string) => Effect.Effect<FailedAttemptRecord[]>
  readonly decayTick: (projectId: ProjectID) => Effect.Effect<void>
}

export class Service extends Context.Service<Service, Interface>()("@jekko/MemoryOS") {}

export const layer = Layer.effect(
  Service,
  Effect.gen(function* () {
    const db = <T>(fn: (d: Parameters<typeof Database.use>[0] extends (trx: infer D) => any ? D : never) => T) =>
      Effect.sync(() => Database.use(fn))

    const record = Effect.fn("MemoryOS.record")(function* (event: MemoryEvent) {
      yield* db((tx) => {
        const timeCreated = Date.now()
        let tier: MemoryTier
        let subject = ""
        let predicate = ""
        let object = ""
        let snippet = ""
        let searchText = ""

        switch (event.type) {
          case "record_working":
            tier = "working"
            subject = event.fact.taskId || ""
            predicate = "observed"
            object = event.fact.kind || ""
            snippet = event.fact.body || ""
            searchText = `${subject} ${predicate} ${object} ${snippet}`
            break
          case "record_episodic":
            tier = "episodic"
            subject = event.fact.sessionId || ""
            predicate = "resulted_in"
            object = event.fact.outcome || ""
            snippet = event.fact.summary || ""
            searchText = `${subject} ${snippet}`
            break
          case "record_semantic":
            tier = "semantic"
            subject = event.fact.subject || ""
            predicate = event.fact.predicate || ""
            object = event.fact.object || ""
            snippet = `${subject} ${predicate} ${object}`
            searchText = snippet
            break
          case "record_procedural":
            tier = "procedural"
            subject = event.fact.name || ""
            predicate = "triggered_by"
            object = event.fact.triggerPattern || ""
            snippet = event.fact.stepsYaml || ""
            searchText = `${subject} ${object} ${snippet}`
            break
          case "record_negative":
            tier = "negative"
            subject = event.fact.signature || ""
            predicate = "failed_with"
            object = event.fact.failureKind || ""
            snippet = `Avoid: ${subject} failed due to ${object}`
            searchText = `${subject} ${object}`
            break
          case "upsert_rule":
            tier = "rule"
            subject = event.rule.scope || "global"
            predicate = "enforces"
            object = event.rule.id || ""
            snippet = event.rule.statement || ""
            searchText = `${subject} ${snippet}`
            break
        }

        tx.insert(MemoryEvidenceTable)
          .values({
            project_id: event.projectId,
            tier,
            subject,
            predicate,
            object,
            snippet,
            search_text: searchText,
            payload_json: JSON.stringify("fact" in event ? event.fact : event.rule),
            time_created: timeCreated,
            time_updated: timeCreated,
          })
          .run()
      })
    })

    const recall = Effect.fn("MemoryOS.recall")(function* (query: MemoryQuery) {
      return yield* db((tx) => {
        const conditions: any[] = [eq(MemoryEvidenceTable.project_id, query.projectId)]

        if (query.tier) {
          conditions.push(eq(MemoryEvidenceTable.tier, query.tier))
        }

        if (query.queryText) {
          conditions.push(sql`instr(${MemoryEvidenceTable.search_text}, ${query.queryText}) > 0`)
        }

        let sqlQuery = tx
          .select()
          .from(MemoryEvidenceTable)
          .where(and(...conditions))
          .orderBy(desc(MemoryEvidenceTable.time_updated)) as any

        if (query.limit) {
          sqlQuery = sqlQuery.limit(query.limit)
        }

        const rows = sqlQuery.all()

        return rows.map((row: any) => ({
          tier: row.tier as MemoryTier,
          snippet: row.snippet,
          payloadJson: row.payload_json,
        }))
      })
    })

    const retryDecision = Effect.fn("MemoryOS.retryDecision")(function* (
      projectId: ProjectID,
      fingerprint: Omit<FailureFingerprint, "failureKind">,
    ) {
      return yield* db((tx) => {
        const conditions = [
          eq(FailedAttemptTable.project_id, projectId),
          eq(FailedAttemptTable.signature, fingerprint.signature),
          eq(FailedAttemptTable.attempted_fix_hash, fingerprint.attemptedFixHash),
        ]

        if (typeof fingerprint.evidenceHash === "string" && fingerprint.evidenceHash.length > 0) {
          conditions.push(eq(FailedAttemptTable.evidence_hash, fingerprint.evidenceHash))
        }

        const existing = tx
          .select()
          .from(FailedAttemptTable)
          .where(and(...conditions))
          .get()

        if (existing) {
          return {
            type: "block",
            reason: `Doom-loop prevented: exact same failed fix already observed for ${fingerprint.signature} (${existing.seen_count}x). You must alter your approach or gather different evidence.`,
          } as RetryDecision
        }

        return { type: "allow" } as RetryDecision
      })
    })

    const recordFailedAttempt = Effect.fn("MemoryOS.recordFailedAttempt")(function* (
      projectId: ProjectID,
      fingerprint: FailureFingerprint,
      sessionId?: string,
    ) {
      yield* db((tx) => {
        const now = Date.now()
        tx.insert(FailedAttemptTable)
          .values({
            project_id: projectId,
            signature: fingerprint.signature,
            failure_kind: fingerprint.failureKind,
            attempted_fix_hash: fingerprint.attemptedFixHash,
            evidence_hash: fingerprint.evidenceHash || "",
            session_id: sessionId,
            time_created: now,
            time_updated: now,
            seen_count: 1,
          })
          .onConflictDoUpdate({
            target: [
              FailedAttemptTable.project_id,
              FailedAttemptTable.signature,
              FailedAttemptTable.failure_kind,
              FailedAttemptTable.owner,
              FailedAttemptTable.attempted_fix_hash,
              FailedAttemptTable.evidence_hash,
            ],
            set: {
              seen_count: sql`${FailedAttemptTable.seen_count} + 1`,
              time_updated: now,
              session_id: sessionId || sql`${FailedAttemptTable.session_id}`,
            },
          })
          .run()
      })
    })

    const failedAttemptsForSignature = Effect.fn("MemoryOS.failedAttemptsForSignature")(function* (
      projectId: ProjectID,
      signature: string,
    ) {
      return yield* db((tx) => {
        const rows = tx
          .select()
          .from(FailedAttemptTable)
          .where(and(eq(FailedAttemptTable.project_id, projectId), eq(FailedAttemptTable.signature, signature)))
          .orderBy(desc(FailedAttemptTable.time_updated))
          .all()

        return rows.map((row) => ({
          fingerprint: {
            signature: row.signature,
            failureKind: row.failure_kind,
            attemptedFixHash: row.attempted_fix_hash,
            evidenceHash: row.evidence_hash,
          },
          seenCount: row.seen_count,
          lastSeenAt: row.time_updated,
        }))
      })
    })

    const decayTick = Effect.fn("MemoryOS.decayTick")(function* (projectId: ProjectID) {
      yield* db((tx) => {
        tx.delete(MemoryEvidenceTable)
          .where(and(eq(MemoryEvidenceTable.project_id, projectId), eq(MemoryEvidenceTable.tier, "working")))
          .run()
      })
    })

    return Service.of({
      record,
      recall,
      retryDecision,
      recordFailedAttempt,
      failedAttemptsForSignature,
      decayTick,
    })
  }),
)

export * as MemoryOS from "./memory"
