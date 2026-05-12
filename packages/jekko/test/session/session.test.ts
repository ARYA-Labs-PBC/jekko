import { describe, expect, test } from "bun:test"
import path from "path"
import { Effect, Schema } from "effect"
import { Session as SessionNs } from "@/session/session"
import { Bus } from "../../src/bus"
import * as Log from "@jekko-ai/core/util/log"
import { Instance } from "../../src/project/instance"
import { WithInstance } from "../../src/project/with-instance"
import { DaemonStore } from "../../src/session/daemon-store"
import { DaemonArtifactTable, DaemonEventTable, DaemonIterationTable, DaemonRunTable, DaemonTaskMemoryTable, DaemonTaskPassTable, DaemonTaskTable, DaemonWorkerTable } from "../../src/session/daemon.sql"
import { ZyalScriptSchema } from "../../src/agent-script/schema"
import { MessageV2 } from "../../src/session/message"
import { MessageID, PartID, type SessionID } from "../../src/session/schema"
import { AppRuntime } from "../../src/effect/app-runtime"
import { Database } from "../../src/storage/db"
import { eq } from "drizzle-orm"
import { tmpdir } from "../fixture/fixture"

const projectRoot = path.join(__dirname, "../..")
void Log.init({ print: false })

function create(input?: SessionNs.CreateInput) {
  return AppRuntime.runPromise(SessionNs.Service.use((svc) => svc.create(input)))
}

function get(id: SessionID) {
  return AppRuntime.runPromise(SessionNs.Service.use((svc) => svc.get(id)))
}

function remove(id: SessionID) {
  return AppRuntime.runPromise(SessionNs.Service.use((svc) => svc.remove(id)))
}

function daemonSpec() {
  return Schema.decodeUnknownSync(ZyalScriptSchema)({
    version: "v1",
    intent: "daemon",
    confirm: "RUN_FOREVER",
    id: "session-remove-cleanup",
    job: {
      name: "Cleanup regression",
      objective: "Exercise daemon row cleanup on session removal.",
    },
    stop: {
      all: [{ git_clean: { allow_untracked: false } }],
    },
  })
}

function updateMessage<T extends MessageV2.Info>(msg: T) {
  return AppRuntime.runPromise(SessionNs.Service.use((svc) => svc.updateMessage(msg)))
}

function updatePart<T extends MessageV2.Part>(part: T) {
  return AppRuntime.runPromise(SessionNs.Service.use((svc) => svc.updatePart(part)))
}

describe("session.created event", () => {
  test("should emit session.created event when session is created", async () => {
    await WithInstance.provide({
      directory: projectRoot,
      fn: async () => {
        let eventReceived = false
        let receivedInfo: SessionNs.Info | undefined

        const unsub = Bus.subscribe(SessionNs.Event.Created, (event) => {
          eventReceived = true
          receivedInfo = event.properties.info as SessionNs.Info
        })

        const info = await create({})
        await new Promise((resolve) => setTimeout(resolve, 100))
        unsub()

        expect(eventReceived).toBe(true)
        expect(receivedInfo).toBeDefined()
        expect(receivedInfo?.id).toBe(info.id)
        expect(receivedInfo?.projectID).toBe(info.projectID)
        expect(receivedInfo?.directory).toBe(info.directory)
        expect(receivedInfo?.path).toBe(info.path)
        expect(receivedInfo?.title).toBe(info.title)

        await remove(info.id)
      },
    })
  })

  test("session.created event should be emitted before session.updated", async () => {
    await WithInstance.provide({
      directory: projectRoot,
      fn: async () => {
        const events: string[] = []

        const unsubCreated = Bus.subscribe(SessionNs.Event.Created, () => {
          events.push("created")
        })

        const unsubUpdated = Bus.subscribe(SessionNs.Event.Updated, () => {
          events.push("updated")
        })

        const info = await create({})
        await new Promise((resolve) => setTimeout(resolve, 100))
        unsubCreated()
        unsubUpdated()

        expect(events).toContain("created")
        expect(events).toContain("updated")
        expect(events.indexOf("created")).toBeLessThan(events.indexOf("updated"))

        await remove(info.id)
      },
    })
  })
})

describe("step-finish token propagation via Bus event", () => {
  test(
    "non-zero tokens propagate through PartUpdated event",
    async () => {
      await WithInstance.provide({
        directory: projectRoot,
        fn: async () => {
          const info = await create({})

          const messageID = MessageID.ascending()
          await updateMessage({
            id: messageID,
            sessionID: info.id,
            role: "user",
            time: { created: Date.now() },
            agent: "user",
            model: { providerID: "test", modelID: "test" },
            tools: {},
            mode: "",
          } as unknown as MessageV2.Info)

          // Bus subscribers receive readonly Schema.Type payloads; `MessageV2.Part`
          // is the mutable domain type. Cast bridges the two — safe because the
          // test only reads the value afterwards.
          let received: MessageV2.Part | undefined
          const unsub = Bus.subscribe(MessageV2.Event.PartUpdated, (event) => {
            received = event.properties.part as MessageV2.Part
          })

          const tokens = {
            total: 1500,
            input: 500,
            output: 800,
            reasoning: 200,
            cache: { read: 100, write: 50 },
          }

          const partInput = {
            id: PartID.ascending(),
            messageID,
            sessionID: info.id,
            type: "step-finish" as const,
            reason: "stop",
            cost: 0.005,
            tokens,
          }

          await updatePart(partInput)
          await new Promise((resolve) => setTimeout(resolve, 100))

          expect(received).toBeDefined()
          expect(received!.type).toBe("step-finish")
          const finish = received as MessageV2.StepFinishPart
          expect(finish.tokens.input).toBe(500)
          expect(finish.tokens.output).toBe(800)
          expect(finish.tokens.reasoning).toBe(200)
          expect(finish.tokens.total).toBe(1500)
          expect(finish.tokens.cache.read).toBe(100)
          expect(finish.tokens.cache.write).toBe(50)
          expect(finish.cost).toBe(0.005)
          expect(received).not.toBe(partInput)

          unsub()
          await remove(info.id)
        },
      })
    },
    { timeout: 30000 },
  )
})

describe("Session", () => {
  test("remove clears daemon runtime rows explicitly", async () => {
    await using tmp = await tmpdir({ git: true })

    const seeded = await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await create({ title: "remove-daemon-cleanup" })
        const store = await AppRuntime.runPromise(DaemonStore.Service.use((svc) => Effect.succeed(svc)))
        const run = await AppRuntime.runPromise(
          store.createRun({
            rootSessionID: session.id,
            activeSessionID: session.id,
            spec: daemonSpec(),
            specHash: "sha256:cleanup-test",
          }),
        )

        const task = await AppRuntime.runPromise(
          store.upsertTask({
            id: "task_remove_cleanup",
            run_id: run.id,
            external_id: "cleanup-task",
            title: "Cleanup task",
            body_json: { objective: "seed daemon data" },
            status: "queued",
            priority: 10,
            lease_worker_id: null,
            lease_expires_at: null,
            locked_paths_json: null,
            evidence_json: null,
          } as any),
        )

        const pass = await AppRuntime.runPromise(
          store.beginTaskPass({
            runID: run.id,
            taskID: task.id,
            passType: "scout",
            contextMode: "blind",
            sessionID: session.id,
          }),
        )

        await AppRuntime.runPromise(
          store.appendEvent({
            runID: run.id,
            iteration: 1,
            eventType: "task.seeded",
            payload: { taskID: task.id },
          }),
        )
        await AppRuntime.runPromise(
          store.appendIteration({
            runID: run.id,
            iteration: 1,
            sessionID: session.id,
            terminalReason: "stop",
            result: { ok: true },
          }),
        )
        await AppRuntime.runPromise(
          store.appendTaskMemory({
            runID: run.id,
            taskID: task.id,
            kind: "note",
            title: "Seed",
            summary: "Seeded daemon rows for cleanup.",
            sourcePassID: pass.id,
          }),
        )
        await AppRuntime.runPromise(
          store.upsertWorker({
            id: "worker_remove_cleanup",
            run_id: run.id,
            role: "worker",
            session_id: null,
            worktree_path: null,
            branch: null,
            status: "active",
            lease_task_id: null,
            last_heartbeat_at: null,
          } as any),
        )
        await AppRuntime.runPromise(
          store.upsertArtifact({
            id: "artifact_remove_cleanup",
            run_id: run.id,
            task_id: task.id,
            pass_id: pass.id,
            kind: "note",
            path_or_ref: "cleanup.txt",
            sha: null,
            payload_json: null,
          } as any),
        )

        return { session, runID: run.id }
      },
    })

    await remove(seeded.session.id)

    const state = await AppRuntime.runPromise(
      Effect.sync(() =>
        Database.use((db) => ({
          runs: db.select().from(DaemonRunTable).where(eq(DaemonRunTable.id, seeded.runID)).all().length,
          root: db.select().from(DaemonRunTable).where(eq(DaemonRunTable.root_session_id, seeded.session.id)).all().length,
          active: db.select().from(DaemonRunTable).where(eq(DaemonRunTable.active_session_id, seeded.session.id)).all().length,
          tasks: db.select().from(DaemonTaskTable).where(eq(DaemonTaskTable.run_id, seeded.runID)).all().length,
          passes: db.select().from(DaemonTaskPassTable).where(eq(DaemonTaskPassTable.run_id, seeded.runID)).all().length,
          memory: db.select().from(DaemonTaskMemoryTable).where(eq(DaemonTaskMemoryTable.run_id, seeded.runID)).all().length,
          workers: db.select().from(DaemonWorkerTable).where(eq(DaemonWorkerTable.run_id, seeded.runID)).all().length,
          iterations: db.select().from(DaemonIterationTable).where(eq(DaemonIterationTable.run_id, seeded.runID)).all().length,
          events: db.select().from(DaemonEventTable).where(eq(DaemonEventTable.run_id, seeded.runID)).all().length,
          artifacts: db.select().from(DaemonArtifactTable).where(eq(DaemonArtifactTable.run_id, seeded.runID)).all().length,
        })),
      ),
    )

    expect(state.runs).toBe(0)
    expect(state.root).toBe(0)
    expect(state.active).toBe(0)
    expect(state.tasks).toBe(0)
    expect(state.passes).toBe(0)
    expect(state.memory).toBe(0)
    expect(state.workers).toBe(0)
    expect(state.iterations).toBe(0)
    expect(state.events).toBe(0)
    expect(state.artifacts).toBe(0)
  })

  test("remove works without an instance", async () => {
    await using tmp = await tmpdir({ git: true })

    const info = await WithInstance.provide({
      directory: tmp.path,
      fn: () => create({ title: "remove-without-instance" }),
    })

    await expect(async () => {
      await remove(info.id)
    }).not.toThrow()

    let missing = false
    await get(info.id).catch(() => {
      missing = true
    })

    expect(missing).toBe(true)
  })
})
