import { Schema } from "effect"

export const ZyalInteractionUser = Schema.Union([
  Schema.Literal("none"),
  Schema.Literal("async"),
  Schema.Literal("present"),
])
export type ZyalInteractionUser = Schema.Schema.Type<typeof ZyalInteractionUser>

export const ZyalInteractionOnAmbiguity = Schema.Union([
  Schema.Literal("best_effort"),
  Schema.Literal("pause"),
  Schema.Literal("skip"),
])
export type ZyalInteractionOnAmbiguity = Schema.Schema.Type<typeof ZyalInteractionOnAmbiguity>

export const ZyalInteractionOnBlocked = Schema.Union([
  Schema.Literal("skip_and_next"),
  Schema.Literal("pause"),
  Schema.Literal("fail"),
])
export type ZyalInteractionOnBlocked = Schema.Schema.Type<typeof ZyalInteractionOnBlocked>

export const ZyalInteraction = Schema.Struct({
  user: Schema.optional(ZyalInteractionUser),
  on_ambiguity: Schema.optional(ZyalInteractionOnAmbiguity),
  on_blocked: Schema.optional(ZyalInteractionOnBlocked),
  system_inject: Schema.optional(Schema.String),
})
export type ZyalInteraction = Schema.Schema.Type<typeof ZyalInteraction>

export const ZyalTaintRank = Schema.Union([
  Schema.Literal("high"),
  Schema.Literal("medium"),
  Schema.Literal("untrusted"),
  Schema.Literal("untrusted_for_arming"),
  Schema.Literal("hostile"),
])
export type ZyalTaintRank = Schema.Schema.Type<typeof ZyalTaintRank>

export const ZyalTaintLabel = Schema.Struct({
  rank: ZyalTaintRank,
  notes: Schema.optional(Schema.String),
})
export type ZyalTaintLabel = Schema.Schema.Type<typeof ZyalTaintLabel>

export const ZyalTaintForbidAction = Schema.Union([
  Schema.Literal("arm"),
  Schema.Literal("approve"),
  Schema.Literal("grant_capability"),
  Schema.Literal("write_memory_procedural"),
  Schema.Literal("write_memory_semantic"),
  Schema.Literal("exec_shell"),
  Schema.Literal("install_skill"),
  Schema.Literal("modify_objective"),
  Schema.Literal("expose_secret"),
])
export type ZyalTaintForbidAction = Schema.Schema.Type<typeof ZyalTaintForbidAction>

export const ZyalTaintForbidRule = Schema.Struct({
  from: Schema.Array(Schema.String),
  cannot: Schema.Array(ZyalTaintForbidAction),
  unless: Schema.optional(Schema.Array(Schema.String)),
})
export type ZyalTaintForbidRule = Schema.Schema.Type<typeof ZyalTaintForbidRule>

export const ZyalTaintInjectionAction = Schema.Union([
  Schema.Literal("strip"),
  Schema.Literal("quote"),
  Schema.Literal("block"),
  Schema.Literal("pause"),
])
export type ZyalTaintInjectionAction = Schema.Schema.Type<typeof ZyalTaintInjectionAction>

export const ZyalTaintPromptInjection = Schema.Struct({
  detect_patterns: Schema.Array(Schema.String),
  on_detect: ZyalTaintInjectionAction,
  scan_sources: Schema.optional(Schema.Array(Schema.String)),
})
export type ZyalTaintPromptInjection = Schema.Schema.Type<typeof ZyalTaintPromptInjection>

export const ZyalTaint = Schema.Struct({
  default_label: Schema.optional(Schema.String),
  labels: Schema.Record(Schema.String, ZyalTaintLabel),
  forbid: Schema.optional(Schema.Array(ZyalTaintForbidRule)),
  prompt_injection: Schema.optional(ZyalTaintPromptInjection),
})
export type ZyalTaint = Schema.Schema.Type<typeof ZyalTaint>

export const ZyalInteropProtocol = Schema.Struct({
  name: Schema.String,
  target: Schema.optional(Schema.String),
  version: Schema.optional(Schema.String),
  notes: Schema.optional(Schema.String),
})
export type ZyalInteropProtocol = Schema.Schema.Type<typeof ZyalInteropProtocol>

export const ZyalInterop = Schema.Struct({
  protocols: Schema.optional(Schema.Array(ZyalInteropProtocol)),
  adapters: Schema.optional(Schema.Array(Schema.String)),
  compile_to: Schema.optional(Schema.Array(Schema.String)),
  notes: Schema.optional(Schema.String),
})
export type ZyalInterop = Schema.Schema.Type<typeof ZyalInterop>

export const ZyalRuntimeResources = Schema.Struct({
  cpu: Schema.optional(Schema.String),
  memory: Schema.optional(Schema.String),
  disk: Schema.optional(Schema.String),
  processes: Schema.optional(Schema.Number),
})
export type ZyalRuntimeResources = Schema.Schema.Type<typeof ZyalRuntimeResources>

export const ZyalRuntime = Schema.Struct({
  mode: Schema.optional(Schema.Union([Schema.Literal("preview"), Schema.Literal("host_enforced")])),
  image: Schema.optional(Schema.String),
  workspace: Schema.optional(Schema.String),
  network: Schema.optional(Schema.Union([Schema.Literal("allow"), Schema.Literal("deny"), Schema.Literal("allowlist")])),
  env: Schema.optional(Schema.Array(Schema.String)),
  resources: Schema.optional(ZyalRuntimeResources),
})
export type ZyalRuntime = Schema.Schema.Type<typeof ZyalRuntime>

export const ZyalCapabilityNegotiation = Schema.Struct({
  host: Schema.optional(Schema.String),
  required: Schema.optional(Schema.Array(Schema.String)),
  optional: Schema.optional(Schema.Array(Schema.String)),
  fail_closed: Schema.optional(Schema.Boolean),
  degrade_to: Schema.optional(Schema.String),
})
export type ZyalCapabilityNegotiation = Schema.Schema.Type<typeof ZyalCapabilityNegotiation>

export const ZyalMemoryKernelStore = Schema.Struct({
  scope: Schema.String,
  retention: Schema.String,
  searchable: Schema.optional(Schema.Boolean),
})
export type ZyalMemoryKernelStore = Schema.Schema.Type<typeof ZyalMemoryKernelStore>

export const ZyalMemoryKernelRedaction = Schema.Struct({
  patterns: Schema.Array(Schema.String),
  action: Schema.Union([
    Schema.Literal("mask"),
    Schema.Literal("remove"),
    Schema.Literal("hash"),
  ]),
})
export type ZyalMemoryKernelRedaction = Schema.Schema.Type<typeof ZyalMemoryKernelRedaction>

export const ZyalMemoryKernel = Schema.Struct({
  stores: Schema.optional(Schema.Record(Schema.String, ZyalMemoryKernelStore)),
  redaction: Schema.optional(ZyalMemoryKernelRedaction),
  provenance: Schema.optional(Schema.Struct({
    track_source: Schema.optional(Schema.Boolean),
    hash_chain: Schema.optional(Schema.Boolean),
  })),
})
export type ZyalMemoryKernel = Schema.Schema.Type<typeof ZyalMemoryKernel>

export const ZyalEvidenceGraphNode = Schema.Struct({
  type: Schema.String,
  required: Schema.optional(Schema.Boolean),
})
export type ZyalEvidenceGraphNode = Schema.Schema.Type<typeof ZyalEvidenceGraphNode>

export const ZyalEvidenceGraphEdge = Schema.Struct({
  from: Schema.String,
  to: Schema.String,
  kind: Schema.optional(Schema.String),
})
export type ZyalEvidenceGraphEdge = Schema.Schema.Type<typeof ZyalEvidenceGraphEdge>

export const ZyalEvidenceGraph = Schema.Struct({
  nodes: Schema.optional(Schema.Record(Schema.String, ZyalEvidenceGraphNode)),
  edges: Schema.optional(Schema.Array(ZyalEvidenceGraphEdge)),
  merge_witness: Schema.optional(Schema.String),
})
export type ZyalEvidenceGraph = Schema.Schema.Type<typeof ZyalEvidenceGraph>

export const ZyalTrustZonePolicy = Schema.Struct({
  paths: Schema.optional(Schema.Array(Schema.String)),
  taint: Schema.optional(Schema.Union([
    Schema.Literal("clean"),
    Schema.Literal("tainted"),
    Schema.Literal("quarantined"),
  ])),
  require_approval: Schema.optional(Schema.Boolean),
})
export type ZyalTrustZonePolicy = Schema.Schema.Type<typeof ZyalTrustZonePolicy>

export const ZyalTrustPolicy = Schema.Struct({
  zones: Schema.optional(Schema.Record(Schema.String, ZyalTrustZonePolicy)),
  on_taint: Schema.optional(Schema.Union([
    Schema.Literal("abort"),
    Schema.Literal("pause"),
    Schema.Literal("warn"),
  ])),
  notes: Schema.optional(Schema.String),
})
export type ZyalTrustPolicy = Schema.Schema.Type<typeof ZyalTrustPolicy>

export const ZyalRequirements = Schema.Struct({
  must: Schema.optional(Schema.Array(Schema.String)),
  should: Schema.optional(Schema.Array(Schema.String)),
  avoid: Schema.optional(Schema.Array(Schema.String)),
})
export type ZyalRequirements = Schema.Schema.Type<typeof ZyalRequirements>

export const ZyalEvaluationMetric = Schema.Struct({
  name: Schema.String,
  command: Schema.optional(Schema.String),
  threshold: Schema.optional(Schema.Number),
})
export type ZyalEvaluationMetric = Schema.Schema.Type<typeof ZyalEvaluationMetric>

export const ZyalEvaluation = Schema.Struct({
  metrics: Schema.optional(Schema.Array(ZyalEvaluationMetric)),
  compare: Schema.optional(Schema.String),
})
export type ZyalEvaluation = Schema.Schema.Type<typeof ZyalEvaluation>

export const ZyalRelease = Schema.Struct({
  channel: Schema.optional(Schema.String),
  version: Schema.optional(Schema.String),
  gates: Schema.optional(Schema.Array(Schema.String)),
  notes: Schema.optional(Schema.String),
})
export type ZyalRelease = Schema.Schema.Type<typeof ZyalRelease>

export const ZyalRole = Schema.Struct({
  id: Schema.String,
  agent: Schema.optional(Schema.String),
  permissions: Schema.optional(Schema.Array(Schema.String)),
  description: Schema.optional(Schema.String),
})
export type ZyalRole = Schema.Schema.Type<typeof ZyalRole>

export const ZyalRoles = Schema.Struct({
  list: Schema.optional(Schema.Array(ZyalRole)),
})
export type ZyalRoles = Schema.Schema.Type<typeof ZyalRoles>

export const ZyalChannel = Schema.Struct({
  id: Schema.String,
  kind: Schema.optional(Schema.String),
  route: Schema.optional(Schema.String),
  approval: Schema.optional(Schema.String),
})
export type ZyalChannel = Schema.Schema.Type<typeof ZyalChannel>

export const ZyalChannels = Schema.Struct({
  list: Schema.optional(Schema.Array(ZyalChannel)),
})
export type ZyalChannels = Schema.Schema.Type<typeof ZyalChannels>

export const ZyalImportSource = Schema.Struct({
  source: Schema.String,
  optional: Schema.optional(Schema.Boolean),
  pin: Schema.optional(Schema.String),
})
export type ZyalImportSource = Schema.Schema.Type<typeof ZyalImportSource>

export const ZyalImports = Schema.Struct({
  list: Schema.optional(Schema.Array(ZyalImportSource)),
})
export type ZyalImports = Schema.Schema.Type<typeof ZyalImports>

export const ZyalReasoningPrivacy = Schema.Struct({
  store_reasoning: Schema.optional(Schema.Boolean),
  redact_chain_of_thought: Schema.optional(Schema.Boolean),
  summaries_only: Schema.optional(Schema.Boolean),
})
export type ZyalReasoningPrivacy = Schema.Schema.Type<typeof ZyalReasoningPrivacy>

export const ZyalUnsupportedFeaturePolicy = Schema.Struct({
  required: Schema.optional(Schema.Array(Schema.String)),
  optional: Schema.optional(Schema.Array(Schema.String)),
  fail_closed: Schema.optional(Schema.Boolean),
  on_missing: Schema.optional(Schema.Union([
    Schema.Literal("reject"),
    Schema.Literal("warn"),
    Schema.Literal("degrade"),
  ])),
})
export type ZyalUnsupportedFeaturePolicy = Schema.Schema.Type<typeof ZyalUnsupportedFeaturePolicy>
