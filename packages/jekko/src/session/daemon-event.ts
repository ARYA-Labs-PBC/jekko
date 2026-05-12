import { Schema } from "effect"
import { BusEvent } from "@/bus/bus-event"
import { SessionID } from "./schema"

export const DaemonRunStatus = Schema.Union([
  Schema.Literal("created"),
  Schema.Literal("armed"),
  Schema.Literal("running"),
  Schema.Literal("paused"),
  Schema.Literal("blocked"),
  Schema.Literal("satisfied"),
  Schema.Literal("aborted"),
  Schema.Literal("failed"),
])
export type DaemonRunStatus = Schema.Schema.Type<typeof DaemonRunStatus>

export const DaemonPhase = Schema.Union([
  Schema.Literal("created"),
  Schema.Literal("evaluating_stop"),
  Schema.Literal("running_iteration"),
  Schema.Literal("routing_tasks"),
  Schema.Literal("incubating"),
  Schema.Literal("incubator_pass"),
  Schema.Literal("promotion_gate"),
  Schema.Literal("checkpointing"),
  Schema.Literal("compacting"),
  Schema.Literal("sleeping"),
  Schema.Literal("rotating_session"),
  Schema.Literal("paused"),
  Schema.Literal("blocked"),
  Schema.Literal("terminal"),
])
export type DaemonPhase = Schema.Schema.Type<typeof DaemonPhase>

export const Info = Schema.Struct({
  id: Schema.String,
  runID: Schema.String,
  iteration: Schema.Number,
  eventType: Schema.String,
  payload: Schema.Record(Schema.String, Schema.Any),
  timeCreated: Schema.Number,
})

export type Info = Schema.Schema.Type<typeof Info>

export const Event = {
  RunStatus: BusEvent.define("daemon.status", Schema.Struct({
    runID: Schema.String,
    status: DaemonRunStatus,
    phase: DaemonPhase,
  })),
  RunCreated: BusEvent.define("daemon.created", Schema.Struct({
    runID: Schema.String,
    spec: Schema.Any,
  })),
}

export * as DaemonEvent from "./daemon-event"
