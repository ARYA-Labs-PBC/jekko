import { describe, expect, test } from "bun:test"
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
import { AppRuntime } from "../../src/effect/app-runtime"
import {
  attachSessionToWorkspace,
  captureGlobalEvents,
  createWorkspace,
  eventStreamResponse,
  getWorkspace,
  insertProject,
  insertWorkspace,
  listWorkspaces,
  localAdapter,
  recordedAdapter,
  remoteAdapter,
  removeWorkspace,
  serverUrl,
  sessionSequence,
  sessionSequenceOwner,
  sessionUpdatedType,
  startWorkspaceSyncing,
  unique,
  warpWorkspaceSession,
  workspaceInfo,
  workspaceStatus,
  withInstance,
} from "./workspace.test"

void Log.init({ print: false })

const testServerLayer = Layer.mergeAll(
  NodeHttpServer.layer(Http.createServer, { host: "127.0.0.1", port: 0 }),
  WorkspaceOld.defaultLayer,
  SessionNs.defaultLayer,
)
const it = testEffect(testServerLayer)

describe("workspace-prior CRUD", () => {
  test("get returns undefined for a missing workspace", async () => {
    await withInstance(async () => {
      expect(await getWorkspace(WorkspaceID.ascending("wrk_missing_get"))).toBeUndefined()
    })
  })

  test("list maps database rows, filters by project, and sorts by id", async () => {
    await withInstance(async () => {
      const otherProjectID = ProjectID.make("project-other")
      insertProject(otherProjectID, "/tmp/other")
      const a = workspaceInfo(Instance.project.id, "manual", {
        id: WorkspaceID.ascending("wrk_a_list"),
        branch: "a",
        directory: "/a",
        extra: { a: true },
      })
      const b = workspaceInfo(Instance.project.id, "manual", {
        id: WorkspaceID.ascending("wrk_b_list"),
        branch: "b",
        directory: "/b",
        extra: ["b"],
      })
      const other = workspaceInfo(otherProjectID, "manual", { id: WorkspaceID.ascending("wrk_c_list") })
      insertWorkspace(b)
      insertWorkspace(other)
      insertWorkspace(a)

      expect(await listWorkspaces(Instance.project)).toEqual([a, b])
    })
  })

  test("create configures, persists, creates, starts local sync, and passes environment", async () => {
    await withInstance(async (dir) => {
      process.env.JEKKO_AUTH_CONTENT = JSON.stringify({ test: { type: "api", key: "secret" } })
      process.env.OTEL_EXPORTER_OTLP_HEADERS = "authorization=otel"
      process.env.OTEL_EXPORTER_OTLP_ENDPOINT = "https://otel.test"
      process.env.OTEL_RESOURCE_ATTRIBUTES = "service.name=jekko-test"

      const workspaceID = WorkspaceID.ascending("wrk_create_local")
      const type = unique("create-local")
      const targetDir = path.join(dir, "created-local")
      const recorded = recordedAdapter({
        configure(info) {
          return {
            ...info,
            branch: "configured-branch",
            name: "Configured Name",
            directory: targetDir,
            extra: { configured: true },
          }
        },
        async create() {
          await fs.mkdir(targetDir, { recursive: true })
        },
        target() {
          return { type: "local", directory: targetDir }
        },
      })
      registerAdapter(Instance.project.id, type, recorded.adapter)

      const info = await createWorkspace({
        id: workspaceID,
        type,
        branch: null,
        projectID: Instance.project.id,
        extra: null,
      })

      expect(info).toEqual({
        id: workspaceID,
        type,
        branch: "configured-branch",
        name: "Configured Name",
        directory: targetDir,
        extra: { configured: true },
        projectID: Instance.project.id,
      })
      expect(await getWorkspace(workspaceID)).toEqual(info)
      expect(await listWorkspaces(Instance.project)).toEqual([info])
      expect(recorded.calls.configure).toHaveLength(1)
      expect(recorded.calls.configure[0]).toMatchObject({ id: workspaceID, type, directory: null })
      expect(recorded.calls.create).toHaveLength(1)
      expect(recorded.calls.create[0].info).toEqual(info)
      expect(JSON.parse(recorded.calls.create[0].env.JEKKO_AUTH_CONTENT ?? "{}")).toEqual({
        test: { type: "api", key: "secret" },
      })
      expect(recorded.calls.create[0].env.JEKKO_WORKSPACE_ID).toBe(workspaceID)
      expect(recorded.calls.create[0].env.JEKKO_EXPERIMENTAL_WORKSPACES).toBe("true")
      expect(recorded.calls.create[0].env.OTEL_EXPORTER_OTLP_HEADERS).toBe("authorization=otel")
      expect(recorded.calls.create[0].env.OTEL_EXPORTER_OTLP_ENDPOINT).toBe("https://otel.test")
      expect(recorded.calls.create[0].env.OTEL_RESOURCE_ATTRIBUTES).toBe("service.name=jekko-test")
      expect((await workspaceStatus()).find((item) => item.workspaceID === workspaceID)?.status).toBe("connected")

      await removeWorkspace(workspaceID)
      expect((await workspaceStatus()).find((item) => item.workspaceID === workspaceID)?.status).toBeUndefined()
    })
  })

  test("create propagates configure failures and does not insert a workspace", async () => {
    await withInstance(async () => {
      const type = unique("configure-failure")
      registerAdapter(
        Instance.project.id,
        type,
        recordedAdapter({
          configure() {
            throw new Error("configure exploded")
          },
          target() {
            return { type: "local", directory: "/unreferenced" }
          },
        }).adapter,
      )

      await expect(
        createWorkspace({ type, branch: null, projectID: Instance.project.id, extra: null }),
      ).rejects.toThrow("configure exploded")
      expect(await listWorkspaces(Instance.project)).toEqual([])
    })
  })

  test("create leaves the inserted row when adapter create fails", async () => {
    await withInstance(async () => {
      const type = unique("create-failure")
      const recorded = recordedAdapter({
        async create() {
          throw new Error("create exploded")
        },
        target() {
          return { type: "local", directory: "/unreferenced" }
        },
      })
      registerAdapter(Instance.project.id, type, recorded.adapter)

      await expect(
        createWorkspace({ type, branch: "branch", projectID: Instance.project.id, extra: { x: 1 } }),
      ).rejects.toThrow("create exploded")

      const rows = await listWorkspaces(Instance.project)
      expect(rows).toHaveLength(1)
      expect(rows[0]).toMatchObject({ type, branch: "branch", extra: { x: 1 } })
      expect(recorded.calls.target).toHaveLength(0)
      await removeWorkspace(rows[0].id)
    })
  })

  test("create returns after a local workspace reports error", async () => {
    await withInstance(async (dir) => {
      const type = unique("local-error")
      const missing = path.join(dir, "missing-local-target")
      const recorded = localAdapter(missing, { createDir: false })
      registerAdapter(Instance.project.id, type, recorded.adapter)

      const info = await createWorkspace({ type, branch: null, projectID: Instance.project.id, extra: null })

      expect(info.directory).toBe(missing)
      expect((await workspaceStatus()).find((item) => item.workspaceID === info.id)?.status).toBe("error")
      await removeWorkspace(info.id)
    })
  })

  it.live("remote create connects to routed event and history endpoints", () => {
    const calls: any[] = []
    return Effect.gen(function* () {
      yield* HttpServer.serveEffect()(
        Effect.gen(function* () {
          const req = yield* HttpServerRequest.HttpServerRequest
          const bodyText = yield* req.text
          const call = {
            url: new URL(req.url, "http://localhost"),
            method: req.method,
            headers: new Headers(req.headers),
            bodyText,
            json: bodyText ? JSON.parse(bodyText) : undefined,
          }
          calls.push(call)
          if (call.url.pathname === "/base/global/event")
            return yield* HttpServerResponse.fromWeb(eventStreamResponse([], false))
          if (call.url.pathname === "/base/sync/history") return yield* HttpServerResponse.json([])
          return HttpServerResponse.text("unexpected", { status: 500 })
        }),
      )
      const url = yield* serverUrl()
      yield* provideTmpdirInstance(
        (dir) =>
          Effect.gen(function* () {
            const workspace = yield* WorkspaceOld.Service
            const type = unique("remote-create")
            const recorded = remoteAdapter(`${url}/base/?ignored=1#hash`, { directory: dir })
            registerAdapter(Instance.project.id, type, recorded.adapter)

            const info = yield* workspace.create({ type, branch: null, projectID: Instance.project.id, extra: null })

            expect(recorded.calls.create).toHaveLength(1)
            expect(recorded.calls.create[0].info).toEqual(info)
            expect(yield* workspace.get(info.id)).toEqual(info)

            yield* workspace.remove(info.id)
            expect((yield* workspace.status()).find((item) => item.workspaceID === info.id)?.status).toBeUndefined()
          }),
        { git: true },
      )
    })
  })

  test("remove returns undefined for a missing workspace", async () => {
    await withInstance(async () => {
      expect(await removeWorkspace(WorkspaceID.ascending("wrk_missing_remove"))).toBeUndefined()
    })
  })

  test("remove deletes the workspace, associated sessions, adapter resources, and status", async () => {
    await withInstance(async (dir) => {
      const type = unique("remove-local")
      const recorded = localAdapter(path.join(dir, "remove-local"))
      registerAdapter(Instance.project.id, type, recorded.adapter)
      const info = await createWorkspace({ type, branch: null, projectID: Instance.project.id, extra: null })
      const one = await AppRuntime.runPromise(SessionNs.Service.use((svc) => svc.create({})))
      const two = await AppRuntime.runPromise(SessionNs.Service.use((svc) => svc.create({})))
      attachSessionToWorkspace(one.id, info.id)
      attachSessionToWorkspace(two.id, info.id)

      const removed = await removeWorkspace(info.id)

      expect(removed).toEqual(info)
      expect(await getWorkspace(info.id)).toBeUndefined()
      expect(recorded.calls.remove).toEqual([info])
      expect((await workspaceStatus()).find((item) => item.workspaceID === info.id)?.status).toBeUndefined()
      expect(
        Database.use((db) =>
          db.select({ id: SessionTable.id }).from(SessionTable).where(eq(SessionTable.workspace_id, info.id)).all(),
        ),
      ).toEqual([])
    })
  })

  test("remove still deletes the row when the adapter cannot remove resources", async () => {
    await withInstance(async () => {
      const type = unique("remove-throws")
      const info = workspaceInfo(Instance.project.id, type, { id: WorkspaceID.ascending("wrk_remove_throws") })
      registerAdapter(
        Instance.project.id,
        type,
        recordedAdapter({
          async remove() {
            throw new Error("remove exploded")
          },
          target() {
            return { type: "local", directory: "/unreferenced" }
          },
        }).adapter,
      )
      insertWorkspace(info)

      expect(await removeWorkspace(info.id)).toEqual(info)
      expect(await getWorkspace(info.id)).toBeUndefined()
    })
  })

  test("sessionWarp moves a session into a local workspace and claims ownership", async () => {
    await withInstance(async (dir) => {
      const previousType = unique("warp-prev-local")
      const targetType = unique("warp-target-local")
      const previous = workspaceInfo(Instance.project.id, previousType)
      const target = workspaceInfo(Instance.project.id, targetType)
      insertWorkspace(previous)
      insertWorkspace(target)
      registerAdapter(Instance.project.id, previousType, localAdapter(path.join(dir, "warp-prev-local")).adapter)
      registerAdapter(Instance.project.id, targetType, localAdapter(path.join(dir, "warp-target-local")).adapter)
      const session = await AppRuntime.runPromise(SessionNs.Service.use((svc) => svc.create({})))
      attachSessionToWorkspace(session.id, previous.id)

      await warpWorkspaceSession({ workspaceID: target.id, sessionID: session.id })

      expect(
        Database.use((db) =>
          db
            .select({ workspaceID: SessionTable.workspace_id })
            .from(SessionTable)
            .where(eq(SessionTable.id, session.id))
            .get(),
        )?.workspaceID,
      ).toBe(target.id)
      expect(sessionSequenceOwner(session.id)).toBe(target.id)
    })
  })

  test("sessionWarp detaches a session to the local project and claims project ownership", async () => {
    await withInstance(async (dir) => {
      const previousType = unique("warp-detach-local")
      const previous = workspaceInfo(Instance.project.id, previousType)
      insertWorkspace(previous)
      registerAdapter(Instance.project.id, previousType, localAdapter(path.join(dir, "warp-detach-local")).adapter)
      const session = await AppRuntime.runPromise(SessionNs.Service.use((svc) => svc.create({})))
      attachSessionToWorkspace(session.id, previous.id)

      await warpWorkspaceSession({ workspaceID: null, sessionID: session.id })

      expect(
        Database.use((db) =>
          db
            .select({ workspaceID: SessionTable.workspace_id })
            .from(SessionTable)
            .where(eq(SessionTable.id, session.id))
            .get(),
        )?.workspaceID,
      ).toBeNull()
      expect(sessionSequenceOwner(session.id)).toBe(Instance.project.id)
    })
  })

  it.live("sessionWarp syncs previous remote history, replays it, steals, and claims the sequence", () => {
    const calls: any[] = []
    let historySessionID: SessionID | undefined
    let historyNextSeq = 0
    return Effect.gen(function* () {
      yield* HttpServer.serveEffect()(
        Effect.gen(function* () {
          const req = yield* HttpServerRequest.HttpServerRequest
          const bodyText = yield* req.text
          const call = {
            url: new URL(req.url, "http://localhost"),
            method: req.method,
            headers: new Headers(req.headers),
            bodyText,
            json: bodyText ? JSON.parse(bodyText) : undefined,
          }
          calls.push(call)
          if (call.url.pathname === "/warp-source/sync/history") {
            return yield* HttpServerResponse.json([
              {
                id: `evt_${unique("warp-source-history")}`,
                aggregate_id: historySessionID!,
                seq: historyNextSeq,
                type: sessionUpdatedType(),
                data: { sessionID: historySessionID!, info: { title: "from source history" } },
              },
            ])
          }
          if (call.url.pathname === "/warp-target/sync/replay")
            return yield* HttpServerResponse.json({ sessionID: "ok" })
          if (call.url.pathname === "/warp-target/sync/steal")
            return yield* HttpServerResponse.json({ sessionID: "ok" })
          return HttpServerResponse.text("unexpected", { status: 500 })
        }),
      )
      const url = yield* serverUrl()
      yield* provideTmpdirInstance(
        () =>
          Effect.gen(function* () {
            const workspace = yield* WorkspaceOld.Service
            const sessionSvc = yield* SessionNs.Service
            const previousType = unique("warp-remote-source")
            const targetType = unique("warp-remote-target")
            const previous = workspaceInfo(Instance.project.id, previousType)
            const target = workspaceInfo(Instance.project.id, targetType, { directory: "remote-target-dir" })
            insertWorkspace(previous)
            insertWorkspace(target)
            registerAdapter(Instance.project.id, previousType, remoteAdapter(`${url}/warp-source`).adapter)
            registerAdapter(Instance.project.id, targetType, remoteAdapter(`${url}/warp-target`).adapter)
            const session = yield* sessionSvc.create({})
            attachSessionToWorkspace(session.id, previous.id)
            historySessionID = session.id
            historyNextSeq = (sessionSequence(session.id) ?? -1) + 1

            yield* workspace.sessionWarp({ workspaceID: target.id, sessionID: session.id })

            expect(calls.map((call) => `${call.method} ${call.url.pathname}`)).toEqual([
              "POST /warp-source/sync/history",
              "POST /warp-target/sync/replay",
              "POST /warp-target/sync/steal",
            ])
            expect(calls[0].json).toEqual({ [session.id]: historyNextSeq - 1 })
            expect(calls[1].json).toMatchObject({
              directory: "remote-target-dir",
              events: [
                {
                  aggregateID: session.id,
                  seq: 0,
                  type: SyncEvent.versionedType(SessionNs.Event.Created.type, SessionNs.Event.Created.version),
                },
                {
                  aggregateID: session.id,
                  seq: historyNextSeq,
                  type: sessionUpdatedType(),
                },
              ],
            })
            expect(calls[2].json).toEqual({ sessionID: session.id })
            expect((yield* sessionSvc.get(session.id)).title).toBe("from source history")
            expect(sessionSequenceOwner(session.id)).toBe(target.id)
          }),
        { git: true },
      )
    })
  })
})
