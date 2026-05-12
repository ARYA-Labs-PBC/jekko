import { Schema } from "effect"

export const ZyalArmingOrigin = Schema.Union([
  Schema.Literal("trusted_user_message"),
  Schema.Literal("signed_cli_input"),
  Schema.Literal("signed_api_request"),
  Schema.Literal("host_ui_button"),
])
export type ZyalArmingOrigin = Schema.Schema.Type<typeof ZyalArmingOrigin>

export const ZyalArmingPolicy = Schema.Struct({
  preview_hash_required: Schema.optional(Schema.Boolean),
  host_nonce_required: Schema.optional(Schema.Boolean),
  reject_inside_code_fence: Schema.optional(Schema.Boolean),
  reject_from: Schema.optional(Schema.Array(Schema.String)),
  accepted_origins: Schema.optional(Schema.Array(ZyalArmingOrigin)),
  preview_expires_after: Schema.optional(Schema.String),
  arm_token_single_use: Schema.optional(Schema.Boolean),
  bound_to: Schema.optional(Schema.Array(Schema.String)),
})
export type ZyalArmingPolicy = Schema.Schema.Type<typeof ZyalArmingPolicy>

export const ZyalCapabilityRule = Schema.Struct({
  id: Schema.String,
  tool: Schema.optional(Schema.String),
  mcp_profile: Schema.optional(Schema.String),
  paths: Schema.optional(Schema.Array(Schema.String)),
  command_regex: Schema.optional(Schema.String),
  decision: Schema.Union([Schema.Literal("allow"), Schema.Literal("ask"), Schema.Literal("deny")]),
  require_gate: Schema.optional(Schema.String),
  expires: Schema.optional(Schema.String),
  reason: Schema.optional(Schema.String),
})
export type ZyalCapabilityRule = Schema.Schema.Type<typeof ZyalCapabilityRule>

export const ZyalCapabilities = Schema.Struct({
  default: Schema.optional(Schema.Union([Schema.Literal("deny"), Schema.Literal("ask"), Schema.Literal("allow")])),
  rules: Schema.optional(Schema.Array(ZyalCapabilityRule)),
  command_floor: Schema.optional(Schema.Struct({
    always_block: Schema.Array(Schema.String),
  })),
})
export type ZyalCapabilities = Schema.Schema.Type<typeof ZyalCapabilities>

export const ZyalQualityCheck = Schema.Struct({
  name: Schema.String,
  pattern: Schema.optional(Schema.String),
  shell: Schema.optional(Schema.String),
  scope: Schema.optional(Schema.Union([
    Schema.Literal("file_diff"),
    Schema.Literal("commit_message"),
    Schema.Literal("tool_output"),
    Schema.Literal("memory_write"),
  ])),
  on_violation: Schema.Union([
    Schema.Literal("block_checkpoint"),
    Schema.Literal("block_promotion"),
    Schema.Literal("pause"),
    Schema.Literal("warn"),
    Schema.Literal("require_approval"),
  ]),
})
export type ZyalQualityCheck = Schema.Schema.Type<typeof ZyalQualityCheck>

export const ZyalQuality = Schema.Struct({
  anti_vibe: Schema.optional(Schema.Struct({
    enabled: Schema.optional(Schema.Boolean),
    fail_closed: Schema.optional(Schema.Boolean),
    block_test_deletion: Schema.optional(Schema.Boolean),
    block_assertion_weakening: Schema.optional(Schema.Boolean),
    block_silent_catch: Schema.optional(Schema.Boolean),
    block_fake_data_fallback: Schema.optional(Schema.Boolean),
    block_ts_ignore: Schema.optional(Schema.Boolean),
    require_root_cause_for_bugfix: Schema.optional(Schema.Boolean),
    require_failing_test_first_for_bugfix: Schema.optional(Schema.Boolean),
  })),
  diff_budget: Schema.optional(Schema.Struct({
    max_files_changed: Schema.optional(Schema.Number),
    max_added_lines: Schema.optional(Schema.Number),
    max_deleted_lines: Schema.optional(Schema.Number),
    on_violation: Schema.optional(Schema.Union([
      Schema.Literal("block_checkpoint"),
      Schema.Literal("require_approval"),
      Schema.Literal("warn"),
    ])),
  })),
  checks: Schema.optional(Schema.Array(ZyalQualityCheck)),
})
export type ZyalQuality = Schema.Schema.Type<typeof ZyalQuality>

export const ZyalExperimentLane = Schema.Struct({
  id: Schema.String,
  hypothesis: Schema.String,
  prompt_strategy: Schema.optional(Schema.String),
  agent: Schema.optional(Schema.String),
  model: Schema.optional(Schema.String),
  isolation: Schema.optional(Schema.Union([Schema.Literal("git_worktree"), Schema.Literal("same_session")])),
  timeout: Schema.optional(Schema.String),
  budget: Schema.optional(Schema.Struct({
    max_iterations: Schema.optional(Schema.Number),
    max_diff_lines: Schema.optional(Schema.Number),
    max_cost_usd: Schema.optional(Schema.Number),
  })),
})
export type ZyalExperimentLane = Schema.Schema.Type<typeof ZyalExperimentLane>

export const ZyalExperiments = Schema.Struct({
  strategy: Schema.optional(Schema.Union([
    Schema.Literal("disjoint_tournament"),
    Schema.Literal("parallel_distill_refine"),
    Schema.Literal("ablation"),
    Schema.Literal("portfolio_search"),
  ])),
  diversity: Schema.optional(Schema.Struct({
    require_distinct_plan: Schema.optional(Schema.Boolean),
    min_plan_distance: Schema.optional(Schema.Number),
    axes: Schema.optional(Schema.Array(Schema.String)),
  })),
  lanes: Schema.Array(ZyalExperimentLane),
  fork_from: Schema.optional(Schema.Union([
    Schema.Literal("last_green_checkpoint"),
    Schema.Literal("current_head"),
    Schema.Literal("origin_main"),
  ])),
  max_parallel: Schema.optional(Schema.Number),
  scoring: Schema.optional(Schema.Struct({
    weights: Schema.optional(Schema.Record(Schema.String, Schema.Number)),
    command: Schema.optional(Schema.String),
    primary: Schema.optional(Schema.String),
    judge: Schema.optional(Schema.Struct({
      agent: Schema.String,
      blind: Schema.optional(Schema.Boolean),
      must_use_different_provider: Schema.optional(Schema.Boolean),
    })),
  })),
  reduce: Schema.optional(Schema.Struct({
    strategy: Schema.Union([
      Schema.Literal("best_verified_patch"),
      Schema.Literal("synthesize_best"),
      Schema.Literal("cherry_pick_minimal"),
      Schema.Literal("vote"),
    ]),
    require_final_verification: Schema.optional(Schema.Boolean),
  })),
  on_partial_failure: Schema.optional(Schema.Union([
    Schema.Literal("continue"),
    Schema.Literal("abort"),
    Schema.Literal("pause"),
  ])),
  preserve_failed_lanes_as_negative_memory: Schema.optional(Schema.Boolean),
})
export type ZyalExperiments = Schema.Schema.Type<typeof ZyalExperiments>

export const ZyalModelProfile = Schema.Struct({
  provider: Schema.optional(Schema.String),
  model: Schema.optional(Schema.String),
  temperature: Schema.optional(Schema.Number),
  reasoning: Schema.optional(Schema.Boolean),
  budget_usd: Schema.optional(Schema.Number),
})
export type ZyalModelProfile = Schema.Schema.Type<typeof ZyalModelProfile>

export const ZyalModels = Schema.Struct({
  profiles: Schema.optional(Schema.Record(Schema.String, ZyalModelProfile)),
  routes: Schema.optional(Schema.Record(Schema.String, Schema.String)),
  critic: Schema.optional(Schema.Struct({
    must_differ_from_builder: Schema.optional(Schema.Boolean),
    must_use_different_provider: Schema.optional(Schema.Boolean),
  })),
  redundancy: Schema.optional(Schema.Struct({
    on_rate_limit: Schema.optional(Schema.String),
    on_context_overflow: Schema.optional(Schema.String),
    chain: Schema.optional(Schema.Array(Schema.String)),
    cooldown: Schema.optional(Schema.String),
  })),
  confidence_cap: Schema.optional(Schema.Number),
})
export type ZyalModels = Schema.Schema.Type<typeof ZyalModels>

export const ZyalBudgetScope = Schema.Struct({
  wall_clock: Schema.optional(Schema.String),
  iterations: Schema.optional(Schema.Number),
  tokens: Schema.optional(Schema.Number),
  cost_usd: Schema.optional(Schema.Number),
  tool_calls: Schema.optional(Schema.Number),
  diff_lines: Schema.optional(Schema.Number),
  on_exhaust: Schema.optional(Schema.Union([
    Schema.Literal("pause"),
    Schema.Literal("park"),
    Schema.Literal("abort"),
    Schema.Literal("renew_with_approval"),
  ])),
})
export type ZyalBudgetScope = Schema.Schema.Type<typeof ZyalBudgetScope>

export const ZyalBudgets = Schema.Struct({
  run: Schema.optional(ZyalBudgetScope),
  task: Schema.optional(ZyalBudgetScope),
  iteration: Schema.optional(ZyalBudgetScope),
  experiment_lane: Schema.optional(ZyalBudgetScope),
})
export type ZyalBudgets = Schema.Schema.Type<typeof ZyalBudgets>

export const ZyalTrigger = Schema.Struct({
  id: Schema.String,
  kind: Schema.Union([
    Schema.Literal("manual"),
    Schema.Literal("cron"),
    Schema.Literal("github_issue"),
    Schema.Literal("github_pr_comment"),
    Schema.Literal("ci_failure"),
    Schema.Literal("webhook"),
    Schema.Literal("slack_command"),
  ]),
  schedule: Schema.optional(Schema.String),
  filter: Schema.optional(Schema.Record(Schema.String, Schema.Unknown)),
  idempotency_key_template: Schema.optional(Schema.String),
  max_runs_per_sha: Schema.optional(Schema.Number),
  allow_create_more_cron: Schema.optional(Schema.Boolean),
})
export type ZyalTrigger = Schema.Schema.Type<typeof ZyalTrigger>

export const ZyalTriggers = Schema.Struct({
  list: Schema.Array(ZyalTrigger),
  anti_recursion: Schema.optional(Schema.Boolean),
})
export type ZyalTriggers = Schema.Schema.Type<typeof ZyalTriggers>

export const ZyalRollback = Schema.Struct({
  required_when: Schema.optional(Schema.Struct({
    touches_paths: Schema.optional(Schema.Array(Schema.String)),
    risk_score_gte: Schema.optional(Schema.Number),
  })),
  plan_required: Schema.optional(Schema.Boolean),
  verify_command: Schema.optional(Schema.String),
  on_failure_after_merge: Schema.optional(Schema.Union([
    Schema.Literal("revert_commit"),
    Schema.Literal("feature_flag_off"),
    Schema.Literal("migration_down"),
    Schema.Literal("manual"),
  ])),
})
export type ZyalRollback = Schema.Schema.Type<typeof ZyalRollback>

export const ZyalDone = Schema.Struct({
  require: Schema.optional(Schema.Array(Schema.String)),
  forbid: Schema.optional(Schema.Array(Schema.String)),
})
export type ZyalDone = Schema.Schema.Type<typeof ZyalDone>

export const ZyalRepoIntelligence = Schema.Struct({
  scale: Schema.optional(Schema.Union([
    Schema.Literal("small"),
    Schema.Literal("medium"),
    Schema.Literal("large"),
    Schema.Literal("billion_loc"),
  ])),
  indexes: Schema.optional(Schema.Array(Schema.String)),
  generated_zones: Schema.optional(Schema.Array(Schema.String)),
  scope_control: Schema.optional(Schema.Struct({
    require_scope_before_edit: Schema.optional(Schema.Boolean),
    max_initial_scope_files: Schema.optional(Schema.Number),
    expand_scope_requires_evidence: Schema.optional(Schema.Boolean),
  })),
  blast_radius: Schema.optional(Schema.Struct({
    compute_on: Schema.optional(Schema.Array(Schema.String)),
    pause_when_score_gte: Schema.optional(Schema.Number),
  })),
})
export type ZyalRepoIntelligence = Schema.Schema.Type<typeof ZyalRepoIntelligence>

export const ZYAL_FLEET_MAX_WORKERS = 20

export const ZyalFleetTelemetryHeaders = Schema.Record(Schema.String, Schema.String)

export const ZyalFleetJnoccio = Schema.Struct({
  enabled: Schema.optional(Schema.Boolean),
  base_url: Schema.optional(Schema.String),
  metrics_ws: Schema.optional(Schema.String),
  spawn_on_demand: Schema.optional(Schema.Boolean),
  register_workers: Schema.optional(Schema.Boolean),
  heartbeat_path: Schema.optional(Schema.String),
  heartbeat_interval: Schema.optional(Schema.String),
  max_instances: Schema.optional(
    Schema.Int.check(Schema.isGreaterThanOrEqualTo(1), Schema.isLessThanOrEqualTo(20)),
  ),
})
export type ZyalFleetJnoccio = Schema.Schema.Type<typeof ZyalFleetJnoccio>

export const ZyalFleetTelemetry = Schema.Struct({
  publish_to: Schema.optional(Schema.Union([
    Schema.Literal("jnoccio"),
    Schema.Literal("opentelemetry"),
    Schema.Literal("none"),
  ])),
  headers: Schema.optional(ZyalFleetTelemetryHeaders),
})
export type ZyalFleetTelemetry = Schema.Schema.Type<typeof ZyalFleetTelemetry>

export const ZyalFleet = Schema.Struct({
  max_workers: Schema.Int.check(Schema.isGreaterThanOrEqualTo(1), Schema.isLessThanOrEqualTo(20)),
  isolation: Schema.optional(Schema.Union([
    Schema.Literal("same_session"),
    Schema.Literal("git_worktree"),
    Schema.Literal("hybrid"),
  ])),
  jnoccio: Schema.optional(ZyalFleetJnoccio),
  telemetry: Schema.optional(ZyalFleetTelemetry),
})
export type ZyalFleet = Schema.Schema.Type<typeof ZyalFleet>
