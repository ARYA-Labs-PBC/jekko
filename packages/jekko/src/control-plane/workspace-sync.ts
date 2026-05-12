import { Effect } from "effect"
import { eq } from "drizzle-orm"
import { Project } from "@/project/project"
import { WorkspaceTable } from "./workspace.sql"
import { SessionTable } from "@/session/session.sql"
import * as Log from "@jekko-ai/core/util/log"
import { createWorkspaceSyncRuntime, type SyncDeps } from "./workspace-sync-runtime"
import { db } from "./workspace-shared"

export function createWorkspaceSync(deps: SyncDeps) {
  const log = Log.create({ service: "workspace-sync" })
  const runtime = createWorkspaceSyncRuntime(deps, log)

  const startWorkspaceSyncing = Effect.fn("Workspace.startWorkspaceSyncing")(function* (projectID: Project.Info) {
    const rows = yield* db((db) =>
      db
        .selectDistinct({ workspace: WorkspaceTable })
        .from(WorkspaceTable)
        .innerJoin(SessionTable, eq(SessionTable.workspace_id, WorkspaceTable.id))
        .where(eq(WorkspaceTable.project_id, projectID))
        .all(),
    )

    for (const { workspace } of rows) {
      yield* runtime.startSync({
        id: workspace.id,
        type: workspace.type,
        branch: workspace.branch,
        name: workspace.name,
        directory: workspace.directory,
        extra: workspace.extra,
        projectID: workspace.project_id,
      }).pipe(
        Effect.catch((error) =>
          Effect.sync(() => {
            deps.setStatus(workspace.id, "error")
            log.warn("workspace sync failed to start", {
              workspaceID: workspace.id,
              error,
            })
          }),
        ),
        Effect.forkDetach,
      )
    }
  })

  return {
    syncHistory: runtime.syncHistory,
    startSync: runtime.startSync,
    stopSync: runtime.stopSync,
    status: runtime.status,
    isSyncing: runtime.isSyncing,
    waitForSync: runtime.waitForSync,
    startWorkspaceSyncing,
  }
}

export const makeWorkspaceSync = createWorkspaceSync
