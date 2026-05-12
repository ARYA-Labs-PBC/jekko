import { Schema } from "effect"
import {
  ZyalShellAssert,
  ZyalShellCheck,
  ZyalSignal,
} from "./schema-primitives"

export const ZyalOnAction = Schema.Union([
  Schema.Struct({ switch_agent: Schema.String }),
  Schema.Struct({ run: Schema.String }),
  Schema.Struct({ incubate_current_task: Schema.Literal(true) }),
  Schema.Struct({ checkpoint: Schema.Literal(true) }),
  Schema.Struct({ pause: Schema.Literal(true) }),
  Schema.Struct({ abort: Schema.Literal(true) }),
  Schema.Struct({ notify: Schema.String }),
  Schema.Struct({ set_context: Schema.Record(Schema.String, Schema.Unknown) }),
])
export type ZyalOnAction = Schema.Schema.Type<typeof ZyalOnAction>

export const ZyalOnHandler = Schema.Struct({
  signal: ZyalSignal,
  count_gte: Schema.optional(Schema.Number),
  message_contains: Schema.optional(Schema.String),
  if: Schema.optional(ZyalShellCheck),
  do: Schema.Array(ZyalOnAction),
})
export type ZyalOnHandler = Schema.Schema.Type<typeof ZyalOnHandler>

export const ZyalFanOutSplit = Schema.Union([
  Schema.Struct({ shell: Schema.String }),
  Schema.Struct({ items: Schema.Array(Schema.String) }),
])
export type ZyalFanOutSplit = Schema.Schema.Type<typeof ZyalFanOutSplit>

export const ZyalFanOutReduceStrategy = Schema.Union([
  Schema.Literal("merge_all"),
  Schema.Literal("best_score"),
  Schema.Literal("vote"),
  Schema.Literal("custom_shell"),
])
export type ZyalFanOutReduceStrategy = Schema.Schema.Type<typeof ZyalFanOutReduceStrategy>

export const ZyalFanOutReduce = Schema.Struct({
  strategy: ZyalFanOutReduceStrategy,
  score_key: Schema.optional(Schema.String),
  command: Schema.optional(Schema.String),
})
export type ZyalFanOutReduce = Schema.Schema.Type<typeof ZyalFanOutReduce>

export const ZyalFanOut = Schema.Struct({
  strategy: Schema.optional(Schema.Union([
    Schema.Literal("map_reduce"),
    Schema.Literal("scatter_gather"),
  ])),
  split: ZyalFanOutSplit,
  worker: Schema.Struct({
    agent: Schema.optional(Schema.String),
    isolation: Schema.optional(Schema.Union([
      Schema.Literal("git_worktree"),
      Schema.Literal("same_session"),
    ])),
    timeout: Schema.optional(Schema.String),
    max_parallel: Schema.optional(Schema.Number),
  }),
  reduce: ZyalFanOutReduce,
  on_partial_failure: Schema.optional(Schema.Union([
    Schema.Literal("continue"),
    Schema.Literal("abort"),
    Schema.Literal("pause"),
  ])),
})
export type ZyalFanOut = Schema.Schema.Type<typeof ZyalFanOut>

export const ZyalGuardrailAction = Schema.Union([
  Schema.Literal("block"),
  Schema.Literal("retry"),
  Schema.Literal("pause"),
  Schema.Literal("abort"),
  Schema.Literal("warn"),
])
export type ZyalGuardrailAction = Schema.Schema.Type<typeof ZyalGuardrailAction>

export const ZyalPatternGuardrail = Schema.Struct({
  name: Schema.String,
  deny_patterns: Schema.Array(Schema.String),
  scope: Schema.optional(Schema.Union([
    Schema.Literal("tool_input"),
    Schema.Literal("tool_output"),
    Schema.Literal("file_diff"),
    Schema.Literal("commit_message"),
  ])),
  action: ZyalGuardrailAction,
})
export type ZyalPatternGuardrail = Schema.Schema.Type<typeof ZyalPatternGuardrail>

export const ZyalShellGuardrail = Schema.Struct({
  name: Schema.String,
  shell: Schema.String,
  assert: Schema.optional(ZyalShellAssert),
  on_fail: Schema.optional(ZyalGuardrailAction),
  max_retries: Schema.optional(Schema.Number),
})
export type ZyalShellGuardrail = Schema.Schema.Type<typeof ZyalShellGuardrail>

export const ZyalGuardrails = Schema.Struct({
  input: Schema.optional(Schema.Array(ZyalPatternGuardrail)),
  output: Schema.optional(Schema.Array(Schema.Union([ZyalPatternGuardrail, ZyalShellGuardrail]))),
  iteration: Schema.optional(Schema.Array(ZyalShellGuardrail)),
})
export type ZyalGuardrails = Schema.Schema.Type<typeof ZyalGuardrails>

export const ZyalAssertions = Schema.Struct({
  require_structured_output: Schema.optional(Schema.Boolean),
  schema: Schema.optional(Schema.Record(Schema.String, Schema.Unknown)),
  on_invalid: Schema.optional(Schema.Union([
    Schema.Literal("retry"),
    Schema.Literal("pause"),
    Schema.Literal("abort"),
    Schema.Literal("warn"),
  ])),
  max_retries: Schema.optional(Schema.Number),
})
export type ZyalAssertions = Schema.Schema.Type<typeof ZyalAssertions>

export const ZyalRetryBackoff = Schema.Union([
  Schema.Literal("none"),
  Schema.Literal("linear"),
  Schema.Literal("exponential"),
])
export type ZyalRetryBackoff = Schema.Schema.Type<typeof ZyalRetryBackoff>

export const ZyalRetryPolicy = Schema.Struct({
  max_attempts: Schema.optional(Schema.Number),
  backoff: Schema.optional(ZyalRetryBackoff),
  initial_delay: Schema.optional(Schema.String),
  max_delay: Schema.optional(Schema.String),
  jitter: Schema.optional(Schema.Boolean),
})
export type ZyalRetryPolicy = Schema.Schema.Type<typeof ZyalRetryPolicy>

export const ZyalRetry = Schema.Struct({
  default: Schema.optional(ZyalRetryPolicy),
  overrides: Schema.optional(Schema.Struct({
    shell_checks: Schema.optional(ZyalRetryPolicy),
    checkpoint: Schema.optional(ZyalRetryPolicy),
    worker_spawn: Schema.optional(ZyalRetryPolicy),
    stop_evaluation: Schema.optional(ZyalRetryPolicy),
  })),
})
export type ZyalRetry = Schema.Schema.Type<typeof ZyalRetry>

export const ZyalHookStep = Schema.Struct({
  run: Schema.String,
  assert: Schema.optional(ZyalShellAssert),
  on_fail: Schema.optional(Schema.Union([
    Schema.Literal("pause"),
    Schema.Literal("abort"),
    Schema.Literal("warn"),
    Schema.Literal("block_promotion"),
    Schema.Literal("continue"),
  ])),
  timeout: Schema.optional(Schema.String),
})
export type ZyalHookStep = Schema.Schema.Type<typeof ZyalHookStep>

export const ZyalHooks = Schema.Struct({
  on_start: Schema.optional(Schema.Array(ZyalHookStep)),
  before_iteration: Schema.optional(Schema.Array(ZyalHookStep)),
  after_iteration: Schema.optional(Schema.Array(ZyalHookStep)),
  before_checkpoint: Schema.optional(Schema.Array(ZyalHookStep)),
  after_checkpoint: Schema.optional(Schema.Array(ZyalHookStep)),
  on_promote: Schema.optional(Schema.Array(ZyalHookStep)),
  on_exhaust: Schema.optional(Schema.Array(ZyalHookStep)),
  on_stop: Schema.optional(Schema.Array(ZyalHookStep)),
})
export type ZyalHooks = Schema.Schema.Type<typeof ZyalHooks>

export const ZyalConstraintInvariant = Schema.Union([
  Schema.Literal("gte_baseline"),
  Schema.Literal("lte_baseline"),
  Schema.Literal("equals_baseline"),
  Schema.Literal("equals_zero"),
  Schema.Literal("non_zero"),
])
export type ZyalConstraintInvariant = Schema.Schema.Type<typeof ZyalConstraintInvariant>

export const ZyalConstraint = Schema.Struct({
  name: Schema.String,
  check: Schema.Struct({
    shell: Schema.String,
    timeout: Schema.optional(Schema.String),
  }),
  baseline: Schema.optional(Schema.Union([
    Schema.Literal("capture_on_start"),
    Schema.Literal("capture_on_checkpoint"),
  ])),
  invariant: ZyalConstraintInvariant,
  on_violation: Schema.optional(Schema.Union([
    Schema.Literal("abort"),
    Schema.Literal("pause"),
    Schema.Literal("block"),
    Schema.Literal("warn"),
    Schema.Literal("retry"),
  ])),
})
export type ZyalConstraint = Schema.Schema.Type<typeof ZyalConstraint>

export const ZyalWorkflowTransitionCondition = Schema.Struct({
  evidence_exists: Schema.optional(Schema.String),
  risk_score_gte: Schema.optional(Schema.Number),
  approval_granted: Schema.optional(Schema.String),
  all_checks_pass: Schema.optional(Schema.Boolean),
  checks_failed: Schema.optional(Schema.Boolean),
  constraint_violated: Schema.optional(Schema.Boolean),
  shell: Schema.optional(ZyalShellCheck),
})
export type ZyalWorkflowTransitionCondition = Schema.Schema.Type<typeof ZyalWorkflowTransitionCondition>

export const ZyalWorkflowTransition = Schema.Struct({
  to: Schema.String,
  when: ZyalWorkflowTransitionCondition,
})
export type ZyalWorkflowTransition = Schema.Schema.Type<typeof ZyalWorkflowTransition>

export const ZyalWorkflowState = Schema.Struct({
  agent: Schema.optional(Schema.String),
  writes: Schema.optional(Schema.Union([
    Schema.Literal("none"),
    Schema.Literal("scratch_only"),
    Schema.Literal("isolated_worktree"),
    Schema.Literal("working_tree"),
  ])),
  requires: Schema.optional(Schema.Array(Schema.String)),
  produces: Schema.optional(Schema.Array(Schema.String)),
  approval: Schema.optional(Schema.String),
  terminal: Schema.optional(Schema.Boolean),
  timeout: Schema.optional(Schema.String),
  hooks: Schema.optional(Schema.Struct({
    on_enter: Schema.optional(Schema.Array(ZyalHookStep)),
    on_exit: Schema.optional(Schema.Array(ZyalHookStep)),
  })),
  transitions: Schema.optional(Schema.Array(ZyalWorkflowTransition)),
})
export type ZyalWorkflowState = Schema.Schema.Type<typeof ZyalWorkflowState>

export const ZyalWorkflow = Schema.Struct({
  type: Schema.Union([
    Schema.Literal("state_machine"),
    Schema.Literal("dag"),
    Schema.Literal("pipeline"),
  ]),
  initial: Schema.String,
  states: Schema.Record(Schema.String, ZyalWorkflowState),
  on_stuck: Schema.optional(Schema.Union([
    Schema.Literal("pause"),
    Schema.Literal("abort"),
    Schema.Literal("incubate"),
  ])),
  max_total_time: Schema.optional(Schema.String),
})
export type ZyalWorkflow = Schema.Schema.Type<typeof ZyalWorkflow>
