import { Effect, FiberMap } from "effect"
import { HttpBody, HttpClientRequest } from "effect/unstable/http"
import { eq, inArray } from "drizzle-orm"
import { GlobalBus } from "@/bus/global"
import { SyncEvent } from "@/sync"
import { EventSequenceTable } from "@/sync/event.sql"
import { Flag } from "@jekko-ai/core/flag/flag"
import * as Log from "@jekko-ai/core/util/log"
import { Filesystem } from "@/util/filesystem"
import { getAdapter } from "./adapters"
import {
  type Info,
  ConnectionStatus,
  SyncAbortedError,
  SyncHttpError,
  SyncLoopError,
  SyncTimeoutError,
  WaitForSyncError,
  TIMEOUT,
  db,
  route,
  synced,
} from "./workspace-shared"
import { EffectBridge } from "@/effect/bridge"
import { WorkspaceContext } from "./workspace-context"
import { WorkspaceID } from "./schema"
import { waitEvent } from "./util"
import { SessionTable } from "@/session/session.sql"
import { createWorkspaceSyncCommunication } from "./workspace-sync-communication"

type SyncLogger = ReturnType<typeof Log.create>

export type SyncDeps = {
  http: { execute: (request: Parameters<typeof HttpClientRequest.post>[1]) => Effect.Effect<any> }
  sync: {
    replay: (event: any, options?: { publish?: boolean }) => Effect.Effect<void>
    run: (event: any, payload: any) => Effect.Effect<void>
  }
  connections: Map<WorkspaceID, ConnectionStatus>
  syncFibers: FiberMap.FiberMap<WorkspaceID, void, SyncLoopError>
  setStatus: (id: WorkspaceID, status: ConnectionStatus["status"]) => void
}

export function createWorkspaceSyncRuntime(deps: SyncDeps, log: SyncLogger) {
  const { connectSSE, parseSSE } = createWorkspaceSyncCommunication(deps)

  const syncHistory = Effect.fn("Workspace.syncHistory")(function* (
    space: Info,
    url: URL | string,
    headers: HeadersInit | undefined,
  ) {
    const sessionIDs = yield* db((db) =>
      db
        .select({ id: SessionTable.id })
        .from(SessionTable)
        .where(eq(SessionTable.workspace_id, space.id))
        .all()
        .map((row) => row.id),
    )
    const state = sessionIDs.length
      ? Object.fromEntries(
          (yield* db((db) =>
            db.select().from(EventSequenceTable).where(inArray(EventSequenceTable.aggregate_id, sessionIDs)).all(),
          )).map((row) => [row.aggregate_id, row.seq]),
        )
      : {}

    log.info("syncing workspace history", {
      workspaceID: space.id,
      sessions: sessionIDs.length,
      known: Object.keys(state).length,
    })

    const response = yield* deps.http.execute(
      HttpClientRequest.post(route(url, "/sync/history"), {
        headers: new Headers(headers),
        body: HttpBody.jsonUnsafe(state),
      }),
    )

    if (response.status < 200 || response.status >= 300) {
      const body = yield* response.text
      return yield* new SyncHttpError({
        message: `Workspace history HTTP failure: ${response.status} ${body}`,
        status: response.status,
        body,
      })
    }

    const events = (yield* response.json) as HistoryEvent[]

    log.info("workspace history synced", {
      workspaceID: space.id,
      events: events.length,
    })

    yield* Effect.promise(async () => {
      await WorkspaceContext.provide({
        workspaceID: space.id,
        async fn() {
          await Effect.runPromise(
            Effect.forEach(
              events,
              (event) =>
                deps.sync.replay(
                  {
                    id: event.id,
                    aggregateID: event.aggregate_id,
                    seq: event.seq,
                    type: event.type,
                    data: event.data,
                  },
                  { publish: true },
                ),
              { discard: true },
            ),
          )
        },
      })
    })
  })

  const syncWorkspaceLoop = Effect.fn("Workspace.syncWorkspaceLoop")(function* (space: Info) {
    const adapter = getAdapter(space.projectID, space.type)
    const target = yield* EffectBridge.fromPromise(() => adapter.target(space))

    if (target.type === "local") return

    let attempt = 0

    while (true) {
      log.info("connecting to global sync", { workspace: space.name })
      deps.setStatus(space.id, "connecting")

      const streamResult = yield* connectSSE(target.url, target.headers).pipe(
        Effect.tap(() => syncHistory(space, target.url, target.headers)),
        Effect.either,
      )

      if (streamResult._tag === "Right") {
        const stream = streamResult.right
        attempt = 0

        log.info("global sync connected", { workspace: space.name })
        deps.setStatus(space.id, "connected")

        yield* parseSSE(stream, (evt) =>
          Effect.gen(function* () {
            if (!evt || typeof evt !== "object" || !("payload" in evt)) return
            const payload = evt.payload as { type?: string; syncEvent?: SyncEvent.SerializedEvent }
            if (payload.type === "server.heartbeat") return

            if (payload.type === "sync" && payload.syncEvent) {
              const failed = yield* deps.sync.replay(payload.syncEvent).pipe(
                Effect.as(false),
                Effect.catchCause((error) =>
                  Effect.sync(() => {
                    log.info("failed to replay global event", {
                      workspaceID: space.id,
                      error,
                    })
                    return true
                  }),
                ),
              )
              if (failed) return
            }

            try {
              const event = evt as { directory?: string; project?: string; payload: unknown }
              GlobalBus.emit("event", {
                directory: event.directory,
                project: event.project,
                workspace: space.id,
                payload: event.payload,
              })
            } catch (error) {
              log.info("failed to replay global event", {
                workspaceID: space.id,
                error,
              })
            }
          }),
        )

        log.info("disconnected from global sync: " + space.id)
        deps.setStatus(space.id, "disconnected")
      } else {
        deps.setStatus(space.id, "error")
        log.info("failed to connect to global sync", {
          workspace: space.name,
          err: streamResult.left,
        })
      }

      yield* Effect.sleep(`${Math.min(120_000, 1_000 * 2 ** attempt)} millis`)
      attempt += 1
    }
  })

  const startSync = Effect.fn("Workspace.startSync")(function* (space: Info) {
    if (!Flag.JEKKO_EXPERIMENTAL_WORKSPACES) return

    const adapter = getAdapter(space.projectID, space.type)
    const target = yield* EffectBridge.fromPromise(() => adapter.target(space))

    if (target.type === "local") {
      deps.setStatus(space.id, (yield* Effect.promise(() => Filesystem.exists(target.directory))) ? "connected" : "error")
      return
    }

    const exists = yield* FiberMap.has(deps.syncFibers, space.id)
    if (exists && deps.connections.get(space.id)?.status !== "error") return

    deps.setStatus(space.id, "disconnected")

    yield* FiberMap.run(
      deps.syncFibers,
      space.id,
      syncWorkspaceLoop(space).pipe(
        Effect.catch((error) =>
          Effect.sync(() => {
            deps.setStatus(space.id, "error")
            log.warn("workspace listener failed", {
              workspaceID: space.id,
              error,
            })
          }),
        ),
      ),
    )
  })

  const stopSync = Effect.fn("Workspace.stopSync")(function* (id: WorkspaceID) {
    yield* FiberMap.remove(deps.syncFibers, id)
    deps.connections.delete(id)
  })

  const status = Effect.fn("Workspace.status")(function* () {
    return [...deps.connections.values()]
  })

  const isSyncing = Effect.fn("Workspace.isSyncing")(function* (workspaceID: WorkspaceID) {
    const exists = yield* FiberMap.has(deps.syncFibers, workspaceID)
    return exists && deps.connections.get(workspaceID)?.status !== "error"
  })

  const waitForSync = Effect.fn("Workspace.waitForSync")(function* (
    workspaceID: WorkspaceID,
    state: Record<string, number>,
    signal?: AbortSignal,
  ) {
    if (synced(state)) return

    yield* Effect.catch(
      waitEvent({
        timeout: TIMEOUT,
        signal,
        fn(event) {
          if (event.workspace !== workspaceID && event.payload.type !== "sync") {
            return false
          }
          return synced(state)
        },
      }),
      (): Effect.Effect<never, WaitForSyncError> =>
        signal?.aborted
          ? Effect.fail(
              new SyncAbortedError({
                message: signal.reason instanceof Error ? signal.reason.message : "Request aborted",
                cause: signal.reason,
              }),
            )
          : Effect.fail(
              new SyncTimeoutError({
                message: `Timed out waiting for sync fence: ${JSON.stringify(state)}`,
                state,
              }),
            ),
    )
  })

  return {
    syncHistory,
    startSync,
    stopSync,
    status,
    isSyncing,
    waitForSync,
  }
}

type HistoryEvent = {
  id: string
  aggregate_id: string
  seq: number
  type: string
  data: Record<string, unknown>
}
