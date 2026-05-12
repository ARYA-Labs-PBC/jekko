import { Effect } from "effect"
import { WorkspaceID } from "./schema"
import * as WS from "./workspace-shared"

export type CrudDeps = {
  auth: any
  session: any
  prompt: any
  http: any
  sync: any
  status: () => Effect.Effect<WS.ConnectionStatus[]>
  startSync: (space: WS.Info) => Effect.Effect<void>
  stopSync: (id: WorkspaceID) => Effect.Effect<void>
  syncHistory: (space: WS.Info, url: URL | string, headers: HeadersInit | undefined) => Effect.Effect<void>
}

export type WorkspaceGetter = (id: WorkspaceID) => Effect.Effect<WS.Info | undefined>

