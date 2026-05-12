// jankurai:allow HLT-001-DEAD-MARKER reason=shared control-plane test helpers by design expires=2027-01-01
import { afterEach, beforeEach, describe, mock, test } from "bun:test"
import fs from "node:fs/promises"
import Http from "node:http"
import path from "node:path"
import { setTimeout as delay } from "node:timers/promises"
import { NodeHttpServer } from "@effect/platform-node"
import { Duration, Effect, Layer, Schedule } from "effect"
import { HttpServer, HttpServerRequest, HttpServerResponse } from "effect/unstable/http"
import { eq } from "drizzle-orm"
import * as Log from "@jekko-ai/core/util/log"
import { Flag } from "@jekko-ai/core/flag/flag"
import { GlobalBus, type GlobalEvent } from "@/bus/global"
import { Database } from "@/storage/db"
import { ProjectID } from "@/project/schema"
import { ProjectTable } from "@/project/project.sql"
import { Instance } from "@/project/instance"
import { WithInstance } from "../../src/project/with-instance"
import { Session as SessionNs } from "@/session/session"
import { SessionID } from "@/session/schema"
import { SessionTable } from "@/session/session.sql"
import { SyncEvent } from "@/sync"
import { EventSequenceTable } from "@/sync/event.sql"
import { resetDatabase } from "../fixture/db"
import { disposeAllInstances, provideTmpdirInstance, tmpdir } from "../fixture/fixture"
import { testEffect } from "../lib/effect"
import { registerAdapter } from "../../src/control-plane/adapters"
import { WorkspaceID } from "../../src/control-plane/schema"
import { WorkspaceTable } from "../../src/control-plane/workspace.sql"
import type { Target, WorkspaceAdapter, WorkspaceInfo } from "../../src/control-plane/types"
import * as WorkspaceOld from "../../src/control-plane/workspace"
import { AppRuntime } from "@/effect/app-runtime"

void Log.init({ print: false })

const testServerLayer = Layer.mergeAll(
  NodeHttpServer.layer(Http.createServer, { host: "127.0.0.1", port: 0 }),
  WorkspaceOld.defaultLayer,
  SessionNs.defaultLayer,
)
export const it = testEffect(testServerLayer)

const originalWorkspacesFlag = Flag.JEKKO_EXPERIMENTAL_WORKSPACES
const originalEnv = {
  JEKKO_AUTH_CONTENT: process.env.JEKKO_AUTH_CONTENT,
  OTEL_EXPORTER_OTLP_HEADERS: process.env.OTEL_EXPORTER_OTLP_HEADERS,
  OTEL_EXPORTER_OTLP_ENDPOINT: process.env.OTEL_EXPORTER_OTLP_ENDPOINT,
  OTEL_RESOURCE_ATTRIBUTES: process.env.OTEL_RESOURCE_ATTRIBUTES,
}

type RecordedCreate = {
  info: WorkspaceInfo
  env: Record<string, string | undefined>
  from?: WorkspaceInfo
}

type RecordedAdapter = {
  adapter: WorkspaceAdapter
  calls: {
    configure: WorkspaceInfo[]
    create: RecordedCreate[]
    remove: WorkspaceInfo[]
    target: WorkspaceInfo[]
  }
}

type FetchCall = {
  url: URL
  method: string
  headers: Headers
  bodyText?: string
  json?: unknown
}

export function unique(prefix: string) {
  return `${prefix}-${Math.random().toString(36).slice(2)}`
}

export function restoreEnv() {
  Object.entries(originalEnv).forEach(([key, value]) => {
    if (value === undefined) {
      delete process.env[key]
      return
    }
    process.env[key] = value
  })
}

beforeEach(() => {
  Database.close()
  Flag.JEKKO_EXPERIMENTAL_WORKSPACES = true
  restoreEnv()
})

afterEach(async () => {
  mock.restore()
  await disposeAllInstances()
  Flag.JEKKO_EXPERIMENTAL_WORKSPACES = originalWorkspacesFlag
  restoreEnv()
  await resetDatabase()
})

export async function withInstance<T>(fn: (dir: string) => T | Promise<T>) {
  await using tmp = await tmpdir({ git: true })
  return WithInstance.provide({
    directory: tmp.path,
    fn: () => fn(tmp.path),
  })
}

export const runWorkspace = <A, E>(effect: Effect.Effect<A, E, WorkspaceOld.Service>) => AppRuntime.runPromise(effect)
export const createWorkspace = (input: WorkspaceOld.CreateInput) =>
  runWorkspace(WorkspaceOld.Service.use((workspace) => workspace.create(input)))
export const warpWorkspaceSession = (input: WorkspaceOld.SessionWarpInput) =>
  runWorkspace(WorkspaceOld.Service.use((workspace) => workspace.sessionWarp(input)))
export const listWorkspaces = (project: Parameters<WorkspaceOld.Interface["list"]>[0]) =>
  runWorkspace(WorkspaceOld.Service.use((workspace) => workspace.list(project)))
export const getWorkspace = (id: WorkspaceID) => runWorkspace(WorkspaceOld.Service.use((workspace) => workspace.get(id)))
export const removeWorkspace = (id: WorkspaceID) => runWorkspace(WorkspaceOld.Service.use((workspace) => workspace.remove(id)))
export const workspaceStatus = () => runWorkspace(WorkspaceOld.Service.use((workspace) => workspace.status()))
export const isWorkspaceSyncing = (id: WorkspaceID) =>
  runWorkspace(WorkspaceOld.Service.use((workspace) => workspace.isSyncing(id)))
export const startWorkspaceSyncing = (projectID: ProjectID) => {
  void runWorkspace(WorkspaceOld.Service.use((workspace) => workspace.startWorkspaceSyncing(projectID)))
}
export const waitForWorkspaceSync = (workspaceID: WorkspaceID, state: Record<string, number>, signal?: AbortSignal) =>
  runWorkspace(WorkspaceOld.Service.use((workspace) => workspace.waitForSync(workspaceID, state, signal)))

export function captureGlobalEvents() {
  const events: GlobalEvent[] = []
  const handler = (event: GlobalEvent) => events.push(event)
  GlobalBus.on("event", handler)
  return {
    events,
    dispose() {
      GlobalBus.off("event", handler)
    },
  }
}

export async function eventually<T>(fn: () => T | Promise<T>, timeout = 1500) {
  const started = Date.now()
  let last: unknown
  while (Date.now() - started < timeout) {
    try {
      return await fn()
    } catch (err) {
      last = err
      await delay(10)
    }
  }
  throw last ?? new Error("Timed out waiting for condition")
}

export function eventuallyEffect<E, R>(effect: Effect.Effect<void, E, R>, timeout = 1500) {
  return effect.pipe(Effect.retry(Schedule.spaced("10 millis")), Effect.timeout(Duration.millis(timeout)))
}

export function recordedAdapter(input: {
  target: (info: WorkspaceInfo) => Target | Promise<Target>
  configure?: (info: WorkspaceInfo) => WorkspaceInfo | Promise<WorkspaceInfo>
  create?: (info: WorkspaceInfo, env: Record<string, string | undefined>, from?: WorkspaceInfo) => Promise<void>
  remove?: (info: WorkspaceInfo) => Promise<void>
}): RecordedAdapter {
  const calls: RecordedAdapter["calls"] = {
    configure: [],
    create: [],
    remove: [],
    target: [],
  }

  return {
    calls,
    adapter: {
      name: "recorded",
      description: "recorded",
      configure(info) {
        calls.configure.push(structuredClone(info))
        return input.configure?.(info) ?? info
      },
      async create(info, env, from) {
        calls.create.push({
          info: structuredClone(info),
          env: { ...env },
          from: from ? structuredClone(from) : undefined,
        })
        await input.create?.(info, env, from)
      },
      async remove(info) {
        calls.remove.push(structuredClone(info))
        await input.remove?.(info)
      },
      target(info) {
        calls.target.push(structuredClone(info))
        return input.target(info)
      },
    },
  }
}

export function localAdapter(dir: string, input?: { createDir?: boolean; remove?: (info: WorkspaceInfo) => Promise<void> }) {
  return recordedAdapter({
    configure(info) {
      return { ...info, directory: dir }
    },
    async create() {
      if (input?.createDir === false) return
      await fs.mkdir(dir, { recursive: true })
    },
    remove: input?.remove,
    target() {
      return { type: "local", directory: dir }
    },
  })
}

export function remoteAdapter(url: string, input?: { directory?: string | null; headers?: HeadersInit }) {
  return recordedAdapter({
    configure(info) {
      return { ...info, directory: input?.directory ?? info.directory }
    },
    target() {
      return { type: "remote", url, headers: input?.headers }
    },
  })
}

export function eventStreamResponse(events: unknown[] = [], keepOpen = true) {
  const encoder = new TextEncoder()
  return new Response(
    new ReadableStream<Uint8Array>({
      start(controller) {
        if (keepOpen) controller.enqueue(encoder.encode(":\n\n"))
        events.forEach((event) => controller.enqueue(encoder.encode(`data: ${JSON.stringify(event)}\n\n`)))
        if (!keepOpen) controller.close()
      },
    }),
    { status: 200, headers: { "content-type": "text/event-stream" } },
  )
}

export function serverUrl() {
  return Effect.gen(function* () {
    return HttpServer.formatAddress((yield* HttpServer.HttpServer).address)
  })
}

export function workspaceInfo(projectID: ProjectID, type: string, input?: Partial<WorkspaceInfo>): WorkspaceInfo {
  return {
    id: input?.id ?? WorkspaceID.ascending(),
    type,
    name: input?.name ?? unique("workspace"),
    branch: input?.branch ?? null,
    directory: input?.directory ?? null,
    extra: input?.extra ?? null,
    projectID,
  }
}

export function insertWorkspace(info: WorkspaceInfo) {
  Database.use((db) =>
    db
      .insert(WorkspaceTable)
      .values({
        id: info.id,
        type: info.type,
        branch: info.branch,
        name: info.name,
        directory: info.directory,
        extra: info.extra,
        project_id: info.projectID,
      })
      .run(),
  )
}

export function insertProject(id: ProjectID, worktree: string) {
  Database.use((db) =>
    db
      .insert(ProjectTable)
      .values({
        id,
        worktree,
        vcs: null,
        name: null,
        time_created: Date.now(),
        time_updated: Date.now(),
        sandboxes: [],
      })
      .run(),
  )
}

export function attachSessionToWorkspace(sessionID: SessionID, workspaceID: WorkspaceID) {
  Database.use((db) =>
    db.update(SessionTable).set({ workspace_id: workspaceID }).where(eq(SessionTable.id, sessionID)).run(),
  )
}

export function sessionSequence(sessionID: SessionID) {
  return Database.use((db) =>
    db
      .select({ seq: EventSequenceTable.seq })
      .from(EventSequenceTable)
      .where(eq(EventSequenceTable.aggregate_id, sessionID))
      .get(),
  )?.seq
}

export function sessionSequenceOwner(sessionID: SessionID) {
  return Database.use((db) =>
    db
      .select({ ownerID: EventSequenceTable.owner_id })
      .from(EventSequenceTable)
      .where(eq(EventSequenceTable.aggregate_id, sessionID))
      .get(),
  )?.ownerID
}

export function sessionUpdatedType() {
  return SyncEvent.versionedType(SessionNs.Event.Updated.type, SessionNs.Event.Updated.version)
}

describe("workspace helpers", () => {
  test("load", () => {})
})
