import { errorData } from "@/util/error"
import { asc, eq } from "drizzle-orm"
import { Effect, Iterable } from "effect"
import * as WS from "./workspace-shared"
import type { CrudDeps, WorkspaceGetter } from "./workspace-crud-types"

export function makeWorkspaceSessionWarp(deps: CrudDeps, get: WorkspaceGetter) {
  return Effect.fn("Workspace.sessionWarp")(function* (input: WS.SessionWarpInput) {
    return yield* Effect.gen(function* () {
      WS.log.info("session warp requested", {
        workspaceID: input.workspaceID,
        sessionID: input.sessionID,
      })

      const current = yield* WS.db((db) =>
        db
          .select({ workspaceID: WS.SessionTable.workspace_id })
          .from(WS.SessionTable)
          .where(eq(WS.SessionTable.id, input.sessionID))
          .get(),
      )

      if (current?.workspaceID) {
        const previous = yield* get(current.workspaceID)
        if (previous) {
          const adapter = WS.getAdapter(previous.projectID, previous.type)
          const target = yield* WS.EffectBridge.fromPromise(() => adapter.target(previous))

          if (target.type === "remote") {
            yield* deps.syncHistory(previous, target.url, target.headers).pipe(
              Effect.catch((error) =>
                Effect.sync(() => {
                  WS.log.warn("session warp final source sync failed", {
                    workspaceID: previous.id,
                    sessionID: input.sessionID,
                    error: errorData(error),
                  })
                }),
              ),
            )
          } else {
            yield* deps.prompt.cancel(input.sessionID)
          }

          WS.SyncEvent.claim(input.sessionID, input.workspaceID ?? WS.Instance.project.id)
        }
      }

      if (input.workspaceID === null) {
        yield* Effect.sync(() =>
          WS.SyncEvent.run(WS.Session.Event.Updated, {
            sessionID: input.sessionID,
            info: {
              workspaceID: null,
            },
          }),
        )

        WS.log.info("session warp complete", {
          workspaceID: input.workspaceID,
          sessionID: input.sessionID,
          target: "local",
        })
        return
      }

      const workspaceID = input.workspaceID
      const space = yield* get(workspaceID)
      if (!space)
        return yield* new WS.WorkspaceNotFoundError({
          message: `Workspace not found: ${workspaceID}`,
          workspaceID,
        })

      const adapter = WS.getAdapter(space.projectID, space.type)
      const target = yield* WS.EffectBridge.fromPromise(() => adapter.target(space))

      if (target.type === "local") {
        yield* deps.sync.run(WS.Session.Event.Updated, {
          sessionID: input.sessionID,
          info: {
            workspaceID: input.workspaceID,
          },
        })

        WS.log.info("session warp complete", {
          workspaceID: input.workspaceID,
          sessionID: input.sessionID,
          target: target.directory,
        })
        return
      }

      const rows = yield* WS.db((db) =>
        db
          .select({
            id: WS.EventTable.id,
            aggregateID: WS.EventTable.aggregate_id,
            seq: WS.EventTable.seq,
            type: WS.EventTable.type,
            data: WS.EventTable.data,
          })
          .from(WS.EventTable)
          .where(eq(WS.EventTable.aggregate_id, input.sessionID))
          .orderBy(asc(WS.EventTable.seq))
          .all(),
      )
      if (rows.length === 0)
        return yield* new WS.SessionEventsNotFoundError({
          message: `No events found for session: ${input.sessionID}`,
          sessionID: input.sessionID,
        })

      const batches = Iterable.chunksOf(rows, 10)
      const total = Iterable.size(batches)

      WS.log.info("session warp prepared", {
        workspaceID: input.workspaceID,
        sessionID: input.sessionID,
        target: String(WS.route(target.url, "/sync/replay")),
        events: rows.length,
        batches: total,
        first: rows[0]?.seq,
        last: rows.at(-1)?.seq,
      })

      yield* Effect.forEach(
        batches,
        (events, i) =>
          Effect.gen(function* () {
            const response = yield* deps.http.execute(
              WS.HttpClientRequest.post(WS.route(target.url, "/sync/replay"), {
                headers: new Headers(target.headers),
                body: WS.HttpBody.jsonUnsafe({
                  directory: space.directory ?? "",
                  events,
                }),
              }),
            )

            if (response.status < 200 || response.status >= 300) {
              const body = yield* response.text
              WS.log.error("session warp batch failed", {
                workspaceID: input.workspaceID,
                sessionID: input.sessionID,
                step: i + 1,
                total,
                status: response.status,
                body,
              })
              return yield* new WS.SessionWarpHttpError({
                message: `Failed to warp session ${input.sessionID} into workspace ${workspaceID}: HTTP ${response.status} ${body}`,
                workspaceID,
                sessionID: input.sessionID,
                status: response.status,
                body,
              })
            }

            WS.log.info("session warp batch posted", {
              workspaceID: input.workspaceID,
              sessionID: input.sessionID,
              step: i + 1,
              total,
              status: response.status,
            })
          }),
        { discard: true },
      )

      const response = yield* deps.http.execute(
        WS.HttpClientRequest.post(WS.route(target.url, "/sync/steal"), {
          headers: new Headers(target.headers),
          body: WS.HttpBody.jsonUnsafe({ sessionID: input.sessionID }),
        }),
      )
      if (response.status < 200 || response.status >= 300) {
        const body = yield* response.text
        WS.log.error("session warp steal failed", {
          workspaceID: input.workspaceID,
          sessionID: input.sessionID,
          status: response.status,
          body,
        })
        return yield* new WS.SessionWarpHttpError({
          message: `Failed to steal session ${input.sessionID} into workspace ${workspaceID}: HTTP ${response.status} ${body}`,
          workspaceID,
          sessionID: input.sessionID,
          status: response.status,
          body,
        })
      }

      WS.log.info("session warp complete", {
        workspaceID: input.workspaceID,
        sessionID: input.sessionID,
        target: String(target.url),
        batches: total,
      })
    }).pipe(
      Effect.tapError((err) =>
        Effect.sync(() =>
          WS.log.error("session warp failed", {
            workspaceID: input.workspaceID,
            sessionID: input.sessionID,
            error: errorData(err),
          }),
        ),
      ),
    )
  })
}

