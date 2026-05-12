import { describe, expect } from "bun:test"
import { CrossSpawnSpawner } from "@jekko-ai/core/cross-spawn-spawner"
import { Effect, Layer } from "effect"
import fs from "fs/promises"
import path from "path"
import { Daemon } from "../../src/session/daemon"
import { DaemonCheckpoint } from "../../src/session/daemon-checkpoint"
import { DaemonChecks } from "../../src/session/daemon-checks"
import { DaemonStore } from "../../src/session/daemon-store"
import { MCP } from "../../src/mcp"
import { parseZyal } from "../../src/agent-script/parser"
import { ProjectTable } from "../../src/project/project.sql"
import { ProjectID } from "../../src/project/schema"
import { SessionTable } from "../../src/session/session.sql"
import { Session } from "../../src/session/session"
import { SessionID } from "../../src/session/schema"
import { SessionPrompt } from "../../src/session/prompt"
import { Worktree } from "../../src/worktree"
import { Database } from "../../src/storage/db"
import { testEffect } from "../lib/effect"
import { provideTmpdirInstance } from "../fixture/fixture"

function seedProjectAndSession(input: {
  projectID: string
  sessionID: string
  directory: string
}) {
  return Effect.sync(() =>
    Database.use((db) => {
      const now = Date.now()
      db.insert(ProjectTable)
        .values({
          id: input.projectID,
          worktree: "/",
          vcs: "git",
          name: "Memory Benchmark Test",
          sandboxes: [],
          time_created: now,
          time_updated: now,
        })
        .run()
      db.insert(SessionTable)
        .values({
          id: input.sessionID,
          project_id: input.projectID,
          slug: "memory-benchmark",
          directory: input.directory,
          title: "Memory Benchmark Test",
          version: "1.0.0",
          time_created: now,
          time_updated: now,
        })
        .run()
    }),
  )
}

function sessionService(input: {
  sessionID: string
  directory: string
}) {
  const session = {
    id: input.sessionID,
    directory: input.directory,
    permission: [],
    agent: "build",
    model: undefined,
  } as any

  return Session.Service.of({
    list: () => Effect.succeed([]),
    create: () => Effect.die("unexpected Session.create"),
    fork: () => Effect.die("unexpected Session.fork"),
    touch: () => Effect.void,
    get: () => Effect.succeed(session),
    setTitle: () => Effect.void,
    setArchived: () => Effect.void,
    setPermission: () => Effect.void,
    setRevert: () => Effect.void,
    clearRevert: () => Effect.void,
    setSummary: () => Effect.void,
    diff: () => Effect.succeed([]),
    messages: () => Effect.succeed([]),
    children: () => Effect.succeed([]),
    remove: () => Effect.void,
    updateMessage: (msg: any) => Effect.succeed(msg),
    removeMessage: () => Effect.succeed(input.sessionID as any),
    removePart: () => Effect.succeed("part" as any),
    getPart: () => Effect.succeed(undefined),
    updatePart: (part: any) => Effect.succeed(part),
    updatePartDelta: () => Effect.void,
    findMessage: () => Effect.succeed({ _tag: "None" } as any),
  } as any)
}

function promptService() {
  return {
    cancel: () => Effect.void,
    prompt: () => Effect.void,
    loop: () => Effect.never,
    loopResult: () => Effect.never,
    shell: () => Effect.never,
    command: () => Effect.never,
    resolvePromptParts: () => Effect.succeed([]),
  } as any
}

function checksService() {
  return {
    runShellCheck: () => Effect.never,
    gitClean: () => Effect.succeed({ clean: true, dirty: [] }),
    evaluateJsonPath: () => Effect.succeed(undefined),
  } as any
}

function checkpointService() {
  return {
    runCheckpoint: () => Effect.succeed({ ok: true }),
  } as any
}

function mcpService() {
  return {
    status: () => Effect.succeed({}),
    clients: () => Effect.succeed({}),
    tools: () => Effect.succeed({}),
    prompts: () => Effect.succeed({}),
    resources: () => Effect.succeed({}),
    add: () => Effect.succeed({ status: { status: "disabled" } }),
    connect: () => Effect.void,
    disconnect: () => Effect.void,
    getPrompt: () => Effect.succeed(undefined),
    readResource: () => Effect.succeed(undefined),
    startAuth: () => Effect.die("unexpected MCP auth"),
    authenticate: () => Effect.die("unexpected MCP auth"),
    finishAuth: () => Effect.die("unexpected MCP auth"),
    removeAuth: () => Effect.void,
    supportsOAuth: () => Effect.succeed(false),
    hasStoredTokens: () => Effect.succeed(false),
    getAuthStatus: () => Effect.succeed("not_authenticated" as const),
  } as any
}

function worktreeService() {
  return {
    makeWorktreeInfo: () => Effect.die("unexpected worktree"),
    createFromInfo: () => Effect.void,
    create: () => Effect.die("unexpected worktree"),
    remove: () => Effect.succeed(true),
    reset: () => Effect.succeed(true),
  } as any
}

const it = testEffect(Layer.mergeAll(Daemon.defaultLayer, DaemonStore.defaultLayer, CrossSpawnSpawner.defaultLayer))

async function memoryBenchmarkFiles() {
  const dir = path.resolve(import.meta.dir, "../../../../docs/ZYAL/examples/memory-benchmark")
  return (await fs.readdir(dir)).filter((file) => file.endsWith(".zyal")).sort().map((file) => path.join(dir, file))
}

describe("memory benchmark daemon smoke", () => {
  it.effect(
    "parses, previews, and mirrors every bundled memory benchmark example under a root worktree",
    provideTmpdirInstance(
      (directory) =>
        Effect.gen(function* () {
          const projectID = ProjectID.make("proj_memory_benchmark_smoke")
          const sessionID = SessionID.make("ses_memory_benchmark_smoke")
          yield* seedProjectAndSession({
            projectID,
            sessionID,
            directory,
          })

          const daemon = yield* Daemon.Service
          const store = yield* DaemonStore.Service
          const files = yield* Effect.promise(() => memoryBenchmarkFiles())

          for (const file of files) {
            const text = yield* Effect.promise(() => fs.readFile(file, "utf8"))
            const parsed = yield* parseZyal(text)
            const preview = yield* daemon.preview({ text })

            expect(preview.spec.id).toBe(parsed.spec.id)
            expect(preview.preview.id).toBe(parsed.spec.id)
            expect(preview.preview.armed).toBe(true)

            const run = yield* store.createRun({
              rootSessionID: sessionID,
              activeSessionID: sessionID,
              spec: parsed.spec,
              specHash: parsed.specHash,
            })

            const runDir = path.join(directory, ".jekko", "daemon", run.id)
            expect(runDir).toContain(directory)
            expect(runDir).not.toBe(path.join("/", ".jekko", "daemon", run.id))
            expect(yield* Effect.promise(() => fs.readFile(path.join(runDir, "ledger.jsonl"), "utf8"))).toContain(
              `"event_type":"run.created"`,
            )
            expect(yield* Effect.promise(() => fs.readFile(path.join(runDir, "STATE.md"), "utf8"))).toContain(parsed.spec.job.name)
          }
        }),
    ),
  )

  it.effect(
    "starts the copied chase spec from a root worktree without mirroring to /.jekko",
    provideTmpdirInstance(
      (directory) =>
        Effect.gen(function* () {
          const projectID = ProjectID.make("proj_memory_benchmark_start")
          const sessionID = SessionID.make("ses_memory_benchmark_start")
          yield* seedProjectAndSession({
            projectID,
            sessionID,
            directory,
          })

          const daemon = yield* Daemon.Service
          const text = yield* Effect.promise(() =>
            fs.readFile(
              path.resolve(import.meta.dir, "../../../../docs/ZYAL/examples/memory-benchmark/autoresearch-chase.zyal"),
              "utf8",
            ),
          )

          const run = yield* daemon.start({
            sessionID,
            prompt: {
              parts: [{ type: "text", text }],
              agent: "build",
              noReply: true,
            } as any,
          }).pipe(
            Effect.provideService(Session.Service, sessionService({ sessionID, directory })),
            Effect.provideService(SessionPrompt.Service, promptService()),
            Effect.provideService(DaemonChecks.Service, checksService()),
            Effect.provideService(DaemonCheckpoint.Service, checkpointService()),
            Effect.provideService(MCP.Service, mcpService()),
            Effect.provideService(Worktree.Service, worktreeService()),
          )

          expect(run.status).toBe("armed")
          expect(run.phase).toBe("evaluating_stop")
          expect(run.last_error).toBeNull()
          const events = yield* daemon.events(run.id)
          expect(events.map((event) => event.event_type)).toContain("run.previewed")

          const runDir = path.join(directory, ".jekko", "daemon", run.id)
          expect(runDir).toContain(directory)
          expect(runDir).not.toBe(path.join("/", ".jekko", "daemon", run.id))
          expect(yield* Effect.promise(() => fs.readFile(path.join(runDir, "ledger.jsonl"), "utf8"))).toContain(
            `"event_type":"run.created"`,
          )
        }),
    ),
  )
})
