import { Schema } from "effect"

export const ZyalJankuraiAuditMode = Schema.Union([
  Schema.Literal("advisory"),
  Schema.Literal("guarded"),
  Schema.Literal("standard"),
  Schema.Literal("ratchet"),
  Schema.Literal("release"),
])
export type ZyalJankuraiAuditMode = Schema.Schema.Type<typeof ZyalJankuraiAuditMode>

export const ZyalJankuraiRisk = Schema.Union([
  Schema.Literal("low"),
  Schema.Literal("medium"),
  Schema.Literal("high"),
  Schema.Literal("critical"),
])
export type ZyalJankuraiRisk = Schema.Schema.Type<typeof ZyalJankuraiRisk>

export const ZyalJankuraiTaskSource = Schema.Union([
  Schema.Literal("repair_plan"),
  Schema.Literal("findings"),
  Schema.Literal("agent_fix_queue"),
  Schema.Literal("repair_queue_jsonl"),
])
export type ZyalJankuraiTaskSource = Schema.Schema.Type<typeof ZyalJankuraiTaskSource>

export const ZyalJankuraiSelectionOrder = Schema.Union([
  Schema.Literal("quick_wins_first"),
  Schema.Literal("severity_first"),
  Schema.Literal("random"),
])
export type ZyalJankuraiSelectionOrder = Schema.Schema.Type<typeof ZyalJankuraiSelectionOrder>

export const ZyalJankuraiAuditDelta = Schema.Union([
  Schema.Literal("no_new_findings"),
  Schema.Literal("no_score_drop"),
  Schema.Literal("target_fingerprint_removed"),
  Schema.Literal("none"),
])
export type ZyalJankuraiAuditDelta = Schema.Schema.Type<typeof ZyalJankuraiAuditDelta>

export const ZyalJankuraiAudit = Schema.Struct({
  mode: Schema.optional(ZyalJankuraiAuditMode),
  json: Schema.optional(Schema.String),
  md: Schema.optional(Schema.String),
  repair_queue_jsonl: Schema.optional(Schema.String),
  sarif: Schema.optional(Schema.String),
  no_score_history: Schema.optional(Schema.Boolean),
})
export type ZyalJankuraiAudit = Schema.Schema.Type<typeof ZyalJankuraiAudit>

export const ZyalJankuraiRepairPlan = Schema.Struct({
  enabled: Schema.optional(Schema.Boolean),
  json: Schema.optional(Schema.String),
  md: Schema.optional(Schema.String),
})
export type ZyalJankuraiRepairPlan = Schema.Schema.Type<typeof ZyalJankuraiRepairPlan>

export const ZyalJankuraiSelection = Schema.Struct({
  order: Schema.optional(ZyalJankuraiSelectionOrder),
  randomize_ties: Schema.optional(Schema.Boolean),
  max_risk: Schema.optional(ZyalJankuraiRisk),
  skip_human_review_required: Schema.optional(Schema.Boolean),
  incubate_risk_at: Schema.optional(ZyalJankuraiRisk),
  defer_rules: Schema.optional(Schema.Array(Schema.String)),
  incubate_rules: Schema.optional(Schema.Array(Schema.String)),
})
export type ZyalJankuraiSelection = Schema.Schema.Type<typeof ZyalJankuraiSelection>

export const ZyalJankuraiRegression = Schema.Struct({
  main_ref: Schema.optional(Schema.String),
  compare_every_iterations: Schema.optional(Schema.Number),
  mode: Schema.optional(ZyalJankuraiAuditMode),
  max_new_hard_findings: Schema.optional(Schema.Number),
  max_score_drop: Schema.optional(Schema.Number),
})
export type ZyalJankuraiRegression = Schema.Schema.Type<typeof ZyalJankuraiRegression>

export const ZyalJankuraiVerification = Schema.Struct({
  require_clean_start: Schema.optional(Schema.Boolean),
  require_clean_after_checkpoint: Schema.optional(Schema.Boolean),
  proof_from_test_map: Schema.optional(Schema.Boolean),
  commands: Schema.optional(Schema.Array(Schema.String)),
  audit_delta: Schema.optional(ZyalJankuraiAuditDelta),
  rollback_unverified: Schema.optional(Schema.Boolean),
})
export type ZyalJankuraiVerification = Schema.Schema.Type<typeof ZyalJankuraiVerification>

export const ZyalJankurai = Schema.Struct({
  enabled: Schema.optional(Schema.Boolean),
  root: Schema.optional(Schema.String),
  audit: Schema.optional(ZyalJankuraiAudit),
  repair_plan: Schema.optional(ZyalJankuraiRepairPlan),
  task_source: Schema.optional(ZyalJankuraiTaskSource),
  selection: Schema.optional(ZyalJankuraiSelection),
  regression: Schema.optional(ZyalJankuraiRegression),
  verification: Schema.optional(ZyalJankuraiVerification),
})
export type ZyalJankurai = Schema.Schema.Type<typeof ZyalJankurai>
