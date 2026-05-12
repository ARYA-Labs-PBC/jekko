import { SessionPrompt } from "@/session/prompt"
import { Schema, Struct } from "effect"
import { HttpApi, HttpApiEndpoint, HttpApiError, HttpApiGroup, OpenApi } from "effect/unstable/httpapi"
import { described } from "./metadata"

export const DaemonPreviewPayload = Schema.Struct({
  text: Schema.String,
})

export const DaemonStartPayload = Schema.Struct(Struct.omit(SessionPrompt.PromptInput.fields, ["sessionID"]))
export const DaemonTaskActionPayload = Schema.Struct({
  reason: Schema.optional(Schema.String),
})

const JsonRecord = Schema.Record(Schema.String, Schema.Unknown)

export const DaemonPaths = {
  preview: "/daemon/preview",
  list: "/daemon",
  get: "/daemon/:runID",
  events: "/daemon/:runID/events",
  pause: "/daemon/:runID/pause",
  resume: "/daemon/:runID/resume",
  abort: "/daemon/:runID/abort",
  compact: "/daemon/:runID/compact",
  rotateSession: "/daemon/:runID/rotate-session",
  start: "/session/:sessionID/daemon/start",
  tasks: "/daemon/:runID/tasks",
  task: "/daemon/:runID/tasks/:taskID",
  taskPasses: "/daemon/:runID/tasks/:taskID/passes",
  taskMemory: "/daemon/:runID/tasks/:taskID/memory",
  incubateTask: "/daemon/:runID/tasks/:taskID/incubate",
  promoteTask: "/daemon/:runID/tasks/:taskID/promote",
  blockTask: "/daemon/:runID/tasks/:taskID/block",
  archiveTask: "/daemon/:runID/tasks/:taskID/archive",
  incubator: "/daemon/:runID/incubator",
} as const

export const DaemonApi = HttpApi.make("daemon").add(
  HttpApiGroup.make("daemon")
    .add(
      HttpApiEndpoint.post("preview", DaemonPaths.preview, {
        payload: DaemonPreviewPayload,
        success: described(JsonRecord, "Parsed ZYAL preview"),
        error: HttpApiError.BadRequest,
      }).annotateMerge(
        OpenApi.annotations({
          identifier: "daemon.preview",
          summary: "Preview ZYAL",
          description: "Parse an ZYAL daemon draft and return a preview without starting execution.",
        }),
      ),
      HttpApiEndpoint.get("list", DaemonPaths.list, {
        success: described(Schema.Array(JsonRecord), "List daemon runs"),
      }).annotateMerge(
        OpenApi.annotations({
          identifier: "daemon.list",
          summary: "List daemon runs",
          description: "List daemon runs known to the local runtime.",
        }),
      ),
      HttpApiEndpoint.get("get", DaemonPaths.get, {
        params: { runID: Schema.String },
        success: described(JsonRecord, "Daemon run"),
        error: HttpApiError.NotFound,
      }).annotateMerge(
        OpenApi.annotations({
          identifier: "daemon.get",
          summary: "Get daemon run",
          description: "Get a single daemon run by ID.",
        }),
      ),
      HttpApiEndpoint.get("events", DaemonPaths.events, {
        params: { runID: Schema.String },
        success: described(Schema.Array(JsonRecord), "Daemon run events"),
        error: HttpApiError.NotFound,
      }).annotateMerge(
        OpenApi.annotations({
          identifier: "daemon.events",
          summary: "Get daemon events",
          description: "Get the event stream for a daemon run.",
        }),
      ),
      HttpApiEndpoint.post("pause", DaemonPaths.pause, {
        params: { runID: Schema.String },
        success: described(JsonRecord, "Paused daemon run"),
        error: HttpApiError.NotFound,
      }).annotateMerge(
        OpenApi.annotations({
          identifier: "daemon.pause",
          summary: "Pause daemon run",
          description: "Pause a daemon run.",
        }),
      ),
      HttpApiEndpoint.post("resume", DaemonPaths.resume, {
        params: { runID: Schema.String },
        success: described(JsonRecord, "Resumed daemon run"),
        error: HttpApiError.NotFound,
      }).annotateMerge(
        OpenApi.annotations({
          identifier: "daemon.resume",
          summary: "Resume daemon run",
          description: "Resume a paused daemon run.",
        }),
      ),
      HttpApiEndpoint.post("abort", DaemonPaths.abort, {
        params: { runID: Schema.String },
        success: described(JsonRecord, "Aborted daemon run"),
        error: HttpApiError.NotFound,
      }).annotateMerge(
        OpenApi.annotations({
          identifier: "daemon.abort",
          summary: "Abort daemon run",
          description: "Abort a daemon run.",
        }),
      ),
      HttpApiEndpoint.post("compact", DaemonPaths.compact, {
        params: { runID: Schema.String },
        success: described(JsonRecord, "Compacted daemon run"),
        error: HttpApiError.NotFound,
      }).annotateMerge(
        OpenApi.annotations({
          identifier: "daemon.compact",
          summary: "Compact daemon run",
          description: "Request compaction for a daemon run.",
        }),
      ),
      HttpApiEndpoint.post("rotateSession", DaemonPaths.rotateSession, {
        params: { runID: Schema.String },
        success: described(JsonRecord, "Rotated daemon session"),
        error: HttpApiError.NotFound,
      }).annotateMerge(
        OpenApi.annotations({
          identifier: "daemon.rotate_session",
          summary: "Rotate daemon session",
          description: "Fork the current session and continue the daemon in the child session.",
        }),
      ),
      HttpApiEndpoint.post("start", DaemonPaths.start, {
        params: { sessionID: Schema.String },
        payload: DaemonStartPayload,
        success: described(JsonRecord, "Started daemon run"),
        error: HttpApiError.BadRequest,
      }).annotateMerge(
        OpenApi.annotations({
          identifier: "daemon.start",
          summary: "Start daemon run",
          description: "Parse and start an ZYAL daemon run for the given session.",
        }),
      ),
      HttpApiEndpoint.get("tasks", DaemonPaths.tasks, {
        params: { runID: Schema.String },
        success: described(Schema.Array(JsonRecord), "Daemon tasks"),
        error: HttpApiError.NotFound,
      }),
      HttpApiEndpoint.get("task", DaemonPaths.task, {
        params: { runID: Schema.String, taskID: Schema.String },
        success: described(JsonRecord, "Daemon task"),
        error: HttpApiError.NotFound,
      }),
      HttpApiEndpoint.get("taskPasses", DaemonPaths.taskPasses, {
        params: { runID: Schema.String, taskID: Schema.String },
        success: described(Schema.Array(JsonRecord), "Daemon task passes"),
        error: HttpApiError.NotFound,
      }),
      HttpApiEndpoint.get("taskMemory", DaemonPaths.taskMemory, {
        params: { runID: Schema.String, taskID: Schema.String },
        success: described(Schema.Array(JsonRecord), "Daemon task memory"),
        error: HttpApiError.NotFound,
      }),
      HttpApiEndpoint.post("incubateTask", DaemonPaths.incubateTask, {
        params: { runID: Schema.String, taskID: Schema.String },
        payload: DaemonTaskActionPayload,
        success: described(JsonRecord, "Incubated daemon task"),
        error: HttpApiError.NotFound,
      }),
      HttpApiEndpoint.post("promoteTask", DaemonPaths.promoteTask, {
        params: { runID: Schema.String, taskID: Schema.String },
        payload: DaemonTaskActionPayload,
        success: described(JsonRecord, "Promoted daemon task"),
        error: HttpApiError.NotFound,
      }),
      HttpApiEndpoint.post("blockTask", DaemonPaths.blockTask, {
        params: { runID: Schema.String, taskID: Schema.String },
        payload: DaemonTaskActionPayload,
        success: described(JsonRecord, "Blocked daemon task"),
        error: HttpApiError.NotFound,
      }),
      HttpApiEndpoint.post("archiveTask", DaemonPaths.archiveTask, {
        params: { runID: Schema.String, taskID: Schema.String },
        payload: DaemonTaskActionPayload,
        success: described(JsonRecord, "Archived daemon task"),
        error: HttpApiError.NotFound,
      }),
      HttpApiEndpoint.get("incubator", DaemonPaths.incubator, {
        params: { runID: Schema.String },
        success: described(JsonRecord, "Daemon incubator state"),
        error: HttpApiError.NotFound,
      }),
    )
    .annotateMerge(OpenApi.annotations({ title: "daemon", description: "Daemon runtime routes." })),
)
