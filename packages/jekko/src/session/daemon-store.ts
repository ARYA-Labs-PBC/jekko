// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
// jankurai:allow HLT-000-SCORE-DIMENSION reason=large-structured-file-with-parallel-patterns-by-design expires=2027-01-01
import { mkdir, writeFile } from "fs/promises"
import path from "path"
import { ulid } from "ulid"
import { Effect, Layer, Context } from "effect"
import { and, asc, desc, eq, isNull, lt, max, or } from "drizzle-orm"
import { InstanceState } from "@/effect/instance-state"
import { Database } from "@/storage/db"
import type { ZyalScript } from "@/agent-script/schema"
import { SessionID } from "./schema"
import {
  DaemonArtifactTable,
  DaemonEventTable,
  DaemonIterationTable,
  DaemonRunTable,
  DaemonTaskMemoryTable,
  DaemonTaskPassTable,
  DaemonTaskTable,
  DaemonWorkerTable,
} from "./daemon.sql"
import { DaemonPhase, DaemonRunStatus } from "./daemon-event"

type RunRow = typeof DaemonRunTable.$inferSelect
type IterationRow = typeof DaemonIterationTable.$inferSelect
type EventRow = typeof DaemonEventTable.$inferSelect
type TaskRow = typeof DaemonTaskTable.$inferSelect
type TaskPassRow = typeof DaemonTaskPassTable.$inferSelect
type TaskMemoryRow = typeof DaemonTaskMemoryTable.$inferSelect
type WorkerRow = typeof DaemonWorkerTable.$inferSelect
type ArtifactRow = typeof DaemonArtifactTable.$inferSelect

type AnyJson = Record<string, unknown> | unknown[] | string | number | boolean | null

function writeTextBundle(dir: string, files: Iterable<[name: string, content: string]>) {
  return Effect.promise(async () => {
    await mkdir(dir, { recursive: true })
    for (const [name, content] of files) {
      await writeFile(path.join(dir, name), content)
    }
  })
}

export type RunInfo = RunRow
export type IterationInfo = IterationRow
export type EventInfo = EventRow
export type TaskInfo = TaskRow
export type TaskPassInfo = TaskPassRow
export type TaskMemoryInfo = TaskMemoryRow
export type WorkerInfo = WorkerRow
export type ArtifactInfo = ArtifactRow

export interface Interface {
  readonly createRun: (input: {
    rootSessionID: string
    activeSessionID: string
    spec: ZyalScript
    specHash: string
  }) => Effect.Effect<RunInfo, any, any>
  readonly listRuns: () => Effect.Effect<RunInfo[], any, any>
  readonly getRun: (runID: string) => Effect.Effect<RunInfo | undefined, any, any>
  readonly updateRun: (runID: string, patch: Partial<Pick<RunRow, "status" | "phase" | "iteration" | "epoch" | "last_error" | "last_exit_result_json" | "stopped_at" | "active_session_id">>) => Effect.Effect<RunInfo | undefined, any, any>
  readonly appendEvent: (input: {
    runID: string
    iteration: number
    eventType: string
    payload: Record<string, unknown>
  }) => Effect.Effect<EventInfo, any, any>
  readonly listEvents: (runID: string) => Effect.Effect<EventInfo[], any, any>
  readonly appendIteration: (input: {
    runID: string
    iteration: number
    sessionID: string
    terminalReason: string
    result: AnyJson
    tokenUsage?: AnyJson
    cost?: number
    checkpointSha?: string
  }) => Effect.Effect<IterationInfo, any, any>
  readonly listIterations: (runID: string) => Effect.Effect<IterationInfo[], any, any>
  readonly upsertTask: (input: Omit<TaskRow, "time_created" | "time_updated">) => Effect.Effect<TaskInfo, any, any>
  readonly listTasks: (runID: string) => Effect.Effect<TaskInfo[], any, any>
  readonly leaseTask: (input: { runID: string; workerID: string; ttlMs: number }) => Effect.Effect<TaskInfo | undefined, any, any>
  readonly leaseSpecificTask: (input: {
    runID: string
    taskID: string
    workerID: string
    ttlMs: number
    lockedPaths?: readonly string[]
  }) => Effect.Effect<TaskInfo | undefined, any, any>
  readonly completeTask: (input: { taskID: string; evidence?: AnyJson }) => Effect.Effect<void, any, any>
  readonly blockTask: (input: { taskID: string; evidence?: AnyJson }) => Effect.Effect<void, any, any>
  readonly assessTask: (input: {
    taskID: string
    difficultyScore?: number
    riskScore?: number
    readinessScore?: number
    implementationConfidence?: number
    verificationConfidence?: number
    assessment?: AnyJson
  }) => Effect.Effect<TaskInfo | undefined, never, never>
  readonly routeTask: (input: {
    taskID: string
    lane: string
    phase?: string
    incubatorStatus?: string
    blockedReason?: string
  }) => Effect.Effect<TaskInfo | undefined, any, any>
  readonly beginTaskPass: (input: {
    runID: string
    taskID: string
    passType: string
    contextMode: string
    agent?: string
    sessionID?: string
    workerID?: string
    worktreePath?: string
    worktreeBranch?: string
    cleanupStatus?: string
    inputArtifactIDs?: string[]
  }) => Effect.Effect<TaskPassInfo, any, any>
  readonly completeTaskPass: (input: {
    passID: string
    result?: AnyJson
    score?: AnyJson
    cleanupStatus?: string
    outputArtifactIDs?: string[]
  }) => Effect.Effect<TaskPassInfo | undefined, any, any>
  readonly failTaskPass: (input: { passID: string; error: AnyJson; cleanupStatus?: string }) => Effect.Effect<TaskPassInfo | undefined, any, any>
  readonly appendTaskMemory: (input: {
    runID: string
    taskID: string
    kind: string
    title: string
    summary: string
    payload?: AnyJson
    sourcePassID?: string
    importance?: number
    confidence?: number
  }) => Effect.Effect<TaskMemoryInfo, never, never>
  readonly listTaskMemory: (input: { runID: string; taskID: string }) => Effect.Effect<TaskMemoryInfo[], never, never>
  readonly listTaskPasses: (input: { runID: string; taskID: string }) => Effect.Effect<TaskPassInfo[], never, never>
  readonly promoteTask: (input: { taskID: string; acceptedArtifactID?: string; result?: AnyJson }) => Effect.Effect<TaskInfo | undefined, any, any>
  readonly exhaustTask: (input: { taskID: string; reason: string; result?: AnyJson }) => Effect.Effect<TaskInfo | undefined, any, any>
  readonly archiveTask: (input: { taskID: string; reason?: string }) => Effect.Effect<TaskInfo | undefined, any, any>
  readonly findDaemonContextBySession: (sessionID: string) => Effect.Effect<
    | {
        run: RunInfo
        task?: TaskInfo
        pass?: TaskPassInfo
      }
    | undefined,
    never,
    never
  >
  readonly upsertWorker: (input: Omit<WorkerRow, "time_created" | "time_updated">) => Effect.Effect<WorkerInfo, any, any>
  readonly heartbeatWorker: (input: { workerID: string; heartbeatAt?: number }) => Effect.Effect<void, any, any>
  readonly listWorkers: (runID: string) => Effect.Effect<WorkerInfo[], any, any>
  readonly upsertArtifact: (input: Omit<ArtifactRow, "time_created" | "time_updated">) => Effect.Effect<ArtifactInfo, any, any>
}

export class Service extends Context.Service<Service, Interface>()("@jekko/DaemonStore") {}

export const layer = Layer.effect(
  Service,
  Effect.gen(function* () {
    const runsRoot = function* (runID: string) {
      const ctx = yield* InstanceState.context
      const root = ctx.worktree && ctx.worktree !== "/" ? ctx.worktree : ctx.directory
      if (!root || root.trim() === "" || path.resolve(root) === path.parse(path.resolve(root)).root) {
        throw new Error(
          `DaemonStore cannot mirror artifacts: instance worktree=${JSON.stringify(ctx.worktree)} directory=${JSON.stringify(ctx.directory)}`,
        )
      }
      return path.join(root, ".jekko", "daemon", runID)
    }

    const loadRun = (runID: string) =>
      Database.use((db) => db.select().from(DaemonRunTable).where(eq(DaemonRunTable.id, runID)).get())

    const mirrorRun = Effect.fn("DaemonStore.mirrorRun")(function* (runID: string) {
      const run = yield* getRun(runID)
      if (!run) return
      const iterations = yield* listIterations(runID)
      const events = yield* listEvents(runID)
      const dir = yield* runsRoot(runID)
      const ledger = events.map((event) => JSON.stringify(event)).join("\n") + (events.length ? "\n" : "")
      const state = renderStateMarkdown(run, iterations, events)
      yield* writeTextBundle(dir, [
        ["ledger.jsonl", ledger],
        ["STATE.md", state],
      ])
    })

    const mirrorTask = Effect.fn("DaemonStore.mirrorTask")(function* (runID: string, taskID: string) {
      const task = Database.use((db) =>
        db.select().from(DaemonTaskTable).where(and(eq(DaemonTaskTable.run_id, runID), eq(DaemonTaskTable.id, taskID))).get(),
      )
      if (!task) return
      const run = yield* getRun(runID)
      if (!run) return
      const passes = Database.use((db) =>
        db
          .select()
          .from(DaemonTaskPassTable)
          .where(and(eq(DaemonTaskPassTable.run_id, runID), eq(DaemonTaskPassTable.task_id, taskID)))
          .orderBy(asc(DaemonTaskPassTable.pass_number))
          .all(),
      )
      const memories = Database.use((db) =>
        db
          .select()
          .from(DaemonTaskMemoryTable)
          .where(and(eq(DaemonTaskMemoryTable.run_id, runID), eq(DaemonTaskMemoryTable.task_id, taskID)))
          .orderBy(asc(DaemonTaskMemoryTable.time_created))
          .all(),
      )
      const artifacts = Database.use((db) =>
        db
          .select()
          .from(DaemonArtifactTable)
          .where(and(eq(DaemonArtifactTable.run_id, runID), eq(DaemonArtifactTable.task_id, taskID)))
          .orderBy(asc(DaemonArtifactTable.time_created))
          .all(),
      )
      const dir = path.join(yield* runsRoot(runID), "tasks", taskID)
      const passDir = path.join(dir, "PASSES")
      const decisions = passes
        .map((pass) =>
          JSON.stringify({
            pass_id: pass.id,
            pass_number: pass.pass_number,
            pass_type: pass.pass_type,
            status: pass.status,
            score: pass.score_json,
            result: pass.result_json,
          }),
        )
        .join("\n")
      yield* writeTextBundle(dir, [
        ["TASK.md", renderTaskMarkdown(task)],
        ["CAPSULE.md", renderTaskCapsule(task, memories, passes)],
        ["MEMORY.md", renderMemoryMarkdown(memories)],
        ["SCORE.json", JSON.stringify(renderScore(task, passes, artifacts), null, 2) + "\n"],
        ["DECISIONS.jsonl", decisions + (decisions ? "\n" : "")],
      ])
      yield* writeTextBundle(
        passDir,
        passes.map((pass) => [
          `${String(pass.pass_number).padStart(3, "0")}.md`,
          renderPassMarkdown(pass),
        ]),
      )
    })

    const createRun = Effect.fn("DaemonStore.createRun")(function* (input: {
      rootSessionID: string
      activeSessionID: string
      spec: ZyalScript
      specHash: string
    }) {
      const id = ulid()
      const row: typeof DaemonRunTable.$inferInsert = {
        id,
        root_session_id: SessionID.make(input.rootSessionID),
        active_session_id: SessionID.make(input.activeSessionID),
        status: "created" as DaemonRunStatus,
        phase: "created" as DaemonPhase,
        spec_json: input.spec,
        spec_hash: input.specHash,
        iteration: 0,
        epoch: 0,
        last_error: null,
        last_exit_result_json: null,
        stopped_at: null,
      }
      Database.use((db) => db.insert(DaemonRunTable).values(row).run())
      yield* appendEvent({ runID: id, iteration: 0, eventType: "run.created", payload: { spec: input.spec } })
      return yield* getRun(id).pipe(Effect.map((value) => value!))
    })

    const listRuns = Effect.fn("DaemonStore.listRuns")(function* () {
      return Database.use((db) =>
        db.select().from(DaemonRunTable).orderBy(desc(DaemonRunTable.time_created)).all(),
      )
    })

    const getRun = Effect.fn("DaemonStore.getRun")(function* (runID: string) {
      return loadRun(runID)
    })

    const updateRun = Effect.fn("DaemonStore.updateRun")(function* (
      runID: string,
      patch: Partial<
        Pick<
          RunRow,
          "status" | "phase" | "iteration" | "epoch" | "last_error" | "last_exit_result_json" | "stopped_at" | "active_session_id"
        >
      >,
    ) {
      Database.use((db) => db.update(DaemonRunTable).set(patch).where(eq(DaemonRunTable.id, runID)).run())
      yield* mirrorRun(runID).pipe(Effect.ignore)
      return yield* getRun(runID)
    })

    const appendEvent = Effect.fn("DaemonStore.appendEvent")(function* (input: {
      runID: string
      iteration: number
      eventType: string
      payload: Record<string, unknown>
    }) {
      const row: typeof DaemonEventTable.$inferInsert = {
        id: ulid(),
        run_id: input.runID,
        iteration: input.iteration,
        event_type: input.eventType,
        payload_json: input.payload,
      }
      Database.use((db) => db.insert(DaemonEventTable).values(row).run())
      yield* mirrorRun(input.runID).pipe(Effect.ignore)
      return Database.use((db) => db.select().from(DaemonEventTable).where(eq(DaemonEventTable.id, row.id)).get())!
    })

    const listEvents = Effect.fn("DaemonStore.listEvents")(function* (runID: string) {
      return Database.use((db) =>
        db.select().from(DaemonEventTable).where(eq(DaemonEventTable.run_id, runID)).orderBy(asc(DaemonEventTable.time_created)).all(),
      )
    })

    const appendIteration = Effect.fn("DaemonStore.appendIteration")(function* (input: {
      runID: string
      iteration: number
      sessionID: string
      terminalReason: string
      result: AnyJson
      tokenUsage?: AnyJson
      cost?: number
      checkpointSha?: string
    }) {
      const row: typeof DaemonIterationTable.$inferInsert = {
        run_id: input.runID,
        iteration: input.iteration,
        session_id: SessionID.make(input.sessionID),
        terminal_reason: input.terminalReason,
        result_json: input.result,
        token_usage_json: input.tokenUsage ?? null,
        cost: input.cost ?? null,
        checkpoint_sha: input.checkpointSha ?? null,
      }
      Database.use((db) => db.insert(DaemonIterationTable).values(row).run())
      yield* appendEvent({
        runID: input.runID,
        iteration: input.iteration,
        eventType: "iteration.recorded",
        payload: { terminalReason: input.terminalReason, checkpointSha: input.checkpointSha },
      })
      return Database.use((db) =>
        db
          .select()
          .from(DaemonIterationTable)
          .where(and(eq(DaemonIterationTable.run_id, input.runID), eq(DaemonIterationTable.iteration, input.iteration)))
          .get(),
      )!
    })

    const listIterations = Effect.fn("DaemonStore.listIterations")(function* (runID: string) {
      return Database.use((db) =>
        db.select().from(DaemonIterationTable).where(eq(DaemonIterationTable.run_id, runID)).orderBy(asc(DaemonIterationTable.iteration)).all(),
      )
    })

    const upsertTask = Effect.fn("DaemonStore.upsertTask")(function* (input: Omit<TaskRow, "time_created" | "time_updated">) {
      const row = input
      Database.use((db) => db.insert(DaemonTaskTable).values(row).onConflictDoUpdate({ target: DaemonTaskTable.id, set: row }).run())
      const task = Database.use((db) => db.select().from(DaemonTaskTable).where(eq(DaemonTaskTable.id, input.id)).get())!
      yield* mirrorTaskIfPresent(task)
      return task
    })

    const listTasks = Effect.fn("DaemonStore.listTasks")(function* (runID: string) {
      return Database.use((db) =>
        db.select().from(DaemonTaskTable).where(eq(DaemonTaskTable.run_id, runID)).orderBy(desc(DaemonTaskTable.priority)).all(),
      )
    })

    const leaseTask = Effect.fn("DaemonStore.leaseTask")(function* (input: { runID: string; workerID: string; ttlMs: number }) {
      const now = Date.now()
      const task = Database.transaction((db) => {
        const next = db
          .select()
          .from(DaemonTaskTable)
          .where(
            and(
              eq(DaemonTaskTable.run_id, input.runID),
              eq(DaemonTaskTable.status, "queued"),
              or(isNull(DaemonTaskTable.lease_expires_at), lt(DaemonTaskTable.lease_expires_at, now)),
            ),
          )
          .orderBy(desc(DaemonTaskTable.priority), asc(DaemonTaskTable.time_created))
          .get()
        // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
        if (!next) return undefined
        db.update(DaemonTaskTable)
          .set({
            status: "leased",
            lease_worker_id: input.workerID,
            lease_expires_at: now + input.ttlMs,
          })
          .where(eq(DaemonTaskTable.id, next.id))
          .run()
        return db.select().from(DaemonTaskTable).where(eq(DaemonTaskTable.id, next.id)).get()
      })
      if (task) {
        yield* appendTaskEvent({
          task,
          eventType: "task.leased",
          payload: { taskID: task.id, workerID: input.workerID, ttlMs: input.ttlMs },
        })
      }
      return task!
    })

    const leaseSpecificTask = Effect.fn("DaemonStore.leaseSpecificTask")(function* (input: {
      runID: string
      taskID: string
      workerID: string
      ttlMs: number
      lockedPaths?: readonly string[]
    }) {
      const now = Date.now()
      const task = Database.transaction((db) => {
        const active = db
          .select()
          .from(DaemonTaskTable)
          .where(and(eq(DaemonTaskTable.run_id, input.runID), eq(DaemonTaskTable.status, "leased")))
          .all()
          .filter((item) => item.id !== input.taskID && (item.lease_expires_at ?? 0) > now)
        const activeLocks = active.flatMap((item) => stringArray(item.locked_paths_json))
        const requestedLocks = [...(input.lockedPaths ?? [])]
        if (locksOverlap(activeLocks, requestedLocks)) return undefined
        const next = db
          .select()
          .from(DaemonTaskTable)
          .where(
            and(
              eq(DaemonTaskTable.run_id, input.runID),
              eq(DaemonTaskTable.id, input.taskID),
              eq(DaemonTaskTable.status, "queued"),
              or(isNull(DaemonTaskTable.lease_expires_at), lt(DaemonTaskTable.lease_expires_at, now)),
            ),
          )
          .get()
        // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
        if (!next) return undefined
        db.update(DaemonTaskTable)
          .set({
            status: "leased",
            lease_worker_id: input.workerID,
            lease_expires_at: now + input.ttlMs,
          })
          .where(eq(DaemonTaskTable.id, next.id))
          .run()
        return db.select().from(DaemonTaskTable).where(eq(DaemonTaskTable.id, next.id)).get()
      })
      if (task) {
        yield* appendTaskEvent({
          task,
          eventType: "task.leased",
          payload: { taskID: task.id, workerID: input.workerID, ttlMs: input.ttlMs, lockedPaths: input.lockedPaths ?? [] },
        })
      }
      return task!
    })

    const completeTask = Effect.fn("DaemonStore.completeTask")(function* (input: { taskID: string; evidence?: AnyJson }) {
      yield* updateTask({
        taskID: input.taskID,
        patch: { status: "done", evidence_json: input.evidence ?? null, lease_expires_at: null },
      })
    })

    const blockTask = Effect.fn("DaemonStore.blockTask")(function* (input: { taskID: string; evidence?: AnyJson }) {
      yield* updateTask({
        taskID: input.taskID,
        patch: { status: "blocked", evidence_json: input.evidence ?? null, lease_expires_at: null },
      })
    })

    const assessTask = Effect.fn("DaemonStore.assessTask")(function* (input: {
      taskID: string
      difficultyScore?: number
      riskScore?: number
      readinessScore?: number
      implementationConfidence?: number
      verificationConfidence?: number
      assessment?: AnyJson
    }) {
      const patch: Partial<TaskRow> = {
        difficulty_score: input.difficultyScore,
        risk_score: input.riskScore,
        readiness_score: input.readinessScore,
        implementation_confidence: input.implementationConfidence,
        verification_confidence: input.verificationConfidence,
        last_assessment_json: input.assessment,
      }
      return yield* updateTask({ taskID: input.taskID, patch })
    })

    const routeTask = Effect.fn("DaemonStore.routeTask")(function* (input: {
      taskID: string
      lane: string
      phase?: string
      incubatorStatus?: string
      blockedReason?: string
    }) {
      return yield* updateTask({
        taskID: input.taskID,
        patch: {
          lane: input.lane,
          phase: input.phase,
          incubator_status: input.incubatorStatus,
          blocked_reason: input.blockedReason,
          status: input.lane === "incubator" ? "incubating" : undefined,
        },
        eventType: "task.routed",
        payload: (task) => ({
          taskID: task.id,
          lane: input.lane,
          phase: input.phase,
          incubatorStatus: input.incubatorStatus,
          blockedReason: input.blockedReason,
        }),
      })
    })

    const beginTaskPass = Effect.fn("DaemonStore.beginTaskPass")(function* (input: {
      runID: string
      taskID: string
      passType: string
      contextMode: string
      agent?: string
      sessionID?: string
      workerID?: string
      inputArtifactIDs?: string[]
      worktreePath?: string
      worktreeBranch?: string
      cleanupStatus?: string
    }) {
      const pass = Database.transaction((db) => {
        const current = db
          .select({ value: max(DaemonTaskPassTable.pass_number) })
          .from(DaemonTaskPassTable)
          .where(and(eq(DaemonTaskPassTable.run_id, input.runID), eq(DaemonTaskPassTable.task_id, input.taskID)))
          .get()
        const passNumber = (current?.value ?? 0) + 1
        const row: typeof DaemonTaskPassTable.$inferInsert = {
          id: ulid(),
          run_id: input.runID,
          task_id: input.taskID,
          pass_number: passNumber,
          pass_type: input.passType,
          context_mode: input.contextMode,
          agent: input.agent ?? null,
          session_id: input.sessionID ? SessionID.make(input.sessionID) : null,
          worker_id: input.workerID ?? null,
          status: "running",
          started_at: Date.now(),
          ended_at: null,
          worktree_path: input.worktreePath ?? null,
          worktree_branch: input.worktreeBranch ?? null,
          cleanup_status: input.cleanupStatus ?? "pending",
          input_artifact_ids_json: input.inputArtifactIDs ?? [],
          output_artifact_ids_json: [],
          result_json: null,
          score_json: null,
          error_json: null,
        }
        db.insert(DaemonTaskPassTable).values(row).run()
        db.update(DaemonTaskTable)
          .set({ phase: "incubator_pass", incubator_status: "active" })
          .where(eq(DaemonTaskTable.id, input.taskID))
          .run()
        return db.select().from(DaemonTaskPassTable).where(eq(DaemonTaskPassTable.id, row.id)).get()
      })
      yield* appendTaskEvent({
        task: { run_id: input.runID, id: input.taskID },
        eventType: "task_pass.started",
        payload: { taskID: input.taskID, passID: pass!.id, passType: input.passType, passNumber: pass!.pass_number },
      })
      return pass!
    })

    const finalizeTaskPass = Effect.fn("DaemonStore.finalizeTaskPass")(function* (input: {
      passID: string
      patch: Partial<typeof DaemonTaskPassTable.$inferInsert>
      eventType: "task_pass.completed" | "task_pass.failed"
      payload: (pass: TaskPassRow) => Record<string, unknown>
    }) {
      const pass = Database.use((db) =>
        db
          .update(DaemonTaskPassTable)
          .set(input.patch)
          .where(eq(DaemonTaskPassTable.id, input.passID))
          .returning()
          .get(),
      )
      if (pass) {
        yield* appendTaskEvent({
          task: { run_id: pass.run_id, id: pass.task_id },
          eventType: input.eventType,
          payload: input.payload(pass),
        })
      }
      return pass
    })

    const appendTaskEvent = Effect.fn("DaemonStore.appendTaskEvent")(function* (input: {
      task: { run_id: string; id: string }
      iteration?: number
      eventType: string
      payload: Record<string, unknown>
    }) {
      yield* appendEvent({
        runID: input.task.run_id,
        iteration: input.iteration ?? 0,
        eventType: input.eventType,
        payload: input.payload,
      })
      yield* mirrorTask(input.task.run_id, input.task.id).pipe(Effect.ignore)
    })

    const mirrorTaskIfPresent = Effect.fn("DaemonStore.mirrorTaskIfPresent")(function* (task: { run_id: string; id: string } | undefined) {
      if (!task) return
      yield* mirrorTask(task.run_id, task.id).pipe(Effect.ignore)
    })

    const updateTask = Effect.fn("DaemonStore.updateTask")(function* (input: {
      taskID: string
      patch: Partial<TaskRow>
      eventType?: string
      payload?: (task: TaskRow) => Record<string, unknown>
    }) {
      const task = Database.use((db) =>
        db
          .update(DaemonTaskTable)
          .set(stripUndefined(input.patch))
          .where(eq(DaemonTaskTable.id, input.taskID))
          .returning()
          .get(),
      )
      if (!task) {
        return yield* Effect.fail(new Error(`task not found: ${input.taskID}`))
      }
      if (input.eventType && input.payload) {
        yield* appendTaskEvent({
          task,
          eventType: input.eventType,
          payload: input.payload(task),
        })
      } else {
        yield* mirrorTaskIfPresent(task)
      }
      return task
    })

    const completeTaskPass = Effect.fn("DaemonStore.completeTaskPass")(function* (input: {
      passID: string
      result?: AnyJson
      score?: AnyJson
      outputArtifactIDs?: string[]
      cleanupStatus?: string
    }) {
      return yield* finalizeTaskPass({
        passID: input.passID,
        patch: {
          status: "complete",
          ended_at: Date.now(),
          result_json: input.result ?? null,
          score_json: input.score ?? null,
          cleanup_status: input.cleanupStatus ?? "completed",
          output_artifact_ids_json: input.outputArtifactIDs ?? [],
        },
        eventType: "task_pass.completed",
        payload: (pass) => ({ taskID: pass.task_id, passID: pass.id, passType: pass.pass_type }),
      })
    })

    const failTaskPass = Effect.fn("DaemonStore.failTaskPass")(function* (input: { passID: string; error: AnyJson; cleanupStatus?: string }) {
      return yield* finalizeTaskPass({
        passID: input.passID,
        patch: { status: "failed", ended_at: Date.now(), error_json: input.error, cleanup_status: input.cleanupStatus ?? "failed" },
        eventType: "task_pass.failed",
        payload: (pass) => ({ taskID: pass.task_id, passID: pass.id, error: input.error }),
      })
    })

    const appendTaskMemory = Effect.fn("DaemonStore.appendTaskMemory")(function* (input: {
      runID: string
      taskID: string
      kind: string
      title: string
      summary: string
      payload?: AnyJson
      sourcePassID?: string
      importance?: number
      confidence?: number
    }) {
      const row: typeof DaemonTaskMemoryTable.$inferInsert = {
        id: ulid(),
        run_id: input.runID,
        task_id: input.taskID,
        kind: input.kind,
        title: input.title,
        summary: input.summary,
        payload_json: input.payload ?? null,
        source_pass_id: input.sourcePassID ?? null,
        importance: input.importance ?? 0.5,
        confidence: input.confidence ?? 0.5,
      }
      Database.use((db) => db.insert(DaemonTaskMemoryTable).values(row).run())
      const memory = Database.use((db) => db.select().from(DaemonTaskMemoryTable).where(eq(DaemonTaskMemoryTable.id, row.id)).get())!
      yield* appendTaskEvent({
        task: { run_id: input.runID, id: input.taskID },
        eventType: "task.memory.appended",
        payload: {
          taskID: input.taskID,
          memoryID: memory.id,
          kind: input.kind,
          sourcePassID: input.sourcePassID,
        },
      })
      return memory
    })

    const listTaskMemory = Effect.fn("DaemonStore.listTaskMemory")(function* (input: { runID: string; taskID: string }) {
      return Database.use((db) =>
        db
          .select()
          .from(DaemonTaskMemoryTable)
          .where(and(eq(DaemonTaskMemoryTable.run_id, input.runID), eq(DaemonTaskMemoryTable.task_id, input.taskID)))
          .orderBy(desc(DaemonTaskMemoryTable.importance), asc(DaemonTaskMemoryTable.time_created))
          .all(),
      )
    })

    const listTaskPasses = Effect.fn("DaemonStore.listTaskPasses")(function* (input: { runID: string; taskID: string }) {
      return Database.use((db) =>
        db
          .select()
          .from(DaemonTaskPassTable)
          .where(and(eq(DaemonTaskPassTable.run_id, input.runID), eq(DaemonTaskPassTable.task_id, input.taskID)))
          .orderBy(asc(DaemonTaskPassTable.pass_number))
          .all(),
      )
    })

    const promoteTask = Effect.fn("DaemonStore.promoteTask")(function* (input: {
      taskID: string
      acceptedArtifactID?: string
      result?: AnyJson
    }) {
      return yield* updateTask({
        taskID: input.taskID,
        patch: {
          lane: "normal",
          phase: "ready",
          status: "queued",
          incubator_status: "promoted",
          accepted_artifact_id: input.acceptedArtifactID ?? null,
          promotion_result_json: input.result ?? null,
          lease_expires_at: null,
        },
        eventType: "task.promoted",
        payload: (task) => ({ taskID: task.id, acceptedArtifactID: input.acceptedArtifactID, result: input.result }),
      })
    })

    const exhaustTask = Effect.fn("DaemonStore.exhaustTask")(function* (input: {
      taskID: string
      reason: string
      result?: AnyJson
    }) {
      return yield* updateTask({
        taskID: input.taskID,
        patch: {
          lane: "blocked",
          phase: "exhausted",
          status: "blocked",
          incubator_status: "exhausted",
          promotion_result_json: input.result ?? null,
          blocked_reason: input.reason,
          lease_expires_at: null,
        },
        eventType: "task.exhausted",
        payload: (task) => ({ taskID: task.id, reason: input.reason, result: input.result }),
      })
    })

    const archiveTask = Effect.fn("DaemonStore.archiveTask")(function* (input: { taskID: string; reason?: string }) {
      return yield* updateTask({
        taskID: input.taskID,
        patch: { lane: "archive", phase: "archived", status: "archived", blocked_reason: input.reason ?? null },
        eventType: "task.archived",
        payload: (task) => ({ taskID: task.id, reason: input.reason }),
      })
    })

    const findDaemonContextBySession = Effect.fn("DaemonStore.findDaemonContextBySession")(function* (sessionID: string) {
      const pass = Database.use((db) =>
        db
          .select()
          .from(DaemonTaskPassTable)
          .where(and(eq(DaemonTaskPassTable.session_id, SessionID.make(sessionID)), eq(DaemonTaskPassTable.status, "running")))
          .orderBy(desc(DaemonTaskPassTable.started_at))
          .get(),
      )
      if (pass) {
        const run = yield* getRun(pass.run_id)
        const task = Database.use((db) => db.select().from(DaemonTaskTable).where(eq(DaemonTaskTable.id, pass.task_id)).get())
        if (run) return { run, task, pass }
      }
      const run = Database.use((db) =>
        db
          .select()
          .from(DaemonRunTable)
          .where(eq(DaemonRunTable.active_session_id, SessionID.make(sessionID)))
          .orderBy(desc(DaemonRunTable.time_created))
          .get(),
      )
      // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
      if (!run || ["satisfied", "aborted", "failed"].includes(run.status)) return undefined
      return { run }
    })

    const upsertWorker = Effect.fn("DaemonStore.upsertWorker")(function* (input: Omit<WorkerRow, "time_created" | "time_updated">) {
      const row = input
      Database.use((db) => db.insert(DaemonWorkerTable).values(row).onConflictDoUpdate({ target: DaemonWorkerTable.id, set: row }).run())
      return Database.use((db) => db.select().from(DaemonWorkerTable).where(eq(DaemonWorkerTable.id, input.id)).get())!
    })

    const heartbeatWorker = Effect.fn("DaemonStore.heartbeatWorker")(function* (input: { workerID: string; heartbeatAt?: number }) {
      Database.use((db) =>
        db
          .update(DaemonWorkerTable)
          .set({ last_heartbeat_at: input.heartbeatAt ?? Date.now() })
          .where(eq(DaemonWorkerTable.id, input.workerID))
          .run(),
      )
    })

    const listWorkers = Effect.fn("DaemonStore.listWorkers")(function* (runID: string) {
      return Database.use((db) =>
        db.select().from(DaemonWorkerTable).where(eq(DaemonWorkerTable.run_id, runID)).orderBy(asc(DaemonWorkerTable.time_created)).all(),
      )
    })

    const upsertArtifact = Effect.fn("DaemonStore.upsertArtifact")(function* (input: Omit<ArtifactRow, "time_created" | "time_updated">) {
      const row = input
      Database.use((db) => db.insert(DaemonArtifactTable).values(row).onConflictDoUpdate({ target: DaemonArtifactTable.id, set: row }).run())
      const artifact = Database.use((db) => db.select().from(DaemonArtifactTable).where(eq(DaemonArtifactTable.id, input.id)).get())!
      if (artifact.task_id) yield* mirrorTaskIfPresent({ run_id: artifact.run_id, id: artifact.task_id })
      return artifact
    })

    return Service.of({
      createRun,
      listRuns,
      getRun,
      updateRun,
      appendEvent,
      listEvents,
      appendIteration,
      listIterations,
      upsertTask,
      listTasks,
      leaseTask,
      leaseSpecificTask,
      completeTask,
      blockTask,
      assessTask,
      routeTask,
      beginTaskPass,
      completeTaskPass,
      failTaskPass,
      appendTaskMemory,
      listTaskMemory,
      listTaskPasses,
      promoteTask,
      exhaustTask,
      archiveTask,
      findDaemonContextBySession,
      upsertWorker,
      heartbeatWorker,
      listWorkers,
      upsertArtifact,
    })
  }),
)

export const defaultLayer = layer

function renderStateMarkdown(run: RunInfo, iterations: IterationInfo[], events: EventInfo[]) {
  const spec = run.spec_json as ZyalScript
  const lastIteration = iterations.at(-1)
  return renderMarkdownReport(spec.job.name, [
    `- Run ID: ${run.id}`,
    `- Status: ${run.status}`,
    `- Phase: ${run.phase}`,
    `- Iteration: ${run.iteration}`,
    `- Epoch: ${run.epoch}`,
    `- Objective: ${spec.job.objective}`,
    `- Last error: ${run.last_error ?? "(none)"}`,
    `- Events: ${events.length}`,
    `- Last iteration reason: ${lastIteration?.terminal_reason ?? "(none)"}`,
  ])
}

function renderTaskMarkdown(task: TaskInfo) {
  return renderMarkdownReport(task.title, [
    `- Task ID: ${task.id}`,
    `- Run ID: ${task.run_id}`,
    `- Status: ${task.status}`,
    `- Lane: ${task.lane}`,
    `- Phase: ${task.phase}`,
    `- Priority: ${task.priority}`,
    `- Incubator: ${task.incubator_status} round ${task.incubator_round}`,
    `- Blocked reason: ${task.blocked_reason ?? "(none)"}`,
  ], [{ title: "Body", content: fencedJson(task.body_json) }])
}

function renderTaskCapsule(task: TaskInfo, memories: TaskMemoryInfo[], passes: TaskPassInfo[]) {
  const currentBest = memories.find((item) => item.kind === "current_best_plan") ?? memories.find((item) => item.kind === "synthesis")
  const objections = memories.filter((item) => item.kind.includes("objection") || item.kind === "critic")
  return renderMarkdownReport(`Capsule: ${task.title}`, [
    `Readiness: ${task.readiness_score.toFixed(3)}`,
    `Risk: ${task.risk_score.toFixed(3)}`,
    `Implementation confidence: ${task.implementation_confidence.toFixed(3)}`,
    `Verification confidence: ${task.verification_confidence.toFixed(3)}`,
    `Passes: ${passes.length}`,
  ], [
    { title: "Current Best", content: currentBest ? `${currentBest.title}\n\n${currentBest.summary}` : "(none)" },
    { title: "Critical Objections", content: objections.length ? objections.map((item) => `- ${item.title}: ${item.summary}`).join("\n") : "(none)" },
    { title: "Memory Summary", content: memories.length ? memories.map((item) => `- ${item.kind}: ${item.title} - ${item.summary}`).join("\n") : "(none)" },
  ])
}

function renderMemoryMarkdown(memories: TaskMemoryInfo[]) {
  if (!memories.length) return "# Memory\n\n(none)\n"
  return renderMarkdownReport("Memory", [], memories.map((item) => ({
    title: item.title,
    content: [
      `- Kind: ${item.kind}`,
      `- Importance: ${item.importance}`,
      `- Confidence: ${item.confidence}`,
      `- Source pass: ${item.source_pass_id ?? "(none)"}`,
      "",
      item.summary,
    ].join("\n"),
  })))
}

function renderScore(task: TaskInfo, passes: TaskPassInfo[], artifacts: ArtifactInfo[]) {
  return {
    task_id: task.id,
    lane: task.lane,
    phase: task.phase,
    status: task.status,
    difficulty_score: task.difficulty_score,
    risk_score: task.risk_score,
    readiness_score: task.readiness_score,
    implementation_confidence: task.implementation_confidence,
    verification_confidence: task.verification_confidence,
    attempt_count: task.attempt_count,
    no_progress_count: task.no_progress_count,
    incubator_round: task.incubator_round,
    incubator_status: task.incubator_status,
    pass_count: passes.length,
    artifact_count: artifacts.length,
    accepted_artifact_id: task.accepted_artifact_id,
    promotion_result: task.promotion_result_json,
  }
}

function renderPassMarkdown(pass: TaskPassInfo) {
  return renderMarkdownReport(`Pass ${pass.pass_number}: ${pass.pass_type}`, [
    `- Pass ID: ${pass.id}`,
    `- Task ID: ${pass.task_id}`,
    `- Context: ${pass.context_mode}`,
    `- Agent: ${pass.agent ?? "(default)"}`,
    `- Status: ${pass.status}`,
    `- Started: ${pass.started_at ?? "(none)"}`,
    `- Ended: ${pass.ended_at ?? "(none)"}`,
  ], [
    { title: "Result", content: fencedJson(pass.result_json) },
    { title: "Score", content: fencedJson(pass.score_json) },
    { title: "Error", content: fencedJson(pass.error_json) },
  ])
}

function renderMarkdownReport(title: string, metadata: string[], sections: Array<{ title: string; content: string }> = []) {
  return [`# ${title}`, "", ...metadata, ...sections.flatMap((section) => ["", `## ${section.title}`, "", section.content])].join("\n")
}

function fencedJson(value: unknown) {
  return ["```json", JSON.stringify(value ?? null, null, 2), "```"].join("\n")
}

function stripUndefined<T extends Record<string, unknown>>(value: T): Partial<T> {
  return Object.fromEntries(Object.entries(value).filter(([, item]) => item !== undefined)) as Partial<T>
}

function stringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  return value.filter((item): item is string => typeof item === "string")
}

function locksOverlap(left: readonly string[], right: readonly string[]) {
  for (const a of left) {
    for (const b of right) {
      if (a === b || a.startsWith(`${b}/`) || b.startsWith(`${a}/`)) return true
    }
  }
  return false
}

export * as DaemonStore from "./daemon-store"
