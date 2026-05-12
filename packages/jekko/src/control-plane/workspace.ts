import { Context, Effect, FiberMap, Layer } from "effect"
import { FetchHttpClient, HttpClient } from "effect/unstable/http"
import { Auth } from "@/auth"
import { Session } from "@/session/session"
import { SessionPrompt } from "@/session/prompt"
import { WorkspaceID } from "./schema"
import {
  ConnectionStatus,
  CreateError,
  CreateInput,
  Event,
  Info,
  SessionWarpError,
  SessionWarpInput,
  SyncLoopError,
  WaitForSyncError,
  SyncTimeoutError,
  SyncAbortedError,
  SyncHttpError,
  WorkspaceNotFoundError,
  SessionEventsNotFoundError,
  SessionWarpHttpError,
  SyncEvent,
  Project,
  ProjectID,
  GlobalBus,
} from "./workspace-shared"
import { makeWorkspaceSync } from "./workspace-sync"
import { makeWorkspaceCrud } from "./workspace-crud"

export type { Info, ConnectionStatus, CreateInput, SessionWarpInput, CreateError, SessionWarpError, WaitForSyncError }
export {
  Event,
  SyncHttpError,
  WorkspaceNotFoundError,
  SessionEventsNotFoundError,
  SessionWarpHttpError,
  SyncTimeoutError,
  SyncAbortedError,
}

export interface Interface {
  readonly create: (input: CreateInput) => Effect.Effect<Info, CreateError>
  readonly sessionWarp: (input: SessionWarpInput) => Effect.Effect<void, SessionWarpError>
  readonly list: (project: Project.Info) => Effect.Effect<Info[]>
  readonly get: (id: WorkspaceID) => Effect.Effect<Info | undefined>
  readonly remove: (id: WorkspaceID) => Effect.Effect<Info | undefined>
  readonly status: () => Effect.Effect<ConnectionStatus[]>
  readonly isSyncing: (workspaceID: WorkspaceID) => Effect.Effect<boolean>
  readonly waitForSync: (
    workspaceID: WorkspaceID,
    state: Record<string, number>,
    signal?: AbortSignal,
  ) => Effect.Effect<void, WaitForSyncError>
  readonly startWorkspaceSyncing: (projectID: ProjectID) => Effect.Effect<void>
}

export class Service extends Context.Service<Service, Interface>()("@jekko/Workspace") {}

export const layer = Layer.effect(
  Service,
  Effect.gen(function* () {
    const auth = yield* Auth.Service
    const session = yield* Session.Service
    const prompt = yield* SessionPrompt.Service
    const http = yield* HttpClient.HttpClient
    const sync = yield* SyncEvent.Service
    const connections = new Map<WorkspaceID, ConnectionStatus>()
    const syncFibers = yield* FiberMap.make<WorkspaceID, void, SyncLoopError>()

    const setStatus = (id: WorkspaceID, status: ConnectionStatus["status"]) => {
      const prev = connections.get(id)
      if (prev?.status === status) return
      const next = { workspaceID: id, status }
      connections.set(id, next)

      GlobalBus.emit("event", {
        directory: "global",
        workspace: id,
        payload: {
          type: Event.Status.type,
          properties: next,
        },
      })
    }

    const syncActions = makeWorkspaceSync({
      auth,
      session,
      prompt,
      http,
      sync,
      connections,
      syncFibers,
      setStatus,
    })

    const crudActions = makeWorkspaceCrud({
      auth,
      session,
      prompt,
      http,
      sync,
      status: syncActions.status,
      startSync: syncActions.startSync,
      stopSync: syncActions.stopSync,
      syncHistory: syncActions.syncHistory,
    })

    return Service.of({
      create: crudActions.create,
      sessionWarp: crudActions.sessionWarp,
      list: crudActions.list,
      get: crudActions.get,
      remove: crudActions.remove,
      status: syncActions.status,
      isSyncing: syncActions.isSyncing,
      waitForSync: syncActions.waitForSync,
      startWorkspaceSyncing: syncActions.startWorkspaceSyncing,
    })
  }),
)

export const defaultLayer = layer.pipe(
  Layer.provide(Auth.defaultLayer),
  Layer.provide(Session.defaultLayer),
  Layer.provide(SyncEvent.defaultLayer),
  Layer.provide(SessionPrompt.defaultLayer),
  Layer.provide(FetchHttpClient.layer),
)

export const Workspace = {
  Service,
  Info,
  CreateInput,
  SessionWarpInput,
  ConnectionStatus,
  Event,
  SyncHttpError,
  WorkspaceNotFoundError,
  SessionEventsNotFoundError,
  SessionWarpHttpError,
  SyncTimeoutError,
  SyncAbortedError,
  layer,
  defaultLayer,
} as const

export * from "./workspace-shared"
