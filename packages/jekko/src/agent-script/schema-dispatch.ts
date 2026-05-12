import { Schema } from "effect"

// `dispatch` is a **general** orchestration primitive. A classifier emits a
// lane id for the current work item; the runtime forwards that item to the
// downstream primitive named by `dispatch_to`. Jankurai's finding triage is
// one consumer — others (CI failure routing, issue triage, experiment lane
// selection) are expected to reuse the same block.
//
// Shipped here as parser/preview-only. Runtime evaluator lands in a follow-up
// (alongside example 17). Schema lives in its own file so other consumers can
// grow it without touching consumer-specific schemas.

export const ZyalDispatchOnNoMatch = Schema.Union([
  Schema.Literal("pause"),
  Schema.Literal("abort"),
  Schema.Literal("skip"),
  Schema.Literal("default"),
])
export type ZyalDispatchOnNoMatch = Schema.Schema.Type<typeof ZyalDispatchOnNoMatch>

export const ZyalDispatchLane = Schema.Struct({
  id: Schema.String,
  // Downstream primitive: `fan_out`, `experiments`, `incubator`, `research`,
  // `approvals.gates.<gate_id>`, or a custom pipeline name resolved by the
  // host. Strings — validated lazily at start so consumers can name targets
  // that ship later.
  dispatch_to: Schema.String,
  description: Schema.optional(Schema.String),
})
export type ZyalDispatchLane = Schema.Schema.Type<typeof ZyalDispatchLane>

export const ZyalDispatchClassifier = Schema.Struct({
  // Shell that decides the lane id for the current work item. Stdout must be
  // either a bare lane id on a single line, or JSON `{"lane":"<id>","reason":"<short>"}`.
  command: Schema.optional(Schema.String),
  timeout: Schema.optional(Schema.String),
  // Where to persist the route decision (JSON) for TUI / downstream consumers.
  write_to: Schema.optional(Schema.String),
})
export type ZyalDispatchClassifier = Schema.Schema.Type<typeof ZyalDispatchClassifier>

export const ZyalDispatch = Schema.Struct({
  enabled: Schema.optional(Schema.Boolean),
  classifier: Schema.optional(ZyalDispatchClassifier),
  lanes: Schema.optional(Schema.Array(ZyalDispatchLane)),
  default_lane: Schema.optional(Schema.String),
  on_no_match: Schema.optional(ZyalDispatchOnNoMatch),
})
export type ZyalDispatch = Schema.Schema.Type<typeof ZyalDispatch>
