import { Effect } from "effect"
import { eq } from "drizzle-orm"
import { WorkspaceID } from "./schema"
import * as WS from "./workspace-shared"

export function makeWorkspaceRead() {
  const list = Effect.fn("Workspace.list")(function* (project: WS.Project.Info) {
    return yield* WS.db((db) =>
      db
        .select()
        .from(WS.WorkspaceTable)
        .where(eq(WS.WorkspaceTable.project_id, project.id))
        .all()
        .map(WS.fromRow)
        .sort((a, b) => a.id.localeCompare(b.id)),
    )
  })

  const get = Effect.fn("Workspace.get")(function* (id: WorkspaceID) {
    const row = yield* WS.db((db) => db.select().from(WS.WorkspaceTable).where(eq(WS.WorkspaceTable.id, id)).get())
    if (!row) return
    return WS.fromRow(row)
  })

  return {
    list,
    get,
  }
}

