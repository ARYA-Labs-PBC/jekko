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
  eventually,
  eventuallyEffect,
  eventStreamResponse,
  insertWorkspace,
  isWorkspaceSyncing,
  localAdapter,
  removeWorkspace,
  remoteAdapter,
  serverUrl,
  sessionSequence,
  sessionUpdatedType,
  startWorkspaceSyncing,
  unique,
  workspaceInfo,
  workspaceStatus,
  withInstance,
  it,
} from "./workspace.test"

void Log.init({ print: false })

const testServerLayer = Layer.mergeAll(
  NodeHttpServer.layer(Http.createServer, { host: "127.0.0.1", port: 0 }),
  WorkspaceOld.defaultLayer,
  SessionNs.defaultLayer,
)
const _it = testEffect(testServerLayer)

describe("workspace-prior sync state", () => {
  test("startWorkspaceSyncing is disabled by the experimental workspace flag", async () => {
    await withInstance(async (dir) => {
      Flag.JEKKO_EXPERIMENTAL_WORKSPACES = false
      const type = unique("flag-disabled")
      const info = workspaceInfo(Instance.project.id, type)
      const session = await AppRuntime.runPromise(SessionNs.Service.use((svc) => svc.create({})))
      attachSessionToWorkspace(session.id, info.id)
      insertWorkspace(info)
      registerAdapter(Instance.project.id, type, localAdapter(path.join(dir, "flag-disabled")).adapter)

      startWorkspaceSyncing(Instance.project.id)
      await delay(25)

      expect((await workspaceStatus()).find((item) => item.workspaceID === info.id)?.status).toBeUndefined()
    })
  })

  test("startWorkspaceSyncing starts only workspaces with sessions", async () => {
    await withInstance(async (dir) => {
      const withSessionType = unique("with-session")
      const withoutSessionType = unique("without-session")
      const withSession = workspaceInfo(Instance.project.id, withSessionType)
      const withoutSession = workspaceInfo(Instance.project.id, withoutSessionType)
      const withSessionDir = path.join(dir, "with-session")
      const withoutSessionDir = path.join(dir, "without-session")
      await fs.mkdir(withSessionDir, { recursive: true })
      await fs.mkdir(withoutSessionDir, { recursive: true })
      insertWorkspace(withSession)
      insertWorkspace(withoutSession)
      registerAdapter(Instance.project.id, withSessionType, localAdapter(withSessionDir).adapter)
      registerAdapter(Instance.project.id, withoutSessionType, localAdapter(withoutSessionDir).adapter)
      attachSessionToWorkspace(
        (await AppRuntime.runPromise(SessionNs.Service.use((svc) => svc.create({})))).id,
        withSession.id,
      )

      startWorkspaceSyncing(Instance.project.id)

      await eventually(() =>
        workspaceStatus().then((status) =>
          expect(status.find((item) => item.workspaceID === withSession.id)?.status).toBe("connected"),
        ),
      )
      expect((await workspaceStatus()).find((item) => item.workspaceID === withoutSession.id)?.status).toBeUndefined()
      await removeWorkspace(withSession.id)
      await removeWorkspace(withoutSession.id)
    })
  })

  test("local start reports error when the target directory is missing", async () => {
    await withInstance(async (dir) => {
      const type = unique("missing-local")
      const info = workspaceInfo(Instance.project.id, type)
      insertWorkspace(info)
      registerAdapter(
        Instance.project.id,
        type,
        localAdapter(path.join(dir, "missing-target"), { createDir: false }).adapter,
      )
      attachSessionToWorkspace(
        (await AppRuntime.runPromise(SessionNs.Service.use((svc) => svc.create({})))).id,
        info.id,
      )

      startWorkspaceSyncing(Instance.project.id)

      await eventually(() =>
        workspaceStatus().then((status) =>
          expect(status.find((item) => item.workspaceID === info.id)?.status).toBe("error"),
        ),
      )
      expect(await isWorkspaceSyncing(info.id)).toBe(false)
      await removeWorkspace(info.id)
    })
  })

  it.live("remote start emits disconnected, connecting, and connected then refuses duplicate listeners", () => {
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
          if (call.url.pathname === "/sync/global/event") return HttpServerResponse.fromWeb(eventStreamResponse())
          if (call.url.pathname === "/sync/sync/history") return HttpServerResponse.fromWeb(Response.json([]))
          return HttpServerResponse.text("unexpected", { status: 500 })
        }),
      )
      const url = yield* serverUrl()
      yield* provideTmpdirInstance(
        () =>
          Effect.gen(function* () {
            const workspace = yield* WorkspaceOld.Service
            const sessionSvc = yield* SessionNs.Service
            const captured = captureGlobalEvents()
            try {
              const type = unique("remote-start")
              const info = workspaceInfo(Instance.project.id, type)
              insertWorkspace(info)
              registerAdapter(Instance.project.id, type, remoteAdapter(`${url}/sync`).adapter)
              attachSessionToWorkspace((yield* sessionSvc.create({})).id, info.id)

              yield* workspace.startWorkspaceSyncing(Instance.project.id)
              yield* eventuallyEffect(
                Effect.gen(function* () {
                  expect((yield* workspace.status()).find((item) => item.workspaceID === info.id)?.status).toBe(
                    "connected",
                  )
                }),
              )
              yield* workspace.startWorkspaceSyncing(Instance.project.id)
              yield* Effect.sleep("25 millis")

              expect(
                captured.events
                  .filter(
                    (event) => event.workspace === info.id && event.payload.type === WorkspaceOld.Event.Status.type,
                  )
                  .map((event) => event.payload.properties.status),
              ).toEqual(["disconnected", "connecting", "connected"])
              expect(calls.filter((call) => call.url.pathname === "/sync/global/event")).toHaveLength(1)
              expect(calls.filter((call) => call.url.pathname === "/sync/sync/history")).toHaveLength(1)
              expect(yield* workspace.isSyncing(info.id)).toBe(true)

              yield* workspace.remove(info.id)
              expect(yield* workspace.isSyncing(info.id)).toBe(false)
            } finally {
              captured.dispose()
            }
          }),
        { git: true },
      )
    })
  })

  it.live("remote connection HTTP failures set error and clear syncing", () =>
    Effect.gen(function* () {
      yield* HttpServer.serveEffect()(
        Effect.gen(function* () {
          const req = yield* HttpServerRequest.HttpServerRequest
          if (new URL(req.url, "http://localhost").pathname === "/failed/global/event")
            return HttpServerResponse.text("nope", { status: 503 })
          return HttpServerResponse.fromWeb(Response.json([]))
        }),
      )
      const url = yield* serverUrl()
      yield* provideTmpdirInstance(
        () =>
          Effect.gen(function* () {
            const workspace = yield* WorkspaceOld.Service
            const sessionSvc = yield* SessionNs.Service
            const type = unique("remote-connect-fail")
            const info = workspaceInfo(Instance.project.id, type)
            insertWorkspace(info)
            registerAdapter(Instance.project.id, type, remoteAdapter(`${url}/failed`).adapter)
            attachSessionToWorkspace((yield* sessionSvc.create({})).id, info.id)

            yield* workspace.startWorkspaceSyncing(Instance.project.id)

            yield* eventuallyEffect(
              Effect.gen(function* () {
                expect((yield* workspace.status()).find((item) => item.workspaceID === info.id)?.status).toBe("error")
              }),
            )
            expect(yield* workspace.isSyncing(info.id)).toBe(false)
            yield* workspace.remove(info.id)
          }),
        { git: true },
      )
    }),
  )

  it.live("remote history HTTP failures set error", () =>
    Effect.gen(function* () {
      yield* HttpServer.serveEffect()(
        Effect.gen(function* () {
          const req = yield* HttpServerRequest.HttpServerRequest
          const url = new URL(req.url, "http://localhost")
          if (url.pathname === "/history-failed/global/event")
            return HttpServerResponse.fromWeb(eventStreamResponse([], false))
          if (url.pathname === "/history-failed/sync/history")
            return HttpServerResponse.text("history failed", { status: 500 })
          return HttpServerResponse.fromWeb(Response.json([]))
        }),
      )
      const url = yield* serverUrl()
      yield* provideTmpdirInstance(
        () =>
          Effect.gen(function* () {
            const workspace = yield* WorkspaceOld.Service
            const sessionSvc = yield* SessionNs.Service
            const type = unique("remote-history-fail")
            const info = workspaceInfo(Instance.project.id, type)
            insertWorkspace(info)
            registerAdapter(Instance.project.id, type, remoteAdapter(`${url}/history-failed`).adapter)
            attachSessionToWorkspace((yield* sessionSvc.create({})).id, info.id)

            yield* workspace.startWorkspaceSyncing(Instance.project.id)

            yield* eventuallyEffect(
              Effect.gen(function* () {
                expect((yield* workspace.status()).find((item) => item.workspaceID === info.id)?.status).toBe("error")
              }),
            )
            expect(yield* workspace.isSyncing(info.id)).toBe(false)
            yield* workspace.remove(info.id)
          }),
        { git: true },
      )
    }),
  )

  it.live("sync history sends the local sequence fence and replays returned events in workspace context", () => {
    const historyBodies: unknown[] = []
    let historySessionID: SessionID | undefined
    let historyNextSeq = 0
    return Effect.gen(function* () {
      yield* HttpServer.serveEffect()(
        Effect.gen(function* () {
          const req = yield* HttpServerRequest.HttpServerRequest
          const bodyText = yield* req.text
          const url = new URL(req.url, "http://localhost")
          if (url.pathname === "/history/global/event") return HttpServerResponse.fromWeb(eventStreamResponse())
          if (url.pathname === "/history/sync/history") {
            historyBodies.push(bodyText ? JSON.parse(bodyText) : undefined)
            return HttpServerResponse.fromWeb(
              Response.json([
                {
                  id: `evt_${unique("history")}`,
                  aggregate_id: historySessionID!,
                  seq: historyNextSeq,
                  type: sessionUpdatedType(),
                  data: { sessionID: historySessionID!, info: { title: "from history" } },
                },
              ]),
            )
          }
          return HttpServerResponse.text("unexpected", { status: 500 })
        }),
      )
      const url = yield* serverUrl()
      yield* provideTmpdirInstance(
        () =>
          Effect.gen(function* () {
            const workspace = yield* WorkspaceOld.Service
            const sessionSvc = yield* SessionNs.Service
            const captured = captureGlobalEvents()
            try {
              const type = unique("history-replay")
              const info = workspaceInfo(Instance.project.id, type)
              insertWorkspace(info)
              registerAdapter(Instance.project.id, type, remoteAdapter(`${url}/history`).adapter)
              const session = yield* sessionSvc.create({ title: "before history" })
              attachSessionToWorkspace(session.id, info.id)
              historySessionID = session.id
              historyNextSeq = (sessionSequence(session.id) ?? -1) + 1

              yield* workspace.startWorkspaceSyncing(Instance.project.id)

              yield* eventuallyEffect(
                Effect.gen(function* () {
                  expect((yield* sessionSvc.get(session.id).pipe(Effect.orDie)).title).toBe("from history")
                }),
              )
              expect(historyBodies).toEqual([{ [session.id]: historyNextSeq - 1 }])
              expect(
                captured.events.some(
                  (event) =>
                    event.workspace === info.id &&
                    event.payload.type === "sync" &&
                    event.payload.syncEvent.seq === historyNextSeq,
                ),
              ).toBe(true)
              yield* workspace.remove(info.id)
            } finally {
              captured.dispose()
            }
          }),
        { git: true },
      )
    })
  })

  it.live("SSE forwards non-heartbeat events and ignores heartbeats", () =>
    Effect.gen(function* () {
      yield* HttpServer.serveEffect()(
        Effect.gen(function* () {
          const req = yield* HttpServerRequest.HttpServerRequest
          const url = new URL(req.url, "http://localhost")
          if (url.pathname === "/sse-forward/global/event")
            return HttpServerResponse.fromWeb(
              eventStreamResponse(
                [
                  { directory: "remote-dir", project: "remote-project", payload: { type: "server.heartbeat" } },
                  {
                    directory: "remote-dir",
                    project: "remote-project",
                    payload: { type: "custom.remote", properties: { ok: true } },
                  },
                ],
                false,
              ),
            )
          if (url.pathname === "/sse-forward/sync/history") return HttpServerResponse.fromWeb(Response.json([]))
          return HttpServerResponse.text("unexpected", { status: 500 })
        }),
      )
      const url = yield* serverUrl()
      yield* provideTmpdirInstance(
        () =>
          Effect.gen(function* () {
            const workspace = yield* WorkspaceOld.Service
            const sessionSvc = yield* SessionNs.Service
            const captured = captureGlobalEvents()
            try {
              const type = unique("sse-forward")
              const info = workspaceInfo(Instance.project.id, type)
              insertWorkspace(info)
              registerAdapter(Instance.project.id, type, remoteAdapter(`${url}/sse-forward`).adapter)
              attachSessionToWorkspace((yield* sessionSvc.create({})).id, info.id)

              yield* workspace.startWorkspaceSyncing(Instance.project.id)

              yield* eventuallyEffect(
                Effect.sync(() =>
                  expect(
                    captured.events.some(
                      (event) => event.workspace === info.id && event.payload.type === "custom.remote",
                    ),
                  ).toBe(true),
                ),
              )
              expect(
                captured.events.some(
                  (event) => event.workspace === info.id && event.payload.type === "server.heartbeat",
                ),
              ).toBe(false)
              expect(
                captured.events.find((event) => event.workspace === info.id && event.payload.type === "custom.remote"),
              ).toMatchObject({
                directory: "remote-dir",
                project: "remote-project",
                payload: { properties: { ok: true } },
              })
              yield* workspace.remove(info.id)
            } finally {
              captured.dispose()
            }
          }),
        { git: true },
      )
    }),
  )

  it.live("SSE sync events are replayed and forwarded", () => {
    let sseSessionID: SessionID | undefined
    let sseNextSeq = 0
    return Effect.gen(function* () {
      yield* HttpServer.serveEffect()(
        Effect.gen(function* () {
          const req = yield* HttpServerRequest.HttpServerRequest
          const url = new URL(req.url, "http://localhost")
          if (url.pathname === "/sse-sync/global/event")
            return HttpServerResponse.fromWeb(
              eventStreamResponse(
                [
                  {
                    directory: "remote-dir",
                    project: "remote-project",
                    payload: {
                      type: "sync",
                      syncEvent: {
                        id: `evt_${unique("sse")}`,
                        aggregateID: sseSessionID!,
                        seq: sseNextSeq,
                        type: sessionUpdatedType(),
                        data: { sessionID: sseSessionID!, info: { title: "from sse" } },
                      },
                    },
                  },
                ],
                false,
              ),
            )
          if (url.pathname === "/sse-sync/sync/history") return HttpServerResponse.fromWeb(Response.json([]))
          return HttpServerResponse.text("unexpected", { status: 500 })
        }),
      )
      const url = yield* serverUrl()
      yield* provideTmpdirInstance(
        () =>
          Effect.gen(function* () {
            const workspace = yield* WorkspaceOld.Service
            const sessionSvc = yield* SessionNs.Service
            const captured = captureGlobalEvents()
            try {
              const type = unique("sse-sync")
              const info = workspaceInfo(Instance.project.id, type)
              insertWorkspace(info)
              registerAdapter(Instance.project.id, type, remoteAdapter(`${url}/sse-sync`).adapter)
              const session = yield* sessionSvc.create({ title: "before sse" })
              attachSessionToWorkspace(session.id, info.id)
              sseSessionID = session.id
              sseNextSeq = (sessionSequence(session.id) ?? -1) + 1

              yield* workspace.startWorkspaceSyncing(Instance.project.id)

              yield* eventuallyEffect(
                Effect.gen(function* () {
                  expect((yield* sessionSvc.get(session.id).pipe(Effect.orDie)).title).toBe("from sse")
                }),
              )
              expect(
                captured.events.some(
                  (event) =>
                    event.workspace === info.id &&
                    event.payload.type === "sync" &&
                    event.payload.syncEvent.seq === sseNextSeq,
                ),
              ).toBe(true)
              yield* workspace.remove(info.id)
            } finally {
              captured.dispose()
            }
          }),
        { git: true },
      )
    })
  })
})
