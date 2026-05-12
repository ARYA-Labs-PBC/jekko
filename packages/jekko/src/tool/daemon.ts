import { Effect, Schema } from "effect"
import * as Tool from "./tool"
import DESCRIPTION from "./daemon.txt"
import { DaemonStore } from "@/session/daemon-store"

const ConfidenceParams = Schema.Struct({
  implementation_confidence: Schema.Number,
  verification_confidence: Schema.Number,
  summary: Schema.optional(Schema.String),
})

const IdeaParams = Schema.Struct({
  title: Schema.String,
  summary: Schema.String,
  payload: Schema.optional(Schema.Record(Schema.String, Schema.Unknown)),
})

const PassResultParams = Schema.Struct({
  summary: Schema.String,
  claims: Schema.optional(Schema.Array(Schema.Record(Schema.String, Schema.Unknown))),
  evidence: Schema.optional(Schema.Array(Schema.Record(Schema.String, Schema.Unknown))),
  uncertainty: Schema.optional(Schema.Array(Schema.Record(Schema.String, Schema.Unknown))),
  blockers: Schema.optional(Schema.Array(Schema.String)),
  recommended_next: Schema.optional(Schema.String),
  readiness_delta: Schema.optional(Schema.Number),
})

const MemoryWriteParams = Schema.Struct({
  kind: Schema.String,
  title: Schema.String,
  summary: Schema.String,
  payload: Schema.optional(Schema.Record(Schema.String, Schema.Unknown)),
  importance: Schema.optional(Schema.Number),
  confidence: Schema.optional(Schema.Number),
})

const MemoryReadParams = Schema.Struct({
  kind: Schema.optional(Schema.String),
})

const RequestParams = Schema.Struct({
  reason: Schema.String,
  evidence: Schema.optional(Schema.Record(Schema.String, Schema.Unknown)),
})

const EmptyParams = Schema.Struct({})

type ActivePass = {
  run: DaemonStore.RunInfo
  task: DaemonStore.TaskInfo
  pass: DaemonStore.TaskPassInfo
}

export const DaemonReportConfidenceTool = Tool.define(
  "daemon_report_confidence",
  Effect.gen(function* () {
    const store = yield* DaemonStore.Service
    return daemonDef(ConfidenceParams, (params, ctx) =>
      Effect.gen(function* () {
        const active = yield* requireActivePass(store, ctx.sessionID)
        yield* store.assessTask({
          taskID: active.task.id,
          implementationConfidence: params.implementation_confidence,
          verificationConfidence: params.verification_confidence,
          assessment: { source: "daemon_report_confidence", passID: active.pass.id, summary: params.summary },
        })
        yield* store.appendTaskMemory({
          runID: active.run.id,
          taskID: active.task.id,
          kind: "confidence_report",
          title: "Model confidence report",
          summary: params.summary ?? "Confidence reported by incubator pass.",
          payload: params,
          sourcePassID: active.pass.id,
          importance: 0.25,
          confidence: 0.4,
        })
        return ok("confidence recorded")
      }),
    )
  }),
)

export const DaemonEmitIdeaTool = Tool.define(
  "daemon_emit_idea",
  Effect.gen(function* () {
    const store = yield* DaemonStore.Service
    return daemonDef(IdeaParams, (params, ctx) =>
      Effect.gen(function* () {
        const active = yield* requireActivePass(store, ctx.sessionID)
        yield* store.appendTaskMemory({
          runID: active.run.id,
          taskID: active.task.id,
          kind: "idea",
          title: params.title,
          summary: params.summary,
          payload: params.payload,
          sourcePassID: active.pass.id,
          importance: 0.6,
          confidence: 0.55,
        })
        return ok("idea recorded")
      }),
    )
  }),
)

export const DaemonReportPassResultTool = Tool.define(
  "daemon_report_pass_result",
  Effect.gen(function* () {
    const store = yield* DaemonStore.Service
    return daemonDef(PassResultParams, (params, ctx) =>
      Effect.gen(function* () {
        const active = yield* requireActivePass(store, ctx.sessionID)
        yield* store.appendTaskMemory({
          runID: active.run.id,
          taskID: active.task.id,
          kind: "pass_receipt",
          title: `${active.pass.pass_type} pass receipt`,
          summary: params.summary,
          payload: params,
          sourcePassID: active.pass.id,
          importance: 0.7,
          confidence: 0.6,
        })
        return ok("pass result recorded")
      }),
    )
  }),
)

export const DaemonMemoryWriteTool = Tool.define(
  "daemon_memory_write",
  Effect.gen(function* () {
    const store = yield* DaemonStore.Service
    return daemonDef(MemoryWriteParams, (params, ctx) =>
      Effect.gen(function* () {
        const active = yield* requireActivePass(store, ctx.sessionID)
        yield* store.appendTaskMemory({
          runID: active.run.id,
          taskID: active.task.id,
          kind: params.kind,
          title: params.title,
          summary: params.summary,
          payload: params.payload,
          sourcePassID: active.pass.id,
          importance: params.importance,
          confidence: params.confidence,
        })
        return ok("memory written")
      }),
    )
  }),
)

export const DaemonMemoryReadTool = Tool.define(
  "daemon_memory_read",
  Effect.gen(function* () {
    const store = yield* DaemonStore.Service
    return daemonDef(MemoryReadParams, (params, ctx) =>
      Effect.gen(function* () {
        const active = yield* requireActivePass(store, ctx.sessionID)
        const memories = yield* store.listTaskMemory({ runID: active.run.id, taskID: active.task.id })
        const filtered = params.kind ? memories.filter((item) => item.kind === params.kind) : memories
        return {
          title: "daemon memory",
          metadata: { count: filtered.length },
          output: JSON.stringify(filtered, null, 2),
        }
      }),
    )
  }),
)

export const DaemonRequestIncubationTool = Tool.define(
  "daemon_request_incubation",
  Effect.gen(function* () {
    const store = yield* DaemonStore.Service
    return daemonDef(RequestParams, (params, ctx) =>
      Effect.gen(function* () {
        const active = yield* requireActivePass(store, ctx.sessionID)
        yield* store.appendTaskMemory({
          runID: active.run.id,
          taskID: active.task.id,
          kind: "incubation_request",
          title: "Incubation requested",
          summary: params.reason,
          payload: params.evidence,
          sourcePassID: active.pass.id,
          importance: 0.5,
          confidence: 0.5,
        })
        return ok("incubation request recorded")
      }),
    )
  }),
)

export const DaemonRequestPromotionTool = Tool.define(
  "daemon_request_promotion",
  Effect.gen(function* () {
    const store = yield* DaemonStore.Service
    return daemonDef(RequestParams, (params, ctx) =>
      Effect.gen(function* () {
        const active = yield* requireActivePass(store, ctx.sessionID)
        yield* store.appendTaskMemory({
          runID: active.run.id,
          taskID: active.task.id,
          kind: "promotion_request",
          title: "Promotion requested",
          summary: params.reason,
          payload: params.evidence,
          sourcePassID: active.pass.id,
          importance: 0.6,
          confidence: 0.5,
        })
        return ok("promotion request recorded; host gate still decides")
      }),
    )
  }),
)

export const DaemonContextSnapshotTool = Tool.define(
  "daemon_context_snapshot",
  Effect.gen(function* () {
    const store = yield* DaemonStore.Service
    return daemonDef(EmptyParams, (_params, ctx) =>
      Effect.gen(function* () {
        const active = yield* requireActivePass(store, ctx.sessionID)
        const memories = yield* store.listTaskMemory({ runID: active.run.id, taskID: active.task.id })
        const passes = yield* store.listTaskPasses({ runID: active.run.id, taskID: active.task.id })
        return {
          title: "daemon context",
          metadata: { runID: active.run.id, taskID: active.task.id, passID: active.pass.id },
          output: JSON.stringify({ run: active.run, task: active.task, pass: active.pass, memories, passes }, null, 2),
        }
      }),
    )
  }),
)

export const DaemonTools = [
  DaemonReportConfidenceTool,
  DaemonEmitIdeaTool,
  DaemonReportPassResultTool,
  DaemonMemoryWriteTool,
  DaemonMemoryReadTool,
  DaemonRequestIncubationTool,
  DaemonRequestPromotionTool,
  DaemonContextSnapshotTool,
] as const

function daemonDef<P extends Schema.Decoder<unknown>>(
  parameters: P,
  execute: Tool.DefWithoutID<P>["execute"],
): Tool.DefWithoutID<P> {
  return {
    description: DESCRIPTION,
    enabledByDefault: false,
    parameters,
    execute,
  }
}

function requireActivePass(store: DaemonStore.Interface, sessionID: string): Effect.Effect<ActivePass, never, never> {
  return Effect.gen(function* () {
    const active = yield* store.findDaemonContextBySession(sessionID)
    if (!active?.task || !active.pass) throw new Error("daemon tool used outside an active pass")
    return {
      run: active.run,
      task: active.task,
      pass: active.pass,
    }
  }) as Effect.Effect<ActivePass, never, never>
}

function ok(output: string): Tool.ExecuteResult {
  return {
    title: "",
    output,
    metadata: {},
  }
}
