import { Effect } from "effect"
import { eq } from "drizzle-orm"
import { WorkspaceID } from "./schema"
import * as WS from "./workspace-shared"
import type { CrudDeps } from "./workspace-crud-types"

export function makeWorkspaceRemove(deps: CrudDeps) {
  return Effect.fn("Workspace.remove")(function* (id: WorkspaceID) {
    const sessions = yield* WS.db((db) =>
      db
        .select({ id: WS.SessionTable.id, parentID: WS.SessionTable.parent_id })
        .from(WS.SessionTable)
        .where(eq(WS.SessionTable.workspace_id, id))
        .all(),
    )
    const sessionIDs = new Set(sessions.map((sessionInfo) => sessionInfo.id))
    yield* Effect.forEach(
      sessions.filter((sessionInfo) => !sessionInfo.parentID || !sessionIDs.has(sessionInfo.parentID)),
      (sessionInfo) =>
        deps.session.remove(sessionInfo.id).pipe(Effect.catchIf(WS.NotFoundError.isInstance, () => Effect.void)),
      { discard: true },
    )

    const row = yield* WS.db((db) => db.select().from(WS.WorkspaceTable).where(eq(WS.WorkspaceTable.id, id)).get())
    if (!row) return

    yield* deps.stopSync(id)

    const info = WS.fromRow(row)
    yield* Effect.catchCause(
      Effect.gen(function* () {
        const adapter = WS.getAdapter(info.projectID, row.type)
        yield* WS.EffectBridge.fromPromise(() => adapter.remove(info))
      }),
      () =>
        Effect.sync(() => {
          WS.log.error("adapter not available when removing workspace", { type: row.type })
        }),
    )

    yield* WS.db((db) => db.delete(WS.WorkspaceTable).where(eq(WS.WorkspaceTable.id, id)).run())
    return info
  })
}

