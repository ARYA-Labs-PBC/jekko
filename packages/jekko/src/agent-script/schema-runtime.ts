import { Schema } from "effect"
import { ZyalHookStep } from "./schema-routing"

export const ZyalMemoryStore = Schema.Struct({
  scope: Schema.Union([
    Schema.Literal("task"),
    Schema.Literal("run"),
    Schema.Literal("global"),
    Schema.Literal("agent"),
    Schema.Literal("repo"),
  ]),
  retention: Schema.Union([
    Schema.Literal("until_promotion"),
    Schema.Literal("until_archive"),
    Schema.Literal("permanent"),
    Schema.Literal("session"),
  ]),
  max_entries: Schema.optional(Schema.Number),
  compression: Schema.optional(Schema.String),
  write_policy: Schema.optional(Schema.Union([
    Schema.Literal("append_only"),
    Schema.Literal("upsert"),
    Schema.Literal("overwrite"),
  ])),
  read_policy: Schema.optional(Schema.Union([
    Schema.Literal("inject_at_start"),
    Schema.Literal("on_demand"),
    Schema.Literal("search"),
  ])),
  searchable: Schema.optional(Schema.Boolean),
})
export type ZyalMemoryStore = Schema.Schema.Type<typeof ZyalMemoryStore>

export const ZyalMemoryRedaction = Schema.Struct({
  patterns: Schema.Array(Schema.String),
  action: Schema.Union([
    Schema.Literal("mask"),
    Schema.Literal("remove"),
    Schema.Literal("hash"),
  ]),
})
export type ZyalMemoryRedaction = Schema.Schema.Type<typeof ZyalMemoryRedaction>

export const ZyalMemory = Schema.Struct({
  stores: Schema.optional(Schema.Record(Schema.String, ZyalMemoryStore)),
  redaction: Schema.optional(ZyalMemoryRedaction),
  provenance: Schema.optional(Schema.Struct({
    track_source: Schema.optional(Schema.Boolean),
    hash_chain: Schema.optional(Schema.Boolean),
  })),
})
export type ZyalMemory = Schema.Schema.Type<typeof ZyalMemory>

export const ZyalEvidenceRequirement = Schema.Struct({
  type: Schema.String,
  must_pass: Schema.optional(Schema.Boolean),
  must_be_known: Schema.optional(Schema.Boolean),
  must_exist: Schema.optional(Schema.Boolean),
  max_increase: Schema.optional(Schema.Number),
})
export type ZyalEvidenceRequirement = Schema.Schema.Type<typeof ZyalEvidenceRequirement>

export const ZyalEvidence = Schema.Struct({
  require_before_promote: Schema.optional(Schema.Array(ZyalEvidenceRequirement)),
  bundle_format: Schema.optional(Schema.Union([
    Schema.Literal("json"),
    Schema.Literal("markdown"),
  ])),
  sign: Schema.optional(Schema.Union([
    Schema.Literal("sha256"),
    Schema.Literal("none"),
  ])),
  archive: Schema.optional(Schema.Boolean),
})
export type ZyalEvidence = Schema.Schema.Type<typeof ZyalEvidence>

export const ZyalApprovalDecision = Schema.Union([
  Schema.Literal("approve"),
  Schema.Literal("reject"),
  Schema.Literal("edit"),
  Schema.Literal("escalate"),
])
export type ZyalApprovalDecision = Schema.Schema.Type<typeof ZyalApprovalDecision>

export const ZyalApprovalGate = Schema.Struct({
  required_role: Schema.optional(Schema.String),
  timeout: Schema.optional(Schema.String),
  on_timeout: Schema.optional(Schema.Union([
    Schema.Literal("pause"),
    Schema.Literal("abort"),
    Schema.Literal("escalate"),
  ])),
  decisions: Schema.optional(Schema.Array(ZyalApprovalDecision)),
  require_evidence: Schema.optional(Schema.Array(Schema.String)),
  auto_approve_if: Schema.optional(Schema.Struct({
    risk_score_lt: Schema.optional(Schema.Number),
    all_checks_pass: Schema.optional(Schema.Boolean),
  })),
})
export type ZyalApprovalGate = Schema.Schema.Type<typeof ZyalApprovalGate>

export const ZyalApprovalEscalation = Schema.Struct({
  chain: Schema.optional(Schema.Array(Schema.String)),
  auto_escalate_after: Schema.optional(Schema.String),
})
export type ZyalApprovalEscalation = Schema.Schema.Type<typeof ZyalApprovalEscalation>

export const ZyalApprovals = Schema.Struct({
  gates: Schema.optional(Schema.Record(Schema.String, ZyalApprovalGate)),
  escalation: Schema.optional(ZyalApprovalEscalation),
})
export type ZyalApprovals = Schema.Schema.Type<typeof ZyalApprovals>

export const ZyalSkillDefinition = Schema.Struct({
  description: Schema.optional(Schema.String),
  agent: Schema.optional(Schema.String),
  tools: Schema.optional(Schema.Array(Schema.String)),
  mcp_profile: Schema.optional(Schema.String),
  writes: Schema.optional(Schema.Union([
    Schema.Literal("none"),
    Schema.Literal("scratch_only"),
    Schema.Literal("isolated_worktree"),
    Schema.Literal("working_tree"),
  ])),
  trust: Schema.optional(Schema.Union([
    Schema.Literal("builtin"),
    Schema.Literal("verified"),
    Schema.Literal("unverified"),
    Schema.Literal("community"),
    Schema.Literal("local"),
  ])),
  timeout: Schema.optional(Schema.String),
})
export type ZyalSkillDefinition = Schema.Schema.Type<typeof ZyalSkillDefinition>

export const ZyalSkills = Schema.Struct({
  registry: Schema.optional(Schema.Record(Schema.String, ZyalSkillDefinition)),
  allow_creation: Schema.optional(Schema.Boolean),
  max_skills: Schema.optional(Schema.Number),
})
export type ZyalSkills = Schema.Schema.Type<typeof ZyalSkills>

export const ZyalSandboxPathRule = Schema.Struct({
  path: Schema.String,
  access: Schema.Union([
    Schema.Literal("read"),
    Schema.Literal("write"),
    Schema.Literal("deny"),
  ]),
})
export type ZyalSandboxPathRule = Schema.Schema.Type<typeof ZyalSandboxPathRule>

export const ZyalSandboxNetworkPolicy = Schema.Struct({
  outbound: Schema.optional(Schema.Union([
    Schema.Literal("allow"),
    Schema.Literal("deny"),
    Schema.Literal("allowlist"),
  ])),
  allowlist: Schema.optional(Schema.Array(Schema.String)),
})
export type ZyalSandboxNetworkPolicy = Schema.Schema.Type<typeof ZyalSandboxNetworkPolicy>

export const ZyalSandboxResources = Schema.Struct({
  max_file_size: Schema.optional(Schema.String),
  max_total_disk: Schema.optional(Schema.String),
  max_memory: Schema.optional(Schema.String),
  max_processes: Schema.optional(Schema.Number),
})
export type ZyalSandboxResources = Schema.Schema.Type<typeof ZyalSandboxResources>

export const ZyalSandbox = Schema.Struct({
  paths: Schema.optional(Schema.Array(ZyalSandboxPathRule)),
  network: Schema.optional(ZyalSandboxNetworkPolicy),
  resources: Schema.optional(ZyalSandboxResources),
  env_inherit: Schema.optional(Schema.Array(Schema.String)),
  env_deny: Schema.optional(Schema.Array(Schema.String)),
})
export type ZyalSandbox = Schema.Schema.Type<typeof ZyalSandbox>

export const ZyalSecurityTrustZone = Schema.Struct({
  paths: Schema.optional(Schema.Array(Schema.String)),
  require_approval: Schema.optional(Schema.Boolean),
  max_risk_score: Schema.optional(Schema.Number),
})
export type ZyalSecurityTrustZone = Schema.Schema.Type<typeof ZyalSecurityTrustZone>

export const ZyalSecurityInjection = Schema.Struct({
  scan_inputs: Schema.optional(Schema.Boolean),
  scan_outputs: Schema.optional(Schema.Boolean),
  deny_patterns: Schema.optional(Schema.Array(Schema.String)),
  on_detect: Schema.optional(Schema.Union([
    Schema.Literal("abort"),
    Schema.Literal("pause"),
    Schema.Literal("warn"),
    Schema.Literal("strip"),
  ])),
})
export type ZyalSecurityInjection = Schema.Schema.Type<typeof ZyalSecurityInjection>

export const ZyalSecuritySecrets = Schema.Struct({
  allowed_env: Schema.optional(Schema.Array(Schema.String)),
  redact_from_logs: Schema.optional(Schema.Boolean),
  rotate_after: Schema.optional(Schema.String),
})
export type ZyalSecuritySecrets = Schema.Schema.Type<typeof ZyalSecuritySecrets>

export const ZyalSecurity = Schema.Struct({
  trust_zones: Schema.optional(Schema.Record(Schema.String, ZyalSecurityTrustZone)),
  injection: Schema.optional(ZyalSecurityInjection),
  secrets: Schema.optional(ZyalSecuritySecrets),
})
export type ZyalSecurity = Schema.Schema.Type<typeof ZyalSecurity>

export const ZyalObservabilitySpan = Schema.Struct({
  emit: Schema.optional(Schema.Union([
    Schema.Literal("all"),
    Schema.Literal("errors_only"),
    Schema.Literal("none"),
  ])),
  include_tool_calls: Schema.optional(Schema.Boolean),
  include_model_calls: Schema.optional(Schema.Boolean),
})
export type ZyalObservabilitySpan = Schema.Schema.Type<typeof ZyalObservabilitySpan>

export const ZyalObservabilityMetric = Schema.Struct({
  name: Schema.String,
  type: Schema.Union([
    Schema.Literal("counter"),
    Schema.Literal("gauge"),
    Schema.Literal("histogram"),
  ]),
  source: Schema.String,
})
export type ZyalObservabilityMetric = Schema.Schema.Type<typeof ZyalObservabilityMetric>

export const ZyalObservabilityCost = Schema.Struct({
  budget: Schema.optional(Schema.Number),
  currency: Schema.optional(Schema.String),
  alert_at_percent: Schema.optional(Schema.Number),
  on_budget_exceeded: Schema.optional(Schema.Union([
    Schema.Literal("pause"),
    Schema.Literal("abort"),
    Schema.Literal("warn"),
  ])),
})
export type ZyalObservabilityCost = Schema.Schema.Type<typeof ZyalObservabilityCost>

export const ZyalObservabilityReport = Schema.Struct({
  format: Schema.optional(Schema.Union([
    Schema.Literal("json"),
    Schema.Literal("markdown"),
  ])),
  on_complete: Schema.optional(Schema.Boolean),
  on_checkpoint: Schema.optional(Schema.Boolean),
  include: Schema.optional(Schema.Array(Schema.String)),
})
export type ZyalObservabilityReport = Schema.Schema.Type<typeof ZyalObservabilityReport>

export const ZyalObservability = Schema.Struct({
  spans: Schema.optional(ZyalObservabilitySpan),
  metrics: Schema.optional(Schema.Array(ZyalObservabilityMetric)),
  cost: Schema.optional(ZyalObservabilityCost),
  report: Schema.optional(ZyalObservabilityReport),
})
export type ZyalObservability = Schema.Schema.Type<typeof ZyalObservability>
