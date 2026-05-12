// jankurai:allow HLT-000-SCORE-DIMENSION reason=examples-file-contains-parallel-zyal-templates-by-design expires=2027-01-01
import { ZYAL_RUNTIME_SENTINEL_VERSION } from "./version"

export type ZyalExample = {
  readonly id: string
  readonly title: string
  readonly description: string
  readonly text: string
}

function wrap(id: string, body: string) {
  return `<<<ZYAL ${ZYAL_RUNTIME_SENTINEL_VERSION}:daemon id=${id}>>>\nversion: v1\nintent: daemon\nconfirm: RUN_FOREVER\n\n${body.trim()}\n<<<END_ZYAL id=${id}>>>\nZYAL_ARM RUN_FOREVER id=${id}\n`
}

function example(id: string, title: string, description: string, body: string): ZyalExample {
  return { id, title, description, text: wrap(id, body) }
}

export const ZYAL_EXAMPLES: Record<string, ZyalExample> = {
  "jankurai-clean-worktree": example(
    "jankurai-clean-worktree",
    "Jankurai dirty worktree hardening",
    "Audit uncommitted work, fix regressions, verify, and checkpoint.",
    `job:
  name: "Jankurai dirty worktree hardening"
  objective: |
    Investigate all uncommitted work. Fix regressions. Run confidence checks.
    Commit and push completed verified work.

loop:
  policy: forever
  sleep: 5s
  continue_on: [assistant_stop, max_steps, compaction]
  pause_on: [checkpoint_failed, no_progress]
  circuit_breaker:
    max_consecutive_errors: 3
    on_trip: pause

stop:
  all:
    - git_clean:
        allow_untracked: false

checkpoint:
  when: after_verified_change
  noop_if_clean: true
  verify:
    - command: "bun test --timeout 30000 packages/jekko/test/session/daemon-runtime.test.ts"
  git:
    add: ["."]
    commit_message: "jankurai: daemon verified work item"
    push: allow

permissions:
  shell: allow
  edit: allow
  git_commit: allow
  git_push: allow
  workers: allow
  mcp: allow

ui:
  theme: jekko-gold
  banner: forever`,
  ),

  "until-file-contains-done": example(
    "until-file-contains-done",
    "Wait for file sentinel",
    "Loop until a file contains the required text.",
    `job:
  name: "Wait for DONE"
  objective: "Keep checking the target file until it contains DONE."

loop:
  policy: forever
  sleep: 2s

stop:
  all:
    - shell:
        command: "test -f target.txt && grep -q DONE target.txt"
        timeout: 5s`,
  ),

  "fix-until-tests-pass": example(
    "fix-until-tests-pass",
    "Fix until tests pass",
    "Repair code until the test command succeeds.",
    `job:
  name: "Fix until tests pass"
  objective: "Make the failing tests pass without widening permissions."

stop:
  all:
    - shell:
        command: "bun test --timeout 30000"
        timeout: 30s
        assert:
          exit_code: 0`,
  ),

  "multi-worker-audit": example(
    "multi-worker-audit",
    "Multi worker audit",
    "Split work between workers and verify results.",
    `job:
  name: "Audit and repair"
  objective: "Inspect all findings, delegate isolated fixes, and verify the tree."

agents:
  workers:
    - id: fixers
      count: 3
      agent: build
      isolation: git_worktree

stop:
  all:
    - git_clean:
        allow_untracked: false`,
  ),

  "mcp-health-loop": example(
    "mcp-health-loop",
    "MCP health loop",
    "Keep running until required MCP servers are healthy.",
    `job:
  name: "MCP health loop"
  objective: "Retry until the required MCP profile is healthy."

mcp:
  profiles:
    default:
      servers: ["filesystem"]
      tools: ["read"]
      resources: ["/tmp"]

stop:
  all:
    - shell:
        command: "true"
        timeout: 1s`,
  ),

  "hard-task-incubator": example(
    "hard-task-incubator",
    "Hard task incubator",
    "Route repeated failures and risky paths into bounded incubator passes.",
    `job:
  name: "ZYAL incubator hard task loop"
  objective: "Mature risky daemon runtime tasks before implementation."

loop:
  policy: forever
  sleep: 5s

stop:
  all:
    - git_clean:
        allow_untracked: false

incubator:
  enabled: true
  strategy: generate_pool_strengthen
  route_when:
    any:
      - repeated_attempts_gte: 2
      - no_progress_iterations_gte: 2
      - risk_score_gte: 0.7
      - touches_paths:
          - "packages/jekko/src/session/**"
          - "db/migrations/**"
  exclude_when:
    any:
      - readiness_score_lt: 0.25
      - touches_paths:
          - "docs/**"
  budget:
    max_passes_per_task: 7
    max_rounds_per_task: 2
    max_active_tasks: 1
    max_parallel_idea_passes: 3
  scratch:
    storage: sqlite
    mirror: true
    cleanup: summarize_and_archive
  cleanup:
    summarize_to_task_memory: true
    archive_artifacts: true
    delete_scratch: true
    delete_unmerged_worktrees: true
  readiness:
    promote_at: 0.78
    tests_identified_gte: 1
    scope_bounded_gte: 1
    plan_reviewed_gte: 1
    prototype_validated_gte: 1
    rollback_known_gte: 1
    affected_files_known_gte: 1
    critical_objections_resolved_gte: 1
    model_confidence_cap: 0.06
  passes:
    - id: blind-scout
      type: scout
      context: blind
      writes: scratch_only
    - id: ideas
      type: idea
      context: blind
      count: 3
      writes: scratch_only
    - id: strengthen-best
      type: strengthen
      context: inherit
      reads: [current_best]
      writes: scratch_only
    - id: red-team
      type: critic
      context: pool
      reads: [candidate_pool]
      writes: scratch_only
    - id: synthesize
      type: synthesize
      context: pool
      reads: [candidate_pool, objections]
      writes: scratch_only
    - id: promotion-review
      type: promotion_review
      context: promotion
      writes: scratch_only
  promotion:
    promote_at: 0.78
    require:
      - problem_statement
      - current_best_plan
      - verification_strategy
      - risk_review
    block_on:
      unresolved_critical_objections_gte: 1
    on_promote: move_to_ready_queue
    on_exhausted: park_with_summary`,
  ),

  "concept-strengthening-pool": example(
    "concept-strengthening-pool",
    "Concept strengthening pool",
    "Generate several ideas, critique the pool, then synthesize the strongest packet.",
    `job:
  name: "Strengthen design concepts"
  objective: "Explore competing ideas and promote only evidence-backed implementation packets."

stop:
  all:
    - shell:
        command: "true"
        timeout: 1s

incubator:
  enabled: true
  strategy: generate_pool_strengthen
  route_when:
    any:
      - readiness_score_lt: 0.62
      - risk_score_gte: 0.7
  exclude_when:
    all:
      - repeated_attempts_gte: 0
  budget:
    max_passes_per_task: 6
    max_rounds_per_task: 1
    max_active_tasks: 1
    max_parallel_idea_passes: 3
  scratch:
    storage: sqlite
    mirror: true
    cleanup: summarize_and_archive
  cleanup:
    summarize_to_task_memory: true
    archive_artifacts: true
  passes:
    - id: ideas
      type: idea
      context: blind
      count: 3
      writes: scratch_only
    - id: critic
      type: critic
      context: pool
      reads: [candidate_pool]
      writes: scratch_only
    - id: strengthen
      type: strengthen
      context: strengthen
      reads: [current_best, objections]
      writes: scratch_only
    - id: synthesize
      type: synthesize
      context: pool
      reads: [candidate_pool, objections]
      writes: scratch_only
  promotion:
    promote_at: 0.8
    require: [problem_statement, current_best_plan, verification_strategy, risk_review]
    on_promote: move_to_ready_queue
    on_exhausted: park_with_summary`,
  ),

  "safe-prototype-promotion": example(
    "safe-prototype-promotion",
    "Safe prototype promotion",
    "Allow prototype exploration only in an isolated git worktree.",
    `job:
  name: "Prototype before promotion"
  objective: "Create isolated evidence before promoting a risky task."

stop:
  all:
    - git_clean:
        allow_untracked: false

incubator:
  enabled: true
  strategy: bounded_passes
  route_when:
    any:
      - touches_paths:
          - "packages/jekko/src/session/**"
      - repeated_attempts_gte: 2
  exclude_when:
    any:
      - readiness_score_lt: 0.35
  budget:
    max_passes_per_task: 5
    max_rounds_per_task: 1
    max_active_tasks: 1
    max_parallel_idea_passes: 1
  scratch:
    storage: sqlite
    mirror: true
    cleanup: summarize_and_archive
  cleanup:
    summarize_to_task_memory: true
    delete_scratch: true
  readiness:
    promote_at: 0.72
    tests_identified_gte: 1
    scope_bounded_gte: 1
  passes:
    - id: scout
      type: scout
      context: blind
      writes: scratch_only
    - id: plan
      type: synthesize
      context: inherit
      writes: scratch_only
    - id: prototype
      type: prototype
      context: inherit
      reads: [current_best]
      writes: isolated_worktree
    - id: review
      type: promotion_review
      context: promotion
      writes: scratch_only
  promotion:
    promote_at: 0.78
    require: [problem_statement, current_best_plan, verification_strategy, risk_review]
    block_on:
      unresolved_critical_objections_gte: 1
    on_promote: move_to_ready_queue
    on_exhausted: park_with_summary`,
  ),

  "normal-user-incubator": example(
    "normal-user-incubator",
    "Normal user incubator preset",
    "A small bounded incubator that only wakes up for visibly risky work.",
    `job:
  name: "User-facing hard task helper"
  objective: "Stay in normal chat unless the task becomes clearly risky or repetitive."

stop:
  all:
    - shell:
        command: "true"
        timeout: 1s

incubator:
  enabled: true
  strategy: bounded_passes
  route_when:
    any:
      - repeated_attempts_gte: 2
      - no_progress_iterations_gte: 2
      - risk_score_gte: 0.65
  exclude_when:
    any:
      - readiness_score_lt: 0.2
  budget:
    max_passes_per_task: 4
    max_rounds_per_task: 1
    max_active_tasks: 1
    max_parallel_idea_passes: 2
  scratch:
    storage: sqlite
    mirror: true
    cleanup: summarize_and_archive
  cleanup:
    summarize_to_task_memory: true
    archive_artifacts: true
    delete_scratch: true
  readiness:
    promote_at: 0.7
    tests_identified_gte: 1
    scope_bounded_gte: 1
    plan_reviewed_gte: 1
  passes:
    - id: scout
      type: scout
      context: blind
      writes: scratch_only
    - id: synthesize
      type: synthesize
      context: pool
      reads: [candidate_pool]
      writes: scratch_only
    - id: review
      type: promotion_review
      context: promotion
      writes: scratch_only
  promotion:
    promote_at: 0.7
    require:
      - problem_statement
      - current_best_plan
      - verification_strategy
      - risk_review
    on_promote: move_to_ready_queue
    on_exhausted: park_with_summary`,
  ),

  "full-v1.1-kitchen-sink": example(
    "full-v1.1-kitchen-sink",
    "Full v1.1 kitchen sink",
    "All v1.1 capabilities: on handlers, fan-out, guardrails, retry, hooks, constraints.",
    `job:
  name: "v1.1 kitchen sink"
  objective: "Demonstrate all v1.1 ZYAL capabilities."

loop:
  policy: forever
  sleep: 5s

stop:
  all:
    - git_clean:
        allow_untracked: false

on:
  - signal: no_progress
    count_gte: 3
    do:
      - switch_agent: plan
  - signal: error
    message_contains: "ENOMEM"
    do:
      - pause: true
      - notify: "Out of memory detected"
  - signal: checkpoint_failed
    do:
      - incubate_current_task: true

guardrails:
  input:
    - name: no-force-push
      deny_patterns:
        - "git push --force"
        - "git push -f"
      action: block
    - name: no-drop-table
      deny_patterns:
        - "DROP TABLE"
        - "DROP DATABASE"
      action: block
  output:
    - name: type-safety
      shell: "npx tsgo --noEmit 2>&1 | tail -1"
      on_fail: retry
      max_retries: 3
  iteration:
    - name: diff-size
      shell: "git diff --stat | wc -l"
      on_fail: warn

retry:
  default:
    max_attempts: 3
    backoff: exponential
    initial_delay: 2s
    max_delay: 30s
    jitter: true
  overrides:
    shell_checks:
      max_attempts: 5
      backoff: linear
      initial_delay: 1s

hooks:
  on_start:
    - run: "git fetch origin main"
  before_iteration:
    - run: "git rebase origin/main --autostash"
      on_fail: warn
  after_checkpoint:
    - run: "echo 'checkpoint committed'"
      on_fail: continue

constraints:
  - name: test-count-stable
    check:
      shell: "bun test --dry-run 2>&1 | grep -c test"
    baseline: capture_on_start
    invariant: gte_baseline
    on_violation: pause
  - name: no-binary-files
    check:
      shell: "git diff --cached --diff-filter=A --name-only | xargs file 2>/dev/null | grep -c binary || echo 0"
    invariant: equals_zero
    on_violation: block

permissions:
  shell: allow
  edit: allow
  git_commit: allow
  git_push: allow

ui:
  theme: jekko-gold
  banner: forever`,
  ),

  "parallel-fan-out-audit": example(
    "parallel-fan-out-audit",
    "Parallel fan-out audit",
    "Split audit tasks across parallel workers and merge results.",
    `job:
  name: "Parallel audit"
  objective: "Decompose audit into parallel subtasks, merge results."

stop:
  all:
    - git_clean:
        allow_untracked: false

fan_out:
  strategy: map_reduce
  split:
    items: ["lint", "typecheck", "unit-tests", "integration-tests"]
  worker:
    agent: build
    isolation: git_worktree
    timeout: 10m
    max_parallel: 4
  reduce:
    strategy: merge_all
  on_partial_failure: continue

hooks:
  on_start:
    - run: "bun install"`,
  ),

  "guardrailed-ci-daemon": example(
    "guardrailed-ci-daemon",
    "Guardrailed CI daemon",
    "Automated CI fixes with safety guardrails and structured assertions.",
    `job:
  name: "Safe CI fixer"
  objective: "Fix CI failures with guardrails preventing dangerous operations."

loop:
  policy: forever
  sleep: 10s

stop:
  all:
    - shell:
        command: "bun test --timeout 30000"
        timeout: 60s
        assert:
          exit_code: 0

guardrails:
  input:
    - name: no-force-push
      deny_patterns: ["git push --force", "git push -f"]
      action: block
    - name: no-secrets
      # jankurai:allow HLT-010-SECRET-SPRAWL reason=deny-pattern-list-not-leaked-token expires=2026-12-31
      deny_patterns: ["example-secret-", "example-aws-key-"]
      scope: tool_output
      action: block
  output:
    - name: lint-check
      shell: "bun run lint 2>&1 | tail -1"
      on_fail: retry
      max_retries: 2

assertions:
  require_structured_output: true
  on_invalid: retry
  max_retries: 2

constraints:
  - name: migration-stability
    check:
      shell: "ls db/migrations/ 2>/dev/null | wc -l"
    baseline: capture_on_start
    invariant: gte_baseline
    on_violation: abort

permissions:
  shell: allow
  edit: allow
  git_commit: allow`,
  ),

  "v2-workflow-driven": example(
    "v2-workflow-driven",
    "Workflow-driven feature implementation",
    "Evidence-gated state machine with memory, approvals, and proof bundles.",
    `job:
  name: v2-workflow-feature
  objective: Implement a feature using workflow-driven orchestration
  risk: medium

stop:
  all:
    - git_clean: {}

workflow:
  type: state_machine
  initial: discover
  states:
    discover:
      agent: plan
      writes: scratch_only
      produces:
        - impact_map
        - task_graph
      transitions:
        - to: plan
          when:
            evidence_exists: impact_map
    plan:
      agent: plan
      writes: scratch_only
      requires:
        - impact_map
      produces:
        - change_plan
        - test_plan
        - rollback_plan
      approval: plan_review
      transitions:
        - to: implement
          when:
            approval_granted: plan_review
    implement:
      agent: build
      writes: isolated_worktree
      requires:
        - change_plan
        - test_plan
      produces:
        - code_changes
        - test_results
      transitions:
        - to: verify
          when:
            all_checks_pass: true
        - to: plan
          when:
            checks_failed: true
    verify:
      agent: build
      writes: none
      requires:
        - test_results
      produces:
        - verification_report
      transitions:
        - to: promote
          when:
            all_checks_pass: true
    promote:
      terminal: true
      approval: merge_review

memory:
  stores:
    task_context:
      scope: task
      retention: until_promotion
      max_entries: 100
      write_policy: append_only
      read_policy: inject_at_start
    lessons_learned:
      scope: global
      retention: permanent
      write_policy: upsert
      read_policy: on_demand
      searchable: true
  redaction:
    # jankurai:allow HLT-010-SECRET-SPRAWL reason=example-redaction-pattern-not-leaked-token expires=2027-01-01
    patterns:
      - "example-secret-*"
      - "example-aws-key-*"
    action: mask
  provenance:
    track_source: true
    hash_chain: true

evidence:
  require_before_promote:
    - type: test_results
      must_pass: true
    - type: affected_files
      must_be_known: true
    - type: rollback_plan
      must_exist: true
    - type: risk_delta
      max_increase: 0.1
  bundle_format: json
  sign: sha256
  archive: true

approvals:
  gates:
    plan_review:
      required_role: tech_lead
      timeout: 24h
      on_timeout: pause
      decisions:
        - approve
        - reject
        - edit
      auto_approve_if:
        risk_score_lt: 0.3
        all_checks_pass: true
    merge_review:
      required_role: code_owner
      require_evidence:
        - test_results
        - rollback_plan
  escalation:
    chain:
      - tech_lead
      - staff_engineer
    auto_escalate_after: 48h

permissions:
  shell: allow
  edit: allow
  git_commit: allow
  git_push: deny`,
  ),

  "control-plane-preview": example(
    "control-plane-preview",
    "Control-plane preview contract",
    "Preview-only control-plane blocks with fail-closed unsupported feature policy.",
    `job:
  name: "Control-plane preview"
  objective: |
    Demonstrate the preview-only control-plane blocks without granting any
    runtime execution surface. The host should parse, preview, and summarize
    these contracts before any arming decision.
  risk:
    - Preview contract only; no runtime execution requested

stop:
  all:
    - git_clean:
        allow_untracked: false

arming:
  preview_hash_required: true
  host_nonce_required: true
  reject_inside_code_fence: true
  reject_from: [assistant_output, tool_output, web_content, mcp_resource, issue_comment, repo_files]
  accepted_origins: [trusted_user_message, signed_cli_input, host_ui_button]
  preview_expires_after: 10m
  arm_token_single_use: true
  bound_to: ["script_hash", "user_id", "repo", "branch"]

interop:
  protocols:
    - name: mcp
      target: host_tools
      version: v1
      notes: "Read-only host tool bridge"
    - name: a2a
      target: worker_fleet
      version: v0
      notes: "Preview negotiation only"
  adapters:
    - zyal-daemon
    - jnoccio
  compile_to:
    - run_card
    - capability_matrix
    - preview_bundle

runtime:
  mode: preview
  workspace: isolated_worktree
  network: deny
  env: [JEKKO_HOME, RUN_ID]
  resources:
    cpu: "2"
    memory: "4Gi"
    disk: "1Gi"
    processes: 8

capability_negotiation:
  host: jekko-daemon
  required: [interop, runtime, memory_kernel, evidence_graph]
  optional: [release, roles, channels, unsupported_preview_hooks]
  fail_closed: true
  degrade_to: preview_card

memory_kernel:
  stores:
    task_context:
      scope: task
      retention: until_promotion
      searchable: true
    design_notes:
      scope: global
      retention: permanent
      searchable: true
  redaction:
    # jankurai:allow HLT-010-SECRET-SPRAWL reason=example-redaction-pattern-not-leaked-token expires=2027-01-01
    patterns: ["example-secret-*", "example-gh-token-*", "example-aws-key-*"]
    action: mask
  provenance:
    track_source: true
    hash_chain: true

evidence_graph:
  nodes:
    preview_bundle:
      type: proof_bundle
      required: true
    runtime_contract:
      type: host_contract
      required: true
    release_candidate:
      type: host_gate
  edges:
    - from: preview_bundle
      to: runtime_contract
      kind: validates
    - from: runtime_contract
      to: release_candidate
      kind: gates
  merge_witness: preview_bundle

trust:
  zones:
    trusted_user_prompt:
      paths: ["prompt", "cli"]
      taint: clean
      require_approval: false
    assistant_output:
      paths: ["assistant", "tool"]
      taint: tainted
      require_approval: true
  on_taint: pause
  notes: "Preview contracts do not inherit trust from assistant output."

taint:
  default_label: external
  labels:
    trusted_user:
      rank: high
      notes: "Direct human operator input."
    external:
      rank: untrusted
      notes: "Web, issue, tool, MCP, or remote-agent content."
    hostile_instruction:
      rank: hostile
      notes: "Detected prompt-injection or delegated-authority text."
  forbid:
    - from: [external, hostile_instruction]
      cannot: [arm, approve, grant_capability, exec_shell, expose_secret]
      unless: [human_review, signed_sanitizer]
  prompt_injection:
    detect_patterns:
      - "[Ii]gnore (all )?previous instructions"
      - "[Rr]eveal (the )?(secret|token|key)"
    on_detect: pause
    scan_sources: [web_content, issue_comment, mcp_resource, tool_output]

requirements:
  must:
    - parse_successfully
    - summarize_all_preview_blocks
    - fail_closed_for_unknown_required_features
  should:
    - keep_runtime_mode_preview_only
    - keep_network_denied
  avoid:
    - runtime_side_effects
    - code_fence_wrapping

evaluation:
  metrics:
    - name: parse_latency_ms
      command: "bun test packages/jekko/src/agent-script/parser.test.ts"
      threshold: 5000
    - name: preview_summary_complete
      command: "bun run --filter @jekko-ai/jekko typecheck"
      threshold: 1
  compare: baseline

release:
  channel: preview
  version: 0.0.0-preview
  gates: [parse_success, preview_summary_complete]
  notes: "Preview-only contract for schema and parser coverage."

roles:
  list:
    - id: host
      agent: plan
      permissions: [read, preview]
      description: "Host evaluates and previews the contract."
    - id: reviewer
      agent: critic
      permissions: [read]
      description: "Human or critic reviews the preview bundle."

channels:
  list:
    - id: prompt
      kind: input
      route: prompt_preview
      approval: review_required
    - id: daemon
      kind: status
      route: daemon_preview
    - id: report
      kind: artifact
      route: preview_bundle

imports:
  list:
    - source: "docs/ZYAL/examples/08-full-power-runbook.zyal"
      optional: true
    - source: "zyal://stdlib/control-plane-preview@1"
      optional: true
      pin: "1"

reasoning_privacy:
  store_reasoning: false
  redact_chain_of_thought: true
  summaries_only: true

unsupported_feature_policy:
  required: [interop, runtime, capability_negotiation, memory_kernel, evidence_graph, trust, taint, requirements, evaluation, release, roles, channels, imports, reasoning_privacy]
  optional: [unsupported_preview_hooks]
  fail_closed: true
  on_missing: reject`,
  ),

  "jankurai-continuous-repair": example(
    "jankurai-continuous-repair",
    "Jankurai continuous repair",
    "Host-enforced jankurai audit, repair-plan ingestion, verification, commit, and push.",
    `job:
  name: "Jankurai continuous repair"
  objective: "Run low-risk jankurai quick wins with ten workers until audit findings are gone and git is clean."

loop:
  policy: forever
  sleep: 5s
  continue_on: [assistant_stop, max_steps, compaction, checkpoint_failed]

fleet:
  max_workers: 10
  isolation: git_worktree

agents:
  workers:
    - id: jankurai-low-risk
      count: 10
      agent: build
      isolation: git_worktree

jankurai:
  enabled: true
  root: "."
  audit:
    mode: advisory
    json: target/jankurai/repo-score.json
    md: target/jankurai/repo-score.md
    repair_queue_jsonl: target/jankurai/repair-queue.jsonl
    no_score_history: true
  repair_plan:
    enabled: true
    json: target/jankurai/repair-plan.json
    md: target/jankurai/repair-plan.md
  task_source: repair_plan
  selection:
    order: quick_wins_first
    randomize_ties: true
    max_risk: low
    skip_human_review_required: true
    defer_rules: [HLT-010-SECRET-SPRAWL]
  verification:
    require_clean_start: true
    require_clean_after_checkpoint: true
    proof_from_test_map: true
    commands: ["just fast"]
    audit_delta: no_new_findings
    rollback_unverified: true

stop:
  all:
    - shell:
        command: "jankurai audit . --mode advisory --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md --no-score-history >/dev/null 2>&1 && jq -e '(.findings // []) | length == 0' target/jankurai/repo-score.json"
        timeout: 10m
    - git_clean:
        allow_untracked: false

checkpoint:
  when: after_verified_change
  noop_if_clean: true
  verify:
    - command: "just fast"
      timeout: 20m
  git:
    add: ["."]
    commit_message: "jankurai: verified low-risk repair"
    push: allow

permissions:
  shell: allow
  edit: allow
  git_commit: allow
  git_push: allow
  workers: allow

unsupported_feature_policy:
  required: [jankurai]
  fail_closed: true
  on_missing: reject`,
  ),

  "jankurai-porting-advanced": example(
    "jankurai-porting-advanced",
    "Jankurai advanced porting",
    "Jankurai repair with incubator routing, regression checks, memory, evidence, taint, and metrics.",
    `job:
  name: "Jankurai advanced porting loop"
  objective: "Continuously port and repair jankurai findings with branch/main regression checks and incubator routing."

loop:
  policy: forever
  sleep: 5s
  continue_on: [assistant_stop, max_steps, compaction, checkpoint_failed]

fleet:
  max_workers: 10
  isolation: hybrid
  jnoccio:
    enabled: true
    register_workers: true
    max_instances: 10

agents:
  workers:
    - id: jankurai-repair
      count: 10
      agent: build
      isolation: git_worktree

jankurai:
  enabled: true
  audit:
    mode: advisory
    json: target/jankurai/repo-score.json
    md: target/jankurai/repo-score.md
    repair_queue_jsonl: target/jankurai/repair-queue.jsonl
    sarif: target/jankurai/jankurai.sarif
    no_score_history: true
  repair_plan:
    enabled: true
    json: target/jankurai/repair-plan.json
    md: target/jankurai/repair-plan.md
  task_source: repair_plan
  selection:
    order: quick_wins_first
    max_risk: low
    skip_human_review_required: true
    incubate_risk_at: medium
    defer_rules: [HLT-010-SECRET-SPRAWL, HLT-021-DESTRUCTIVE-MIGRATION]
    incubate_rules: [HLT-006-DIRECT-DB-WRONG-LAYER, HLT-023-INPUT-BOUNDARY-GAP]
  regression:
    main_ref: origin/main
    compare_every_iterations: 5
    mode: advisory
    max_new_hard_findings: 0
    max_score_drop: 0
  verification:
    require_clean_start: true
    require_clean_after_checkpoint: true
    proof_from_test_map: true
    commands: ["just fast"]
    audit_delta: no_new_findings
    rollback_unverified: true

incubator:
  enabled: true
  strategy: generate_pool_strengthen
  route_when:
    any:
      - risk_score_gte: 0.5
      - touches_paths: ["db/migrations/**", "packages/jekko/src/session/**"]
  budget:
    max_passes_per_task: 8
    max_rounds_per_task: 2
    max_active_tasks: 1
    max_parallel_idea_passes: 3
  passes:
    - id: scout
      type: scout
      context: blind
      writes: scratch_only
    - id: ideas
      type: idea
      context: blind
      count: 3
      writes: scratch_only
    - id: critique
      type: critic
      context: pool
      writes: scratch_only
    - id: synthesize
      type: synthesize
      context: pool
      writes: scratch_only
    - id: prototype
      type: prototype
      context: inherit
      writes: isolated_worktree
    - id: promotion-review
      type: promotion_review
      context: promotion
      writes: scratch_only
  promotion:
    promote_at: 0.8
    require: [problem_statement, current_best_plan, verification_strategy, rollback_plan]
    block_on:
      unresolved_critical_objections_gte: 1
    on_promote: move_to_ready_queue
    on_exhausted: park_with_summary

experiments:
  strategy: parallel_distill_refine
  max_parallel: 5
  lanes:
    - id: generated-source-plan
      hypothesis: source edits for generated zones
    - id: security-route-plan
      hypothesis: human security routing for secrets
    - id: migration-plan
      hypothesis: rollback proof for migrations
    - id: public-api-plan
      hypothesis: compatibility-preserving API refactor
    - id: proof-plan
      hypothesis: cheapest reliable proof route
  reduce:
    strategy: synthesize_best
    require_final_verification: true

memory:
  stores:
    task_receipts:
      scope: task
      retention: until_archive
      write_policy: append_only
      read_policy: inject_at_start
      searchable: true

evidence:
  require_before_promote:
    - type: jankurai_delta
      max_increase: 0
    - type: proof_lane
      must_pass: true
    - type: branch_main_regression
      must_pass: true

taint:
  default_label: repo
  labels:
    repo:
      rank: medium
    secret:
      rank: hostile
    generated:
      rank: untrusted_for_arming
  forbid:
    - from: [secret]
      cannot: [approve, expose_secret, exec_shell]
  prompt_injection:
    detect_patterns: ["[Ii]gnore previous instructions"]
    on_detect: pause

evaluation:
  metrics:
    - name: hard_findings
      command: "jq -r '.hard_findings // 0' target/jankurai/repo-score.json"
      threshold: 0
  compare: origin/main

stop:
  all:
    - shell:
        command: "jankurai audit . --mode advisory --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md --no-score-history >/dev/null 2>&1 && jq -e '(.findings // []) | length == 0' target/jankurai/repo-score.json"
        timeout: 10m
    - git_clean:
        allow_untracked: false

checkpoint:
  when: after_verified_change
  noop_if_clean: true
  verify:
    - command: "just fast"
  git:
    add: ["."]
    commit_message: "jankurai: verified repair checkpoint"
    push: allow

permissions:
  shell: allow
  edit: allow
  git_commit: allow
  git_push: allow
  workers: allow

unsupported_feature_policy:
  required: [jankurai]
  fail_closed: true
  on_missing: reject`,
  ),

  "advanced-research-loop": example(
    "advanced-research-loop",
    "Advanced research loop",
    "Evidence-first external research with citations and receipt tracking.",
    `job:
  name: "Advanced research loop"
  objective: |
    Research external facts with cited evidence, then feed the verified
    findings back into implementation work.

loop:
  policy: forever
  sleep: 5s

stop:
  all:
    - git_clean:
        allow_untracked: false

permissions:
  research: allow
  websearch: allow
  webfetch: allow

research:
  version: v1
  mode: mixed
  autonomy: require_plan
  max_parallel: 6
  timeout_seconds: 30
  provider_policy:
    prefer: [official_api, primary_source, privacy_first]
    allow: [openalex, crossref, arxiv, pubmed, gdelt, brave, tavily, exa, searxng, firecrawl, jina, github]
    missing_provider: skip_with_receipt
  extraction:
    enabled: true
    max_pages: 12
    allowed_extractors: [built_in, jina, firecrawl]
  evidence:
    require_citations: true
    claim_level: true
    store: sqlite
  safety:
    redact_secrets: true
    block_internal_urls: true
    prompt_injection: quarantine
    taint_label: web_content
  budgets:
    max_queries: 24
    max_pages: 20
    max_cost_usd: 1.0`,
  ),

  "memory-benchmark-autoresearch-basic": example(
    "memory-benchmark-autoresearch-basic",
    "AutoResearch memory benchmark basic",
    "Compact sandboxed tournament with four lanes, executable scoring, and best_verified_patch reduction.",
    `job:
  name: "Memory benchmark AutoResearch basic"
  objective: |
    Run a compact AutoResearch-style tournament against the memory benchmark
    inside a sandbox. Score a small lane set with executable Rust oracles,
    preserve append-only progress memory, and promote only when the new best
    lane beats the current best by at least 0.75 points without new hard-gate
    failures.
  risk:
    - Candidate work must stay in isolated git worktrees
    - Promotion must be blocked by any new privacy, citation, future-leak, or nondeterminism gate
    - Persistent progress memory must remain append-only

arming:
  preview_hash_required: true
  host_nonce_required: true
  reject_inside_code_fence: true
  accepted_origins:
    - trusted_user_message
    - signed_cli_input
    - host_ui_button
  preview_expires_after: 10m
  arm_token_single_use: true

loop:
  policy: forever
  sleep: 5s
  continue_on: [compaction, max_steps]
  pause_on: [checkpoint_failed, no_progress]

stop:
  all:
    - shell:
        command: "test -f .jekko/daemon/memory-benchmark-autoresearch-basic/best-state.json"
        timeout: 10s
        assert: { exit_code: 0 }
    - shell:
        command: "test -f .jekko/daemon/memory-benchmark-autoresearch-basic/scoreboard.tsv"
        timeout: 10s
        assert: { exit_code: 0 }
    - shell:
        command: "test -f .jekko/daemon/memory-benchmark-autoresearch-basic/promotion-decision.json"
        timeout: 10s
        assert: { exit_code: 0 }
    - shell:
        command: "test -f .jekko/daemon/memory-benchmark-autoresearch-basic/best.patch"
        timeout: 10s
        assert: { exit_code: 0 }
    - shell:
        command: "test -f .jekko/daemon/memory-benchmark-autoresearch-basic/negative-memory.jsonl"
        timeout: 10s
        assert: { exit_code: 0 }
    - shell:
        command: "test -f .jekko/daemon/memory-benchmark-autoresearch-basic/curriculum-proposals.json"
        timeout: 10s
        assert: { exit_code: 0 }

sandbox:
  network:
    outbound: deny

capabilities:
  default: deny
  rules:
    - id: read-workspace
      tool: read
      decision: allow
    - id: shell-benchmark
      tool: shell
      command_regex: "^(rtk )?(cargo|just|bun) "
      decision: allow
    - id: worker-fanout
      tool: workers
      decision: allow
    - id: mcp-readonly
      tool: mcp
      decision: allow
      reason: "read-only MCP profiles only"
  command_floor:
    always_block:
      - "git push"
      - "git push --force"
      - 'curl .* \| sh'
      - 'wget .* \| sh'
      - "ssh "
      - "sudo "
      - "cargo install"

triggers:
  anti_recursion: true
  list:
    - id: manual
      kind: manual
      max_runs_per_sha: 1

rollback:
  required_when:
    risk_score_gte: 0.6
  plan_required: true
  verify_command: "rtk just memory-benchmark-fast"
  on_failure_after_merge: revert_commit

done:
  require:
    - promotion_decision_written
    - best_state_written
    - scoreboard_written
    - best_patch_written
    - curriculum_proposals_written
    - determinism_verified
    - population_ledger_persisted
  forbid:
    - hard_gate_regression
    - model_only_claim

repo_intelligence:
  scale: large
  indexes: [rg, ownership, test_graph]
  scope_control:
    require_scope_before_edit: true
    max_initial_scope_files: 20
  blast_radius:
    compute_on: [diff]
    pause_when_score_gte: 0.75

fleet:
  max_workers: 4
  isolation: same_session

fan_out:
  strategy: scatter_gather
  split:
    shell: "cargo run --manifest-path crates/memory-benchmark/Cargo.toml --bin dump_tasks -- exec --population 4 --out .jekko/daemon/memory-benchmark-autoresearch-basic/reports/exec-tasks.jsonl"
  worker:
    agent: build
    isolation: git_worktree
    timeout: 45m
    max_parallel: 4
  reduce:
    strategy: custom_shell
    command: "cargo run --manifest-path crates/memory-benchmark/Cargo.toml --bin population_report -- --lanes .jekko/daemon/memory-benchmark-autoresearch-basic/reports/lanes --population .jekko/daemon/memory-benchmark-autoresearch-basic/memory/population-ledger.jsonl --baseline .jekko/daemon/memory-benchmark-autoresearch-basic/reports/baseline-score.json --exec .jekko/daemon/memory-benchmark-autoresearch-basic/reports/exec-score.json --current-best-state .jekko/daemon/memory-benchmark-autoresearch-basic/best-state.json --best-state .jekko/daemon/memory-benchmark-autoresearch-basic/best-state.json --scoreboard .jekko/daemon/memory-benchmark-autoresearch-basic/scoreboard.tsv --promotion-decision .jekko/daemon/memory-benchmark-autoresearch-basic/promotion-decision.json --negative-memory .jekko/daemon/memory-benchmark-autoresearch-basic/negative-memory.jsonl --best-patch .jekko/daemon/memory-benchmark-autoresearch-basic/best.patch --out .jekko/daemon/memory-benchmark-autoresearch-basic/reports/final-score.json --markdown .jekko/daemon/memory-benchmark-autoresearch-basic/reports/final-score.md --comparison .jekko/daemon/memory-benchmark-autoresearch-basic/reports/comparison-matrix.json --triangulation .jekko/daemon/memory-benchmark-autoresearch-basic/reports/triangulation.json --curriculum .jekko/daemon/memory-benchmark-autoresearch-basic/curriculum-proposals.json"
  on_partial_failure: continue

experiments:
  strategy: disjoint_tournament
  diversity:
    require_distinct_plan: true
    min_plan_distance: 0.40
    axes: [provenance, temporal, privacy, compression]
  lanes:
    - id: arena_lane_00
      hypothesis: "Provenance lane 00 keeps citations and traceability visible."
      prompt_strategy: reference_context_pack
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_04
      hypothesis: "Temporal lane 04 prioritizes contradiction handling and as-of reasoning."
      prompt_strategy: temporal_contradiction_scan
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_08
      hypothesis: "Privacy lane 08 keeps redaction and forgetting rules explicit."
      prompt_strategy: privacy_redaction
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_12
      hypothesis: "Compression lane 12 focuses on context packing and deterministic reuse."
      prompt_strategy: compression_economics
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
  fork_from: current_head
  max_parallel: 4
  scoring:
    weights:
      concept_learning: 12
      transfer_reasoning: 12
      formal_math: 10
      scientific_reasoning: 10
      bitemporal_correctness: 10
      provenance_support: 10
      dependency_maintenance: 8
      contradiction_skepticism: 8
      privacy_forgetting: 8
      compression_fidelity: 5
      procedural_tool_memory: 4
      determinism_efficiency: 3
    primary: executable_rust_oracles
    judge:
      agent: critic
      blind: true
      must_use_different_provider: true
  reduce:
    strategy: best_verified_patch
    require_final_verification: true
  on_partial_failure: continue
  preserve_failed_lanes_as_negative_memory: true

models:
  profiles:
    builder:
      provider: openai
      model: gpt-5.5
      reasoning: true
    critic:
      provider: anthropic
      model: claude-sonnet-4-6
      reasoning: true
    reporter:
      provider: openai
      model: gpt-5.4-mini
  routes:
    build: builder
    judge: critic
    report: reporter
  critic:
    must_differ_from_builder: true
    must_use_different_provider: true
  confidence_cap: 0.68

memory:
  stores:
    population_findings:
      scope: repo
      retention: permanent
      write_policy: append_only
      read_policy: search
      searchable: true
    `,
  ),

  "memory-benchmark-autoresearch-chase": example(
    "memory-benchmark-autoresearch-chase",
    "AutoResearch memory benchmark chase",
    "Advanced sandboxed tournament with 20 lanes, executable scoring, and best_verified_patch reduction.",
    `job:
  name: "Memory benchmark AutoResearch chase"
  objective: |
    Run a 20-agent AutoResearch-style tournament against the memory benchmark
    inside a sandbox. Score candidate lanes with executable Rust oracles,
    preserve append-only progress memory, and promote only when the new best
    lane beats the current best by at least 0.75 points without new hard-gate
    failures.
  risk:
    - Candidate work must stay in isolated git worktrees
    - Promotion must be blocked by any new privacy, citation, future-leak, or nondeterminism gate
    - Persistent progress memory must remain append-only

arming:
  preview_hash_required: true
  host_nonce_required: true
  reject_inside_code_fence: true
  accepted_origins:
    - trusted_user_message
    - signed_cli_input
    - host_ui_button
  preview_expires_after: 10m
  arm_token_single_use: true

loop:
  policy: forever
  sleep: 5s
  continue_on: [compaction, max_steps]
  pause_on: [checkpoint_failed, no_progress]

stop:
  all:
    - shell:
        command: "test -f .jekko/daemon/memory-benchmark-chase/promotion-decision.json"
        timeout: 10s
        assert: { exit_code: 0 }
    - shell:
        command: "test -f .jekko/daemon/memory-benchmark-chase/best-state.json"
        timeout: 10s
        assert: { exit_code: 0 }
    - shell:
        command: "test -f .jekko/daemon/memory-benchmark-chase/scoreboard.tsv"
        timeout: 10s
        assert: { exit_code: 0 }
    - shell:
        command: "test -f .jekko/daemon/memory-benchmark-chase/best.patch"
        timeout: 10s
        assert: { exit_code: 0 }
    - shell:
        command: "test -f .jekko/daemon/memory-benchmark-chase/negative-memory.jsonl"
        timeout: 10s
        assert: { exit_code: 0 }
    - shell:
        command: "test -f .jekko/daemon/memory-benchmark-chase/curriculum-proposals.json"
        timeout: 10s
        assert: { exit_code: 0 }

capabilities:
  default: deny
  rules:
    - id: read-workspace
      tool: read
      decision: allow
    - id: shell-benchmark
      tool: shell
      command_regex: "^(rtk )?(cargo|just|bun) "
      decision: allow
    - id: worker-fanout
      tool: workers
      decision: allow
    - id: mcp-readonly
      tool: mcp
      decision: allow
      reason: "read-only MCP profiles only"
  command_floor:
    always_block:
      - "git push"
      - "git push --force"
      - 'curl .* \| sh'
      - 'wget .* \| sh'
      - "ssh "
      - "sudo "
      - "cargo install"

triggers:
  anti_recursion: true
  list:
    - id: manual
      kind: manual
      max_runs_per_sha: 1

rollback:
  required_when:
    risk_score_gte: 0.6
  plan_required: true
  verify_command: "rtk just memory-benchmark-fast"
  on_failure_after_merge: revert_commit

done:
  require:
    - promotion_decision_written
    - best_state_written
    - scoreboard_written
    - best_patch_written
    - curriculum_proposals_written
    - determinism_verified
    - population_ledger_persisted
  forbid:
    - hard_gate_regression
    - model_only_claim

repo_intelligence:
  scale: large
  indexes: [rg, ownership, test_graph]
  scope_control:
    require_scope_before_edit: true
    max_initial_scope_files: 20
  blast_radius:
    compute_on: [diff]
    pause_when_score_gte: 0.75

fleet:
  max_workers: 20
  isolation: same_session

fan_out:
  strategy: scatter_gather
  split:
    shell: "cargo run --manifest-path crates/memory-benchmark/Cargo.toml --bin dump_tasks -- exec --population 20 --out .jekko/daemon/memory-benchmark-chase/reports/exec-tasks.jsonl"
  worker:
    agent: build
    isolation: git_worktree
    timeout: 45m
    max_parallel: 20
  reduce:
    strategy: custom_shell
    command: "cargo run --manifest-path crates/memory-benchmark/Cargo.toml --bin population_report -- --lanes .jekko/daemon/memory-benchmark-chase/reports/lanes --population .jekko/daemon/memory-benchmark-chase/memory/population-ledger.jsonl --baseline .jekko/daemon/memory-benchmark-chase/reports/baseline-score.json --exec .jekko/daemon/memory-benchmark-chase/reports/exec-score.json --current-best-state .jekko/daemon/memory-benchmark-chase/best-state.json --best-state .jekko/daemon/memory-benchmark-chase/best-state.json --scoreboard .jekko/daemon/memory-benchmark-chase/scoreboard.tsv --promotion-decision .jekko/daemon/memory-benchmark-chase/promotion-decision.json --negative-memory .jekko/daemon/memory-benchmark-chase/negative-memory.jsonl --best-patch .jekko/daemon/memory-benchmark-chase/best.patch --out .jekko/daemon/memory-benchmark-chase/reports/final-score.json --markdown .jekko/daemon/memory-benchmark-chase/reports/final-score.md --comparison .jekko/daemon/memory-benchmark-chase/reports/comparison-matrix.json --triangulation .jekko/daemon/memory-benchmark-chase/reports/triangulation.json --curriculum .jekko/daemon/memory-benchmark-chase/curriculum-proposals.json --current-candidates .jekko/daemon/memory-benchmark-chase/preflight-candidates"
  on_partial_failure: continue

experiments:
  strategy: disjoint_tournament
  diversity:
    require_distinct_plan: true
    min_plan_distance: 0.40
    axes: [provenance, temporal, privacy, compression, adversarial]
  lanes:
    - id: arena_lane_00
      hypothesis: "Provenance lane 00 keeps citations, anchors, and traceability at the top of the scoreboard."
      prompt_strategy: reference_context_pack
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_01
      hypothesis: "Provenance lane 01 emphasizes minimal supporting context with strong citation quality."
      prompt_strategy: citation_floor
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_02
      hypothesis: "Provenance lane 02 prefers evidence-led context packing over broader recall."
      prompt_strategy: provenance_ledger
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_03
      hypothesis: "Provenance lane 03 checks whether citation discipline survives under tighter budgets."
      prompt_strategy: citation_audit
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_04
      hypothesis: "Temporal lane 04 focuses on contradiction handling and as-of reasoning."
      prompt_strategy: temporal_contradiction_scan
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_05
      hypothesis: "Temporal lane 05 checks whether dependency invalidation stays correct across revisions."
      prompt_strategy: dependency_invalidation
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_06
      hypothesis: "Temporal lane 06 favors explicit time indexing for bitemporal queries."
      prompt_strategy: as_of_reasoning
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_07
      hypothesis: "Temporal lane 07 stresses contradiction recovery under stale context."
      prompt_strategy: temporal_edge_cases
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_08
      hypothesis: "Privacy lane 08 keeps redaction and forgetting rules visible during generation."
      prompt_strategy: privacy_redaction
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_09
      hypothesis: "Privacy lane 09 measures whether forgetting rules prevent stale memory bleed."
      prompt_strategy: forgetting_filter
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_10
      hypothesis: "Privacy lane 10 stresses output-channel leakage and secret handling."
      prompt_strategy: output_channel_leakage
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_11
      hypothesis: "Privacy lane 11 keeps the boundary around private data and redacted traces tight."
      prompt_strategy: secret_boundary
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_12
      hypothesis: "Compression lane 12 focuses on context packing economics."
      prompt_strategy: compression_economics
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_13
      hypothesis: "Compression lane 13 prefers denser summaries without losing stable facts."
      prompt_strategy: context_packing
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_14
      hypothesis: "Compression lane 14 protects determinism while shrinking repeated context."
      prompt_strategy: determinism_guard
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_15
      hypothesis: "Compression lane 15 keeps token budgeting explicit while compacting state."
      prompt_strategy: token_budgeting
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_16
      hypothesis: "Adversarial lane 16 probes oracle coverage with generated counterexamples."
      prompt_strategy: adversarial_oracle
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_17
      hypothesis: "Adversarial lane 17 expands coverage with generated challenge cases."
      prompt_strategy: generated_coverage
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_18
      hypothesis: "Adversarial lane 18 grows the curriculum from failed and surprising examples."
      prompt_strategy: curriculum_growth
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
    - id: arena_lane_19
      hypothesis: "Adversarial lane 19 turns prior failures into stronger oracle coverage."
      prompt_strategy: failure_driven_refinement
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 45m
      budget: { max_iterations: 6, max_diff_lines: 1500, max_cost_usd: 4.00 }
  fork_from: current_head
  max_parallel: 20
  scoring:
    weights:
      concept_learning: 12
      transfer_reasoning: 12
      formal_math: 10
      scientific_reasoning: 10
      bitemporal_correctness: 10
      provenance_support: 10
      dependency_maintenance: 8
      contradiction_skepticism: 8
      privacy_forgetting: 8
      compression_fidelity: 5
      procedural_tool_memory: 4
      determinism_efficiency: 3
    command: "rtk just memory-benchmark-chase-preflight"
    primary: executable_rust_oracles
    judge:
      agent: critic
      blind: true
      must_use_different_provider: true
  reduce:
    strategy: best_verified_patch
    require_final_verification: true
  on_partial_failure: continue
  preserve_failed_lanes_as_negative_memory: true

models:
  profiles:
    builder:
      provider: openai
      model: gpt-5.5
      reasoning: true
    critic:
      provider: anthropic
      model: claude-sonnet-4-6
      reasoning: true
    reporter:
      provider: openai
      model: gpt-5.4-mini
  routes:
    build: builder
    judge: critic
    report: reporter
  critic:
    must_differ_from_builder: true
    must_use_different_provider: true
  confidence_cap: 0.68

memory:
  stores:
    population_findings:
      scope: repo
      retention: permanent
      write_policy: append_only
      read_policy: search
      searchable: true
    population_ledger:
      scope: repo
      retention: permanent
      write_policy: append_only
      read_policy: search
      searchable: true
    curriculum_proposals:
      scope: repo
      retention: permanent
      write_policy: append_only
      read_policy: inject_at_start
      searchable: true
    negative_memory:
      scope: repo
      retention: permanent
      write_policy: append_only
      read_policy: search
      searchable: true
  redaction:
    patterns:
      - "MEMORY_BENCH_CANARY_[A-Z0-9]+"
      - "vault-canary-[A-Za-z0-9_-]+"
      - "sk-[A-Za-z0-9]+"
      - "AKIA[A-Z0-9]+"
      - "ghp_[A-Za-z0-9]+"
    action: mask
  provenance:
    track_source: true
    hash_chain: true

research:
  version: v1
  mode: mixed
  autonomy: require_plan
  max_parallel: 4
  timeout_seconds: 45
  provider_policy:
    prefer:
      - official_api
      - primary_source
      - privacy_first
    allow:
      - github
      - arxiv
      - crossref
      - openalex
    missing_provider: skip_with_receipt
  extraction:
    enabled: true
    max_pages: 8
    allowed_extractors:
      - built_in
      - jina
  evidence:
    require_citations: true
    claim_level: true
    store: sqlite
  safety:
    redact_secrets: true
    block_internal_urls: true
    prompt_injection: quarantine
    taint_label: web_content
  budgets:
    max_queries: 12
    max_pages: 12
    max_cost_usd: 0.75

sandbox:
  paths:
    - path: ".jekko/daemon/memory-benchmark-chase"
      access: write
    - path: "crates/memory-benchmark"
      access: read
    - path: "docs/ZYAL/examples/memory-benchmark"
      access: read
    - path: "script/memory-benchmark-seed-commit.ts"
      access: read
    - path: "."
      access: read
  network:
    outbound: deny
  resources:
    max_file_size: 8MiB
    max_total_disk: 1GiB
    max_memory: 4GiB
    max_processes: 32

security:
  trust_zones:
    bench:
      paths:
        - ".jekko/daemon/memory-benchmark-chase"
      require_approval: false
      max_risk_score: 0.8
    specs:
      paths:
        - "crates/memory-benchmark"
      require_approval: false
      max_risk_score: 0.5
  injection:
    scan_inputs: true
    scan_outputs: true
    deny_patterns:
      - "ignore previous instructions"
      - "disregard the system prompt"
      - "you are now"
    on_detect: pause
  secrets:
    redact_from_logs: true
    rotate_after: 7d

observability:
  spans:
    emit: all
    include_tool_calls: true
    include_model_calls: false
  metrics:
    - name: baseline_score
      type: gauge
      source: .jekko/daemon/memory-benchmark-chase/reports/baseline-score.json
    - name: exec_score
      type: gauge
      source: .jekko/daemon/memory-benchmark-chase/reports/exec-score.json
    - name: promotion_decision
      type: counter
      source: .jekko/daemon/memory-benchmark-chase/reports/promotion-decision.json
  cost:
    budget: 25.00
    currency: USD
    alert_at_percent: 80
    on_budget_exceeded: warn
  report:
    format: markdown
    on_complete: true
    on_checkpoint: true
    include:
      - baseline-score.json
      - exec-score.json
      - best-state.json
      - promotion-decision.json
      - scoreboard.tsv
      - best.patch
      - comparison-matrix.json
      - triangulation.json
      - curriculum-proposals.json
      - final-score.json
      - final-score.md

budgets:
  run:
    wall_clock: 300s
    tool_calls: 1200
    cost_usd: 25.00
    on_exhaust: park
  task:
    wall_clock: 30m
    tool_calls: 120
    cost_usd: 2.00
    on_exhaust: park
  iteration:
    wall_clock: 10m
    tool_calls: 80
    cost_usd: 1.00
    on_exhaust: pause
  experiment_lane:
    wall_clock: 45m
    cost_usd: 4.00
    on_exhaust: abort

permissions:
  read: allow
  list: allow
  glob: allow
  grep: allow
  external_directory: deny
  shell: allow
  edit: allow
  git_commit: allow
  git_push: deny
  workers: allow
  mcp: allow
  research: allow
  websearch: allow
  webfetch: allow

mcp:
  profiles:
    filesystem-scoped:
      servers:
        - filesystem
      tools:
        - read_file
        - list_directory
      resources:
        - ".jekko/daemon/memory-benchmark-chase"
        - "docs/ZYAL/examples/memory-benchmark"
    github-readonly:
      servers:
        - github
      tools:
        - list_issues
        - read_issue
        - list_pull_requests
      resources:
        - "repo://main"

ui:
  theme: jekko-gold
  banner: memory-benchmark`,
  ),

}

export function listZyalExamples() {
  return Object.values(ZYAL_EXAMPLES)
}

export function getZyalExample(id: string) {
  return ZYAL_EXAMPLES[id]
}
