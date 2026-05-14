import { ZYAL_CONTRACT_VERSION, ZYAL_RESEARCH_BLOCK_VERSION, ZYAL_RUNTIME_SENTINEL_VERSION } from "./version"

export type ZyalSchemaStatus = "runtime" | "preview" | "generated" | "compat"

export type ZyalSchemaNode =
  | {
      readonly kind: "object"
      readonly description: string
      readonly status: ZyalSchemaStatus
      readonly required?: boolean
      readonly notes?: string
      readonly children: Record<string, ZyalSchemaNode>
    }
  | {
      readonly kind: "record"
      readonly description: string
      readonly status: ZyalSchemaStatus
      readonly required?: boolean
      readonly notes?: string
      readonly value: ZyalSchemaNode
    }
  | {
      readonly kind: "array"
      readonly description: string
      readonly status: ZyalSchemaStatus
      readonly required?: boolean
      readonly notes?: string
      readonly item: ZyalSchemaNode
    }
  | {
      readonly kind: "scalar"
      readonly description: string
      readonly status: ZyalSchemaStatus
      readonly required?: boolean
      readonly notes?: string
    }
  | {
      readonly kind: "enum"
      readonly description: string
      readonly status: ZyalSchemaStatus
      readonly values: readonly string[]
      readonly required?: boolean
      readonly notes?: string
    }

type NodeOpts = {
  readonly status?: ZyalSchemaStatus
  readonly required?: boolean
  readonly notes?: string
}

const status = (opts?: NodeOpts) => opts?.status ?? "runtime"

const scalar = (description: string, opts?: NodeOpts): ZyalSchemaNode => ({
  kind: "scalar",
  description,
  status: status(opts),
  required: opts?.required,
  notes: opts?.notes,
})

const enumNode = (description: string, values: readonly string[], opts?: NodeOpts): ZyalSchemaNode => ({
  kind: "enum",
  description,
  status: status(opts),
  values,
  required: opts?.required,
  notes: opts?.notes,
})

const objectNode = (
  description: string,
  children: Record<string, ZyalSchemaNode>,
  opts?: NodeOpts,
): ZyalSchemaNode => ({
  kind: "object",
  description,
  status: status(opts),
  children,
  required: opts?.required,
  notes: opts?.notes,
})

const recordNode = (description: string, value: ZyalSchemaNode, opts?: NodeOpts): ZyalSchemaNode => ({
  kind: "record",
  description,
  status: status(opts),
  value,
  required: opts?.required,
  notes: opts?.notes,
})

const arrayNode = (description: string, item: ZyalSchemaNode, opts?: NodeOpts): ZyalSchemaNode => ({
  kind: "array",
  description,
  status: status(opts),
  item,
  required: opts?.required,
  notes: opts?.notes,
})

const literalNode = (description: string, value: string, opts?: NodeOpts): ZyalSchemaNode =>
  enumNode(description, [value], opts)

const stringNode = (description: string, opts?: NodeOpts) => scalar(description, opts)
const numberNode = (description: string, opts?: NodeOpts) => scalar(description, opts)
const booleanNode = (description: string, opts?: NodeOpts) => scalar(description, opts)

const stringListNode = (description: string, itemDescription: string, opts?: NodeOpts) =>
  arrayNode(description, stringNode(itemDescription), opts)

const shellAssertNode = objectNode(
  "Shell assertion bundle.",
  {
    exit_code: numberNode("Expected exit code."),
    stdout_contains: stringNode("Required substring in stdout."),
    stdout_regex: stringNode("Required stdout regex."),
    json: stringNode("JSONPath or similar structured assertion."),
  },
  { status: "runtime" },
)

const shellCheckNode = objectNode(
  "Shell command check used by stop conditions, checkpoints, hooks, and constraints.",
  {
    command: stringNode("Shell command to run.", { required: true }),
    timeout: stringNode("Timeout string such as `30s` or `5m`."),
    cwd: stringNode("Optional working directory override."),
    assert: shellAssertNode,
  },
  { status: "runtime" },
)

const stopConditionNode = objectNode(
  "A stop condition entry.",
  {
    shell: shellCheckNode,
    git_clean: objectNode("Repository cleanliness check.", {
      allow_untracked: booleanNode("Allow untracked files."),
    }),
  },
  { status: "runtime" },
)

const loopBreakerNode = objectNode(
  "Loop circuit breaker policy.",
  {
    max_consecutive_errors: numberNode("Maximum consecutive errors before tripping."),
    on_trip: enumNode("Action when the breaker trips.", ["pause", "abort"]),
  },
  { status: "runtime" },
)

const checkpointNode = objectNode(
  "Git checkpoint policy.",
  {
    when: enumNode("Checkpoint timing.", ["after_verified_change", "on_error", "manual"]),
    noop_if_clean: booleanNode("Skip checkpointing when the tree is clean."),
    verify: arrayNode("Verification checks to run before checkpointing.", shellCheckNode),
    git: objectNode("Git checkpoint actions.", {
      add: stringListNode("Files to stage.", "File path."),
      commit_message: stringNode("Commit message template."),
      push: enumNode("Push policy.", ["ask", "allow", "deny"]),
    }),
  },
  { status: "runtime" },
)

const taskNode = objectNode("Task ledger and discovery policy.", {
  ledger: enumNode("Ledger backend.", ["sqlite"]),
  discover: arrayNode("Shell checks used to discover tasks.", shellCheckNode),
})

const workersNode = objectNode(
  "Agent worker pool.",
  {
    id: stringNode("Worker identifier.", { required: true }),
    count: numberNode("Worker count.", { required: true }),
    agent: stringNode("Agent name.", { required: true }),
    isolation: enumNode("Isolation mode.", ["git_worktree", "same_session", "hybrid"]),
    pool_size: numberNode("Optional worker pool size."),
    commit_on_green: booleanNode("Auto-commit verified output."),
    integration_branch: stringNode("Integration branch name."),
    branch_prefix: stringNode("Branch prefix."),
  },
  { status: "runtime" },
)

const mcpProfileNode = objectNode("MCP profile definition.", {
  servers: stringListNode("Servers in the profile.", "Server name.", { required: true }),
  tools: stringListNode("Allowed tools.", "Tool name.", { required: true }),
  resources: stringListNode("Allowed resources.", "Resource path or URI.", { required: true }),
})

const permissionsNode = objectNode("Permission mode for the run.", {
  read: enumNode("Read permission.", ["ask", "allow", "deny"]),
  list: enumNode("List permission.", ["ask", "allow", "deny"]),
  glob: enumNode("Glob permission.", ["ask", "allow", "deny"]),
  grep: enumNode("Grep permission.", ["ask", "allow", "deny"]),
  external_directory: enumNode("External-directory permission.", ["ask", "allow", "deny"]),
  shell: enumNode("Shell permission.", ["ask", "allow", "deny"]),
  edit: enumNode("Edit permission.", ["ask", "allow", "deny"]),
  git_commit: enumNode("Git commit permission.", ["ask", "allow", "deny"]),
  git_push: enumNode("Git push permission.", ["ask", "allow", "deny"]),
  workers: enumNode("Worker-spawn permission.", ["ask", "allow", "deny"]),
  mcp: enumNode("MCP permission.", ["ask", "allow", "deny"]),
  research: enumNode("Research permission.", ["ask", "allow", "deny"]),
  websearch: enumNode("Web search permission.", ["ask", "allow", "deny"]),
  webfetch: enumNode("Web fetch permission.", ["ask", "allow", "deny"]),
})

const onHandlerNode = objectNode("Signal handler entry.", {
  signal: stringNode("Signal name.", { required: true }),
  count_gte: numberNode("Minimum count threshold."),
  message_contains: stringNode("Required message substring."),
  if: shellCheckNode,
  do: arrayNode(
    "Actions to run when the handler fires.",
    objectNode("Handler action.", {
      switch_agent: stringNode("Switch to another agent."),
      run: stringNode("Shell command to run."),
      incubate_current_task: booleanNode("Send the current task to incubator."),
      checkpoint: booleanNode("Force a checkpoint."),
      pause: booleanNode("Pause the daemon."),
      abort: booleanNode("Abort the daemon."),
      notify: stringNode("Notification message."),
      set_context: stringNode("Context update."),
    }),
  ),
})

const fanOutNode = objectNode("Scatter-gather fan-out policy.", {
  strategy: enumNode("Fan-out strategy.", ["map_reduce", "scatter_gather", "best_score", "custom_shell"]),
  split: objectNode("Split policy.", {
    shell: stringNode("Shell command that emits items."),
    items: stringListNode("Static split items.", "Item."),
  }),
  worker: objectNode("Worker config.", {
    agent: stringNode("Worker agent."),
    isolation: enumNode("Worker isolation.", ["git_worktree", "same_session", "hybrid"]),
    timeout: stringNode("Worker timeout."),
    max_parallel: numberNode("Parallel worker cap."),
  }),
  reduce: objectNode("Reduction policy.", {
    strategy: enumNode("Reduction strategy.", ["merge_all", "best_score", "vote", "custom_shell"]),
    score_key: stringNode("Score key used by best_score."),
    command: stringNode("Reduction shell command."),
  }),
  on_partial_failure: enumNode("Partial-failure behavior.", ["continue", "abort", "pause"]),
})

const guardrailEntryNode = objectNode("Guardrail entry.", {
  name: stringNode("Guardrail name."),
  deny_patterns: stringListNode("Blocked patterns.", "Regex pattern."),
  scope: enumNode("Guardrail scope.", ["file_diff", "commit_message", "tool_output", "memory_write"]),
  action: enumNode("Guardrail action.", ["block_checkpoint", "block_promotion", "pause", "warn", "require_approval"]),
  shell: stringNode("Shell command used by the guardrail."),
  assert: shellAssertNode,
  on_fail: enumNode("Failure action.", ["block_checkpoint", "block_promotion", "pause", "warn", "require_approval"]),
  max_retries: numberNode("Maximum retry count."),
})

const assertionsNode = objectNode("Structured output assertions.", {
  require_structured_output: booleanNode("Require structured output."),
  schema: stringNode("Validation schema reference."),
  on_invalid: enumNode("Invalid-output behavior.", ["block_checkpoint", "block_promotion", "pause", "warn"]),
  max_retries: numberNode("Retry limit."),
})

const retryPolicyNode = objectNode("Retry policy.", {
  max_attempts: numberNode("Maximum attempts."),
  backoff: enumNode("Backoff strategy.", ["none", "linear", "exponential"]),
  initial_delay: stringNode("Initial delay."),
  max_delay: stringNode("Maximum delay."),
  jitter: booleanNode("Enable jitter."),
})

const hooksStepNode = objectNode("Hook step.", {
  run: stringNode("Shell command to run."),
  assert: shellAssertNode,
  on_fail: enumNode("Hook failure action.", ["block_checkpoint", "block_promotion", "pause", "warn", "require_approval"]),
  timeout: stringNode("Timeout string."),
})

const hooksNode = objectNode("Lifecycle hooks.", {
  on_start: arrayNode("Steps run on start.", hooksStepNode),
  before_iteration: arrayNode("Steps run before each iteration.", hooksStepNode),
  after_iteration: arrayNode("Steps run after each iteration.", hooksStepNode),
  before_checkpoint: arrayNode("Steps run before checkpoint.", hooksStepNode),
  after_checkpoint: arrayNode("Steps run after checkpoint.", hooksStepNode),
  on_promote: arrayNode("Steps run on promotion.", hooksStepNode),
  on_exhaust: arrayNode("Steps run when exhausted.", hooksStepNode),
  on_stop: arrayNode("Steps run on stop.", hooksStepNode),
})

const constraintNode = objectNode("Named constraint.", {
  name: stringNode("Constraint name.", { required: true }),
  check: objectNode("Shell check backing the constraint.", {
    shell: stringNode("Shell command.", { required: true }),
    timeout: stringNode("Timeout."),
  }),
  baseline: stringNode("Baseline reference."),
  invariant: enumNode("Constraint invariant.", ["equals_zero", "non_zero", "contains", "matches"]),
  on_violation: enumNode("Violation action.", ["block_checkpoint", "require_approval", "warn"]),
})

const workflowConditionNode = objectNode("Workflow transition condition.", {
  evidence_exists: stringNode("Required evidence artifact name."),
  risk_score_gte: numberNode("Risk threshold."),
  approval_granted: stringNode("Required approval gate name."),
  all_checks_pass: booleanNode("All checks must pass."),
  checks_failed: booleanNode("Checks must fail."),
  constraint_violated: booleanNode("Constraint must be violated."),
  shell: shellCheckNode,
})

const workflowTransitionNode = objectNode("Workflow transition.", {
  to: stringNode("Next state.", { required: true }),
  when: workflowConditionNode,
})

const workflowStateNode = objectNode("Workflow state.", {
  agent: stringNode("State agent."),
  writes: enumNode("State write scope.", ["scratch_only", "isolated_worktree", "main_worktree"]),
  requires: stringListNode("Required artifacts.", "Artifact name."),
  produces: stringListNode("Produced artifacts.", "Artifact name."),
  approval: stringNode("Approval gate name."),
  terminal: booleanNode("Terminal state."),
  timeout: stringNode("State timeout."),
  hooks: objectNode("State-specific hooks.", {
    on_enter: arrayNode("Hooks run on enter.", hooksStepNode),
    on_exit: arrayNode("Hooks run on exit.", hooksStepNode),
  }),
  transitions: arrayNode("State transitions.", workflowTransitionNode),
})

const workflowNode = objectNode("Durable workflow state machine.", {
  type: stringNode("Workflow type.", { required: true }),
  initial: stringNode("Initial state.", { required: true }),
  states: recordNode("Named workflow states.", workflowStateNode, { required: true }),
  on_stuck: stringNode("Fallback state or action."),
  max_total_time: stringNode("Maximum workflow duration."),
})

const memoryStoreNode = objectNode("Memory store definition.", {
  scope: stringNode("Store scope."),
  retention: stringNode("Retention policy."),
  max_entries: numberNode("Maximum entries."),
  compression: stringNode("Compression policy."),
  write_policy: stringNode("Write policy."),
  read_policy: stringNode("Read policy."),
  searchable: booleanNode("Whether the store is searchable."),
  path: stringNode("On-disk path."),
})

const memoryRedactionNode = objectNode("Memory redaction policy.", {
  patterns: stringListNode("Redaction patterns.", "Regex pattern.", { required: true }),
  action: enumNode("Redaction action.", ["mask", "remove", "hash"], { required: true }),
})

const memoryNode = objectNode("Governed memory stores.", {
  stores: recordNode("Memory stores.", memoryStoreNode),
  redaction: memoryRedactionNode,
  provenance: objectNode("Provenance tracking.", {
    track_source: booleanNode("Track source bytes."),
    hash_chain: booleanNode("Hash-chain writes."),
  }),
})

const evidenceRequirementNode = objectNode("Evidence requirement.", {
  type: stringNode("Evidence type.", { required: true }),
  must_pass: booleanNode("Evidence must pass."),
  must_be_known: booleanNode("Evidence must be known."),
  must_exist: booleanNode("Evidence must exist."),
  max_increase: numberNode("Maximum increase."),
})

const evidenceNode = objectNode("Evidence bundle policy.", {
  require_before_promote: arrayNode("Evidence requirements.", evidenceRequirementNode),
  bundle_format: enumNode("Evidence bundle format.", ["json", "markdown"]),
  sign: enumNode("Evidence signing mode.", ["sha256", "none"]),
  archive: booleanNode("Archive evidence bundles."),
})

const approvalsGateNode = objectNode("Approval gate.", {
  required_role: stringNode("Required role."),
  timeout: stringNode("Gate timeout."),
  on_timeout: enumNode("Timeout action.", ["pause", "abort", "auto_approve", "escalate"]),
  decisions: stringListNode("Allowed decisions.", "Decision.", { required: true }),
  require_evidence: stringListNode("Required evidence identifiers.", "Evidence id."),
  auto_approve_if: objectNode("Auto-approval guard.", {
    risk_score_lt: numberNode("Max risk score."),
    all_checks_pass: booleanNode("Require all checks to pass."),
  }),
})

const approvalsNode = objectNode("Human approval gates.", {
  gates: recordNode("Approval gates.", approvalsGateNode),
  escalation: objectNode("Escalation policy.", {
    chain: stringListNode("Escalation chain.", "Role or gate."),
    auto_escalate_after: stringNode("Escalation delay."),
  }),
})

const skillsRegistryNode = objectNode("Skill registry entry.", {
  description: stringNode("Skill description."),
  agent: stringNode("Preferred agent."),
  tools: stringListNode("Allowed tools.", "Tool name."),
  mcp_profile: stringNode("MCP profile."),
  writes: stringListNode("Allowed write scopes.", "Write scope."),
  trust: stringNode("Trust level."),
  timeout: stringNode("Timeout."),
})

const skillsNode = objectNode("Skill discovery and promotion.", {
  registry: recordNode("Named skills.", skillsRegistryNode),
  allow_creation: booleanNode("Allow skill creation."),
  max_skills: numberNode("Maximum skills."),
})

const sandboxPathRuleNode = objectNode("Sandbox path rule.", {
  path: stringNode("Path pattern."),
  access: enumNode("Access mode.", ["read", "write", "read_write", "deny"]),
})

const sandboxNetworkNode = objectNode("Sandbox network policy.", {
  outbound: enumNode("Outbound policy.", ["allow", "deny", "allowlist"]),
  allowlist: stringListNode("Allowed endpoints.", "Endpoint."),
})

const sandboxNode = objectNode("Execution sandbox.", {
  paths: arrayNode("Path rules.", sandboxPathRuleNode),
  network: sandboxNetworkNode,
  resources: objectNode("Resource limits.", {
    max_file_size: numberNode("Maximum file size."),
    max_total_disk: numberNode("Maximum disk usage."),
    max_memory: numberNode("Maximum memory."),
    max_processes: numberNode("Maximum processes."),
  }),
  env_inherit: stringListNode("Environment keys to inherit.", "Env key."),
  env_deny: stringListNode("Environment keys to deny.", "Env key."),
})

const securityTrustZoneNode = objectNode("Security trust zone.", {
  paths: stringListNode("Protected paths.", "Path."),
  require_approval: booleanNode("Require approval for this zone."),
  max_risk_score: numberNode("Maximum risk score."),
})

const securityInjectionNode = objectNode("Security injection policy.", {
  scan_inputs: booleanNode("Scan inputs."),
  scan_outputs: booleanNode("Scan outputs."),
  deny_patterns: stringListNode("Denied patterns.", "Regex pattern."),
  on_detect: enumNode("Detection action.", ["strip", "quote", "block", "pause"]),
})

const securitySecretsNode = objectNode("Security secrets policy.", {
  allowed_env: stringListNode("Allowed environment keys.", "Env key."),
  redact_from_logs: booleanNode("Redact from logs."),
  rotate_after: stringNode("Rotation interval."),
})

const securityNode = objectNode("Secrets and trust policy.", {
  trust_zones: recordNode("Trust zones.", securityTrustZoneNode),
  injection: securityInjectionNode,
  secrets: securitySecretsNode,
})

const observabilitySpanNode = objectNode("Span export policy.", {
  emit: stringNode("Emission target."),
  include_tool_calls: booleanNode("Include tool calls."),
  include_model_calls: booleanNode("Include model calls."),
})

const observabilityMetricNode = objectNode("Metric declaration.", {
  name: stringNode("Metric name.", { required: true }),
  type: stringNode("Metric type."),
  source: stringNode("Metric source.", { required: true }),
})

const observabilityCostNode = objectNode("Cost policy.", {
  budget: numberNode("Budget."),
  currency: stringNode("Currency."),
  alert_at_percent: numberNode("Alert threshold."),
  on_budget_exceeded: enumNode("Budget action.", ["pause", "abort", "warn"]),
})

const observabilityReportNode = objectNode("Reporting policy.", {
  format: stringNode("Report format."),
  on_complete: stringNode("Completion action."),
  on_checkpoint: stringNode("Checkpoint action."),
  include: stringListNode("Report sections.", "Section."),
})

const observabilityNode = objectNode("Spans, metrics, cost, and reports.", {
  spans: observabilitySpanNode,
  metrics: arrayNode("Metric list.", observabilityMetricNode),
  cost: observabilityCostNode,
  report: observabilityReportNode,
})

const armingNode = objectNode("Host arming policy.", {
  preview_hash_required: booleanNode("Require a preview hash."),
  host_nonce_required: booleanNode("Require a host nonce."),
  reject_inside_code_fence: booleanNode("Reject code-fenced input."),
  reject_from: stringListNode("Rejected origins.", "Origin."),
  accepted_origins: arrayNode(
    "Accepted origins.",
    enumNode("Allowed arming origin.", ["trusted_user_message", "signed_cli_input", "signed_api_request", "host_ui_button"]),
  ),
  preview_expires_after: stringNode("Preview expiry."),
  arm_token_single_use: booleanNode("Single-use arm token."),
  bound_to: stringListNode("Binding constraints.", "Binding key."),
})

const capabilityRuleNode = objectNode("Capability rule.", {
  id: stringNode("Rule identifier.", { required: true }),
  tool: stringNode("Tool name."),
  mcp_profile: stringNode("MCP profile."),
  paths: stringListNode("Allowed paths.", "Path."),
  command_regex: stringNode("Command regex."),
  decision: enumNode("Capability decision.", ["allow", "ask", "deny"], { required: true }),
  require_gate: stringNode("Required approval gate."),
  expires: stringNode("Rule expiry."),
  reason: stringNode("Rule reason."),
})

const capabilitiesNode = objectNode("Capability lease policy.", {
  default: enumNode("Default capability decision.", ["deny", "ask", "allow"]),
  rules: arrayNode("Capability rules.", capabilityRuleNode),
  command_floor: objectNode("Always-blocked commands.", {
    always_block: stringListNode("Blocked commands.", "Command.", { required: true }),
  }),
})

const qualityCheckNode = objectNode("Quality check.", {
  name: stringNode("Check name.", { required: true }),
  pattern: stringNode("Regex pattern."),
  shell: stringNode("Shell command."),
  scope: enumNode("Check scope.", ["file_diff", "commit_message", "tool_output", "memory_write"]),
  on_violation: enumNode("Violation action.", ["block_checkpoint", "block_promotion", "pause", "warn", "require_approval"], {
    required: true,
  }),
})

const qualityNode = objectNode("Anti-vibe quality gates.", {
  anti_vibe: objectNode("Anti-vibe policy.", {
    enabled: booleanNode("Enable anti-vibe checks."),
    fail_closed: booleanNode("Fail closed."),
    block_test_deletion: booleanNode("Block deleting tests."),
    block_assertion_weakening: booleanNode("Block weakening assertions."),
    block_silent_catch: booleanNode("Block silent catch blocks."),
    block_fake_data_fallback: booleanNode("Block fake-data fallbacks."),
    block_ts_ignore: booleanNode("Block `@ts-ignore`."),
    require_root_cause_for_bugfix: booleanNode("Require root cause before bugfix."),
    require_failing_test_first_for_bugfix: booleanNode("Require failing test first."),
  }),
  diff_budget: objectNode("Diff budget.", {
    max_files_changed: numberNode("Maximum files changed."),
    max_added_lines: numberNode("Maximum added lines."),
    max_deleted_lines: numberNode("Maximum deleted lines."),
    on_violation: enumNode("Violation action.", ["block_checkpoint", "require_approval", "warn"]),
  }),
  checks: arrayNode("Additional quality checks.", qualityCheckNode),
})

const experimentBudgetNode = objectNode("Experiment lane budget.", {
  max_iterations: numberNode("Maximum iterations."),
  max_diff_lines: numberNode("Maximum diff lines."),
  max_cost_usd: numberNode("Maximum cost."),
})

const experimentLaneNode = objectNode("Experiment lane.", {
  id: stringNode("Lane identifier.", { required: true }),
  hypothesis: stringNode("Lane hypothesis.", { required: true }),
  prompt_strategy: stringNode("Prompt strategy."),
  agent: stringNode("Agent."),
  model: stringNode("Model."),
  isolation: enumNode("Isolation mode.", ["git_worktree", "same_session"]),
  timeout: stringNode("Lane timeout."),
  budget: experimentBudgetNode,
})

const experimentScoringNode = objectNode("Experiment scoring.", {
  weights: recordNode("Weight map.", numberNode("Weight value.")),
  command: stringNode("Scoring shell command."),
  primary: stringNode("Primary scoring strategy."),
  goal_direction: enumNode("Goal direction.", ["maximize", "minimize"]),
  judge: objectNode("Blind judge configuration.", {
    agent: stringNode("Judge agent.", { required: true }),
    blind: booleanNode("Judge is blind."),
    must_use_different_provider: booleanNode("Judge must use a different provider."),
  }),
})

const experimentsNode = objectNode("Hypothesis tournament configuration.", {
  strategy: enumNode("Experiment strategy.", ["disjoint_tournament", "parallel_distill_refine", "ablation", "portfolio_search"]),
  diversity: objectNode("Diversity policy.", {
    require_distinct_plan: booleanNode("Require distinct plans."),
    min_plan_distance: numberNode("Minimum plan distance."),
    axes: stringListNode("Diversity axes.", "Axis."),
  }),
  lanes: arrayNode("Experiment lanes.", experimentLaneNode, { required: true }),
  fork_from: enumNode("Fork base.", ["last_green_checkpoint", "current_head", "origin_main"]),
  max_parallel: numberNode("Maximum parallel lanes."),
  scoring: experimentScoringNode,
  reduce: objectNode("Reduction policy.", {
    strategy: enumNode("Reduce strategy.", ["best_verified_patch", "synthesize_best", "cherry_pick_minimal", "vote"], { required: true }),
    require_final_verification: booleanNode("Require final verification."),
  }),
  on_partial_failure: enumNode("Partial-failure action.", ["continue", "abort", "pause"]),
  preserve_failed_lanes_as_negative_memory: booleanNode("Store failed lanes as negative memory."),
})

const modelProfileNode = objectNode("Model profile.", {
  provider: stringNode("Provider."),
  model: stringNode("Model name."),
  temperature: numberNode("Temperature."),
  reasoning: booleanNode("Enable reasoning."),
  budget_usd: numberNode("Budget in USD."),
})

const modelsNode = objectNode("Model routing and redundancy.", {
  profiles: recordNode("Named model profiles.", modelProfileNode),
  routes: recordNode("Route-to-profile mapping.", stringNode("Profile name.")),
  critic: objectNode("Critic policy.", {
    must_differ_from_builder: booleanNode("Critic must differ from builder."),
    must_use_different_provider: booleanNode("Critic must use a different provider."),
  }),
  fallback: objectNode("Legacy fallback routing.", {
    on_rate_limit: stringNode("Rate-limit fallback."),
    on_context_overflow: stringNode("Context-overflow fallback."),
    chain: stringListNode("Fallback chain.", "Profile name."),
    cooldown: stringNode("Cooldown."),
  }),
  redundancy: objectNode("Fallback chain.", {
    on_rate_limit: stringNode("Rate-limit fallback."),
    on_context_overflow: stringNode("Context-overflow fallback."),
    chain: stringListNode("Fallback chain.", "Profile name."),
    cooldown: stringNode("Cooldown."),
  }),
  confidence_cap: numberNode("Confidence cap."),
})

const budgetScopeNode = objectNode("Budget scope.", {
  wall_clock: stringNode("Wall-clock budget."),
  iterations: numberNode("Iteration budget."),
  tokens: numberNode("Token budget."),
  cost_usd: numberNode("Cost budget."),
  tool_calls: numberNode("Tool-call budget."),
  diff_lines: numberNode("Diff-line budget."),
  on_exhaust: enumNode("Exhaustion action.", ["pause", "park", "abort", "renew_with_approval"]),
})

const budgetsNode = objectNode("Nested budgets.", {
  run: budgetScopeNode,
  task: budgetScopeNode,
  iteration: budgetScopeNode,
  experiment_lane: budgetScopeNode,
})

const triggerNode = objectNode("Trigger rule.", {
  id: stringNode("Trigger identifier.", { required: true }),
  kind: enumNode("Trigger kind.", ["manual", "cron", "github_issue", "github_pr_comment", "ci_failure", "webhook", "slack_command"], {
    required: true,
  }),
  schedule: stringNode("Cron or schedule expression."),
  filter: recordNode("Trigger filter.", scalar("Arbitrary filter value.")),
  idempotency_key_template: stringNode("Idempotency key template."),
  max_runs_per_sha: numberNode("Per-SHA run cap."),
  allow_create_more_cron: booleanNode("Allow creating more cron triggers."),
})

const triggersNode = objectNode("Trigger list and anti-recursion guard.", {
  list: arrayNode("Trigger rules.", triggerNode, { required: true }),
  anti_recursion: booleanNode("Prevent recursive triggering."),
})

const rollbackNode = objectNode("Rollback policy.", {
  required_when: objectNode("Rollback requirement predicate.", {
    touches_paths: stringListNode("Paths that trigger rollback.", "Path."),
    risk_score_gte: numberNode("Risk threshold."),
  }),
  plan_required: booleanNode("Require a rollback plan."),
  verify_command: stringNode("Rollback verification command."),
  on_failure_after_merge: enumNode("Post-merge failure action.", ["revert_commit", "feature_flag_off", "migration_down", "manual"]),
})

const doneNode = objectNode("Completion policy.", {
  require: stringListNode("Required completion conditions.", "Condition."),
  forbid: stringListNode("Forbidden completion conditions.", "Condition."),
})

const repoIntelNode = objectNode("Repository-intelligence policy.", {
  scale: enumNode("Repo scale.", ["small", "medium", "large", "billion_loc"]),
  indexes: stringListNode("Required indexes.", "Index."),
  generated_zones: booleanNode("Respect generated zones."),
  scope_control: objectNode("Scope control policy.", {
    require_scope_before_edit: booleanNode("Require scope before editing."),
    max_initial_scope_files: numberNode("Initial scope ceiling."),
    expand_scope_requires_evidence: booleanNode("Require evidence to expand scope."),
  }),
  blast_radius: objectNode("Blast-radius policy.", {
    compute_on: stringListNode("Inputs used to compute blast radius.", "Input."),
    pause_when_score_gte: numberNode("Pause threshold."),
  }),
})

const fleetNode = objectNode("Fleet and Jnoccio coordination.", {
  max_workers: numberNode("Maximum workers.", { required: true }),
  isolation: enumNode("Fleet isolation mode.", ["git_worktree", "same_session", "hybrid"]),
  jnoccio: objectNode("Jnoccio integration.", {
    enabled: booleanNode("Enable Jnoccio integration."),
    base_url: stringNode("Jnoccio base URL."),
    metrics_ws: stringNode("Metrics WebSocket path."),
    spawn_on_demand: booleanNode("Spawn the server on demand."),
    register_workers: booleanNode("Register workers with the server."),
    heartbeat_path: stringNode("Worker heartbeat path."),
    heartbeat_interval: stringNode("Heartbeat interval."),
    max_instances: numberNode("Maximum instances."),
  }),
  telemetry: objectNode("Fleet telemetry.", {
    publish_to: stringNode("Telemetry target."),
    headers: recordNode("Telemetry headers.", stringNode("Header value.")),
  }),
})

const researchNode = objectNode("Cited external evidence gathering.", {
  version: literalNode("Research block version.", ZYAL_RESEARCH_BLOCK_VERSION, { required: true }),
  mode: enumNode("Research mode.", ["auto", "web", "academic", "news", "code", "mixed"]),
  autonomy: enumNode("Research autonomy.", ["agent_decides", "require_plan", "fixed_sources"]),
  max_parallel: numberNode("Maximum parallel research tasks."),
  timeout_seconds: numberNode("Research timeout."),
  provider_policy: objectNode("Provider policy.", {
    prefer: arrayNode(
      "Preferred provider types.",
      enumNode("Preferred provider type.", ["official_api", "primary_source", "privacy_first"]),
    ),
    allow: stringListNode("Allowed provider names.", "Provider name."),
    missing_provider: enumNode("Missing-provider behavior.", ["skip_with_receipt", "pause", "fail"]),
  }),
  extraction: objectNode("Extraction policy.", {
    enabled: booleanNode("Enable extraction."),
    max_pages: numberNode("Maximum pages."),
    allowed_extractors: arrayNode(
      "Allowed extractors.",
      enumNode("Extractor.", ["built_in", "jina", "firecrawl"]),
    ),
  }),
  evidence: objectNode("Research evidence policy.", {
    require_citations: booleanNode("Require citations."),
    claim_level: booleanNode("Track claim-level evidence."),
    store: enumNode("Evidence store.", ["sqlite"]),
  }),
  safety: objectNode("Research safety policy.", {
    redact_secrets: booleanNode("Redact secrets."),
    block_internal_urls: booleanNode("Block internal URLs."),
    prompt_injection: enumNode("Prompt injection action.", ["quarantine"]),
    taint_label: enumNode("Taint label.", ["web_content"]),
  }),
  budgets: objectNode("Research budgets.", {
    max_queries: numberNode("Maximum queries."),
    max_pages: numberNode("Maximum pages."),
    max_cost_usd: numberNode("Maximum cost."),
  }),
  paper_scan: objectNode("Paper scan pipeline.", {
    enabled: booleanNode("Enable paper scanning."),
    domains: stringListNode("Target domains.", "Domain."),
    queries: stringListNode("Search queries.", "Query."),
    open_access: enumNode("Open-access preference.", ["required", "preferred"]),
    max_papers: numberNode("Maximum papers."),
    output_root: stringNode("Output root."),
    raw_receipts: stringNode("Raw receipts path."),
  }),
  full_text: objectNode("Full-text capture.", {
    enabled: booleanNode("Enable full-text capture."),
    store: enumNode("Full-text store.", ["checked_in_json", "target_only"]),
    raw_receipts: stringNode("Raw receipts path."),
    extraction_receipts: stringNode("Extraction receipts path."),
    license_policy: enumNode("License policy.", ["oa_only", "public_license_only"]),
  }),
  dedupe: objectNode("Duplicate suppression.", {
    enabled: booleanNode("Enable dedupe."),
    state_root: stringNode("State root."),
    duplicate_policy: enumNode("Duplicate policy.", ["skip_existing", "fail"]),
    hash_keys: stringListNode("Hash keys.", "Field name."),
  }),
  context_packing: objectNode("Context packing policy.", {
    strategy: enumNode("Packing strategy.", ["hard", "best_effort"]),
    target_fill_ratio: numberNode("Target fill ratio."),
    output_reserve_tokens: numberNode("Output reserve tokens."),
    safe_window_tokens: numberNode("Safe window tokens."),
  }),
  question_bank: objectNode("Question bank pipeline.", {
    output_root: stringNode("Output root."),
    papers_root: stringNode("Papers root."),
    challenges_root: stringNode("Challenges root."),
    rejected_root: stringNode("Rejected root."),
    work_items: arrayNode(
      "Work items.",
      objectNode("Question bank work item.", {
        id: stringNode("Work-item id.", { required: true }),
        publication_hash: stringNode("Publication hash.", { required: true }),
        paper_path: stringNode("Paper path."),
        challenge_path: stringNode("Challenge path."),
        role: enumNode("Work-item role.", [
          "question_generator",
          "publication_extractor",
          "answerer",
          "saturated_answerer",
          "focused_auditor",
          "critic",
          "auditor",
          "judge_reducer",
          "reducer",
          "scorer",
        ]),
      }),
    ),
    acceptance: objectNode("Acceptance policy.", {
      min_auditor_agreement: numberNode("Minimum auditor agreement."),
      min_answerability: numberNode("Minimum answerability."),
      max_blind_correct_rate_for_hard: numberNode("Maximum blind correct rate for hard questions."),
      reject_if_ambiguous: booleanNode("Reject ambiguous items."),
    }),
  }),
  agent_trials: objectNode("Agent trial settings.", {
    question_generators: numberNode("Question generators."),
    answerers: numberNode("Answerers."),
    model_profile: stringNode("Model profile."),
  }),
  audit: objectNode("Research audit policy.", {
    critics: numberNode("Critics."),
    focused_auditors: numberNode("Focused auditors."),
    min_auditor_agreement: numberNode("Minimum auditor agreement."),
    min_answerability: numberNode("Minimum answerability."),
  }),
  route_metadata: objectNode("Route metadata policy.", {
    required: booleanNode("Require route metadata."),
    require_request_id: booleanNode("Require request ids."),
    require_provider: stringNode("Required provider id."),
    require_model_profile: booleanNode("Require model profile metadata."),
  }),
})

const jankuraiAuditNode = objectNode("Jankurai audit targets.", {
  mode: enumNode("Audit mode.", ["advisory", "guarded", "standard", "ratchet", "release"]),
  json: stringNode("JSON output path."),
  md: stringNode("Markdown output path."),
  repair_queue_jsonl: stringNode("Repair queue output path."),
  sarif: stringNode("SARIF output path."),
  no_score_history: booleanNode("Skip score history output."),
})

const jankuraiBootstrapNode = objectNode("Jankurai bootstrap policy.", {
  run_update_on_start: booleanNode("Update on start."),
  ensure_init: booleanNode("Ensure init."),
  ensure_canonical: booleanNode("Ensure canonical layout."),
  yes: booleanNode("Assume yes."),
  strict: booleanNode("Strict mode."),
  dry_run: booleanNode("Dry run."),
})

const jankuraiPoolNode = objectNode("Jankurai worker pool.", {
  size: numberNode("Pool size."),
  hard_cap: numberNode("Absolute hard cap."),
  branch_prefix: stringNode("Branch prefix."),
  integration_branch: stringNode("Integration branch."),
  commit_on_green: booleanNode("Commit on green."),
})

const jankuraiReviewerNode = objectNode("Jankurai critical reviewer.", {
  enabled: booleanNode("Enable reviewer."),
  block_promotion: booleanNode("Block promotion on blockers."),
  checklist: arrayNode(
    "Reviewer checklist.",
    objectNode("Reviewer checklist item.", {
      id: stringNode("Checklist id.", { required: true }),
      prompt: stringNode("Reviewer prompt."),
      severity: enumNode("Severity.", ["info", "warning", "blocker"]),
    }),
  ),
})

const jankuraiVerificationNode = objectNode("Jankurai verification policy.", {
  require_clean_start: booleanNode("Require clean start."),
  require_clean_after_checkpoint: booleanNode("Require clean state after checkpoint."),
  proof_from_test_map: booleanNode("Use agent/test-map.json proof routes."),
  commands: stringListNode("Verification commands.", "Shell command."),
  audit_delta: enumNode("Expected audit delta.", ["no_new_findings", "no_score_drop", "target_fingerprint_removed", "none"]),
  rollback_unverified: booleanNode("Rollback unverified changes."),
})

const jankuraiSelectionNode = objectNode("Jankurai task selection.", {
  order: enumNode("Selection order.", ["quick_wins_first", "severity_first", "random"]),
  randomize_ties: booleanNode("Randomize ties."),
  max_risk: enumNode("Maximum risk to claim.", ["low", "medium", "high", "critical"]),
  skip_human_review_required: booleanNode("Skip tasks that require human review."),
  incubate_risk_at: enumNode("Risk threshold for incubation.", ["low", "medium", "high", "critical"]),
  defer_rules: stringListNode("Deferred rule ids.", "Rule id."),
  incubate_rules: stringListNode("Incubated rule ids.", "Rule id."),
})

const jankuraiRegressionNode = objectNode("Regression sentinel.", {
  main_ref: stringNode("Main branch reference."),
  compare_every_iterations: numberNode("Comparison cadence."),
  mode: enumNode("Comparison mode.", ["advisory", "guarded", "standard", "ratchet", "release"]),
  max_new_hard_findings: numberNode("Maximum new hard findings."),
  max_score_drop: numberNode("Maximum score drop."),
})

const jankuraiRepairPlanNode = objectNode("Repair-plan input.", {
  enabled: booleanNode("Enable repair-plan ingestion."),
  json: stringNode("Repair plan JSON path."),
  md: stringNode("Repair plan markdown path."),
})

const jankuraiNode = objectNode("Host-enforced Jankurai orchestration.", {
  enabled: booleanNode("Enable Jankurai."),
  root: stringNode("Jankurai root."),
  bootstrap: jankuraiBootstrapNode,
  pool: jankuraiPoolNode,
  reviewer: jankuraiReviewerNode,
  audit: jankuraiAuditNode,
  repair_plan: jankuraiRepairPlanNode,
  task_source: enumNode("Task source.", ["repair_plan", "findings", "agent_fix_queue", "repair_queue_jsonl"]),
  selection: jankuraiSelectionNode,
  regression: jankuraiRegressionNode,
  verification: jankuraiVerificationNode,
})

const dispatchNode = objectNode("General classify-then-route dispatch.", {
  enabled: booleanNode("Enable dispatch."),
  classifier: objectNode("Route classifier.", {
    command: stringNode("Classifier shell command."),
    timeout: stringNode("Classifier timeout."),
    write_to: stringNode("Route decision output."),
  }),
  lanes: arrayNode(
    "Dispatch lanes.",
    objectNode("Dispatch lane.", {
      id: stringNode("Lane id.", { required: true }),
      dispatch_to: stringNode("Downstream primitive.", { required: true }),
      description: stringNode("Lane description."),
    }),
  ),
  default_lane: stringNode("Default lane."),
  on_no_match: enumNode("No-match action.", ["pause", "abort", "skip", "default"]),
})

const taintNode = objectNode("Origin-aware taint policy.", {
  default_label: stringNode("Default label."),
  labels: recordNode(
    "Taint labels.",
    objectNode("Taint label.", {
      rank: enumNode("Rank.", ["high", "medium", "untrusted", "untrusted_for_arming", "hostile"], { required: true }),
      notes: stringNode("Label notes."),
    }),
    { required: true },
  ),
  forbid: arrayNode(
    "Forbidden actions per label.",
    objectNode("Forbid rule.", {
      from: stringListNode("Source labels.", "Label.", { required: true }),
      cannot: arrayNode(
        "Forbidden actions.",
        enumNode(
          "Forbidden action.",
          [
            "arm",
            "approve",
            "grant_capability",
            "write_memory_procedural",
            "write_memory_semantic",
            "exec_shell",
            "install_skill",
            "modify_objective",
            "expose_secret",
          ],
          { required: true },
        ),
      ),
      unless: stringListNode("Exemption conditions.", "Condition."),
    }),
  ),
  prompt_injection: objectNode("Prompt injection scanner.", {
    detect_patterns: stringListNode("Regex patterns to detect.", "Regex pattern.", { required: true }),
    on_detect: enumNode("Detection action.", ["strip", "quote", "block", "pause"], { required: true }),
    scan_sources: stringListNode("Source labels to scan.", "Label."),
  }),
})

const interactionNode = objectNode("Interaction handling.", {
  user: enumNode("User interaction mode.", ["none", "async", "present"]),
  on_ambiguity: enumNode("Ambiguity behavior.", ["best_effort", "pause", "skip"]),
  on_blocked: enumNode("Blocked behavior.", ["skip_and_next", "pause", "fail"]),
  system_inject: stringNode("System injection text."),
})

const interopNode = objectNode("Interop adapters and translation targets.", {
  protocols: arrayNode(
    "Interop protocols.",
    objectNode("Interop protocol.", {
      name: stringNode("Protocol name.", { required: true }),
      target: stringNode("Protocol target."),
      version: stringNode("Protocol version."),
      notes: stringNode("Protocol notes."),
    }),
  ),
  adapters: stringListNode("Adapter names.", "Adapter."),
  compile_to: stringListNode("Compilation targets.", "Target."),
  notes: stringNode("Interop notes."),
})

const runtimeNode = objectNode("Runtime deployment hints.", {
  mode: enumNode("Runtime mode.", ["preview", "host_enforced"]),
  image: stringNode("Container or host image."),
  workspace: stringNode("Workspace path."),
  network: enumNode("Network policy.", ["allow", "deny", "allowlist"]),
  env: stringListNode("Environment keys.", "Env key."),
  resources: objectNode("Runtime resources.", {
    cpu: stringNode("CPU limit."),
    memory: stringNode("Memory limit."),
    disk: stringNode("Disk limit."),
    processes: numberNode("Process limit."),
  }),
})

const capabilityNegotiationNode = objectNode("Capability negotiation.", {
  host: stringNode("Host name."),
  required: stringListNode("Required capabilities.", "Capability."),
  optional: stringListNode("Optional capabilities.", "Capability."),
  fail_closed: booleanNode("Fail closed."),
  degrade_to: stringNode("Fallback mode."),
})

const memoryKernelNode = objectNode("Kernel-memory policy.", {
  stores: recordNode(
    "Kernel-memory stores.",
    objectNode("Kernel-memory store.", {
      scope: stringNode("Store scope.", { required: true }),
      retention: stringNode("Retention policy.", { required: true }),
      searchable: booleanNode("Searchable flag."),
    }),
  ),
  redaction: objectNode("Kernel-memory redaction.", {
    patterns: stringListNode("Patterns.", "Regex pattern.", { required: true }),
    action: enumNode("Redaction action.", ["mask", "remove", "hash"], { required: true }),
  }),
  provenance: objectNode("Kernel-memory provenance.", {
    track_source: booleanNode("Track source."),
    hash_chain: booleanNode("Hash chain."),
  }),
})

const evidenceGraphNode = objectNode("Evidence graph.", {
  nodes: recordNode(
    "Evidence graph nodes.",
    objectNode("Evidence graph node.", {
      type: stringNode("Node type.", { required: true }),
      required: booleanNode("Required node."),
    }),
  ),
  edges: arrayNode(
    "Evidence graph edges.",
    objectNode("Evidence graph edge.", {
      from: stringNode("Source node.", { required: true }),
      to: stringNode("Target node.", { required: true }),
      kind: stringNode("Edge kind."),
    }),
  ),
  merge_witness: stringNode("Merge witness path."),
})

const trustNode = objectNode("Trust policy.", {
  zones: recordNode(
    "Trust zones.",
    objectNode("Trust zone policy.", {
      paths: stringListNode("Zone paths.", "Path."),
      taint: enumNode("Zone taint.", ["clean", "tainted", "quarantined"]),
      require_approval: booleanNode("Require approval."),
    }),
  ),
  on_taint: enumNode("Taint behavior.", ["abort", "pause", "warn"]),
  notes: stringNode("Trust notes."),
})

const requirementsNode = objectNode("Requirement set.", {
  must: stringListNode("Must-have requirements.", "Requirement."),
  should: stringListNode("Should-have requirements.", "Requirement."),
  avoid: stringListNode("Avoided behaviors.", "Behavior."),
})

const evaluationNode = objectNode("Evaluation policy.", {
  metrics: arrayNode(
    "Evaluation metrics.",
    objectNode("Evaluation metric.", {
      name: stringNode("Metric name.", { required: true }),
      command: stringNode("Metric command."),
      threshold: numberNode("Threshold."),
    }),
  ),
  compare: stringNode("Comparison target."),
})

const releaseNode = objectNode("Release metadata.", {
  channel: stringNode("Release channel."),
  version: stringNode("Release version."),
  gates: stringListNode("Release gates.", "Gate."),
  notes: stringNode("Release notes."),
})

const rolesNode = objectNode("Role definitions.", {
  list: arrayNode(
    "Roles.",
    objectNode("Role.", {
      id: stringNode("Role id.", { required: true }),
      agent: stringNode("Role agent."),
      permissions: stringListNode("Role permissions.", "Permission."),
      description: stringNode("Role description."),
    }),
  ),
})

const channelsNode = objectNode("Channel definitions.", {
  list: arrayNode(
    "Channels.",
    objectNode("Channel.", {
      id: stringNode("Channel id.", { required: true }),
      kind: stringNode("Channel kind."),
      route: stringNode("Route."),
      approval: stringNode("Approval gate."),
    }),
  ),
})

const importsNode = objectNode("Import declarations.", {
  list: arrayNode(
    "Imports.",
    objectNode("Import source.", {
      source: stringNode("Import source.", { required: true }),
      optional: booleanNode("Optional import."),
      pin: stringNode("Pinned version."),
    }),
  ),
})

const reasoningPrivacyNode = objectNode("Reasoning privacy policy.", {
  store_reasoning: booleanNode("Store reasoning."),
  redact_chain_of_thought: booleanNode("Redact chain-of-thought."),
  summaries_only: booleanNode("Summaries only."),
})

const unsupportedFeaturePolicyNode = objectNode("Unsupported-feature policy.", {
  required: stringListNode("Required features.", "Feature."),
  optional: stringListNode("Optional features.", "Feature."),
  fail_closed: booleanNode("Fail closed."),
  on_missing: enumNode("Missing-feature action.", ["reject", "warn", "degrade"]),
})

const topLevelChildren = {
  version: literalNode("Runtime sentinel version.", ZYAL_RUNTIME_SENTINEL_VERSION, { required: true }),
  intent: literalNode("Intent marker.", "daemon", { required: true }),
  confirm: literalNode("Run confirmation literal.", "RUN_FOREVER", { required: true }),
  id: stringNode("Run identifier.", { required: true }),
  job: objectNode(
    "Primary task framing.",
    {
      name: stringNode("Human-readable job name.", { required: true }),
      objective: stringNode("Objective text.", { required: true }),
      risk: stringListNode("Risk notes.", "Risk note."),
    },
    { required: true, status: "runtime" },
  ),
  loop: objectNode("Loop policy.", {
    policy: enumNode("Loop policy.", ["once", "bounded", "forever"]),
    sleep: stringNode("Sleep interval."),
    continue_on: stringListNode("Signals that continue the loop.", "Signal."),
    pause_on: stringListNode("Signals that pause the loop.", "Signal."),
    circuit_breaker: loopBreakerNode,
  }),
  stop: objectNode("Host-evaluated stop conditions.", {
    all: arrayNode("Required stop conditions.", stopConditionNode, { required: true }),
    any: arrayNode("Optional stop conditions.", stopConditionNode),
  }),
  context: objectNode("Context retention policy.", {
    strategy: enumNode("Context strategy.", ["soft", "hard", "hybrid"]),
    compact_every: numberNode("Compact cadence."),
    hard_clear_every: numberNode("Hard-clear cadence."),
    preserve: stringListNode("Fields to preserve.", "Field name."),
  }),
  checkpoint: checkpointNode,
  tasks: taskNode,
  incubator: objectNode("Incubator policy.", {
    enabled: booleanNode("Enable incubator."),
    strategy: stringNode("Incubator strategy."),
    route_when: objectNode("Routing predicate.", {
      any: arrayNode("Any-match predicates.", objectNode("Routing predicate.", {
        repeated_attempts_gte: numberNode("Repeated attempts threshold."),
        no_progress_iterations_gte: numberNode("No-progress threshold."),
        risk_score_gte: numberNode("Risk threshold."),
        readiness_score_lt: numberNode("Readiness threshold."),
        touches_paths: stringListNode("Path patterns.", "Path."),
      })),
      all: arrayNode("All-match predicates.", objectNode("Routing predicate.", {
        repeated_attempts_gte: numberNode("Repeated attempts threshold."),
        no_progress_iterations_gte: numberNode("No-progress threshold."),
        risk_score_gte: numberNode("Risk threshold."),
        readiness_score_lt: numberNode("Readiness threshold."),
        touches_paths: stringListNode("Path patterns.", "Path."),
      })),
    }),
    exclude_when: objectNode("Exclusion predicate.", {
      any: arrayNode("Any-match exclusion predicates.", objectNode("Routing predicate.", {
        repeated_attempts_gte: numberNode("Repeated attempts threshold."),
        no_progress_iterations_gte: numberNode("No-progress threshold."),
        risk_score_gte: numberNode("Risk threshold."),
        readiness_score_lt: numberNode("Readiness threshold."),
        touches_paths: stringListNode("Path patterns.", "Path."),
      })),
      all: arrayNode("All-match exclusion predicates.", objectNode("Routing predicate.", {
        repeated_attempts_gte: numberNode("Repeated attempts threshold."),
        no_progress_iterations_gte: numberNode("No-progress threshold."),
        risk_score_gte: numberNode("Risk threshold."),
        readiness_score_lt: numberNode("Readiness threshold."),
        touches_paths: stringListNode("Path patterns.", "Path."),
      })),
    }),
    budget: objectNode("Incubator budget.", {
      max_passes_per_task: numberNode("Pass limit."),
      max_rounds_per_task: numberNode("Round limit."),
      max_active_tasks: numberNode("Active task limit."),
      max_parallel_idea_passes: numberNode("Parallel idea-pass limit."),
    }),
    scratch: objectNode("Scratch workspace.", {
      storage: stringNode("Scratch storage."),
      mirror: booleanNode("Mirror scratch state."),
      cleanup: stringNode("Cleanup policy."),
    }),
    cleanup: objectNode("Cleanup policy.", {
      summarize_to_task_memory: booleanNode("Summarize to task memory."),
      archive_artifacts: booleanNode("Archive artifacts."),
      delete_scratch: booleanNode("Delete scratch."),
      delete_unmerged_worktrees: booleanNode("Delete unmerged worktrees."),
    }),
    readiness: objectNode("Promotion readiness.", {
      promote_at: numberNode("Promotion threshold."),
      tests_identified_gte: numberNode("Test count threshold."),
      scope_bounded_gte: numberNode("Scope threshold."),
      plan_reviewed_gte: numberNode("Plan-review threshold."),
      prototype_validated_gte: numberNode("Prototype threshold."),
      rollback_known_gte: numberNode("Rollback-threshold."),
      affected_files_known_gte: numberNode("Affected-files threshold."),
      critical_objections_resolved_gte: numberNode("Critical objections threshold."),
      model_confidence_cap: numberNode("Confidence cap."),
    }),
    passes: arrayNode(
      "Incubator passes.",
      objectNode("Incubator pass.", {
        id: stringNode("Pass id.", { required: true }),
        type: stringNode("Pass type.", { required: true }),
        context: stringNode("Pass context."),
        reads: stringListNode("Read scope.", "Scope."),
        writes: stringNode("Write scope."),
        count: numberNode("Pass count."),
        agent: stringNode("Agent."),
        mcp_profile: stringNode("MCP profile."),
      }),
      { required: true },
    ),
    promotion: objectNode("Promotion policy.", {
      promote_at: numberNode("Promotion threshold."),
      require: stringListNode("Required artifacts.", "Artifact."),
      block_on: objectNode("Promotion blockers.", {
        unresolved_critical_objections_gte: numberNode("Critical-objection threshold."),
      }),
      on_promote: stringNode("Promotion action."),
      on_exhausted: stringNode("Exhausted action."),
    }),
  }),
  agents: objectNode("Agent orchestration.", {
    supervisor: objectNode("Supervisor agent.", {
      agent: stringNode("Supervisor agent name."),
    }),
    workers: arrayNode("Worker pool.", workersNode),
  }),
  mcp: objectNode("MCP profiles.", {
    profiles: recordNode("Named MCP profiles.", mcpProfileNode),
  }),
  permissions: permissionsNode,
  ui: objectNode("UI hints.", {
    theme: stringNode("Theme."),
    banner: stringNode("Banner text."),
  }),
  on: arrayNode("Signal handlers.", onHandlerNode),
  fan_out: fanOutNode,
  dispatch: objectNode(dispatchNode.description, dispatchNode.kind === "object" ? dispatchNode.children : {}, { status: "preview" }),
  guardrails: objectNode("Guardrail configuration.", {
    input: arrayNode("Input guardrails.", guardrailEntryNode),
    output: arrayNode("Output guardrails.", guardrailEntryNode),
    iteration: arrayNode("Iteration guardrails.", guardrailEntryNode),
  }),
  assertions: assertionsNode,
  retry: objectNode("Retry configuration.", {
    default: retryPolicyNode,
    overrides: objectNode("Retry overrides.", {
      shell_checks: retryPolicyNode,
      checkpoint: retryPolicyNode,
      worker_spawn: retryPolicyNode,
      stop_evaluation: retryPolicyNode,
    }),
  }),
  hooks: hooksNode,
  constraints: arrayNode("Constraint list.", constraintNode),
  workflow: workflowNode,
  memory: memoryNode,
  evidence: evidenceNode,
  approvals: approvalsNode,
  skills: skillsNode,
  sandbox: sandboxNode,
  security: securityNode,
  observability: observabilityNode,
  arming: armingNode,
  capabilities: capabilitiesNode,
  quality: objectNode(qualityNode.description, qualityNode.kind === "object" ? qualityNode.children : {}, { status: "preview" }),
  experiments: objectNode(experimentsNode.description, experimentsNode.kind === "object" ? experimentsNode.children : {}, { status: "preview" }),
  models: objectNode(modelsNode.description, modelsNode.kind === "object" ? modelsNode.children : {}, { status: "preview" }),
  budgets: budgetsNode,
  triggers: triggersNode,
  rollback: rollbackNode,
  promotion_gates: recordNode(
    "Promotion gate policy consumed by domain-specific reducers.",
    scalar("Reducer-specific gate value."),
    { status: "preview" },
  ),
  done: doneNode,
  repo_intelligence: objectNode(repoIntelNode.description, repoIntelNode.kind === "object" ? repoIntelNode.children : {}, { status: "preview" }),
  fleet: fleetNode,
  research: objectNode(researchNode.description, researchNode.kind === "object" ? researchNode.children : {}, { status: "preview" }),
  jankurai: jankuraiNode,
  taint: objectNode(taintNode.description, taintNode.kind === "object" ? taintNode.children : {}, { status: "preview" }),
  interaction: objectNode(interactionNode.description, interactionNode.kind === "object" ? interactionNode.children : {}, { status: "preview" }),
  interop: objectNode(interopNode.description, interopNode.kind === "object" ? interopNode.children : {}, { status: "preview" }),
  runtime: objectNode(runtimeNode.description, runtimeNode.kind === "object" ? runtimeNode.children : {}, { status: "preview" }),
  capability_negotiation: objectNode(
    capabilityNegotiationNode.description,
    capabilityNegotiationNode.kind === "object" ? capabilityNegotiationNode.children : {},
    { status: "preview" },
  ),
  memory_kernel: objectNode(memoryKernelNode.description, memoryKernelNode.kind === "object" ? memoryKernelNode.children : {}, { status: "preview" }),
  evidence_graph: objectNode(evidenceGraphNode.description, evidenceGraphNode.kind === "object" ? evidenceGraphNode.children : {}, { status: "preview" }),
  trust: objectNode(trustNode.description, trustNode.kind === "object" ? trustNode.children : {}, { status: "preview" }),
  requirements: objectNode(requirementsNode.description, requirementsNode.kind === "object" ? requirementsNode.children : {}, { status: "preview" }),
  evaluation: objectNode(evaluationNode.description, evaluationNode.kind === "object" ? evaluationNode.children : {}, { status: "preview" }),
  release: objectNode(releaseNode.description, releaseNode.kind === "object" ? releaseNode.children : {}, { status: "preview" }),
  roles: objectNode(rolesNode.description, rolesNode.kind === "object" ? rolesNode.children : {}, { status: "preview" }),
  channels: objectNode(channelsNode.description, channelsNode.kind === "object" ? channelsNode.children : {}, { status: "preview" }),
  imports: objectNode(importsNode.description, importsNode.kind === "object" ? importsNode.children : {}, { status: "preview" }),
  reasoning_privacy: objectNode(reasoningPrivacyNode.description, reasoningPrivacyNode.kind === "object" ? reasoningPrivacyNode.children : {}, { status: "preview" }),
  unsupported_feature_policy: objectNode(
    unsupportedFeaturePolicyNode.description,
    unsupportedFeaturePolicyNode.kind === "object" ? unsupportedFeaturePolicyNode.children : {},
    { status: "preview" },
  ),
} satisfies Record<string, ZyalSchemaNode>

export const ZYAL_SCHEMA_SPEC = {
  contractVersion: ZYAL_CONTRACT_VERSION,
  runtimeSentinelVersion: ZYAL_RUNTIME_SENTINEL_VERSION,
  researchBlockVersion: ZYAL_RESEARCH_BLOCK_VERSION,
  root: objectNode("Canonical ZYAL runbook schema.", topLevelChildren, {
    status: "generated",
    notes: "Unknown keys are rejected before schema decode; this tree is the canonical human-facing registry.",
  }),
} as const

export const ZYAL_TOP_LEVEL_KEYS = Object.keys(topLevelChildren).sort()

function validateNode(node: ZyalSchemaNode, value: unknown, path: string) {
  if (node.kind === "array") {
    if (!Array.isArray(value)) return
    value.forEach((item, index) => validateNode(node.item, item, `${path}[${index}]`))
    return
  }
  if (node.kind === "record") {
    if (typeof value !== "object" || value === null || Array.isArray(value)) return
    for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
      validateNode(node.value, child, `${path}.${key}`)
    }
    return
  }
  if (node.kind !== "object") return
  if (typeof value !== "object" || value === null || Array.isArray(value)) return
  const record = value as Record<string, unknown>
  for (const key of Object.keys(record)) {
    const child = node.children[key]
    if (!child) {
      throw new Error(path === "" ? `Unknown ZYAL top-level key: ${key}` : `Unknown ZYAL key: ${path}.${key}`)
    }
    validateNode(child, record[key], path === "" ? key : `${path}.${key}`)
  }
}

export function assertKnownZyalKeys(value: unknown) {
  validateNode(ZYAL_SCHEMA_SPEC.root, value, "")
}

export function renderZyalSpecMarkdown() {
  const lines: string[] = []
  lines.push("<!-- Generated by: bun --cwd packages/jekko ./script/generate-zyal-spec.ts --write -->")
  lines.push("<!-- DO NOT EDIT BY HAND — re-run the generator to update this file. -->")
  lines.push("")
  lines.push("# ZYAL Spec")
  lines.push("")
  lines.push("Canonical human-facing schema for the ZYAL `2.6.0` contract.")
  lines.push("")
  lines.push("## Metadata")
  lines.push("")
  lines.push("| Item | Value |")
  lines.push("|---|---|")
  lines.push(`| Contract version | \`${ZYAL_SCHEMA_SPEC.contractVersion}\` |`)
  lines.push(`| Runtime sentinel | \`${ZYAL_SCHEMA_SPEC.runtimeSentinelVersion}\` |`)
  lines.push(`| Research block version | \`${ZYAL_SCHEMA_SPEC.researchBlockVersion}\` |`)
  lines.push("")
  lines.push("## Runtime Status Legend")
  lines.push("")
  lines.push("| Status | Meaning |")
  lines.push("|---|---|")
  lines.push("| `runtime` | Shipped parser/runtime surface |")
  lines.push("| `preview` | Parsed and previewed, but runtime enforcement is partial or host-enforced |")
  lines.push("| `generated` | Produced from the registry or another source of truth |")
  lines.push("| `compat` | Backward-compatibility shim or alias |")
  lines.push("")
  lines.push("## Compatibility Policy")
  lines.push("")
  lines.push("- Keep `ZYAL_CONTRACT_VERSION` stable for docs-only or example-only changes.")
  lines.push("- Add new schema keys only with an explicit contract bump and matching parser/runtime work.")
  lines.push("- Unknown keys are rejected before schema decode so drift is loud.")
  lines.push("- `VERSION.md` and `CHANGELOG.md` remain metadata; this file is the canonical schema reference.")
  lines.push("")
  lines.push("## Release Checklist")
  lines.push("")
  lines.push("- Regenerate this file with `bun --cwd packages/jekko ./script/generate-zyal-spec.ts --write`.")
  lines.push("- Run `bun --cwd packages/jekko test src/agent-script/schema-spec.test.ts src/agent-script/parser.test.ts`.")
  lines.push("- Run the repo-level ZYAL proof lanes called out in `agent/test-map.json`.")
  lines.push("- Confirm the docs example table and tracked `.zyal` files still parse.")
  lines.push("")
  lines.push("## Top-Level Blocks")
  lines.push("")
  lines.push("| Key | Kind | Required | Status | Description |")
  lines.push("|---|---|---|---|---|")
  for (const [key, node] of Object.entries(ZYAL_SCHEMA_SPEC.root.children)) {
    lines.push(`| \`${key}\` | ${node.kind} | ${node.required ? "yes" : "no"} | ${node.status} | ${escapeMarkdown(node.description)} |`)
  }
  lines.push("")
  lines.push("## Schema Tree")
  lines.push("")
  for (const [key, node] of Object.entries(ZYAL_SCHEMA_SPEC.root.children)) {
    renderNode(lines, node, `\`${key}\``, 0)
  }
  lines.push("")
  lines.push("## Canonical Example Runbooks")
  lines.push("")
  lines.push("| File | Purpose |")
  lines.push("|---|---|")
  lines.push("| [`18-semantic-bug-finder-basic.zyal`](examples/18-semantic-bug-finder-basic.zyal) | Single-worker bug-finder with fail-first gates, PR hook, and hard resets |")
  lines.push("| [`19-semantic-bug-finder-advanced.zyal`](examples/19-semantic-bug-finder-advanced.zyal) | Weighted selection, incubator routing, taint, and proof-map-backed verification |")
  lines.push("| [`20-semantic-bug-finder-ultra.zyal`](examples/20-semantic-bug-finder-ultra.zyal) | Full-power dispatch/fleet/research/memory/taint loop with deep-dive lanes |")
  lines.push("| [`21-semantic-improvement-finder-simple.zyal`](examples/21-semantic-improvement-finder-simple.zyal) | Single-worker improvement loop with KPI evidence, behavior-equivalence proof, and rollback |")
  lines.push("| [`22-semantic-improvement-finder-advanced.zyal`](examples/22-semantic-improvement-finder-advanced.zyal) | Weighted improvement triage with Jankurai proof routing, experiment lanes, memory, taint, and rollback |")
  lines.push("| [`23-semantic-improvement-finder-insane.zyal`](examples/23-semantic-improvement-finder-insane.zyal) | Full-power improvement fleet with dispatch, research, sandbox, security, approvals, and critic review |")
  lines.push("| [`24-semantic-feature-maker-simple.zyal`](examples/24-semantic-feature-maker-simple.zyal) | Single-worker feature recommendation with repo intelligence, evidence graph, first slice, and rollback |")
  lines.push("| [`25-semantic-feature-maker-advanced.zyal`](examples/25-semantic-feature-maker-advanced.zyal) | Weighted feature triage with repo intelligence, experiments, research, taint, and proof-map-backed review |")
  lines.push("| [`26-semantic-feature-maker-insane.zyal`](examples/26-semantic-feature-maker-insane.zyal) | Full-power feature-maker fleet with dispatch, workflow, approvals, research, sandbox, security, and critic review |")
  lines.push("")
  lines.push("## Preview Notes")
  lines.push("")
  lines.push("- Parser validation happens before schema decode so malformed keys are rejected early.")
  lines.push("- `quality`, `experiments`, `research`, and `taint` are preview-heavy surfaces whose docs intentionally over-explain the host contract.")
  lines.push("- `jankurai`, `fleet`, `dispatch`, and `sandbox` are the main routing and containment primitives used by the shipped examples.")
  lines.push("- `repo_intelligence`, `evidence_graph`, `workflow`, and `approvals` anchor the feature-maker runbooks.")
  lines.push("")
  lines.push("_Generated from `packages/jekko/src/agent-script/schema-spec.ts`._")
  lines.push("")
  return lines.join("\n")
}

function renderNode(lines: string[], node: ZyalSchemaNode, path: string, depth: number) {
  const indent = "  ".repeat(depth)
  const required = node.required ? "required" : "optional"
  const extras = [
    `kind: ${node.kind}`,
    `status: ${node.status}`,
    required,
    node.kind === "enum" ? `values: ${node.values.map((value) => `\`${value}\``).join(", ")}` : null,
    node.notes ? `notes: ${node.notes}` : null,
  ]
    .filter(Boolean)
    .join("; ")
  lines.push(`${indent}- ${path} - ${node.description} (${extras})`)
  if (node.kind === "object") {
    for (const [key, child] of Object.entries(node.children)) {
      renderNode(lines, child, `\`${key}\``, depth + 1)
    }
    return
  }
  if (node.kind === "record") {
    renderNode(lines, node.value, "`<record value>`", depth + 1)
    return
  }
  if (node.kind === "array") {
    renderNode(lines, node.item, "`<array item>`", depth + 1)
  }
}

function escapeMarkdown(value: string) {
  return value.replace(/\|/g, "\\|")
}
