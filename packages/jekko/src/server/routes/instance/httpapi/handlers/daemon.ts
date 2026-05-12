import { Cause, Effect, Schema } from "effect"
import { HttpApiBuilder, HttpApiError } from "effect/unstable/httpapi"
import { InstanceHttpApi } from "../api"
import { DaemonPaths, type DaemonPreviewPayload, type DaemonStartPayload, DaemonTaskActionPayload } from "../groups/daemon"
import { Daemon } from "@/session/daemon"
import { SessionID } from "@/session/schema"

function jsonSafe<T>(value: T): T {
  return JSON.parse(JSON.stringify(value))
}

export const daemonHandlers = HttpApiBuilder.group(InstanceHttpApi, "daemon", (handlers) =>
  Effect.gen(function* () {
    const daemon = yield* Daemon.Service

    const preview = Effect.fn("DaemonHttpApi.preview")(function* (ctx: { payload: Schema.Schema.Type<typeof DaemonPreviewPayload> }) {
      return yield* daemon.preview({ text: ctx.payload.text }).pipe(
        Effect.map(jsonSafe),
        Effect.catchCause((cause) =>
          Effect.sync(() => {
            const message = Cause.pretty(cause)
            console.error(message)
            return message
          }).pipe(Effect.flatMap((message) => Effect.fail(new HttpApiError.BadRequest({ message })))),
        ),
      )
    })

    const list = Effect.fn("DaemonHttpApi.list")(function* () {
      return yield* daemon.list().pipe(Effect.orDie)
    })

    const get = Effect.fn("DaemonHttpApi.get")(function* (ctx: { params: { runID: string } }) {
      const run = yield* daemon.get(ctx.params.runID).pipe(Effect.orDie)
      if (!run) return yield* new HttpApiError.NotFound({})
      return run
    })

    const events = Effect.fn("DaemonHttpApi.events")(function* (ctx: { params: { runID: string } }) {
      const current = yield* daemon.get(ctx.params.runID).pipe(Effect.orDie)
      if (!current) return yield* new HttpApiError.NotFound({})
      return yield* daemon.events(ctx.params.runID).pipe(Effect.orDie)
    })

    const pause = Effect.fn("DaemonHttpApi.pause")(function* (ctx: { params: { runID: string } }) {
      const run = yield* daemon.pause(ctx.params.runID).pipe(Effect.orDie)
      if (!run) return yield* new HttpApiError.NotFound({})
      return run
    })

    const resume = Effect.fn("DaemonHttpApi.resume")(function* (ctx: { params: { runID: string } }) {
      const run = yield* daemon.resume(ctx.params.runID).pipe(Effect.orDie)
      if (!run) return yield* new HttpApiError.NotFound({})
      return run
    })

    const abort = Effect.fn("DaemonHttpApi.abort")(function* (ctx: { params: { runID: string } }) {
      const run = yield* daemon.abort(ctx.params.runID).pipe(Effect.orDie)
      if (!run) return yield* new HttpApiError.NotFound({})
      return run
    })

    const compact = Effect.fn("DaemonHttpApi.compact")(function* (ctx: { params: { runID: string } }) {
      const run = yield* daemon.compact(ctx.params.runID).pipe(Effect.orDie)
      if (!run) return yield* new HttpApiError.NotFound({})
      return run
    })

    const rotateSession = Effect.fn("DaemonHttpApi.rotateSession")(function* (ctx: { params: { runID: string } }) {
      const run = yield* daemon.rotateSession(ctx.params.runID).pipe(Effect.orDie)
      if (!run) return yield* new HttpApiError.NotFound({})
      return run
    })

    const start = Effect.fn("DaemonHttpApi.start")(function* (ctx: {
      params: { sessionID: string }
      payload: Schema.Schema.Type<typeof DaemonStartPayload>
    }) {
      return yield* daemon
        .start({
          sessionID: SessionID.make(ctx.params.sessionID),
          prompt: ctx.payload,
        })
        .pipe(Effect.catch(() => Effect.fail(new HttpApiError.BadRequest({}))))
    })

    const tasks = Effect.fn("DaemonHttpApi.tasks")(function* (ctx: { params: { runID: string } }) {
      const run = yield* daemon.get(ctx.params.runID).pipe(Effect.orDie)
      if (!run) return yield* new HttpApiError.NotFound({})
      return yield* daemon.tasks(ctx.params.runID).pipe(Effect.orDie)
    })

    const task = Effect.fn("DaemonHttpApi.task")(function* (ctx: { params: { runID: string; taskID: string } }) {
      const current = yield* daemon.task(ctx.params).pipe(Effect.orDie)
      if (!current) return yield* new HttpApiError.NotFound({})
      return current
    })

    const taskPasses = Effect.fn("DaemonHttpApi.taskPasses")(function* (ctx: { params: { runID: string; taskID: string } }) {
      const current = yield* daemon.task(ctx.params).pipe(Effect.orDie)
      if (!current) return yield* new HttpApiError.NotFound({})
      return yield* daemon.taskPasses(ctx.params).pipe(Effect.orDie)
    })

    const taskMemory = Effect.fn("DaemonHttpApi.taskMemory")(function* (ctx: { params: { runID: string; taskID: string } }) {
      const current = yield* daemon.task(ctx.params).pipe(Effect.orDie)
      if (!current) return yield* new HttpApiError.NotFound({})
      return yield* daemon.taskMemory(ctx.params).pipe(Effect.orDie)
    })

    const incubateTask = Effect.fn("DaemonHttpApi.incubateTask")(function* (ctx: {
      params: { runID: string; taskID: string }
      payload: Schema.Schema.Type<typeof DaemonTaskActionPayload>
    }) {
      const current = yield* daemon.incubateTask(ctx.params).pipe(Effect.orDie)
      if (!current) return yield* new HttpApiError.NotFound({})
      return current
    })

    const promoteTask = Effect.fn("DaemonHttpApi.promoteTask")(function* (ctx: {
      params: { runID: string; taskID: string }
      payload: Schema.Schema.Type<typeof DaemonTaskActionPayload>
    }) {
      const current = yield* daemon.promoteTask(ctx.params).pipe(Effect.orDie)
      if (!current) return yield* new HttpApiError.NotFound({})
      return current
    })

    const blockTask = Effect.fn("DaemonHttpApi.blockTask")(function* (ctx: {
      params: { runID: string; taskID: string }
      payload: Schema.Schema.Type<typeof DaemonTaskActionPayload>
    }) {
      const current = yield* daemon.blockTask({ ...ctx.params, reason: ctx.payload.reason }).pipe(Effect.orDie)
      if (!current) return yield* new HttpApiError.NotFound({})
      return current
    })

    const archiveTask = Effect.fn("DaemonHttpApi.archiveTask")(function* (ctx: {
      params: { runID: string; taskID: string }
      payload: Schema.Schema.Type<typeof DaemonTaskActionPayload>
    }) {
      const current = yield* daemon.archiveTask({ ...ctx.params, reason: ctx.payload.reason }).pipe(Effect.orDie)
      if (!current) return yield* new HttpApiError.NotFound({})
      return current
    })

    const incubator = Effect.fn("DaemonHttpApi.incubator")(function* (ctx: { params: { runID: string } }) {
      const current = yield* daemon.incubator(ctx.params.runID).pipe(Effect.orDie)
      if (!current) return yield* new HttpApiError.NotFound({})
      return current
    })

    return handlers
      .handle("preview", preview)
      .handle("list", list)
      .handle("get", get)
      .handle("events", events)
      .handle("pause", pause)
      .handle("resume", resume)
      .handle("abort", abort)
      .handle("compact", compact)
      .handle("rotateSession", rotateSession)
      .handle("start", start)
      .handle("tasks", tasks)
      .handle("task", task)
      .handle("taskPasses", taskPasses)
      .handle("taskMemory", taskMemory)
      .handle("incubateTask", incubateTask)
      .handle("promoteTask", promoteTask)
      .handle("blockTask", blockTask)
      .handle("archiveTask", archiveTask)
      .handle("incubator", incubator)
  }),
)
