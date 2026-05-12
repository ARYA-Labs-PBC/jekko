import { describe, expect, test } from "bun:test"
import { setTimeout as delay } from "node:timers/promises"
import { eq } from "drizzle-orm"
import { Database } from "@/storage/db"
import { EventSequenceTable } from "@/sync/event.sql"
import { GlobalBus } from "@/bus/global"
import { SessionID } from "@/session/schema"
import { WorkspaceID } from "../../src/control-plane/schema"
import { withInstance, waitForWorkspaceSync } from "./workspace.test"

describe("workspace-prior waitForSync", () => {
  test("returns immediately for an empty fence", async () => {
    await withInstance(async () => {
      await expect(waitForWorkspaceSync(WorkspaceID.ascending("wrk_wait_empty"), {})).resolves.toBeUndefined()
    })
  })

  test("returns immediately when the stored sequence already satisfies the fence", async () => {
    await withInstance(async () => {
      const sessionID = SessionID.descending("ses_wait_done")
      Database.use((db) => db.insert(EventSequenceTable).values({ aggregate_id: sessionID, seq: 4 }).run())

      await expect(
        waitForWorkspaceSync(WorkspaceID.ascending("wrk_wait_done"), { [sessionID]: 4 }),
      ).resolves.toBeUndefined()
      await expect(
        waitForWorkspaceSync(WorkspaceID.ascending("wrk_wait_done_2"), { [sessionID]: 3 }),
      ).resolves.toBeUndefined()
    })
  })

  test("waits until the database reaches the requested sequence and a workspace event arrives", async () => {
    await withInstance(async () => {
      const workspaceID = WorkspaceID.ascending("wrk_wait_event")
      const sessionID = SessionID.descending("ses_wait_event")
      Database.use((db) => db.insert(EventSequenceTable).values({ aggregate_id: sessionID, seq: 1 }).run())

      const waited = waitForWorkspaceSync(workspaceID, { [sessionID]: 2 })
      await delay(10)
      Database.use((db) =>
        db.update(EventSequenceTable).set({ seq: 2 }).where(eq(EventSequenceTable.aggregate_id, sessionID)).run(),
      )
      GlobalBus.emit("event", { workspace: workspaceID, payload: { type: "anything" } })

      await expect(waited).resolves.toBeUndefined()
    })
  })

  test("a sync event for a different workspace can also release the fence", async () => {
    await withInstance(async () => {
      const workspaceID = WorkspaceID.ascending("wrk_wait_sync_any")
      const sessionID = SessionID.descending("ses_wait_sync_any")
      Database.use((db) => db.insert(EventSequenceTable).values({ aggregate_id: sessionID, seq: 0 }).run())

      const waited = waitForWorkspaceSync(workspaceID, { [sessionID]: 1 })
      await delay(10)
      Database.use((db) =>
        db.update(EventSequenceTable).set({ seq: 1 }).where(eq(EventSequenceTable.aggregate_id, sessionID)).run(),
      )
      GlobalBus.emit("event", {
        workspace: WorkspaceID.ascending("wrk_other_workspace"),
        payload: { type: "sync" },
      })

      await expect(waited).resolves.toBeUndefined()
    })
  })

  test("rejects with the abort reason when aborted", async () => {
    await withInstance(async () => {
      const abort = new AbortController()
      const reason = new Error("caller aborted")
      const waited = waitForWorkspaceSync(
        WorkspaceID.ascending("wrk_wait_abort"),
        { [SessionID.descending("ses_wait_abort")]: 1 },
        abort.signal,
      )
      abort.abort(reason)

      await expect(waited).rejects.toMatchObject({
        _tag: "WorkspaceSyncAbortedError",
        message: reason.message,
        cause: reason,
      })
    })
  })

  test("times out with the requested fence in the error message", async () => {
    await withInstance(async () => {
      const sessionID = SessionID.descending("ses_wait_timeout")

      await expect(waitForWorkspaceSync(WorkspaceID.ascending("wrk_wait_timeout"), { [sessionID]: 1 })).rejects.toThrow(
        `Timed out waiting for sync fence: {"${sessionID}":1}`,
      )
    })
  }, 7000)
})
