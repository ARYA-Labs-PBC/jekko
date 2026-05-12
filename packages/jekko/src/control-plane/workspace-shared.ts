// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
// jankurai:allow HLT-000-SCORE-DIMENSION reason=large-structured-file-with-parallel-patterns-by-design expires=2027-01-01
import { Context, Effect, FiberMap, Option, Schema, Stream } from "effect"
import { FetchHttpClient, HttpBody, HttpClient, HttpClientError, HttpClientRequest } from "effect/unstable/http"
import { Database } from "@/storage/db"
import { asc, eq, inArray } from "drizzle-orm"
import { Project } from "@/project/project"
import { Instance } from "@/project/instance"
import { BusEvent } from "@/bus/bus-event"
import { GlobalBus } from "@/bus/global"
import { Auth } from "@/auth"
import { SyncEvent } from "@/sync"
import { EventSequenceTable, EventTable } from "@/sync/event.sql"
import { Flag } from "@jekko-ai/core/flag/flag"
import * as Log from "@jekko-ai/core/util/log"
import { Filesystem } from "@/util/filesystem"
import { ProjectID } from "@/project/schema"
import { Slug } from "@jekko-ai/core/util/slug"
import { WorkspaceTable } from "./workspace.sql"
import { getAdapter } from "./adapters"
import { type WorkspaceInfo, WorkspaceInfo as WorkspaceInfoSchema } from "./types"
import { WorkspaceID } from "./schema"
import { Session } from "@/session/session"
import { SessionPrompt } from "@/session/prompt"
import { SessionTable } from "@/session/session.sql"
import { SessionID } from "@/session/schema"
import { NotFoundError } from "@/storage/storage"
import { errorData } from "@/util/error"
import { WorkspaceContext } from "./workspace-context"
import { EffectBridge } from "@/effect/bridge"
import { withStatics } from "@/util/schema"
import { zod as effectZod, zodObject } from "@/util/effect-zod"

export const Info = WorkspaceInfoSchema
export type Info = WorkspaceInfo

export const ConnectionStatus = Schema.Struct({
  workspaceID: WorkspaceID,
  status: Schema.Literals(["connected", "connecting", "disconnected", "error"]),
})
export type ConnectionStatus = Schema.Schema.Type<typeof ConnectionStatus>

export const Event = {
  Ready: BusEvent.define(
    "workspace.ready",
    Schema.Struct({
      name: Schema.String,
    }),
  ),
  Failed: BusEvent.define(
    "workspace.failed",
    Schema.Struct({
      message: Schema.String,
    }),
  ),
  Status: BusEvent.define("workspace.status", ConnectionStatus),
}

export function fromRow(row: typeof WorkspaceTable.$inferSelect): Info {
  return {
    id: row.id,
    type: row.type,
    branch: row.branch,
    name: row.name,
    directory: row.directory,
    extra: row.extra,
    projectID: row.project_id,
  }
}

export const db = <T>(fn: (d: Parameters<typeof Database.use>[0] extends (trx: infer D) => any ? D : never) => T) =>
  Effect.sync(() => Database.use(fn))

export const log = Log.create({ service: "workspace-sync" })
export const decodeSSEData = Schema.decodeUnknownOption(Schema.fromJsonString(Schema.Unknown))

export const CreateInput = Schema.Struct({
  id: Schema.optional(WorkspaceID),
  type: Info.fields.type,
  branch: Info.fields.branch,
  projectID: ProjectID,
  extra: Schema.optional(Info.fields.extra),
}).pipe(withStatics((s) => ({ zod: effectZod(s), zodObject: zodObject(s) })))
export type CreateInput = Schema.Schema.Type<typeof CreateInput>

export const SessionWarpInput = Schema.Struct({
  workspaceID: Schema.NullOr(WorkspaceID),
  sessionID: SessionID,
}).pipe(withStatics((s) => ({ zod: effectZod(s), zodObject: zodObject(s) })))
export type SessionWarpInput = Schema.Schema.Type<typeof SessionWarpInput>

export class SyncHttpError extends Schema.TaggedErrorClass<SyncHttpError>()("WorkspaceSyncHttpError", {
  message: Schema.String,
  status: Schema.Number,
  body: Schema.optional(Schema.String),
}) {}

export class WorkspaceNotFoundError extends Schema.TaggedErrorClass<WorkspaceNotFoundError>()(
  "WorkspaceNotFoundError",
  {
    message: Schema.String,
    workspaceID: WorkspaceID,
  },
) {}

export class SessionEventsNotFoundError extends Schema.TaggedErrorClass<SessionEventsNotFoundError>()(
  "WorkspaceSessionEventsNotFoundError",
  {
    message: Schema.String,
    sessionID: SessionID,
  },
) {}

export class SessionWarpHttpError extends Schema.TaggedErrorClass<SessionWarpHttpError>()(
  "WorkspaceSessionWarpHttpError",
  {
    message: Schema.String,
    workspaceID: WorkspaceID,
    sessionID: SessionID,
    status: Schema.Number,
    body: Schema.String,
  },
) {}

export class SyncTimeoutError extends Schema.TaggedErrorClass<SyncTimeoutError>()("WorkspaceSyncTimeoutError", {
  message: Schema.String,
  state: Schema.Record(Schema.String, Schema.Number),
}) {}

export class SyncAbortedError extends Schema.TaggedErrorClass<SyncAbortedError>()("WorkspaceSyncAbortedError", {
  message: Schema.String,
  cause: Schema.optional(Schema.Defect),
}) {}

export type CreateError = Auth.AuthError
export type SessionWarpError =
  | WorkspaceNotFoundError
  | SessionEventsNotFoundError
  | SessionWarpHttpError
  | HttpClientError.HttpClientError
export type WaitForSyncError = SyncTimeoutError | SyncAbortedError
export type SyncLoopError = SyncHttpError | HttpClientError.HttpClientError

export const TIMEOUT = 5000

export function synced(state: Record<string, number>) {
  const ids = Object.keys(state)
  if (ids.length === 0) return true

  const done = Object.fromEntries(
    Database.use((db) =>
      db
        .select({
          id: EventSequenceTable.aggregate_id,
          seq: EventSequenceTable.seq,
        })
        .from(EventSequenceTable)
        .where(inArray(EventSequenceTable.aggregate_id, ids))
        .all(),
    ).map((row) => [row.id, row.seq]),
  ) as Record<string, number>

  return ids.every((id) => {
    return (done[id] ?? -1) >= state[id]
  })
}

export function route(url: string | URL, path: string) {
  const next = new URL(url)
  next.pathname = `${next.pathname.replace(/\/$/, "")}${path}`
  next.search = ""
  next.hash = ""
  return next
}

export { Context, Effect, FiberMap, Option, Schema, Stream, FetchHttpClient, HttpBody, HttpClient, HttpClientRequest, Project, Instance, BusEvent, GlobalBus, SyncEvent, EventSequenceTable, EventTable, Flag, Filesystem, ProjectID, Slug, WorkspaceTable, getAdapter, Session, SessionPrompt, SessionTable, NotFoundError, errorData, WorkspaceContext, EffectBridge, asc, eq, inArray }
