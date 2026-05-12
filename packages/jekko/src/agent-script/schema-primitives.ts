import { Schema } from "effect"

export const ZyalSignal = Schema.Union([
  Schema.Literal("assistant_stop"),
  Schema.Literal("max_steps"),
  Schema.Literal("compaction"),
  Schema.Literal("permission_denied"),
  Schema.Literal("checkpoint_failed"),
  Schema.Literal("no_progress"),
  Schema.Literal("tool_calls_done"),
  Schema.Literal("structured_output"),
  Schema.Literal("error"),
  Schema.Literal("cancelled"),
])
export type ZyalSignal = Schema.Schema.Type<typeof ZyalSignal>

export const ZyalShellAssert = Schema.Struct({
  exit_code: Schema.optional(Schema.Number),
  stdout_contains: Schema.optional(Schema.Array(Schema.String)),
  stdout_regex: Schema.optional(Schema.Array(Schema.String)),
  json: Schema.optional(Schema.Record(Schema.String, Schema.Any)),
})

export type ZyalShellAssert = Schema.Schema.Type<typeof ZyalShellAssert>

export const ZyalShellCheck = Schema.Struct({
  command: Schema.String,
  timeout: Schema.optional(Schema.String),
  cwd: Schema.optional(Schema.String),
  assert: Schema.optional(ZyalShellAssert),
})

export type ZyalShellCheck = Schema.Schema.Type<typeof ZyalShellCheck>

export const ZyalGitCleanCheck = Schema.Struct({
  allow_untracked: Schema.optional(Schema.Boolean),
})

export type ZyalGitCleanCheck = Schema.Schema.Type<typeof ZyalGitCleanCheck>

export const ZyalStopCondition = Schema.Union([
  Schema.Struct({
    shell: ZyalShellCheck,
  }),
  Schema.Struct({
    git_clean: ZyalGitCleanCheck,
  }),
])
export type ZyalStopCondition = Schema.Schema.Type<typeof ZyalStopCondition>

export const ZyalJob = Schema.Struct({
  name: Schema.String,
  objective: Schema.String,
  risk: Schema.optional(Schema.Array(Schema.String)),
})

export type ZyalJob = Schema.Schema.Type<typeof ZyalJob>

export const ZyalLoopBreaker = Schema.Struct({
  max_consecutive_errors: Schema.optional(Schema.Number),
  on_trip: Schema.optional(Schema.Union([Schema.Literal("pause"), Schema.Literal("abort"), Schema.Literal("continue")])),
})

export type ZyalLoopBreaker = Schema.Schema.Type<typeof ZyalLoopBreaker>

export const ZyalLoop = Schema.Struct({
  policy: Schema.optional(Schema.Union([Schema.Literal("once"), Schema.Literal("bounded"), Schema.Literal("forever")])),
  sleep: Schema.optional(Schema.String),
  continue_on: Schema.optional(Schema.Array(ZyalSignal)),
  pause_on: Schema.optional(Schema.Array(ZyalSignal)),
  circuit_breaker: Schema.optional(ZyalLoopBreaker),
})

export type ZyalLoop = Schema.Schema.Type<typeof ZyalLoop>

export const ZyalStop = Schema.Struct({
  all: Schema.Array(ZyalStopCondition),
  any: Schema.optional(Schema.Array(ZyalStopCondition)),
})

export type ZyalStop = Schema.Schema.Type<typeof ZyalStop>

export const ZyalContext = Schema.Struct({
  strategy: Schema.optional(Schema.Union([Schema.Literal("soft"), Schema.Literal("hard"), Schema.Literal("hybrid")])),
  compact_every: Schema.optional(Schema.Number),
  hard_clear_every: Schema.optional(Schema.Number),
  preserve: Schema.optional(Schema.Array(Schema.String)),
})

export type ZyalContext = Schema.Schema.Type<typeof ZyalContext>

export const ZyalCheckpoint = Schema.Struct({
  when: Schema.optional(
    Schema.Union([Schema.Literal("after_verified_change"), Schema.Literal("on_error"), Schema.Literal("manual")]),
  ),
  noop_if_clean: Schema.optional(Schema.Boolean),
  verify: Schema.optional(Schema.Array(ZyalShellCheck)),
  git: Schema.optional(
    Schema.Struct({
      add: Schema.Array(Schema.String),
      commit_message: Schema.optional(Schema.String),
      push: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
    }),
  ),
})

export type ZyalCheckpoint = Schema.Schema.Type<typeof ZyalCheckpoint>

export const ZyalTasks = Schema.Struct({
  ledger: Schema.optional(Schema.Literal("sqlite")),
  discover: Schema.optional(Schema.Array(ZyalShellCheck)),
})

export type ZyalTasks = Schema.Schema.Type<typeof ZyalTasks>

export const ZyalIncubatorPassType = Schema.Union([
  Schema.Literal("scout"),
  Schema.Literal("idea"),
  Schema.Literal("strengthen"),
  Schema.Literal("critic"),
  Schema.Literal("synthesize"),
  Schema.Literal("prototype"),
  Schema.Literal("promotion_review"),
  Schema.Literal("compress"),
])
export type ZyalIncubatorPassType = Schema.Schema.Type<typeof ZyalIncubatorPassType>

export const ZyalIncubatorContextMode = Schema.Union([
  Schema.Literal("blind"),
  Schema.Literal("inherit"),
  Schema.Literal("strengthen"),
  Schema.Literal("critic"),
  Schema.Literal("pool"),
  Schema.Literal("promotion"),
  Schema.Literal("ledger_only"),
])
export type ZyalIncubatorContextMode = Schema.Schema.Type<typeof ZyalIncubatorContextMode>

export const ZyalIncubatorWriteScope = Schema.Union([
  Schema.Literal("scratch_only"),
  Schema.Literal("isolated_worktree"),
])
export type ZyalIncubatorWriteScope = Schema.Schema.Type<typeof ZyalIncubatorWriteScope>

export const ZyalIncubatorRouteCondition = Schema.Struct({
  repeated_attempts_gte: Schema.optional(Schema.Number),
  no_progress_iterations_gte: Schema.optional(Schema.Number),
  risk_score_gte: Schema.optional(Schema.Number),
  readiness_score_lt: Schema.optional(Schema.Number),
  touches_paths: Schema.optional(Schema.Array(Schema.String)),
})
export type ZyalIncubatorRouteCondition = Schema.Schema.Type<typeof ZyalIncubatorRouteCondition>

export const ZyalIncubatorRouteWhen = Schema.Struct({
  any: Schema.optional(Schema.Array(ZyalIncubatorRouteCondition)),
  all: Schema.optional(Schema.Array(ZyalIncubatorRouteCondition)),
})
export type ZyalIncubatorRouteWhen = Schema.Schema.Type<typeof ZyalIncubatorRouteWhen>

export const ZyalIncubatorCleanup = Schema.Struct({
  summarize_to_task_memory: Schema.optional(Schema.Boolean),
  archive_artifacts: Schema.optional(Schema.Boolean),
  delete_scratch: Schema.optional(Schema.Boolean),
  delete_unmerged_worktrees: Schema.optional(Schema.Boolean),
})
export type ZyalIncubatorCleanup = Schema.Schema.Type<typeof ZyalIncubatorCleanup>

export const ZyalIncubatorReadiness = Schema.Struct({
  promote_at: Schema.optional(Schema.Number),
  tests_identified_gte: Schema.optional(Schema.Number),
  scope_bounded_gte: Schema.optional(Schema.Number),
  plan_reviewed_gte: Schema.optional(Schema.Number),
  prototype_validated_gte: Schema.optional(Schema.Number),
  rollback_known_gte: Schema.optional(Schema.Number),
  affected_files_known_gte: Schema.optional(Schema.Number),
  critical_objections_resolved_gte: Schema.optional(Schema.Number),
  model_confidence_cap: Schema.optional(Schema.Number),
})
export type ZyalIncubatorReadiness = Schema.Schema.Type<typeof ZyalIncubatorReadiness>

export const ZyalIncubatorBudget = Schema.Struct({
  max_passes_per_task: Schema.Number,
  max_rounds_per_task: Schema.Number,
  max_active_tasks: Schema.optional(Schema.Number),
  max_parallel_idea_passes: Schema.optional(Schema.Number),
})
export type ZyalIncubatorBudget = Schema.Schema.Type<typeof ZyalIncubatorBudget>

export const ZyalIncubatorScratch = Schema.Struct({
  storage: Schema.optional(Schema.Literal("sqlite")),
  mirror: Schema.optional(Schema.Boolean),
  cleanup: Schema.optional(
    Schema.Union([
      Schema.Literal("summarize_and_archive"),
      Schema.Literal("archive"),
      Schema.Literal("keep"),
    ]),
  ),
})
export type ZyalIncubatorScratch = Schema.Schema.Type<typeof ZyalIncubatorScratch>

export const ZyalIncubatorPass = Schema.Struct({
  id: Schema.String,
  type: ZyalIncubatorPassType,
  context: ZyalIncubatorContextMode,
  reads: Schema.optional(Schema.Array(Schema.String)),
  writes: ZyalIncubatorWriteScope,
  count: Schema.optional(Schema.Number),
  agent: Schema.optional(Schema.String),
  mcp_profile: Schema.optional(Schema.String),
})
export type ZyalIncubatorPass = Schema.Schema.Type<typeof ZyalIncubatorPass>

export const ZyalIncubatorPromotion = Schema.Struct({
  promote_at: Schema.Number,
  require: Schema.optional(Schema.Array(Schema.String)),
  block_on: Schema.optional(
    Schema.Struct({
      unresolved_critical_objections_gte: Schema.optional(Schema.Number),
    }),
  ),
  on_promote: Schema.optional(Schema.Literal("move_to_ready_queue")),
  on_exhausted: Schema.optional(Schema.Union([Schema.Literal("park_with_summary"), Schema.Literal("block_with_summary")])),
})
export type ZyalIncubatorPromotion = Schema.Schema.Type<typeof ZyalIncubatorPromotion>

export const ZyalIncubator = Schema.Struct({
  enabled: Schema.Boolean,
  strategy: Schema.optional(Schema.Union([Schema.Literal("generate_pool_strengthen"), Schema.Literal("bounded_passes")])),
  route_when: Schema.optional(ZyalIncubatorRouteWhen),
  exclude_when: Schema.optional(ZyalIncubatorRouteWhen),
  budget: ZyalIncubatorBudget,
  scratch: Schema.optional(ZyalIncubatorScratch),
  cleanup: Schema.optional(ZyalIncubatorCleanup),
  readiness: Schema.optional(ZyalIncubatorReadiness),
  passes: Schema.Array(ZyalIncubatorPass),
  promotion: ZyalIncubatorPromotion,
})
export type ZyalIncubator = Schema.Schema.Type<typeof ZyalIncubator>

export const ZyalWorkerSpec = Schema.Struct({
  id: Schema.String,
  count: Schema.Number,
  agent: Schema.String,
  isolation: Schema.optional(Schema.Union([Schema.Literal("git_worktree"), Schema.Literal("same_session")])),
})

export type ZyalWorkerSpec = Schema.Schema.Type<typeof ZyalWorkerSpec>

export const ZyalAgents = Schema.Struct({
  supervisor: Schema.optional(Schema.Struct({ agent: Schema.String })),
  workers: Schema.optional(Schema.Array(ZyalWorkerSpec)),
})

export type ZyalAgents = Schema.Schema.Type<typeof ZyalAgents>

export const ZyalMcpProfile = Schema.Struct({
  servers: Schema.optional(Schema.Array(Schema.String)),
  tools: Schema.optional(Schema.Array(Schema.String)),
  resources: Schema.optional(Schema.Array(Schema.String)),
})

export type ZyalMcpProfile = Schema.Schema.Type<typeof ZyalMcpProfile>

export const ZyalMcp = Schema.Struct({
  profiles: Schema.optional(Schema.Record(Schema.String, ZyalMcpProfile)),
})

export type ZyalMcp = Schema.Schema.Type<typeof ZyalMcp>

export const ZyalPermissionMode = Schema.Struct({
  read: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
  list: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
  glob: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
  grep: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
  research: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
  websearch: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
  webfetch: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
  external_directory: Schema.optional(
    Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")]),
  ),
  shell: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
  edit: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
  git_commit: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
  git_push: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
  workers: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
  mcp: Schema.optional(Schema.Union([Schema.Literal("ask"), Schema.Literal("allow"), Schema.Literal("deny")])),
})

export type ZyalPermissionMode = Schema.Schema.Type<typeof ZyalPermissionMode>

export const ZyalUi = Schema.Struct({
  theme: Schema.optional(Schema.String),
  banner: Schema.optional(Schema.String),
})

export type ZyalUi = Schema.Schema.Type<typeof ZyalUi>
