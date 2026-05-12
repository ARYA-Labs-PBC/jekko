import type { ZyalArm, ZyalScript } from "./schema-core"
import type { ZyalPreview } from "./schema-preview"
import {
  ZYAL_CONTRACT_VERSION,
  ZYAL_RUNTIME_SENTINEL_VERSION,
  ZYAL_RESEARCH_BLOCK_VERSION,
} from "./version"

export function buildZyalPreview(input: { spec: ZyalScript; arm?: ZyalArm }): ZyalPreview {
  const stopChecks = input.spec.stop.all.map(describeCondition)
  const checkpointChecks = input.spec.checkpoint?.verify?.map(describeShellCheck) ?? []
  const workers = input.spec.agents?.workers ?? []
  const perms = input.spec.permissions
    ? Object.entries(input.spec.permissions)
        .filter(([, value]) => value !== undefined)
        .map(([key, value]) => `${key}:${value}`)
    : []
  const risks = [
    ...(input.spec.loop?.policy === "forever" ? ["loop:forever"] : []),
    ...(input.spec.permissions?.shell === "allow" ? ["shell:allow"] : []),
    ...(input.spec.permissions?.git_push === "allow" ? ["git_push:allow"] : []),
    ...(workers.some((worker) => (worker.count ?? 0) > 1) ? ["worker_fanout"] : []),
    ...(input.spec.incubator?.enabled ? ["incubator:enabled"] : []),
  ]
  const incubator = input.spec.incubator
  const ideaFanout = incubator?.passes.filter((pass) => pass.type === "idea").reduce((sum, pass) => sum + (pass.count ?? 1), 0) ?? 0
  const incubatorRisks = incubator?.enabled
    ? [
        ...(incubator.passes.some((pass) => pass.writes === "isolated_worktree") ? ["prototype:isolated_worktree"] : []),
        ...(ideaFanout > 1 ? [`idea_fanout:${ideaFanout}`] : []),
        ...(incubator.budget.max_passes_per_task > 5 ? [`pass_budget:${incubator.budget.max_passes_per_task}`] : []),
        ...(incubator.budget.max_rounds_per_task > 1 ? [`round_budget:${incubator.budget.max_rounds_per_task}`] : []),
      ]
    : []
  const summarizeCleanup = incubator?.cleanup
    ? Object.entries(incubator.cleanup)
        .filter(([, value]) => value === true)
        .map(([key]) => key)
        .join(", ") || "(none)"
    : undefined
  const summarizeReadiness = incubator?.readiness
    ? Object.entries(incubator.readiness)
        .filter(([, value]) => value !== undefined)
        .map(([key, value]) => `${key}:${value}`)
        .join(" ")
    : undefined
  const research = input.spec.research
  const summarizeResearchProviders = research?.provider_policy
    ? [
        research.provider_policy.prefer?.length ? `prefer:${research.provider_policy.prefer.join(",")}` : null,
        research.provider_policy.allow?.length ? `allow:${research.provider_policy.allow.length}` : null,
        research.provider_policy.missing_provider ? `missing:${research.provider_policy.missing_provider}` : null,
      ]
        .filter(Boolean)
        .join(" ") || "configured"
    : undefined
  const summarizeResearchExtraction = research?.extraction
    ? [
        research.extraction.enabled ? "extraction:on" : null,
        research.extraction.max_pages ? `pages:${research.extraction.max_pages}` : null,
        research.extraction.allowed_extractors?.length ? `extractors:${research.extraction.allowed_extractors.join(",")}` : null,
      ]
        .filter(Boolean)
        .join(" ") || "configured"
    : undefined
  const summarizeResearchEvidence = research?.evidence
    ? [
        research.evidence.require_citations ? "citations" : null,
        research.evidence.claim_level ? "claim_level" : null,
        research.evidence.store ? `store:${research.evidence.store}` : null,
      ]
        .filter(Boolean)
        .join(" ") || "configured"
    : undefined
  const summarizeResearchSafety = research?.safety
    ? [
        research.safety.redact_secrets ? "redact" : null,
        research.safety.block_internal_urls ? "block_internal" : null,
        research.safety.prompt_injection ? `prompt_injection:${research.safety.prompt_injection}` : null,
        research.safety.taint_label ? `taint:${research.safety.taint_label}` : null,
      ]
        .filter(Boolean)
        .join(" ") || "configured"
    : undefined
  const summarizeResearchBudget = research?.budgets
    ? [
        research.budgets.max_queries ? `queries:${research.budgets.max_queries}` : null,
        research.budgets.max_pages ? `pages:${research.budgets.max_pages}` : null,
        research.budgets.max_cost_usd ? `cost:$${research.budgets.max_cost_usd}` : null,
      ]
        .filter(Boolean)
        .join(" ") || "configured"
    : undefined
  const experimentsScoringSummary = input.spec.experiments?.scoring
    ? [
        input.spec.experiments.scoring.primary ? `primary:${input.spec.experiments.scoring.primary}` : null,
        input.spec.experiments.scoring.command ? "command" : null,
        input.spec.experiments.scoring.judge?.agent ? `judge:${input.spec.experiments.scoring.judge.agent}` : null,
      ]
        .filter(Boolean)
        .join(" ") || "configured"
    : undefined
  const jankurai = input.spec.jankurai
  const jankuraiAuditMode = jankurai?.audit?.mode ?? "advisory"
  const jankuraiTaskSource = jankurai?.task_source ?? "repair_plan"
  const jankuraiMaxRisk = jankurai?.selection?.max_risk ?? "low"
  const jankuraiWorkerCount = input.spec.fleet?.max_workers ?? workers.reduce((sum: number, worker) => sum + worker.count, 0)
  const jankuraiVerification = jankurai?.verification
    ? [
        jankurai.verification.require_clean_start ? "clean_start" : null,
        jankurai.verification.require_clean_after_checkpoint ? "clean_checkpoint" : null,
        jankurai.verification.proof_from_test_map ? "test_map" : null,
        jankurai.verification.commands?.length ? `commands:${jankurai.verification.commands.length}` : null,
        `audit_delta:${jankurai.verification.audit_delta ?? "no_new_findings"}`,
        jankurai.verification.rollback_unverified !== false ? "rollback_unverified" : null,
      ]
        .filter(Boolean)
        .join(" ")
    : undefined
  const jankuraiRegression = jankurai?.regression
    ? [
        `main:${jankurai.regression.main_ref ?? "origin/main"}`,
        `every:${jankurai.regression.compare_every_iterations ?? 5}`,
        `mode:${jankurai.regression.mode ?? jankuraiAuditMode}`,
        `max_new_hard:${jankurai.regression.max_new_hard_findings ?? 0}`,
        `max_score_drop:${jankurai.regression.max_score_drop ?? 0}`,
      ].join(" ")
    : undefined

  return {
    id: input.spec.id,
    contract_version: ZYAL_CONTRACT_VERSION,
    runtime_sentinel_version: ZYAL_RUNTIME_SENTINEL_VERSION,
    research_block_version: ZYAL_RESEARCH_BLOCK_VERSION,
    armed: input.arm !== undefined,
    objective: input.spec.job.objective,
    loop_policy: input.spec.loop?.policy,
    stop_checks: stopChecks,
    checkpoint_checks: checkpointChecks,
    worker_count: workers.reduce((sum: number, worker) => sum + worker.count, 0),
    permissions: perms,
    risks,
    incubator_enabled: incubator?.enabled === true,
    incubator_passes: incubator?.passes.map((pass) => `${pass.id}:${pass.type}:${pass.writes}`) ?? [],
    incubator_budget: incubator
      ? {
          max_passes_per_task: incubator.budget.max_passes_per_task,
          max_rounds_per_task: incubator.budget.max_rounds_per_task,
          max_active_tasks: incubator.budget.max_active_tasks ?? 1,
          max_parallel_idea_passes: incubator.budget.max_parallel_idea_passes ?? 1,
        }
      : undefined,
    promotion_threshold: incubator?.promotion.promote_at,
    routing_summary: incubator?.route_when
      ? describeRouteWhen(incubator.route_when)
      : incubator?.enabled
        ? "runtime default hard-task routing"
        : undefined,
    exclusion_summary: incubator?.exclude_when ? describeRouteWhen(incubator.exclude_when) : undefined,
    cleanup_summary: summarizeCleanup,
    readiness_summary: summarizeReadiness,
    incubator_risks: incubatorRisks,
    on_handler_count: input.spec.on?.length ?? 0,
    fan_out_enabled: input.spec.fan_out !== undefined,
    fan_out_summary: input.spec.fan_out
      ? `${input.spec.fan_out.strategy ?? "map_reduce"} → ${input.spec.fan_out.reduce.strategy} (max_parallel:${input.spec.fan_out.worker.max_parallel ?? 1})`
      : undefined,
    guardrail_count:
      (input.spec.guardrails?.input?.length ?? 0) +
      (input.spec.guardrails?.output?.length ?? 0) +
      (input.spec.guardrails?.iteration?.length ?? 0),
    guardrails_summary: input.spec.guardrails
      ? `input:${input.spec.guardrails.input?.length ?? 0} output:${input.spec.guardrails.output?.length ?? 0} iteration:${input.spec.guardrails.iteration?.length ?? 0}`
      : undefined,
    assertions_enabled: input.spec.assertions?.require_structured_output === true,
    retry_enabled: input.spec.retry !== undefined,
    hook_count: input.spec.hooks ? Object.values(input.spec.hooks).reduce((sum, arr) => sum + (arr?.length ?? 0), 0) : 0,
    hooks_summary: input.spec.hooks
      ? Object.entries(input.spec.hooks)
          .filter(([, arr]) => arr && arr.length > 0)
          .map(([key, arr]) => `${key}:${arr!.length}`)
          .join(" ")
      : undefined,
    constraint_count: input.spec.constraints?.length ?? 0,
    constraints_summary: input.spec.constraints?.length
      ? input.spec.constraints.map((c) => `${c.name}:${c.invariant}`).join(", ")
      : undefined,
    workflow_enabled: input.spec.workflow !== undefined,
    workflow_summary: input.spec.workflow
      ? `${input.spec.workflow.type} initial:${input.spec.workflow.initial} states:${Object.keys(input.spec.workflow.states).length}`
      : undefined,
    memory_store_count: input.spec.memory?.stores ? Object.keys(input.spec.memory.stores).length : 0,
    memory_summary: input.spec.memory?.stores
      ? Object.entries(input.spec.memory.stores).map(([k, v]) => `${k}:${v.scope}`).join(", ")
      : undefined,
    evidence_enabled: (input.spec.evidence?.require_before_promote?.length ?? 0) > 0,
    evidence_summary: input.spec.evidence?.require_before_promote ? input.spec.evidence.require_before_promote.map((r) => r.type).join(", ") : undefined,
    approval_gate_count: input.spec.approvals?.gates ? Object.keys(input.spec.approvals.gates).length : 0,
    approvals_summary: input.spec.approvals?.gates
      ? Object.entries(input.spec.approvals.gates).map(([k, v]) => `${k}${v.required_role ? `:${v.required_role}` : ""}`).join(", ")
      : undefined,
    skills_count: input.spec.skills?.registry ? Object.keys(input.spec.skills.registry).length : 0,
    skills_summary: input.spec.skills?.registry
      ? Object.entries(input.spec.skills.registry).map(([k, v]) => `${k}${v.trust ? `:${v.trust}` : ""}`).join(", ")
      : undefined,
    sandbox_enabled: input.spec.sandbox !== undefined,
    sandbox_summary: input.spec.sandbox
      ? [
          input.spec.sandbox.paths?.length ? `paths:${input.spec.sandbox.paths.length}` : null,
          input.spec.sandbox.network?.outbound ? `net:${input.spec.sandbox.network.outbound}` : null,
          input.spec.sandbox.resources ? "resources:limited" : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    security_enabled: input.spec.security !== undefined,
    security_summary: input.spec.security
      ? [
          input.spec.security.trust_zones ? `zones:${Object.keys(input.spec.security.trust_zones).length}` : null,
          input.spec.security.injection?.scan_inputs ? "scan:input" : null,
          input.spec.security.injection?.scan_outputs ? "scan:output" : null,
          input.spec.security.secrets ? "secrets:managed" : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    observability_enabled: input.spec.observability !== undefined,
    observability_summary: input.spec.observability
      ? [
          input.spec.observability.spans ? `spans:${input.spec.observability.spans.emit ?? "all"}` : null,
          input.spec.observability.metrics?.length ? `metrics:${input.spec.observability.metrics.length}` : null,
          input.spec.observability.cost?.budget ? `budget:$${input.spec.observability.cost.budget}` : null,
          input.spec.observability.report ? "report:enabled" : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    arming_enabled: input.spec.arming !== undefined,
    arming_summary: input.spec.arming
      ? [
          input.spec.arming.preview_hash_required ? "hash" : null,
          input.spec.arming.host_nonce_required ? "nonce" : null,
          input.spec.arming.accepted_origins?.length ? `origins:${input.spec.arming.accepted_origins.length}` : null,
          input.spec.arming.arm_token_single_use ? "single_use" : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    capabilities_rule_count: input.spec.capabilities?.rules?.length ?? 0,
    capabilities_summary: input.spec.capabilities
      ? [
          `default:${input.spec.capabilities.default ?? "deny"}`,
          input.spec.capabilities.rules?.length ? `rules:${input.spec.capabilities.rules.length}` : null,
          input.spec.capabilities.command_floor ? `floor:${input.spec.capabilities.command_floor.always_block.length}` : null,
        ]
          .filter(Boolean)
          .join(" ")
      : undefined,
    quality_enabled: input.spec.quality !== undefined,
    quality_summary: input.spec.quality
      ? [
          input.spec.quality.anti_vibe?.enabled ? "anti_vibe" : null,
          input.spec.quality.diff_budget ? "diff_budget" : null,
          input.spec.quality.checks?.length ? `checks:${input.spec.quality.checks.length}` : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    experiments_enabled: input.spec.experiments !== undefined,
    experiments_summary: input.spec.experiments
      ? `${input.spec.experiments.strategy ?? "disjoint_tournament"} lanes:${input.spec.experiments.lanes.length}${input.spec.experiments.reduce ? ` → ${input.spec.experiments.reduce.strategy}` : ""}${experimentsScoringSummary ? ` scoring:${experimentsScoringSummary}` : ""}`
      : undefined,
    models_enabled: input.spec.models !== undefined,
    models_summary: input.spec.models
      ? [
          input.spec.models.profiles ? `profiles:${Object.keys(input.spec.models.profiles).length}` : null,
          input.spec.models.routes ? `routes:${Object.keys(input.spec.models.routes).length}` : null,
          input.spec.models.critic?.must_differ_from_builder ? "critic_distinct" : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    budgets_enabled: input.spec.budgets !== undefined,
    budgets_summary: input.spec.budgets
      ? Object.entries(input.spec.budgets)
          .filter(([, v]) => v !== undefined)
          .map(([k]) => k)
          .join(",")
      : undefined,
    triggers_count: input.spec.triggers?.list.length ?? 0,
    triggers_summary: input.spec.triggers?.list.length ? input.spec.triggers.list.map((t) => `${t.id}:${t.kind}`).join(", ") : undefined,
    rollback_enabled: input.spec.rollback !== undefined,
    rollback_summary: input.spec.rollback
      ? [
          input.spec.rollback.plan_required ? "plan_required" : null,
          input.spec.rollback.verify_command ? "verify" : null,
          input.spec.rollback.on_failure_after_merge ? `on_fail:${input.spec.rollback.on_failure_after_merge}` : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    done_enabled: input.spec.done !== undefined,
    done_summary: input.spec.done
      ? [
          input.spec.done.require?.length ? `require:${input.spec.done.require.length}` : null,
          input.spec.done.forbid?.length ? `forbid:${input.spec.done.forbid.length}` : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    repo_intel_enabled: input.spec.repo_intelligence !== undefined,
    repo_intel_summary: input.spec.repo_intelligence
      ? [
          input.spec.repo_intelligence.scale ? `scale:${input.spec.repo_intelligence.scale}` : null,
          input.spec.repo_intelligence.indexes?.length ? `indexes:${input.spec.repo_intelligence.indexes.length}` : null,
          input.spec.repo_intelligence.scope_control?.require_scope_before_edit ? "scoped" : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    fleet_enabled: input.spec.fleet !== undefined,
    fleet_max_workers: input.spec.fleet?.max_workers ?? 0,
    fleet_summary: input.spec.fleet
      ? [
          `max:${input.spec.fleet.max_workers}`,
          input.spec.fleet.isolation ? `iso:${input.spec.fleet.isolation}` : null,
          input.spec.fleet.jnoccio?.enabled ? "jnoccio:on" : null,
          input.spec.fleet.telemetry?.publish_to ? `telem:${input.spec.fleet.telemetry.publish_to}` : null,
        ]
          .filter(Boolean)
          .join(" ")
      : undefined,
    research_enabled: research !== undefined,
    research_mode: research?.mode,
    research_max_parallel: research?.max_parallel ?? 0,
    research_timeout_seconds: research?.timeout_seconds,
    research_summary: research
      ? [
          research.mode ? `mode:${research.mode}` : null,
          research.autonomy ? `autonomy:${research.autonomy}` : null,
          research.max_parallel ? `parallel:${research.max_parallel}` : null,
          research.timeout_seconds ? `timeout:${research.timeout_seconds}s` : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    research_provider_summary: summarizeResearchProviders,
    research_extraction_summary: summarizeResearchExtraction,
    research_evidence_summary: summarizeResearchEvidence,
    research_safety_summary: summarizeResearchSafety,
    research_budget_summary: summarizeResearchBudget,
    jankurai_enabled: jankurai?.enabled === true,
    jankurai_summary: jankurai
      ? [
          `mode:${jankuraiAuditMode}`,
          `source:${jankuraiTaskSource}`,
          `order:${jankurai.selection?.order ?? "quick_wins_first"}`,
          `max_risk:${jankuraiMaxRisk}`,
          `workers:${jankuraiWorkerCount}`,
        ].join(" ")
      : undefined,
    jankurai_task_source: jankurai ? jankuraiTaskSource : undefined,
    jankurai_max_risk: jankurai ? jankuraiMaxRisk : undefined,
    jankurai_verification_summary: jankuraiVerification,
    jankurai_regression_summary: jankuraiRegression,
    taint_enabled: input.spec.taint !== undefined,
    taint_label_count: input.spec.taint ? Object.keys(input.spec.taint.labels).length : 0,
    taint_forbid_count: input.spec.taint?.forbid?.length ?? 0,
    taint_summary: input.spec.taint
      ? [
          `labels:${Object.keys(input.spec.taint.labels).length}`,
          (input.spec.taint.forbid?.length ?? 0) > 0 ? `forbid:${input.spec.taint.forbid?.length}` : null,
          input.spec.taint.prompt_injection ? `injection:${input.spec.taint.prompt_injection.on_detect}` : null,
        ]
          .filter(Boolean)
          .join(" ")
      : undefined,
    interop_enabled: input.spec.interop !== undefined,
    interop_summary: input.spec.interop
      ? [
          input.spec.interop.protocols?.length ? `protocols:${input.spec.interop.protocols.length}` : null,
          input.spec.interop.adapters?.length ? `adapters:${input.spec.interop.adapters.length}` : null,
          input.spec.interop.compile_to?.length ? `compile_to:${input.spec.interop.compile_to.length}` : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    runtime_enabled: input.spec.runtime !== undefined,
    runtime_summary: input.spec.runtime
      ? [
          `mode:${input.spec.runtime.mode ?? "preview"}`,
          input.spec.runtime.image ? `image:${input.spec.runtime.image}` : null,
          input.spec.runtime.workspace ? `workspace:${input.spec.runtime.workspace}` : null,
          input.spec.runtime.network ? `network:${input.spec.runtime.network}` : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    capability_negotiation_enabled: input.spec.capability_negotiation !== undefined,
    capability_negotiation_summary: input.spec.capability_negotiation
      ? [
          input.spec.capability_negotiation.host ? `host:${input.spec.capability_negotiation.host}` : null,
          input.spec.capability_negotiation.required?.length ? `required:${input.spec.capability_negotiation.required.length}` : null,
          input.spec.capability_negotiation.optional?.length ? `optional:${input.spec.capability_negotiation.optional.length}` : null,
          input.spec.capability_negotiation.fail_closed ? "fail_closed" : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    memory_kernel_enabled: input.spec.memory_kernel !== undefined,
    memory_kernel_summary: input.spec.memory_kernel
      ? [
          input.spec.memory_kernel.stores ? `stores:${Object.keys(input.spec.memory_kernel.stores).length}` : null,
          input.spec.memory_kernel.redaction ? "redaction" : null,
          input.spec.memory_kernel.provenance?.track_source ? "provenance" : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    evidence_graph_enabled: input.spec.evidence_graph !== undefined,
    evidence_graph_summary: input.spec.evidence_graph
      ? [
          input.spec.evidence_graph.nodes ? `nodes:${Object.keys(input.spec.evidence_graph.nodes).length}` : null,
          input.spec.evidence_graph.edges?.length ? `edges:${input.spec.evidence_graph.edges.length}` : null,
          input.spec.evidence_graph.merge_witness ? "merge_witness" : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    trust_enabled: input.spec.trust !== undefined,
    trust_summary: input.spec.trust
      ? [
          input.spec.trust.zones ? `zones:${Object.keys(input.spec.trust.zones).length}` : null,
          input.spec.trust.on_taint ? `on_taint:${input.spec.trust.on_taint}` : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    requirements_enabled: input.spec.requirements !== undefined,
    requirements_summary: input.spec.requirements
      ? [
          input.spec.requirements.must?.length ? `must:${input.spec.requirements.must.length}` : null,
          input.spec.requirements.should?.length ? `should:${input.spec.requirements.should.length}` : null,
          input.spec.requirements.avoid?.length ? `avoid:${input.spec.requirements.avoid.length}` : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    evaluation_enabled: input.spec.evaluation !== undefined,
    evaluation_summary: input.spec.evaluation
      ? [
          input.spec.evaluation.metrics?.length ? `metrics:${input.spec.evaluation.metrics.length}` : null,
          input.spec.evaluation.compare ? `compare:${input.spec.evaluation.compare}` : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    release_enabled: input.spec.release !== undefined,
    release_summary: input.spec.release
      ? [
          input.spec.release.channel ? `channel:${input.spec.release.channel}` : null,
          input.spec.release.version ? `version:${input.spec.release.version}` : null,
          input.spec.release.gates?.length ? `gates:${input.spec.release.gates.length}` : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    roles_count: input.spec.roles?.list?.length ?? 0,
    roles_summary: input.spec.roles?.list?.length ? input.spec.roles.list.map((role) => role.id).join(", ") : undefined,
    channels_count: input.spec.channels?.list?.length ?? 0,
    channels_summary: input.spec.channels?.list?.length
      ? input.spec.channels.list.map((channel) => `${channel.id}${channel.kind ? `:${channel.kind}` : ""}`).join(", ")
      : undefined,
    imports_count: input.spec.imports?.list?.length ?? 0,
    imports_summary: input.spec.imports?.list?.length ? input.spec.imports.list.map((item) => item.source).join(", ") : undefined,
    reasoning_privacy_enabled: input.spec.reasoning_privacy !== undefined,
    reasoning_privacy_summary: input.spec.reasoning_privacy
      ? [
          input.spec.reasoning_privacy.store_reasoning ? "store_reasoning" : null,
          input.spec.reasoning_privacy.redact_chain_of_thought ? "redact_chain_of_thought" : null,
          input.spec.reasoning_privacy.summaries_only ? "summaries_only" : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
    unsupported_feature_policy_enabled: input.spec.unsupported_feature_policy !== undefined,
    unsupported_feature_policy_summary: input.spec.unsupported_feature_policy
      ? [
          input.spec.unsupported_feature_policy.required?.length ? `required:${input.spec.unsupported_feature_policy.required.length}` : null,
          input.spec.unsupported_feature_policy.optional?.length ? `optional:${input.spec.unsupported_feature_policy.optional.length}` : null,
          input.spec.unsupported_feature_policy.fail_closed ? "fail_closed" : null,
          input.spec.unsupported_feature_policy.on_missing ? `on_missing:${input.spec.unsupported_feature_policy.on_missing}` : null,
        ]
          .filter(Boolean)
          .join(" ") || "configured"
      : undefined,
  }
}

function describeCondition(condition: ZyalScript["stop"]["all"][number]) {
  if ("shell" in condition) return `shell:${condition.shell.command}`
  return `git_clean${condition.git_clean.allow_untracked ? ":allow_untracked" : ""}`
}

function describeShellCheck(check: { command: string }) {
  return check.command
}

function describeRouteWhen(route: { any?: readonly unknown[]; all?: readonly unknown[] }) {
  const any = route.any?.length ?? 0
  const all = route.all?.length ?? 0
  return [`any:${any}`, `all:${all}`].join(" ")
}
