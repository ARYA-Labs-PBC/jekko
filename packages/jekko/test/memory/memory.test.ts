import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { MemoryOS } from "../../src/memory/memory"
import { Database } from "../../src/storage/db"
import { MemoryEvidenceTable, FailedAttemptTable } from "../../src/memory/schema"
import { ProjectTable } from "../../src/project/project.sql"
import { sql } from "drizzle-orm"
import { Effect, Layer } from "effect"

// The Memory module uses Database.use() which needs the DB initialized.
// We'll verify this works with the actual DB since that's how jekko operates.

describe("Memory OS", () => {
  const projectId = "test-project-memory" as any
  let testLayer: Layer.Layer<MemoryOS.Service, never, never>

  beforeEach(() => {
    testLayer = MemoryOS.layer
    // Clear test data before each test
    Database.use((db) => {
      db.insert(ProjectTable)
        .values({
          id: projectId,
          worktree: "/test/worktree",
          sandboxes: [],
          time_created: Date.now(),
          time_updated: Date.now(),
        })
        .onConflictDoNothing()
        .run()

      db.delete(MemoryEvidenceTable)
        .where(sql`${MemoryEvidenceTable.project_id} = ${projectId}`)
        .run()
      db.delete(FailedAttemptTable)
        .where(sql`${FailedAttemptTable.project_id} = ${projectId}`)
        .run()
    })
  })

  const runTest = <A>(eff: Effect.Effect<A, any, MemoryOS.Service>) =>
    Effect.runPromise(eff.pipe(Effect.provide(testLayer)))

  describe("record and recall", () => {
    test("record and recall working fact", async () => {
      await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        yield* mem.record({
          type: "record_working",
          projectId,
          fact: { taskId: "turn-1", kind: "observation", body: "found a bug in auth module", tokens: 50 },
        })
      }))

      const hits = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        return yield* mem.recall({ projectId, tier: "working" })
      }))
      
      expect(hits.length).toBeGreaterThanOrEqual(1)
      expect(hits[0].tier).toBe("working")
      expect(hits[0].snippet).toContain("found a bug")
    })

    test("record and recall episodic fact", async () => {
      await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        yield* mem.record({
          type: "record_episodic",
          projectId,
          fact: { sessionId: "session-1", summary: "fixed auth module", outcome: "success" },
        })
      }))

      const hits = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        return yield* mem.recall({ projectId, tier: "episodic" })
      }))
      expect(hits.length).toBeGreaterThanOrEqual(1)
      expect(hits[0].tier).toBe("episodic")
    })

    test("record and recall semantic fact", async () => {
      await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        yield* mem.record({
          type: "record_semantic",
          projectId,
          fact: { subject: "auth.ts", predicate: "exports", object: "AuthService class", confidence: 0.9 },
        })
      }))

      const hits = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        return yield* mem.recall({ projectId, tier: "semantic" })
      }))
      expect(hits.length).toBeGreaterThanOrEqual(1)
      expect(hits[0].tier).toBe("semantic")
    })

    test("record and recall procedural skill", async () => {
      await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        yield* mem.record({
          type: "record_procedural",
          projectId,
          fact: {
            name: "run-tests",
            triggerPattern: "after editing test files",
            stepsYaml: "- run: bun test",
            successCount: 5,
            failureCount: 0,
          },
        })
      }))

      const hits = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        return yield* mem.recall({ projectId, tier: "procedural" })
      }))
      expect(hits.length).toBeGreaterThanOrEqual(1)
      expect(hits[0].tier).toBe("procedural")
    })

    test("record and recall negative signature", async () => {
      await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        yield* mem.record({
          type: "record_negative",
          projectId,
          fact: { signature: "test::auth::login_fails", failureKind: "test_failure", seenCount: 1 },
        })
      }))

      const hits = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        return yield* mem.recall({ projectId, tier: "negative" })
      }))
      expect(hits.length).toBeGreaterThanOrEqual(1)
      expect(hits[0].tier).toBe("negative")
    })

    test("record and recall rule", async () => {
      await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        yield* mem.record({
          type: "upsert_rule",
          projectId,
          rule: { id: "rule-1", state: "active", scope: "global", statement: "always run lint before commit", confidence: 1.0 },
        })
      }))

      const hits = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        return yield* mem.recall({ projectId, tier: "rule" })
      }))
      expect(hits.length).toBeGreaterThanOrEqual(1)
      expect(hits[0].tier).toBe("rule")
    })

    test("recall with text search", async () => {
      await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        yield* mem.record({
          type: "record_working",
          projectId,
          fact: { taskId: "turn-2", kind: "observation", body: "database connection pool exhausted", tokens: 30 },
        })
      }))

      const hits = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        return yield* mem.recall({ projectId, queryText: "database connection" })
      }))
      expect(hits.length).toBeGreaterThanOrEqual(1)
    })

    test("recall respects limit", async () => {
      await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        for (let i = 0; i < 5; i++) {
          yield* mem.record({
            type: "record_working",
            projectId,
            fact: { taskId: `turn-${i}`, kind: "observation", body: `observation ${i}`, tokens: 10 },
          })
        }
      }))

      const hits = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        return yield* mem.recall({ projectId, limit: 2 })
      }))
      expect(hits.length).toBe(2)
    })
  })

  describe("doom-loop prevention", () => {
    test("retry decision allows first attempt", async () => {
      const fingerprint: MemoryOS.FailureFingerprint = {
        signature: "test::auth::login",
        failureKind: "test_failure",
        attemptedFixHash: "fix-hash-1",
        evidenceHash: "evidence-hash-1",
      }

      const decision = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        return yield* mem.retryDecision(projectId, fingerprint)
      }))
      expect(decision.type).toBe("allow")
    })

    test("retry decision blocks identical fix", async () => {
      const fingerprint: MemoryOS.FailureFingerprint = {
        signature: "test::auth::login",
        failureKind: "test_failure",
        attemptedFixHash: "fix-hash-2",
        evidenceHash: "evidence-hash-2",
      }

      const decision = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        yield* mem.recordFailedAttempt(projectId, fingerprint, "session-1")
        return yield* mem.retryDecision(projectId, fingerprint)
      }))
      
      expect(decision.type).toBe("block")
      if (decision.type === "block") {
        expect(decision.reason).toContain("same failed fix already observed")
        expect(decision.reason).toContain("test::auth::login")
      }
    })

    test("retry decision allows different fix hash", async () => {
      const fingerprint1: MemoryOS.FailureFingerprint = {
        signature: "test::auth::login",
        failureKind: "test_failure",
        attemptedFixHash: "fix-hash-A",
        evidenceHash: "evidence-hash-X",
      }

      const fingerprint2: MemoryOS.FailureFingerprint = {
        ...fingerprint1,
        attemptedFixHash: "fix-hash-B",
      }

      const decision = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        yield* mem.recordFailedAttempt(projectId, fingerprint1, "session-1")
        return yield* mem.retryDecision(projectId, fingerprint2)
      }))
      expect(decision.type).toBe("allow")
    })

    test("retry decision allows different evidence hash", async () => {
      const fingerprint1: MemoryOS.FailureFingerprint = {
        signature: "test::auth::login",
        failureKind: "test_failure",
        attemptedFixHash: "fix-hash-C",
        evidenceHash: "evidence-hash-Y",
      }

      const fingerprint2: MemoryOS.FailureFingerprint = {
        ...fingerprint1,
        evidenceHash: "evidence-hash-Z",
      }

      const decision = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        yield* mem.recordFailedAttempt(projectId, fingerprint1, "session-1")
        return yield* mem.retryDecision(projectId, fingerprint2)
      }))
      expect(decision.type).toBe("allow")
    })

    test("failed attempt count increments on duplicate", async () => {
      const fingerprint: MemoryOS.FailureFingerprint = {
        signature: "test::compile::error",
        failureKind: "compile_error",
        attemptedFixHash: "fix-dup",
        evidenceHash: "evidence-dup",
      }

      const records = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        yield* mem.recordFailedAttempt(projectId, fingerprint)
        yield* mem.recordFailedAttempt(projectId, fingerprint)
        return yield* mem.failedAttemptsForSignature(projectId, "test::compile::error")
      }))
      
      expect(records.length).toBeGreaterThanOrEqual(1)
      const match = records.find((r) => r.fingerprint.attemptedFixHash === "fix-dup")
      expect(match).toBeTruthy()
      expect(match!.seenCount).toBe(2)
    })
  })

  describe("failed attempts query", () => {
    test("query by signature returns matching records", async () => {
      const records = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        yield* mem.recordFailedAttempt(projectId, {
          signature: "test::query_target",
          failureKind: "runtime_error",
          attemptedFixHash: "fix-q1",
          evidenceHash: "evidence-q1",
        })
        yield* mem.recordFailedAttempt(projectId, {
          signature: "test::query_target",
          failureKind: "runtime_error",
          attemptedFixHash: "fix-q2",
          evidenceHash: "evidence-q2",
        })
        yield* mem.recordFailedAttempt(projectId, {
          signature: "test::other_target",
          failureKind: "runtime_error",
          attemptedFixHash: "fix-q3",
          evidenceHash: "evidence-q3",
        })
        return yield* mem.failedAttemptsForSignature(projectId, "test::query_target")
      }))

      expect(records.length).toBe(2)
      expect(records.every((r) => r.fingerprint.signature === "test::query_target")).toBe(true)
    })
  })

  describe("decay", () => {
    test("decay tick clears working memory but preserves other tiers", async () => {
      const { workingBefore, workingAfter, semantic } = await runTest(Effect.gen(function* () {
        const mem = yield* MemoryOS.Service
        yield* mem.record({
          type: "record_working",
          projectId,
          fact: { taskId: "turn-decay", kind: "obs", body: "working fact", tokens: 10 },
        })
        yield* mem.record({
          type: "record_semantic",
          projectId,
          fact: { subject: "X", predicate: "is", object: "Y", confidence: 1.0 },
        })

        const workingBefore = yield* mem.recall({ projectId, tier: "working" })
        yield* mem.decayTick(projectId)
        const workingAfter = yield* mem.recall({ projectId, tier: "working" })
        const semantic = yield* mem.recall({ projectId, tier: "semantic" })

        return { workingBefore, workingAfter, semantic }
      }))

      expect(workingBefore.length).toBeGreaterThanOrEqual(1)
      expect(workingAfter.length).toBe(0)
      expect(semantic.length).toBeGreaterThanOrEqual(1)
    })
  })
})
