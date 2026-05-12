import { Schema } from "effect"
import { zod } from "@/util/effect-zod"
import { withStatics } from "@/util/schema"
import * as Primitives from "./schema-primitives"
import * as Routing from "./schema-routing"
import * as Runtime from "./schema-runtime"
import * as Power from "./schema-power"
import * as Research from "./schema-research"
import * as PreviewBlocks from "./schema-preview-blocks"
import * as Jankurai from "./schema-jankurai"
import { ZYAL_RUNTIME_SENTINEL_VERSION } from "./version"

const DaemonAction = Schema.Literal("daemon")
const ArmAction = Schema.Literal("RUN_FOREVER")

export const ZyalArm = Schema.Struct({
  action: ArmAction,
  id: Schema.String,
})

export type ZyalArm = Schema.Schema.Type<typeof ZyalArm>

export const ZyalSpec = Schema.Struct({
  version: Schema.Literal(ZYAL_RUNTIME_SENTINEL_VERSION),
  intent: DaemonAction,
  confirm: ArmAction,
  id: Schema.String,
  job: Primitives.ZyalJob,
  loop: Schema.optional(Primitives.ZyalLoop),
  stop: Primitives.ZyalStop,
  context: Schema.optional(Primitives.ZyalContext),
  checkpoint: Schema.optional(Primitives.ZyalCheckpoint),
  tasks: Schema.optional(Primitives.ZyalTasks),
  incubator: Schema.optional(Primitives.ZyalIncubator),
  agents: Schema.optional(Primitives.ZyalAgents),
  mcp: Schema.optional(Primitives.ZyalMcp),
  permissions: Schema.optional(Primitives.ZyalPermissionMode),
  ui: Schema.optional(Primitives.ZyalUi),
  on: Schema.optional(Schema.Array(Routing.ZyalOnHandler)),
  fan_out: Schema.optional(Routing.ZyalFanOut),
  guardrails: Schema.optional(Routing.ZyalGuardrails),
  assertions: Schema.optional(Routing.ZyalAssertions),
  retry: Schema.optional(Routing.ZyalRetry),
  hooks: Schema.optional(Routing.ZyalHooks),
  constraints: Schema.optional(Schema.Array(Routing.ZyalConstraint)),
  workflow: Schema.optional(Routing.ZyalWorkflow),
  memory: Schema.optional(Runtime.ZyalMemory),
  evidence: Schema.optional(Runtime.ZyalEvidence),
  approvals: Schema.optional(Runtime.ZyalApprovals),
  skills: Schema.optional(Runtime.ZyalSkills),
  sandbox: Schema.optional(Runtime.ZyalSandbox),
  security: Schema.optional(Runtime.ZyalSecurity),
  observability: Schema.optional(Runtime.ZyalObservability),
  arming: Schema.optional(Power.ZyalArmingPolicy),
  capabilities: Schema.optional(Power.ZyalCapabilities),
  quality: Schema.optional(Power.ZyalQuality),
  experiments: Schema.optional(Power.ZyalExperiments),
  models: Schema.optional(Power.ZyalModels),
  budgets: Schema.optional(Power.ZyalBudgets),
  triggers: Schema.optional(Power.ZyalTriggers),
  rollback: Schema.optional(Power.ZyalRollback),
  done: Schema.optional(Power.ZyalDone),
  repo_intelligence: Schema.optional(Power.ZyalRepoIntelligence),
  fleet: Schema.optional(Power.ZyalFleet),
  research: Schema.optional(Research.ZyalResearch),
  jankurai: Schema.optional(Jankurai.ZyalJankurai),
  taint: Schema.optional(PreviewBlocks.ZyalTaint),
  interaction: Schema.optional(PreviewBlocks.ZyalInteraction),
  interop: Schema.optional(PreviewBlocks.ZyalInterop),
  runtime: Schema.optional(PreviewBlocks.ZyalRuntime),
  capability_negotiation: Schema.optional(PreviewBlocks.ZyalCapabilityNegotiation),
  memory_kernel: Schema.optional(PreviewBlocks.ZyalMemoryKernel),
  evidence_graph: Schema.optional(PreviewBlocks.ZyalEvidenceGraph),
  trust: Schema.optional(PreviewBlocks.ZyalTrustPolicy),
  requirements: Schema.optional(PreviewBlocks.ZyalRequirements),
  evaluation: Schema.optional(PreviewBlocks.ZyalEvaluation),
  release: Schema.optional(PreviewBlocks.ZyalRelease),
  roles: Schema.optional(PreviewBlocks.ZyalRoles),
  channels: Schema.optional(PreviewBlocks.ZyalChannels),
  imports: Schema.optional(PreviewBlocks.ZyalImports),
  reasoning_privacy: Schema.optional(PreviewBlocks.ZyalReasoningPrivacy),
  unsupported_feature_policy: Schema.optional(PreviewBlocks.ZyalUnsupportedFeaturePolicy),
})

export type ZyalSpec = Schema.Schema.Type<typeof ZyalSpec>

export const ZyalScriptSchema = ZyalSpec.pipe(withStatics((schema) => ({ zod: zod(schema) })))
export type ZyalScript = Schema.Schema.Type<typeof ZyalScriptSchema>

export function schemaToEffectError(message: string) {
  return new Error(message)
}
