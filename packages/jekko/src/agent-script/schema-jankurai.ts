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

// Self-bootstrap policy for repos that may not yet have jankurai configured.
// Consumed by the daemon at run-start and by the `jekko zyal jankurai bootstrap`
// CLI subcommand.
export const ZyalJankuraiBootstrap = Schema.Struct({
  run_update_on_start: Schema.optional(Schema.Boolean),
  ensure_init: Schema.optional(Schema.Boolean),
  ensure_canonical: Schema.optional(Schema.Boolean),
  yes: Schema.optional(Schema.Boolean),
  strict: Schema.optional(Schema.Boolean),
  dry_run: Schema.optional(Schema.Boolean),
})
export type ZyalJankuraiBootstrap = Schema.Schema.Type<typeof ZyalJankuraiBootstrap>

// Worker pool semantics for jankurai forever-runs. Parser/preview only at
// PR2; runtime evaluator lands in the jankurai-runner Rust crate (PR3) and
// the daemon-side worker pool (PR4). Defaults are documented here and applied
// at start by the runtime, not enforced by the schema.
export const ZyalJankuraiPool = Schema.Struct({
  // Concurrent worker slots. Resolved at runtime to
  // `min(size ?? 5, hard_cap ?? 20, jnoccio.spawn_batch_limit)`.
  size: Schema.optional(Schema.Number),
  // Absolute upper bound on slots. Defaults to 20 to match jnoccio.
  hard_cap: Schema.optional(Schema.Number),
  // Worker branch namespace. Default `"zyal"`. Branches end up as
  // `<branch_prefix>/<run_id>/<worker_id>/<finding_id>`.
  branch_prefix: Schema.optional(Schema.String),
  // Branch that worker branches rebase onto + push to. Defaults to
  // `<branch_prefix>/<run_id>/integration`. Main is never touched.
  integration_branch: Schema.optional(Schema.String),
  // Auto-commit verified worker output. Default true.
  commit_on_green: Schema.optional(Schema.Boolean),
})
export type ZyalJankuraiPool = Schema.Schema.Type<typeof ZyalJankuraiPool>

export const ZyalJankuraiReviewerSeverity = Schema.Union([
  Schema.Literal("info"),
  Schema.Literal("warning"),
  Schema.Literal("blocker"),
])
export type ZyalJankuraiReviewerSeverity = Schema.Schema.Type<typeof ZyalJankuraiReviewerSeverity>

export const ZyalJankuraiReviewerChecklistItem = Schema.Struct({
  id: Schema.String,
  prompt: Schema.optional(Schema.String),
  severity: Schema.optional(ZyalJankuraiReviewerSeverity),
})
export type ZyalJankuraiReviewerChecklistItem = Schema.Schema.Type<typeof ZyalJankuraiReviewerChecklistItem>

// Critical reviewer ("what are we missing") pass spec. Lives between
// `prototype` and `promotion_review` in the incubator chain when the new
// `critical_reviewer` pass type is used. Parser/preview only at PR2.
export const ZyalJankuraiReviewer = Schema.Struct({
  enabled: Schema.optional(Schema.Boolean),
  // Block promotion when any checklist item resolves to severity `blocker`.
  // Default true.
  block_promotion: Schema.optional(Schema.Boolean),
  checklist: Schema.optional(Schema.Array(ZyalJankuraiReviewerChecklistItem)),
})
export type ZyalJankuraiReviewer = Schema.Schema.Type<typeof ZyalJankuraiReviewer>

export const ZyalJankurai = Schema.Struct({
  enabled: Schema.optional(Schema.Boolean),
  root: Schema.optional(Schema.String),
  bootstrap: Schema.optional(ZyalJankuraiBootstrap),
  pool: Schema.optional(ZyalJankuraiPool),
  reviewer: Schema.optional(ZyalJankuraiReviewer),
  audit: Schema.optional(ZyalJankuraiAudit),
  repair_plan: Schema.optional(ZyalJankuraiRepairPlan),
  task_source: Schema.optional(ZyalJankuraiTaskSource),
  selection: Schema.optional(ZyalJankuraiSelection),
  regression: Schema.optional(ZyalJankuraiRegression),
  verification: Schema.optional(ZyalJankuraiVerification),
})
export type ZyalJankurai = Schema.Schema.Type<typeof ZyalJankurai>
