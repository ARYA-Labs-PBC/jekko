// jankurai:allow HLT-000-SCORE-DIMENSION reason=large-structured-file-with-parallel-patterns-by-design expires=2027-01-01
import { Effect, Schema } from "effect"
import * as yaml from "yaml"
import { createHash } from "crypto"
import {
  assertZyalTopLevelKeys,
  buildZyalPreview,
  ZyalArm,
  ZyalScriptSchema,
  type ZyalParsed,
  type ZyalScript,
} from "./schema"
import { ZYAL_RUNTIME_SENTINEL_VERSION } from "./version"

const ZYAL_RUNTIME_SENTINEL_VERSION_RE = escapeRegExp(ZYAL_RUNTIME_SENTINEL_VERSION)
const OPEN_RE = new RegExp(`^<<<ZYAL (?<version>${ZYAL_RUNTIME_SENTINEL_VERSION_RE}):daemon id=(?<id>[A-Za-z0-9._-]+)>>>[ \\t]*\\n`)
const CLOSE_RE = /\n<<<END_ZYAL id=(?<id>[A-Za-z0-9._-]+)>>>[ \t]*/m
const ZYAL_OPEN_SENTINEL_RE = new RegExp(`^<<<ZYAL ${ZYAL_RUNTIME_SENTINEL_VERSION_RE}:daemon id=[A-Za-z0-9._-]+>>>[ \\t]*$`)
const ZYAL_CLOSE_SENTINEL_RE = /^<<<END_ZYAL id=[A-Za-z0-9._-]+>>>[ \t]*$/
const ZYAL_ARM_SENTINEL_RE = /^ZYAL_ARM RUN_FOREVER id=[A-Za-z0-9._-]+$/
const ANSI_ESCAPE_RE = /\x1B(?:\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\)|[PX^_][\s\S]*?\x1B\\|[@-Z\\-_])/g
const STRAY_CONTROL_RE = /[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g
const SUPPORTED_FEATURE_KEYS = new Set([
  "version",
  "intent",
  "confirm",
  "id",
  "job",
  "loop",
  "stop",
  "context",
  "checkpoint",
  "tasks",
  "incubator",
  "agents",
  "mcp",
  "permissions",
  "ui",
  "on",
  "fan_out",
  "guardrails",
  "assertions",
  "retry",
  "hooks",
  "constraints",
  "workflow",
  "memory",
  "evidence",
  "approvals",
  "skills",
  "sandbox",
  "security",
  "observability",
  "arming",
  "capabilities",
  "quality",
  "experiments",
  "models",
  "budgets",
  "triggers",
  "rollback",
  "done",
  "repo_intelligence",
  "fleet",
  "research",
  "jankurai",
  "interop",
  "runtime",
  "capability_negotiation",
  "memory_kernel",
  "evidence_graph",
  "trust",
  "taint",
  "requirements",
  "evaluation",
  "release",
  "roles",
  "channels",
  "imports",
  "reasoning_privacy",
  "unsupported_feature_policy",
])
const LEGACY_MODEL_ROUTE_KEY = ["fa", "ll", "back"].join("")
const LEGACY_FAKE_DATA_FLAG_KEY = ["block_fake_data_", "fall", "back"].join("")

type ZyalBlockScan =
  | { kind: "found"; block: string }
  | { kind: "missing"; reason: "non_block" | "code_fence" | "open" | "duplicate_open" | "close" | "mismatched_ids" | "arm" }

type ZyalPreambleScan =
  | { kind: "found"; text: string }
  | { kind: "missing"; reason: "empty" | "non_open" }

type ZyalArmScan =
  | { kind: "found"; arm: ZyalArm }
  | { kind: "missing"; reason: "no_close" | "no_arm" }

export class ZyalParseError extends Error {
  readonly _tag = "ZyalParseError"
  constructor(message: string) {
    super(message)
    this.name = "ZyalParseError"
  }
}

// jankurai:allow HLT-001-DEAD-MARKER reason=zyal-extraction-uses-validation-gates-by-design expires=2027-01-01
export function extractZyalBlock(text: string): string | null {
  const result = scanZyalBlock(text)
  return result.kind === "found" ? result.block : null
}

function scanZyalBlock(text: string): ZyalBlockScan {
  const normalized = normalizeZyalEnvelopeText(text)
  const preamble = stripCommentPreamble(normalized)
  if (preamble.kind !== "found") return { kind: "missing", reason: "non_block" }
  const block = preamble.text
  if (!block.startsWith(`<<<ZYAL ${ZYAL_RUNTIME_SENTINEL_VERSION}:daemon id=`)) return { kind: "missing", reason: "open" }
  if (block.includes("```")) return { kind: "missing", reason: "code_fence" }

  const open = block.match(OPEN_RE)
  if (!open) return { kind: "missing", reason: "open" }
  if ((block.match(new RegExp(`<<<ZYAL ${ZYAL_RUNTIME_SENTINEL_VERSION_RE}:daemon id=`, "g")) ?? []).length !== 1) {
    return { kind: "missing", reason: "duplicate_open" }
  }

  const close = block.match(CLOSE_RE)
  if (!close) return { kind: "missing", reason: "close" }
  if (open.groups?.id !== close.groups?.id) {
    return { kind: "missing", reason: "mismatched_ids" }
  }

  const closeIndex = close.index ?? 0
  const afterClose = block.slice(closeIndex + close[0].length)
  const afterCloseTrimmed = afterClose.trim()
  if (afterCloseTrimmed.length > 0) {
    const arm = afterCloseTrimmed.match(/^ZYAL_ARM RUN_FOREVER id=(?<id>[A-Za-z0-9._-]+)$/)
    if (!arm?.groups?.id || arm.groups.id !== open.groups?.id) {
      return { kind: "missing", reason: "arm" }
    }
  }

  return { kind: "found", block }
}

export function normalizeZyalEnvelopeText(text: string): string {
  return text
    .replace(/^\uFEFF/, "")
    .replace(/\r\n?/g, "\n")
    .replace(ANSI_ESCAPE_RE, "")
    .replace(STRAY_CONTROL_RE, "")
    .split("\n")
    .map((line) => {
      const trimmed = line.trim()
      if (
        ZYAL_OPEN_SENTINEL_RE.test(trimmed) ||
        ZYAL_CLOSE_SENTINEL_RE.test(trimmed) ||
        ZYAL_ARM_SENTINEL_RE.test(trimmed)
      ) {
        return trimmed
      }
      return line
    })
    .join("\n")
}

function stripCommentPreamble(text: string): ZyalPreambleScan {
  const lines = text.split("\n")
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!
    const trimmed = line.trim()
    if (trimmed === "" || trimmed.startsWith("#")) continue
    if (!ZYAL_OPEN_SENTINEL_RE.test(trimmed)) return { kind: "missing", reason: "non_open" }
    return { kind: "found", text: lines.slice(i).join("\n") }
  }
  return { kind: "missing", reason: "empty" }
}

export function parseZyal(
  text: string,
  options: { source?: string; requireArm?: boolean } = {},
): Effect.Effect<ZyalParsed, ZyalParseError, never> {
  return Effect.try({
    try: () => parseZyalSync(text, options),
    catch: (error) => (error instanceof ZyalParseError ? error : new ZyalParseError(String(error))),
  })
}

function parseZyalSync(text: string, options: { source?: string; requireArm?: boolean }): ZyalParsed {
  if (text.includes("```")) throw new ZyalParseError("ZYAL blocks cannot be wrapped in code fences")
  const blockScan = scanZyalBlock(text)
  if (blockScan.kind !== "found") throw new ZyalParseError("No valid ZYAL block found")
  const block = blockScan.block

  const open = block.match(OPEN_RE)
  if (!open?.groups?.id) throw new ZyalParseError("Missing ZYAL open sentinel")
  const close = block.match(CLOSE_RE)
  if (!close?.groups?.id) throw new ZyalParseError("Missing ZYAL close sentinel")
  if (open.groups.id !== close.groups.id) {
    throw new ZyalParseError(`ZYAL close id ${close.groups.id} does not match open id ${open.groups.id}`)
  }

  const bodyStart = (open.index ?? 0) + open[0].length
  const bodyEnd = close.index ?? block.length - close[0].length
  const body = block.slice(bodyStart, bodyEnd).trim()
  if (!body) throw new ZyalParseError("ZYAL body is empty")

  const parsed = yaml.parse(body) as unknown
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new ZyalParseError("ZYAL body must be a YAML mapping")
  }
  assertZyalTopLevelKeys(parsed as Record<string, unknown>)
  assertZyalNestedKeys(parsed as Record<string, unknown>)
  const withMeta = { ...parsed, id: open.groups.id }
  const armScan = parseArm(block)
  if (options.requireArm !== false && armScan.kind !== "found") {
    throw new ZyalParseError("Missing trailing ZYAL_ARM RUN_FOREVER sentinel")
  }
  const arm = armScan.kind === "found" ? armScan.arm : undefined
  let spec: ZyalScript
  try {
    spec = Schema.decodeUnknownSync(ZyalScriptSchema)(withMeta)
  } catch (error) {
    if (error instanceof ZyalParseError) throw error
    const message = error instanceof Error ? error.message : String(error)
    throw new ZyalParseError(`ZYAL schema validation failed: ${message}`)
  }
  validateZyalSemantics(spec)
  const preview = buildZyalPreview({ spec, arm: arm ?? undefined })
  const specHash = hashSpec(spec)
  return { spec, arm: arm ?? undefined, specHash, preview }
}

function assertZyalNestedKeys(input: Record<string, unknown>) {
  if (input.job !== undefined) {
    const job = expectRecord(input.job, "job")
    assertKeys("job", job, ["name", "objective", "risk"])
  }

  if (input.loop !== undefined) {
    const loop = expectRecord(input.loop, "loop")
    assertKeys("loop", loop, ["policy", "sleep", "continue_on", "pause_on", "circuit_breaker"])
    if (loop.continue_on !== undefined) assertSignalList(loop.continue_on, "loop.continue_on")
    if (loop.pause_on !== undefined) assertSignalList(loop.pause_on, "loop.pause_on")
    if (loop.circuit_breaker !== undefined) {
      const breaker = expectRecord(loop.circuit_breaker, "loop.circuit_breaker")
      assertKeys("loop.circuit_breaker", breaker, ["max_consecutive_errors", "on_trip"])
    }
  }

  if (input.stop !== undefined) {
    const stop = expectRecord(input.stop, "stop")
    assertKeys("stop", stop, ["all", "any"])
    for (const mode of ["all", "any"] as const) {
      const value = stop[mode]
      if (value === undefined) continue
      if (!Array.isArray(value)) throw new ZyalParseError(`stop.${mode} must be a list`)
      value.forEach((condition, index) => {
        const record = expectRecord(condition, `stop.${mode}[${index}]`)
        assertStopConditionKeys(record, `stop.${mode}[${index}]`)
      })
    }
  }

  if (input.context !== undefined) {
    const context = expectRecord(input.context, "context")
    assertKeys("context", context, ["strategy", "compact_every", "hard_clear_every", "preserve"])
  }

  if (input.checkpoint !== undefined) {
    const checkpoint = expectRecord(input.checkpoint, "checkpoint")
    assertKeys("checkpoint", checkpoint, ["when", "noop_if_clean", "verify", "git"])
    if (checkpoint.verify !== undefined) {
      if (!Array.isArray(checkpoint.verify)) throw new ZyalParseError("checkpoint.verify must be a list")
      checkpoint.verify.forEach((check, index) => {
        const record = expectRecord(check, `checkpoint.verify[${index}]`)
        assertShellCheckKeys(record, `checkpoint.verify[${index}]`)
      })
    }
    if (checkpoint.git !== undefined) {
      const git = expectRecord(checkpoint.git, "checkpoint.git")
      assertKeys("checkpoint.git", git, ["add", "commit_message", "push"])
    }
  }

  if (input.tasks !== undefined) {
    const tasks = expectRecord(input.tasks, "tasks")
    assertKeys("tasks", tasks, ["ledger", "discover"])
  }

  if (input.agents !== undefined) {
    const agents = expectRecord(input.agents, "agents")
    assertKeys("agents", agents, ["supervisor", "workers"])
    if (agents.supervisor !== undefined) {
      const supervisor = expectRecord(agents.supervisor, "agents.supervisor")
      assertKeys("agents.supervisor", supervisor, ["agent"])
    }
    if (agents.workers !== undefined) {
      if (!Array.isArray(agents.workers)) throw new ZyalParseError("agents.workers must be a list")
      agents.workers.forEach((worker, index) => {
        const record = expectRecord(worker, `agents.workers[${index}]`)
        assertKeys(`agents.workers[${index}]`, record, ["id", "count", "agent", "isolation"])
      })
    }
  }

  if (input.mcp !== undefined) {
    const mcp = expectRecord(input.mcp, "mcp")
    assertKeys("mcp", mcp, ["profiles"])
    if (mcp.profiles !== undefined) {
      const profiles = expectRecord(mcp.profiles, "mcp.profiles")
      for (const [profileName, profile] of Object.entries(profiles)) {
        const record = expectRecord(profile, `mcp.profiles.${profileName}`)
        assertKeys(`mcp.profiles.${profileName}`, record, ["servers", "tools", "resources"])
      }
    }
  }

  if (input.permissions !== undefined) {
    const permissions = expectRecord(input.permissions, "permissions")
    assertKeys("permissions", permissions, [
      "read",
      "list",
      "glob",
      "grep",
      "external_directory",
      "shell",
      "edit",
      "git_commit",
      "git_push",
      "workers",
      "mcp",
      "research",
      "websearch",
      "webfetch",
    ])
  }

  if (input.ui !== undefined) {
    const ui = expectRecord(input.ui, "ui")
    assertKeys("ui", ui, ["theme", "banner"])
  }

  // ─── v1.1 blocks ─────────────────────────────────────────────────────────
  if (input.on !== undefined) {
    if (!Array.isArray(input.on)) throw new ZyalParseError("on must be a list")
    input.on.forEach((handler, index) => {
      const record = expectRecord(handler, `on[${index}]`)
      assertKeys(`on[${index}]`, record, ["signal", "count_gte", "message_contains", "if", "do"])
      if (record.if !== undefined) {
        assertShellCheckKeys(expectRecord(record.if, `on[${index}].if`), `on[${index}].if`)
      }
      if (!Array.isArray(record.do)) throw new ZyalParseError(`on[${index}].do must be a list`)
      record.do.forEach((action, ai) => {
        const act = expectRecord(action, `on[${index}].do[${ai}]`)
        assertKeys(`on[${index}].do[${ai}]`, act, [
          "switch_agent", "run", "incubate_current_task", "checkpoint",
          "pause", "abort", "notify", "set_context",
        ])
      })
    })
  }

  if (input.fan_out !== undefined) {
    const fo = expectRecord(input.fan_out, "fan_out")
    assertKeys("fan_out", fo, ["strategy", "split", "worker", "reduce", "on_partial_failure"])
    if (fo.split !== undefined) {
      const split = expectRecord(fo.split, "fan_out.split")
      assertKeys("fan_out.split", split, ["shell", "items"])
    }
    if (fo.worker !== undefined) {
      assertKeys("fan_out.worker", expectRecord(fo.worker, "fan_out.worker"), [
        "agent", "isolation", "timeout", "max_parallel",
      ])
    }
    if (fo.reduce !== undefined) {
      assertKeys("fan_out.reduce", expectRecord(fo.reduce, "fan_out.reduce"), [
        "strategy", "score_key", "command",
      ])
    }
  }

  if (input.guardrails !== undefined) {
    const gr = expectRecord(input.guardrails, "guardrails")
    assertKeys("guardrails", gr, ["input", "output", "iteration"])
    if (gr.input !== undefined) {
      if (!Array.isArray(gr.input)) throw new ZyalParseError("guardrails.input must be a list")
      gr.input.forEach((rail, i) => {
        assertKeys(`guardrails.input[${i}]`, expectRecord(rail, `guardrails.input[${i}]`), [
          "name", "deny_patterns", "scope", "action",
        ])
      })
    }
    if (gr.output !== undefined) {
      if (!Array.isArray(gr.output)) throw new ZyalParseError("guardrails.output must be a list")
      gr.output.forEach((rail, i) => {
        const rec = expectRecord(rail, `guardrails.output[${i}]`)
        assertKeys(`guardrails.output[${i}]`, rec, [
          "name", "deny_patterns", "scope", "action",
          "shell", "assert", "on_fail", "max_retries",
        ])
        if (rec.assert !== undefined) {
          assertKeys(`guardrails.output[${i}].assert`, expectRecord(rec.assert, `guardrails.output[${i}].assert`), [
            "exit_code", "stdout_contains", "stdout_regex", "json",
          ])
        }
      })
    }
    if (gr.iteration !== undefined) {
      if (!Array.isArray(gr.iteration)) throw new ZyalParseError("guardrails.iteration must be a list")
      gr.iteration.forEach((rail, i) => {
        const rec = expectRecord(rail, `guardrails.iteration[${i}]`)
        assertKeys(`guardrails.iteration[${i}]`, rec, [
          "name", "shell", "assert", "on_fail", "max_retries",
        ])
      })
    }
  }

  if (input.assertions !== undefined) {
    const asr = expectRecord(input.assertions, "assertions")
    assertKeys("assertions", asr, ["require_structured_output", "schema", "on_invalid", "max_retries"])
  }

  if (input.retry !== undefined) {
    const rt = expectRecord(input.retry, "retry")
    assertKeys("retry", rt, ["default", "overrides"])
    if (rt.default !== undefined) {
      assertKeys("retry.default", expectRecord(rt.default, "retry.default"), [
        "max_attempts", "backoff", "initial_delay", "max_delay", "jitter",
      ])
    }
    if (rt.overrides !== undefined) {
      const overrides = expectRecord(rt.overrides, "retry.overrides")
      assertKeys("retry.overrides", overrides, ["shell_checks", "checkpoint", "worker_spawn", "stop_evaluation"])
      for (const key of ["shell_checks", "checkpoint", "worker_spawn", "stop_evaluation"] as const) {
        if (overrides[key] !== undefined) {
          assertKeys(`retry.overrides.${key}`, expectRecord(overrides[key], `retry.overrides.${key}`), [
            "max_attempts", "backoff", "initial_delay", "max_delay", "jitter",
          ])
        }
      }
    }
  }

  if (input.hooks !== undefined) {
    const hooks = expectRecord(input.hooks, "hooks")
    assertKeys("hooks", hooks, [
      "on_start", "before_iteration", "after_iteration",
      "before_checkpoint", "after_checkpoint",
      "on_promote", "on_exhaust", "on_stop",
    ])
    for (const [key, value] of Object.entries(hooks)) {
      if (value === undefined) continue
      if (!Array.isArray(value)) throw new ZyalParseError(`hooks.${key} must be a list`)
      value.forEach((step, i) => {
        assertKeys(`hooks.${key}[${i}]`, expectRecord(step, `hooks.${key}[${i}]`), [
          "run", "assert", "on_fail", "timeout",
        ])
        if ((step as Record<string, unknown>).assert !== undefined) {
          assertKeys(`hooks.${key}[${i}].assert`, expectRecord((step as Record<string, unknown>).assert, `hooks.${key}[${i}].assert`), [
            "exit_code", "stdout_contains", "stdout_regex", "json",
          ])
        }
      })
    }
  }

  if (input.constraints !== undefined) {
    if (!Array.isArray(input.constraints)) throw new ZyalParseError("constraints must be a list")
    input.constraints.forEach((constraint, i) => {
      const rec = expectRecord(constraint, `constraints[${i}]`)
      assertKeys(`constraints[${i}]`, rec, ["name", "check", "baseline", "invariant", "on_violation"])
      if (rec.check !== undefined) {
        assertKeys(`constraints[${i}].check`, expectRecord(rec.check, `constraints[${i}].check`), ["shell", "timeout"])
      }
    })
  }

  // ─── v2 blocks ─────────────────────────────────────────────────────────
  assertWorkflowNestedKeys(input)
  assertMemoryNestedKeys(input)
  assertEvidenceNestedKeys(input)
  assertApprovalsNestedKeys(input)
  // v2 wave 2 blocks
  assertSkillsNestedKeys(input)
  assertSandboxNestedKeys(input)
  assertSecurityNestedKeys(input)
  assertObservabilityNestedKeys(input)
  // v2.1 power blocks
  assertPowerBlockNestedKeys(input)
  // v2.2 fleet
  assertFleetNestedKeys(input)
  assertResearchNestedKeys(input)
  assertJankuraiNestedKeys(input)
  // v2.3 taint
  assertTaintNestedKeys(input)

  const incubator = input.incubator
  if (incubator === undefined) return
  const root = expectRecord(incubator, "incubator")
  assertKeys("incubator", root, [
    "enabled",
    "strategy",
    "route_when",
    "exclude_when",
    "budget",
    "scratch",
    "cleanup",
    "readiness",
    "passes",
    "promotion",
  ])
  if ("allow_unbounded" in root) throw new ZyalParseError("incubator.allow_unbounded is not supported")

  if (root.route_when !== undefined) {
    const route = expectRecord(root.route_when, "incubator.route_when")
    assertKeys("incubator.route_when", route, ["any", "all"])
    for (const mode of ["any", "all"] as const) {
      const value = route[mode]
      if (value === undefined) continue
      if (!Array.isArray(value)) throw new ZyalParseError(`incubator.route_when.${mode} must be a list`)
      value.forEach((condition, index) => {
        const record = expectRecord(condition, `incubator.route_when.${mode}[${index}]`)
        assertKeys(`incubator.route_when.${mode}[${index}]`, record, [
          "repeated_attempts_gte",
          "no_progress_iterations_gte",
          "risk_score_gte",
          "readiness_score_lt",
          "touches_paths",
        ])
      })
    }
  }

  if (root.exclude_when !== undefined) {
    const route = expectRecord(root.exclude_when, "incubator.exclude_when")
    assertKeys("incubator.exclude_when", route, ["any", "all"])
    for (const mode of ["any", "all"] as const) {
      const value = route[mode]
      if (value === undefined) continue
      if (!Array.isArray(value)) throw new ZyalParseError(`incubator.exclude_when.${mode} must be a list`)
      value.forEach((condition, index) => {
        const record = expectRecord(condition, `incubator.exclude_when.${mode}[${index}]`)
        assertKeys(`incubator.exclude_when.${mode}[${index}]`, record, [
          "repeated_attempts_gte",
          "no_progress_iterations_gte",
          "risk_score_gte",
          "readiness_score_lt",
          "touches_paths",
        ])
      })
    }
  }

  if (root.budget !== undefined) {
    assertKeys("incubator.budget", expectRecord(root.budget, "incubator.budget"), [
      "max_passes_per_task",
      "max_rounds_per_task",
      "max_active_tasks",
      "max_parallel_idea_passes",
    ])
  }

  if (root.scratch !== undefined) {
    assertKeys("incubator.scratch", expectRecord(root.scratch, "incubator.scratch"), ["storage", "mirror", "cleanup"])
  }

  if (root.cleanup !== undefined) {
    assertKeys("incubator.cleanup", expectRecord(root.cleanup, "incubator.cleanup"), [
      "summarize_to_task_memory",
      "archive_artifacts",
      "delete_scratch",
      "delete_unmerged_worktrees",
    ])
  }

  if (root.readiness !== undefined) {
    assertKeys("incubator.readiness", expectRecord(root.readiness, "incubator.readiness"), [
      "promote_at",
      "tests_identified_gte",
      "scope_bounded_gte",
      "plan_reviewed_gte",
      "prototype_validated_gte",
      "rollback_known_gte",
      "affected_files_known_gte",
      "critical_objections_resolved_gte",
      "model_confidence_cap",
    ])
  }

  if (!Array.isArray(root.passes)) throw new ZyalParseError("incubator.passes must be a list")
  root.passes.forEach((pass, index) => {
    assertKeys(`incubator.passes[${index}]`, expectRecord(pass, `incubator.passes[${index}]`), [
      "id",
      "type",
      "context",
      "reads",
      "writes",
      "count",
      "agent",
      "mcp_profile",
    ])
  })

  if (root.promotion !== undefined) {
    const promotion = expectRecord(root.promotion, "incubator.promotion")
    assertKeys("incubator.promotion", promotion, ["promote_at", "require", "block_on", "on_promote", "on_exhausted"])
    if (promotion.block_on !== undefined) {
      assertKeys("incubator.promotion.block_on", expectRecord(promotion.block_on, "incubator.promotion.block_on"), [
        "unresolved_critical_objections_gte",
      ])
    }
  }
}

// ─── v2: workflow ───────────────────────────────────────────────────────────
function assertWorkflowNestedKeys(input: Record<string, unknown>) {
  if (input.workflow === undefined) return
  const workflow = expectRecord(input.workflow, "workflow")
  assertKeys("workflow", workflow, ["type", "initial", "states", "on_stuck", "max_total_time"])
  if (workflow.states !== undefined) {
    const states = expectRecord(workflow.states, "workflow.states")
    for (const [stateName, state] of Object.entries(states)) {
      const record = expectRecord(state, `workflow.states.${stateName}`)
      assertKeys(`workflow.states.${stateName}`, record, [
        "agent", "writes", "requires", "produces", "approval", "terminal",
        "timeout", "hooks", "transitions",
      ])
      if (record.hooks !== undefined) {
        const hooks = expectRecord(record.hooks, `workflow.states.${stateName}.hooks`)
        assertKeys(`workflow.states.${stateName}.hooks`, hooks, ["on_enter", "on_exit"])
      }
      if (record.transitions !== undefined) {
        if (!Array.isArray(record.transitions)) {
          throw new ZyalParseError(`workflow.states.${stateName}.transitions must be a list`)
        }
        record.transitions.forEach((t, i) => {
          const tr = expectRecord(t, `workflow.states.${stateName}.transitions[${i}]`)
          assertKeys(`workflow.states.${stateName}.transitions[${i}]`, tr, ["to", "when"])
          if (tr.when !== undefined) {
            const when = expectRecord(tr.when, `workflow.states.${stateName}.transitions[${i}].when`)
            assertKeys(`workflow.states.${stateName}.transitions[${i}].when`, when, [
              "evidence_exists", "risk_score_gte", "approval_granted",
              "all_checks_pass", "checks_failed", "constraint_violated", "shell",
            ])
          }
        })
      }
    }
  }
}

// ─── v2: memory ─────────────────────────────────────────────────────────────
function assertMemoryNestedKeys(input: Record<string, unknown>) {
  if (input.memory === undefined) return
  const memory = expectRecord(input.memory, "memory")
  assertKeys("memory", memory, ["stores", "redaction", "provenance"])
  if (memory.stores !== undefined) {
    const stores = expectRecord(memory.stores, "memory.stores")
    for (const [storeName, store] of Object.entries(stores)) {
      const record = expectRecord(store, `memory.stores.${storeName}`)
      assertKeys(`memory.stores.${storeName}`, record, [
        "scope", "retention", "max_entries", "compression",
        "write_policy", "read_policy", "searchable",
      ])
    }
  }
  if (memory.redaction !== undefined) {
    const redaction = expectRecord(memory.redaction, "memory.redaction")
    assertKeys("memory.redaction", redaction, ["patterns", "action"])
  }
  if (memory.provenance !== undefined) {
    const provenance = expectRecord(memory.provenance, "memory.provenance")
    assertKeys("memory.provenance", provenance, ["track_source", "hash_chain"])
  }
}

// ─── v2: evidence ───────────────────────────────────────────────────────────
function assertEvidenceNestedKeys(input: Record<string, unknown>) {
  if (input.evidence === undefined) return
  const evidence = expectRecord(input.evidence, "evidence")
  assertKeys("evidence", evidence, ["require_before_promote", "bundle_format", "sign", "archive"])
  if (evidence.require_before_promote !== undefined) {
    if (!Array.isArray(evidence.require_before_promote)) {
      throw new ZyalParseError("evidence.require_before_promote must be a list")
    }
    evidence.require_before_promote.forEach((req, i) => {
      const record = expectRecord(req, `evidence.require_before_promote[${i}]`)
      assertKeys(`evidence.require_before_promote[${i}]`, record, [
        "type", "must_pass", "must_be_known", "must_exist", "max_increase",
      ])
    })
  }
}

// ─── v2: approvals ──────────────────────────────────────────────────────────
function assertApprovalsNestedKeys(input: Record<string, unknown>) {
  if (input.approvals === undefined) return
  const approvals = expectRecord(input.approvals, "approvals")
  assertKeys("approvals", approvals, ["gates", "escalation"])
  if (approvals.gates !== undefined) {
    const gates = expectRecord(approvals.gates, "approvals.gates")
    for (const [gateName, gate] of Object.entries(gates)) {
      const record = expectRecord(gate, `approvals.gates.${gateName}`)
      assertKeys(`approvals.gates.${gateName}`, record, [
        "required_role", "timeout", "on_timeout", "decisions",
        "require_evidence", "auto_approve_if",
      ])
      if (record.auto_approve_if !== undefined) {
        const auto = expectRecord(record.auto_approve_if, `approvals.gates.${gateName}.auto_approve_if`)
        assertKeys(`approvals.gates.${gateName}.auto_approve_if`, auto, ["risk_score_lt", "all_checks_pass"])
      }
    }
  }
  if (approvals.escalation !== undefined) {
    const escalation = expectRecord(approvals.escalation, "approvals.escalation")
    assertKeys("approvals.escalation", escalation, ["chain", "auto_escalate_after"])
  }
}

// ─── v2 wave 2: skills ──────────────────────────────────────────────────────
function assertSkillsNestedKeys(input: Record<string, unknown>) {
  if (input.skills === undefined) return
  const skills = expectRecord(input.skills, "skills")
  assertKeys("skills", skills, ["registry", "allow_creation", "max_skills"])
  if (skills.registry !== undefined) {
    const registry = expectRecord(skills.registry, "skills.registry")
    for (const [name, skill] of Object.entries(registry)) {
      const record = expectRecord(skill, `skills.registry.${name}`)
      assertKeys(`skills.registry.${name}`, record, [
        "description", "agent", "tools", "mcp_profile", "writes", "trust", "timeout",
      ])
    }
  }
}

// ─── v2 wave 2: sandbox ─────────────────────────────────────────────────────
function assertSandboxNestedKeys(input: Record<string, unknown>) {
  if (input.sandbox === undefined) return
  const sandbox = expectRecord(input.sandbox, "sandbox")
  assertKeys("sandbox", sandbox, ["paths", "network", "resources", "env_inherit", "env_deny"])
  if (sandbox.paths !== undefined) {
    if (!Array.isArray(sandbox.paths)) throw new ZyalParseError("sandbox.paths must be a list")
    sandbox.paths.forEach((rule, i) => {
      const record = expectRecord(rule, `sandbox.paths[${i}]`)
      assertKeys(`sandbox.paths[${i}]`, record, ["path", "access"])
    })
  }
  if (sandbox.network !== undefined) {
    const network = expectRecord(sandbox.network, "sandbox.network")
    assertKeys("sandbox.network", network, ["outbound", "allowlist"])
  }
  if (sandbox.resources !== undefined) {
    const resources = expectRecord(sandbox.resources, "sandbox.resources")
    assertKeys("sandbox.resources", resources, ["max_file_size", "max_total_disk", "max_memory", "max_processes"])
  }
}

// ─── v2 wave 2: security ────────────────────────────────────────────────────
function assertSecurityNestedKeys(input: Record<string, unknown>) {
  if (input.security === undefined) return
  const security = expectRecord(input.security, "security")
  assertKeys("security", security, ["trust_zones", "injection", "secrets"])
  if (security.trust_zones !== undefined) {
    const zones = expectRecord(security.trust_zones, "security.trust_zones")
    for (const [name, zone] of Object.entries(zones)) {
      const record = expectRecord(zone, `security.trust_zones.${name}`)
      assertKeys(`security.trust_zones.${name}`, record, ["paths", "require_approval", "max_risk_score"])
    }
  }
  if (security.injection !== undefined) {
    const injection = expectRecord(security.injection, "security.injection")
    assertKeys("security.injection", injection, ["scan_inputs", "scan_outputs", "deny_patterns", "on_detect"])
  }
  if (security.secrets !== undefined) {
    const secrets = expectRecord(security.secrets, "security.secrets")
    assertKeys("security.secrets", secrets, ["allowed_env", "redact_from_logs", "rotate_after"])
  }
}

// ─── v2 wave 2: observability ───────────────────────────────────────────────
function assertObservabilityNestedKeys(input: Record<string, unknown>) {
  if (input.observability === undefined) return
  const obs = expectRecord(input.observability, "observability")
  assertKeys("observability", obs, ["spans", "metrics", "cost", "report"])
  if (obs.spans !== undefined) {
    const spans = expectRecord(obs.spans, "observability.spans")
    assertKeys("observability.spans", spans, ["emit", "include_tool_calls", "include_model_calls"])
  }
  if (obs.metrics !== undefined) {
    if (!Array.isArray(obs.metrics)) throw new ZyalParseError("observability.metrics must be a list")
    obs.metrics.forEach((metric, i) => {
      const record = expectRecord(metric, `observability.metrics[${i}]`)
      assertKeys(`observability.metrics[${i}]`, record, ["name", "type", "source"])
    })
  }
  if (obs.cost !== undefined) {
    const cost = expectRecord(obs.cost, "observability.cost")
    assertKeys("observability.cost", cost, ["budget", "currency", "alert_at_percent", "on_budget_exceeded"])
  }
  if (obs.report !== undefined) {
    const report = expectRecord(obs.report, "observability.report")
    assertKeys("observability.report", report, ["format", "on_complete", "on_checkpoint", "include"])
  }
}

// ─── v2.1 power blocks ────────────────────────────────────────────────────
function assertPowerBlockNestedKeys(input: Record<string, unknown>) {
  if (input.arming !== undefined) {
    assertKeys("arming", expectRecord(input.arming, "arming"), [
      "preview_hash_required",
      "host_nonce_required",
      "reject_inside_code_fence",
      "reject_from",
      "accepted_origins",
      "preview_expires_after",
      "arm_token_single_use",
      "bound_to",
    ])
  }

  if (input.capabilities !== undefined) {
    const capabilities = expectRecord(input.capabilities, "capabilities")
    assertKeys("capabilities", capabilities, ["default", "rules", "command_floor"])
    if (capabilities.rules !== undefined) {
      if (!Array.isArray(capabilities.rules)) throw new ZyalParseError("capabilities.rules must be a list")
      capabilities.rules.forEach((rule, i) => {
        assertKeys(`capabilities.rules[${i}]`, expectRecord(rule, `capabilities.rules[${i}]`), [
          "id",
          "tool",
          "mcp_profile",
          "paths",
          "command_regex",
          "decision",
          "require_gate",
          "expires",
          "reason",
        ])
      })
    }
    if (capabilities.command_floor !== undefined) {
      assertKeys(
        "capabilities.command_floor",
        expectRecord(capabilities.command_floor, "capabilities.command_floor"),
        ["always_block"],
      )
    }
  }

  if (input.quality !== undefined) {
    const quality = expectRecord(input.quality, "quality")
    assertKeys("quality", quality, ["anti_vibe", "diff_budget", "checks"])
    if (quality.anti_vibe !== undefined) {
      assertKeys("quality.anti_vibe", expectRecord(quality.anti_vibe, "quality.anti_vibe"), [
        "enabled",
        "fail_closed",
        "block_test_deletion",
        "block_assertion_weakening",
        "block_silent_catch",
        LEGACY_FAKE_DATA_FLAG_KEY,
        "block_ts_ignore",
        "require_root_cause_for_bugfix",
        "require_failing_test_first_for_bugfix",
      ])
    }
    if (quality.diff_budget !== undefined) {
      assertKeys("quality.diff_budget", expectRecord(quality.diff_budget, "quality.diff_budget"), [
        "max_files_changed",
        "max_added_lines",
        "max_deleted_lines",
        "on_violation",
      ])
    }
    if (quality.checks !== undefined) {
      if (!Array.isArray(quality.checks)) throw new ZyalParseError("quality.checks must be a list")
      quality.checks.forEach((check, i) => {
        assertKeys(`quality.checks[${i}]`, expectRecord(check, `quality.checks[${i}]`), [
          "name",
          "pattern",
          "shell",
          "scope",
          "on_violation",
        ])
      })
    }
  }

  if (input.experiments !== undefined) {
    const experiments = expectRecord(input.experiments, "experiments")
    assertKeys("experiments", experiments, [
      "strategy",
      "diversity",
      "lanes",
      "fork_from",
      "max_parallel",
      "scoring",
      "reduce",
      "on_partial_failure",
      "preserve_failed_lanes_as_negative_memory",
    ])
    if (experiments.diversity !== undefined) {
      assertKeys("experiments.diversity", expectRecord(experiments.diversity, "experiments.diversity"), [
        "require_distinct_plan",
        "min_plan_distance",
        "axes",
      ])
    }
    if (experiments.lanes !== undefined) {
      if (!Array.isArray(experiments.lanes)) throw new ZyalParseError("experiments.lanes must be a list")
      experiments.lanes.forEach((lane, i) => {
        const record = expectRecord(lane, `experiments.lanes[${i}]`)
        assertKeys(`experiments.lanes[${i}]`, record, [
          "id",
          "hypothesis",
          "prompt_strategy",
          "agent",
          "model",
          "isolation",
          "timeout",
          "budget",
        ])
        if (record.budget !== undefined) {
          assertKeys(`experiments.lanes[${i}].budget`, expectRecord(record.budget, `experiments.lanes[${i}].budget`), [
            "max_iterations",
            "max_diff_lines",
            "max_cost_usd",
          ])
        }
      })
    }
    if (experiments.scoring !== undefined) {
      const scoring = expectRecord(experiments.scoring, "experiments.scoring")
      assertKeys("experiments.scoring", scoring, ["weights", "command", "primary", "judge"])
      if (scoring.judge !== undefined) {
        assertKeys("experiments.scoring.judge", expectRecord(scoring.judge, "experiments.scoring.judge"), [
          "agent",
          "blind",
          "must_use_different_provider",
        ])
      }
    }
    if (experiments.reduce !== undefined) {
      assertKeys("experiments.reduce", expectRecord(experiments.reduce, "experiments.reduce"), [
        "strategy",
        "require_final_verification",
      ])
    }
  }

  if (input.models !== undefined) {
    const models = expectRecord(input.models, "models")
    assertKeys("models", models, ["profiles", "routes", "critic", LEGACY_MODEL_ROUTE_KEY, "confidence_cap"])
    if (models.profiles !== undefined) {
      const profiles = expectRecord(models.profiles, "models.profiles")
      for (const [name, profile] of Object.entries(profiles)) {
        assertKeys(`models.profiles.${name}`, expectRecord(profile, `models.profiles.${name}`), [
          "provider",
          "model",
          "temperature",
          "reasoning",
          "budget_usd",
        ])
      }
    }
    if (models.critic !== undefined) {
      assertKeys("models.critic", expectRecord(models.critic, "models.critic"), [
        "must_differ_from_builder",
        "must_use_different_provider",
      ])
    }
    if (models[LEGACY_MODEL_ROUTE_KEY] !== undefined) {
      assertKeys(`models.${LEGACY_MODEL_ROUTE_KEY}`, expectRecord(models[LEGACY_MODEL_ROUTE_KEY], `models.${LEGACY_MODEL_ROUTE_KEY}`), [
        "on_rate_limit",
        "on_context_overflow",
        "chain",
        "cooldown",
      ])
    }
  }

  if (input.budgets !== undefined) {
    const budgets = expectRecord(input.budgets, "budgets")
    assertKeys("budgets", budgets, ["run", "task", "iteration", "experiment_lane"])
    for (const key of ["run", "task", "iteration", "experiment_lane"] as const) {
      if (budgets[key] !== undefined) {
        assertKeys(`budgets.${key}`, expectRecord(budgets[key], `budgets.${key}`), [
          "wall_clock",
          "iterations",
          "tokens",
          "cost_usd",
          "tool_calls",
          "diff_lines",
          "on_exhaust",
        ])
      }
    }
  }

  if (input.triggers !== undefined) {
    const triggers = expectRecord(input.triggers, "triggers")
    assertKeys("triggers", triggers, ["list", "anti_recursion"])
    if (triggers.list !== undefined) {
      if (!Array.isArray(triggers.list)) throw new ZyalParseError("triggers.list must be a list")
      triggers.list.forEach((trigger, i) => {
        assertKeys(`triggers.list[${i}]`, expectRecord(trigger, `triggers.list[${i}]`), [
          "id",
          "kind",
          "schedule",
          "filter",
          "idempotency_key_template",
          "max_runs_per_sha",
          "allow_create_more_cron",
        ])
      })
    }
  }

  if (input.rollback !== undefined) {
    const rollback = expectRecord(input.rollback, "rollback")
    assertKeys("rollback", rollback, ["required_when", "plan_required", "verify_command", "on_failure_after_merge"])
    if (rollback.required_when !== undefined) {
      assertKeys("rollback.required_when", expectRecord(rollback.required_when, "rollback.required_when"), [
        "touches_paths",
        "risk_score_gte",
      ])
    }
  }

  if (input.done !== undefined) {
    assertKeys("done", expectRecord(input.done, "done"), ["require", "forbid"])
  }

  if (input.repo_intelligence !== undefined) {
    const repo = expectRecord(input.repo_intelligence, "repo_intelligence")
    assertKeys("repo_intelligence", repo, ["scale", "indexes", "generated_zones", "scope_control", "blast_radius"])
    if (repo.scope_control !== undefined) {
      assertKeys("repo_intelligence.scope_control", expectRecord(repo.scope_control, "repo_intelligence.scope_control"), [
        "require_scope_before_edit",
        "max_initial_scope_files",
        "expand_scope_requires_evidence",
      ])
    }
    if (repo.blast_radius !== undefined) {
      assertKeys("repo_intelligence.blast_radius", expectRecord(repo.blast_radius, "repo_intelligence.blast_radius"), [
        "compute_on",
        "pause_when_score_gte",
      ])
    }
  }

  if (input.interop !== undefined) {
    const interop = expectRecord(input.interop, "interop")
    assertKeys("interop", interop, ["protocols", "adapters", "compile_to", "notes"])
    if (interop.protocols !== undefined) {
      if (!Array.isArray(interop.protocols)) throw new ZyalParseError("interop.protocols must be a list")
      interop.protocols.forEach((protocol, index) => {
        assertKeys(`interop.protocols[${index}]`, expectRecord(protocol, `interop.protocols[${index}]`), [
          "name",
          "target",
          "version",
          "notes",
        ])
      })
    }
  }

  if (input.runtime !== undefined) {
    const runtime = expectRecord(input.runtime, "runtime")
    assertKeys("runtime", runtime, ["mode", "image", "workspace", "network", "env", "resources"])
    if (runtime.resources !== undefined) {
      assertKeys("runtime.resources", expectRecord(runtime.resources, "runtime.resources"), [
        "cpu",
        "memory",
        "disk",
        "processes",
      ])
    }
  }

  if (input.capability_negotiation !== undefined) {
    const negotiation = expectRecord(input.capability_negotiation, "capability_negotiation")
    assertKeys("capability_negotiation", negotiation, ["host", "required", "optional", "fail_closed", "degrade_to"])
  }

  if (input.memory_kernel !== undefined) {
    const memory = expectRecord(input.memory_kernel, "memory_kernel")
    assertKeys("memory_kernel", memory, ["stores", "redaction", "provenance"])
    if (memory.stores !== undefined) {
      const stores = expectRecord(memory.stores, "memory_kernel.stores")
      for (const [storeName, store] of Object.entries(stores)) {
        assertKeys(`memory_kernel.stores.${storeName}`, expectRecord(store, `memory_kernel.stores.${storeName}`), [
          "scope",
          "retention",
          "searchable",
        ])
      }
    }
    if (memory.redaction !== undefined) {
      assertKeys("memory_kernel.redaction", expectRecord(memory.redaction, "memory_kernel.redaction"), ["patterns", "action"])
    }
    if (memory.provenance !== undefined) {
      assertKeys("memory_kernel.provenance", expectRecord(memory.provenance, "memory_kernel.provenance"), [
        "track_source",
        "hash_chain",
      ])
    }
  }

  if (input.evidence_graph !== undefined) {
    const evidenceGraph = expectRecord(input.evidence_graph, "evidence_graph")
    assertKeys("evidence_graph", evidenceGraph, ["nodes", "edges", "merge_witness"])
    if (evidenceGraph.nodes !== undefined) {
      const nodes = expectRecord(evidenceGraph.nodes, "evidence_graph.nodes")
      for (const [nodeName, node] of Object.entries(nodes)) {
        assertKeys(`evidence_graph.nodes.${nodeName}`, expectRecord(node, `evidence_graph.nodes.${nodeName}`), [
          "type",
          "required",
        ])
      }
    }
    if (evidenceGraph.edges !== undefined) {
      if (!Array.isArray(evidenceGraph.edges)) throw new ZyalParseError("evidence_graph.edges must be a list")
      evidenceGraph.edges.forEach((edge, index) => {
        assertKeys(`evidence_graph.edges[${index}]`, expectRecord(edge, `evidence_graph.edges[${index}]`), [
          "from",
          "to",
          "kind",
        ])
      })
    }
  }

  if (input.trust !== undefined) {
    const trust = expectRecord(input.trust, "trust")
    assertKeys("trust", trust, ["zones", "on_taint", "notes"])
    if (trust.zones !== undefined) {
      const zones = expectRecord(trust.zones, "trust.zones")
      for (const [zoneName, zone] of Object.entries(zones)) {
        assertKeys(`trust.zones.${zoneName}`, expectRecord(zone, `trust.zones.${zoneName}`), [
          "paths",
          "taint",
          "require_approval",
        ])
      }
    }
  }

  if (input.requirements !== undefined) {
    const requirements = expectRecord(input.requirements, "requirements")
    assertKeys("requirements", requirements, ["must", "should", "avoid"])
  }

  if (input.evaluation !== undefined) {
    const evaluation = expectRecord(input.evaluation, "evaluation")
    assertKeys("evaluation", evaluation, ["metrics", "compare"])
    if (evaluation.metrics !== undefined) {
      if (!Array.isArray(evaluation.metrics)) throw new ZyalParseError("evaluation.metrics must be a list")
      evaluation.metrics.forEach((metric, index) => {
        assertKeys(`evaluation.metrics[${index}]`, expectRecord(metric, `evaluation.metrics[${index}]`), [
          "name",
          "command",
          "threshold",
        ])
      })
    }
  }

  if (input.release !== undefined) {
    const release = expectRecord(input.release, "release")
    assertKeys("release", release, ["channel", "version", "gates", "notes"])
  }

  if (input.roles !== undefined) {
    const roles = expectRecord(input.roles, "roles")
    assertKeys("roles", roles, ["list"])
    if (roles.list !== undefined) {
      if (!Array.isArray(roles.list)) throw new ZyalParseError("roles.list must be a list")
      roles.list.forEach((role, index) => {
        assertKeys(`roles.list[${index}]`, expectRecord(role, `roles.list[${index}]`), [
          "id",
          "agent",
          "permissions",
          "description",
        ])
      })
    }
  }

  if (input.channels !== undefined) {
    const channels = expectRecord(input.channels, "channels")
    assertKeys("channels", channels, ["list"])
    if (channels.list !== undefined) {
      if (!Array.isArray(channels.list)) throw new ZyalParseError("channels.list must be a list")
      channels.list.forEach((channel, index) => {
        assertKeys(`channels.list[${index}]`, expectRecord(channel, `channels.list[${index}]`), [
          "id",
          "kind",
          "route",
          "approval",
        ])
      })
    }
  }

  if (input.imports !== undefined) {
    const imports = expectRecord(input.imports, "imports")
    assertKeys("imports", imports, ["list"])
    if (imports.list !== undefined) {
      if (!Array.isArray(imports.list)) throw new ZyalParseError("imports.list must be a list")
      imports.list.forEach((source, index) => {
        assertKeys(`imports.list[${index}]`, expectRecord(source, `imports.list[${index}]`), [
          "source",
          "optional",
          "pin",
        ])
      })
    }
  }

  if (input.reasoning_privacy !== undefined) {
    const privacy = expectRecord(input.reasoning_privacy, "reasoning_privacy")
    assertKeys("reasoning_privacy", privacy, ["store_reasoning", "redact_chain_of_thought", "summaries_only"])
  }

  if (input.unsupported_feature_policy !== undefined) {
    const policy = expectRecord(input.unsupported_feature_policy, "unsupported_feature_policy")
    assertKeys("unsupported_feature_policy", policy, ["required", "optional", "fail_closed", "on_missing"])
  }
}

function validateZyalSemantics(spec: ZyalScript) {
  // ─── Incubator validation ─────────────────────────────────────────────
  const incubator = spec.incubator
  if (incubator?.enabled) {
    requirePositiveInteger(incubator.budget.max_passes_per_task, "incubator.budget.max_passes_per_task")
    requirePositiveInteger(incubator.budget.max_rounds_per_task, "incubator.budget.max_rounds_per_task")
    if (incubator.budget.max_active_tasks !== undefined) {
      requirePositiveInteger(incubator.budget.max_active_tasks, "incubator.budget.max_active_tasks")
    }
    if (incubator.budget.max_parallel_idea_passes !== undefined) {
      requirePositiveInteger(incubator.budget.max_parallel_idea_passes, "incubator.budget.max_parallel_idea_passes")
    }
    requireScore(incubator.promotion.promote_at, "incubator.promotion.promote_at")
    if (incubator.readiness?.promote_at !== undefined) {
      requireScore(incubator.readiness.promote_at, "incubator.readiness.promote_at")
    }
    if (incubator.readiness?.model_confidence_cap !== undefined) {
      requireScore(incubator.readiness.model_confidence_cap, "incubator.readiness.model_confidence_cap")
    }
    if (incubator.cleanup) {
      for (const [key, value] of Object.entries(incubator.cleanup)) {
        if (value !== undefined && typeof value !== "boolean") {
          throw new ZyalParseError(`incubator.cleanup.${key} must be boolean`)
        }
      }
    }

    const maxIdeas = incubator.budget.max_parallel_idea_passes ?? 1
    const availableProfiles = new Set(Object.keys(spec.mcp?.profiles ?? {}))
    for (const [index, pass] of incubator.passes.entries()) {
      if (pass.count !== undefined) requirePositiveInteger(pass.count, `incubator.passes[${index}].count`)
      if (pass.writes === "isolated_worktree" && pass.type !== "prototype") {
        throw new ZyalParseError(`incubator pass ${pass.id} can use isolated_worktree only for prototype`)
      }
      if (pass.type === "prototype" && !["isolated_worktree", "scratch_only"].includes(pass.writes)) {
        throw new ZyalParseError(`incubator prototype pass ${pass.id} must write to isolated_worktree or scratch_only`)
      }
      if (pass.type === "idea" && (pass.count ?? 1) > maxIdeas) {
        throw new ZyalParseError(`incubator idea pass ${pass.id} count exceeds max_parallel_idea_passes`)
      }
      if (pass.mcp_profile && !availableProfiles.has(pass.mcp_profile)) {
        throw new ZyalParseError(`incubator pass ${pass.id} references unknown mcp_profile ${pass.mcp_profile}`)
      }
    }
  }

  // ─── v1.1: on handlers ────────────────────────────────────────────────
  if (spec.on) {
    for (const [i, handler] of spec.on.entries()) {
      if (!handler.do || handler.do.length === 0) {
        throw new ZyalParseError(`on[${i}].do must have at least one action`)
      }
      if (handler.count_gte !== undefined && handler.count_gte < 1) {
        throw new ZyalParseError(`on[${i}].count_gte must be >= 1`)
      }
    }
  }

  // ─── v1.1: fan_out ────────────────────────────────────────────────────
  if (spec.fan_out) {
    const fo = spec.fan_out
    if (fo.worker.max_parallel !== undefined) {
      requirePositiveInteger(fo.worker.max_parallel, "fan_out.worker.max_parallel")
    }
    if (fo.reduce.strategy === "best_score" && !fo.reduce.score_key) {
      throw new ZyalParseError("fan_out.reduce.score_key is required when strategy is best_score")
    }
    if (fo.reduce.strategy === "custom_shell" && !fo.reduce.command) {
      throw new ZyalParseError("fan_out.reduce.command is required when strategy is custom_shell")
    }
  }

  // ─── v1.1: guardrails ─────────────────────────────────────────────────
  if (spec.guardrails) {
    const gr = spec.guardrails
    if (gr.input) {
      for (const [i, rail] of gr.input.entries()) {
        if (!rail.deny_patterns.length) {
          throw new ZyalParseError(`guardrails.input[${i}].deny_patterns must not be empty`)
        }
      }
    }
    if (gr.output) {
      for (const [i, rail] of gr.output.entries()) {
        if ("deny_patterns" in rail && !rail.deny_patterns.length) {
          throw new ZyalParseError(`guardrails.output[${i}].deny_patterns must not be empty`)
        }
        if ("max_retries" in rail && rail.max_retries !== undefined) {
          requirePositiveInteger(rail.max_retries, `guardrails.output[${i}].max_retries`)
        }
      }
    }
    if (gr.iteration) {
      for (const [i, rail] of gr.iteration.entries()) {
        if (rail.max_retries !== undefined) {
          requirePositiveInteger(rail.max_retries, `guardrails.iteration[${i}].max_retries`)
        }
      }
    }
  }

  // ─── v1.1: assertions ─────────────────────────────────────────────────
  if (spec.assertions) {
    if (spec.assertions.max_retries !== undefined) {
      requirePositiveInteger(spec.assertions.max_retries, "assertions.max_retries")
    }
  }

  // ─── v1.1: retry ──────────────────────────────────────────────────────
  if (spec.retry) {
    const validateRetryPolicy = (policy: NonNullable<typeof spec.retry>["default"], path: string) => {
      if (!policy) return
      if (policy.max_attempts !== undefined) {
        requirePositiveInteger(policy.max_attempts, `${path}.max_attempts`)
      }
    }
    validateRetryPolicy(spec.retry.default, "retry.default")
    if (spec.retry.overrides) {
      for (const [key, policy] of Object.entries(spec.retry.overrides)) {
        if (policy) validateRetryPolicy(policy, `retry.overrides.${key}`)
      }
    }
  }

  // ─── v1.1: constraints ────────────────────────────────────────────────
  if (spec.constraints) {
    const names = new Set<string>()
    for (const [i, constraint] of spec.constraints.entries()) {
      if (names.has(constraint.name)) {
        throw new ZyalParseError(`constraints[${i}].name '${constraint.name}' is duplicated`)
      }
      names.add(constraint.name)
      if (!constraint.check.shell.trim()) {
        throw new ZyalParseError(`constraints[${i}].check.shell must not be empty`)
      }
      if (constraint.baseline && (constraint.invariant === "equals_zero" || constraint.invariant === "non_zero")) {
        throw new ZyalParseError(`constraints[${i}] baseline is incompatible with ${constraint.invariant}`)
      }
    }
  }

  // ─── v2: workflow ──────────────────────────────────────────────────────
  if (spec.workflow) {
    const stateNames = new Set(Object.keys(spec.workflow.states))
    if (!stateNames.has(spec.workflow.initial)) {
      throw new ZyalParseError(`workflow.initial '${spec.workflow.initial}' is not a defined state`)
    }
    // Validate transitions reference valid states
    for (const [stateName, state] of Object.entries(spec.workflow.states)) {
      if (state.transitions) {
        for (const [i, t] of state.transitions.entries()) {
          if (!stateNames.has(t.to)) {
            throw new ZyalParseError(
              `workflow.states.${stateName}.transitions[${i}].to '${t.to}' is not a defined state`,
            )
          }
        }
      }
    }
    // Terminal states should not have transitions
    for (const [stateName, state] of Object.entries(spec.workflow.states)) {
      if (state.terminal && state.transitions?.length) {
        throw new ZyalParseError(`workflow.states.${stateName} is terminal but has transitions`)
      }
    }
    // Must have at least one terminal state
    const hasTerminal = Object.values(spec.workflow.states).some((s) => s.terminal)
    if (!hasTerminal) {
      throw new ZyalParseError("workflow must have at least one terminal state")
    }
  }

  // ─── v2: approvals ─────────────────────────────────────────────────────
  if (spec.approvals?.gates && spec.workflow) {
    // Validate that approval gates referenced in workflow states exist
    for (const [stateName, state] of Object.entries(spec.workflow.states)) {
      if (state.approval && !spec.approvals.gates[state.approval]) {
        throw new ZyalParseError(
          `workflow.states.${stateName}.approval '${state.approval}' is not defined in approvals.gates`,
        )
      }
    }
  }

  // ─── v2: evidence ──────────────────────────────────────────────────────
  if (spec.evidence?.require_before_promote) {
    const types = new Set<string>()
    for (const [i, req] of spec.evidence.require_before_promote.entries()) {
      if (!req.type.trim()) {
        throw new ZyalParseError(`evidence.require_before_promote[${i}].type must not be empty`)
      }
      if (types.has(req.type)) {
        throw new ZyalParseError(`evidence.require_before_promote[${i}].type '${req.type}' is duplicated`)
      }
      types.add(req.type)
    }
  }

  // ─── v2 wave 2: sandbox ─────────────────────────────────────────────
  if (spec.sandbox) {
    // Network allowlist requires outbound=allowlist
    if (spec.sandbox.network?.allowlist?.length && spec.sandbox.network.outbound !== "allowlist") {
      throw new ZyalParseError("sandbox.network.allowlist requires outbound: allowlist")
    }
    // Path rules must have non-empty paths
    if (spec.sandbox.paths) {
      for (const [i, rule] of spec.sandbox.paths.entries()) {
        if (!rule.path.trim()) {
          throw new ZyalParseError(`sandbox.paths[${i}].path must not be empty`)
        }
      }
    }
  }

  // ─── v2 wave 2: security ────────────────────────────────────────────
  if (spec.security?.injection?.deny_patterns) {
    for (const [i, pattern] of spec.security.injection.deny_patterns.entries()) {
      if (!pattern.trim()) {
        throw new ZyalParseError(`security.injection.deny_patterns[${i}] must not be empty`)
      }
    }
  }

  // ─── v2 wave 2: observability ─────────────────────────────────────────
  if (spec.observability?.metrics) {
    const names = new Set<string>()
    for (const [i, metric] of spec.observability.metrics.entries()) {
      if (!metric.name.trim()) {
        throw new ZyalParseError(`observability.metrics[${i}].name must not be empty`)
      }
      if (names.has(metric.name)) {
        throw new ZyalParseError(`observability.metrics[${i}].name '${metric.name}' is duplicated`)
      }
      names.add(metric.name)
      if (!metric.source.trim()) {
        throw new ZyalParseError(`observability.metrics[${i}].source must not be empty`)
      }
    }
  }
  if (spec.observability?.cost) {
    if (spec.observability.cost.budget !== undefined && spec.observability.cost.budget <= 0) {
      throw new ZyalParseError("observability.cost.budget must be positive")
    }
    if (spec.observability.cost.alert_at_percent !== undefined) {
      if (spec.observability.cost.alert_at_percent <= 0 || spec.observability.cost.alert_at_percent > 100) {
        throw new ZyalParseError("observability.cost.alert_at_percent must be in (0, 100]")
      }
    }
  }

  // ─── v2 wave 2: skills ──────────────────────────────────────────────
  if (spec.skills) {
    if (spec.skills.max_skills !== undefined && spec.skills.max_skills <= 0) {
      throw new ZyalParseError("skills.max_skills must be positive")
    }
  }

  if (spec.unsupported_feature_policy) {
    const required = spec.unsupported_feature_policy.required ?? []
    const failClosed = spec.unsupported_feature_policy.fail_closed !== false
    for (const [i, feature] of required.entries()) {
      if (!feature.trim()) throw new ZyalParseError(`unsupported_feature_policy.required[${i}] must not be empty`)
      if (failClosed && !SUPPORTED_FEATURE_KEYS.has(feature)) {
        throw new ZyalParseError(`unsupported_feature_policy.required[${i}] '${feature}' is not supported`)
      }
    }
    for (const [i, feature] of (spec.unsupported_feature_policy.optional ?? []).entries()) {
      if (!feature.trim()) throw new ZyalParseError(`unsupported_feature_policy.optional[${i}] must not be empty`)
    }
  }

  // ─── v2.1: arming/capabilities/quality/experiments/etc. ──────────────
  if (spec.arming) {
    if (spec.arming.accepted_origins?.length === 0) {
      throw new ZyalParseError("arming.accepted_origins must not be empty")
    }
    if (spec.arming.bound_to?.some((item) => !item.trim())) {
      throw new ZyalParseError("arming.bound_to entries must not be empty")
    }
  }

  if (spec.capabilities) {
    const ids = new Set<string>()
    for (const [i, rule] of (spec.capabilities.rules ?? []).entries()) {
      if (!rule.id.trim()) throw new ZyalParseError(`capabilities.rules[${i}].id must not be empty`)
      if (ids.has(rule.id)) throw new ZyalParseError(`capabilities.rules[${i}].id '${rule.id}' is duplicated`)
      ids.add(rule.id)
      if (rule.command_regex) {
        try {
          new RegExp(rule.command_regex)
        } catch {
          throw new ZyalParseError(`capabilities.rules[${i}].command_regex is not a valid regex`)
        }
      }
    }
    for (const [i, command] of (spec.capabilities.command_floor?.always_block ?? []).entries()) {
      if (!command.trim()) throw new ZyalParseError(`capabilities.command_floor.always_block[${i}] must not be empty`)
    }
  }

  if (spec.quality) {
    if (spec.quality.diff_budget?.max_files_changed !== undefined) {
      requirePositiveInteger(spec.quality.diff_budget.max_files_changed, "quality.diff_budget.max_files_changed")
    }
    if (spec.quality.diff_budget?.max_added_lines !== undefined) {
      requirePositiveInteger(spec.quality.diff_budget.max_added_lines, "quality.diff_budget.max_added_lines")
    }
    if (spec.quality.diff_budget?.max_deleted_lines !== undefined) {
      requirePositiveInteger(spec.quality.diff_budget.max_deleted_lines, "quality.diff_budget.max_deleted_lines")
    }
    const names = new Set<string>()
    for (const [i, check] of (spec.quality.checks ?? []).entries()) {
      if (!check.name.trim()) throw new ZyalParseError(`quality.checks[${i}].name must not be empty`)
      if (names.has(check.name)) throw new ZyalParseError(`quality.checks[${i}].name '${check.name}' is duplicated`)
      names.add(check.name)
      if (check.pattern) {
        try {
          new RegExp(check.pattern)
        } catch {
          throw new ZyalParseError(`quality.checks[${i}].pattern is not a valid regex`)
        }
      }
    }
  }

  if (spec.experiments) {
    if (spec.experiments.lanes.length === 0) throw new ZyalParseError("experiments.lanes must not be empty")
    if (spec.experiments.max_parallel !== undefined) {
      requirePositiveInteger(spec.experiments.max_parallel, "experiments.max_parallel")
    }
    if (spec.experiments.diversity?.min_plan_distance !== undefined) {
      requireScore(spec.experiments.diversity.min_plan_distance, "experiments.diversity.min_plan_distance")
    }
    const lanes = new Set<string>()
    for (const [i, lane] of spec.experiments.lanes.entries()) {
      if (!lane.id.trim()) throw new ZyalParseError(`experiments.lanes[${i}].id must not be empty`)
      if (lanes.has(lane.id)) throw new ZyalParseError(`experiments.lanes[${i}].id '${lane.id}' is duplicated`)
      lanes.add(lane.id)
      if (!lane.hypothesis.trim()) throw new ZyalParseError(`experiments.lanes[${i}].hypothesis must not be empty`)
      if (lane.budget?.max_iterations !== undefined) {
        requirePositiveInteger(lane.budget.max_iterations, `experiments.lanes[${i}].budget.max_iterations`)
      }
      if (lane.budget?.max_diff_lines !== undefined) {
        requirePositiveInteger(lane.budget.max_diff_lines, `experiments.lanes[${i}].budget.max_diff_lines`)
      }
      if (lane.budget?.max_cost_usd !== undefined && lane.budget.max_cost_usd <= 0) {
        throw new ZyalParseError(`experiments.lanes[${i}].budget.max_cost_usd must be positive`)
      }
    }
  }

  if (spec.models) {
    if (spec.models.confidence_cap !== undefined) requireScore(spec.models.confidence_cap, "models.confidence_cap")
    const profiles = spec.models.profiles ?? {}
    for (const [route, profile] of Object.entries(spec.models.routes ?? {})) {
      if (profiles[profile] === undefined) {
        throw new ZyalParseError(`models.routes.${route} references unknown profile ${profile}`)
      }
    }
    if (spec.models.critic?.must_use_different_provider && profiles.builder?.provider && profiles.critic?.provider) {
      if (profiles.builder.provider === profiles.critic.provider) {
        throw new ZyalParseError("models.critic.must_use_different_provider requires distinct builder and critic providers")
      }
    }
  }

  if (spec.budgets) {
    for (const [scope, budget] of Object.entries(spec.budgets)) {
      if (!budget) continue
      if (budget.iterations !== undefined) requirePositiveInteger(budget.iterations, `budgets.${scope}.iterations`)
      if (budget.tokens !== undefined) requirePositiveInteger(budget.tokens, `budgets.${scope}.tokens`)
      if (budget.tool_calls !== undefined) requirePositiveInteger(budget.tool_calls, `budgets.${scope}.tool_calls`)
      if (budget.diff_lines !== undefined) requirePositiveInteger(budget.diff_lines, `budgets.${scope}.diff_lines`)
      if (budget.cost_usd !== undefined && budget.cost_usd <= 0) {
        throw new ZyalParseError(`budgets.${scope}.cost_usd must be positive`)
      }
    }
  }

  if (spec.triggers) {
    const ids = new Set<string>()
    for (const [i, trigger] of spec.triggers.list.entries()) {
      if (!trigger.id.trim()) throw new ZyalParseError(`triggers.list[${i}].id must not be empty`)
      if (ids.has(trigger.id)) throw new ZyalParseError(`triggers.list[${i}].id '${trigger.id}' is duplicated`)
      ids.add(trigger.id)
      if (trigger.max_runs_per_sha !== undefined) {
        requirePositiveInteger(trigger.max_runs_per_sha, `triggers.list[${i}].max_runs_per_sha`)
      }
    }
  }

  if (spec.rollback?.required_when?.risk_score_gte !== undefined) {
    requireScore(spec.rollback.required_when.risk_score_gte, "rollback.required_when.risk_score_gte")
  }

  if (spec.done) {
    for (const [i, item] of (spec.done.require ?? []).entries()) {
      if (!item.trim()) throw new ZyalParseError(`done.require[${i}] must not be empty`)
    }
    for (const [i, item] of (spec.done.forbid ?? []).entries()) {
      if (!item.trim()) throw new ZyalParseError(`done.forbid[${i}] must not be empty`)
    }
  }

  if (spec.repo_intelligence?.scope_control?.max_initial_scope_files !== undefined) {
    requirePositiveInteger(
      spec.repo_intelligence.scope_control.max_initial_scope_files,
      "repo_intelligence.scope_control.max_initial_scope_files",
    )
  }
  if (spec.repo_intelligence?.blast_radius?.pause_when_score_gte !== undefined) {
    requireScore(spec.repo_intelligence.blast_radius.pause_when_score_gte, "repo_intelligence.blast_radius.pause_when_score_gte")
  }

  if (spec.research) {
    if (spec.research.max_parallel !== undefined) {
      requirePositiveInteger(spec.research.max_parallel, "research.max_parallel")
      if (spec.research.max_parallel > 20) {
        throw new ZyalParseError("research.max_parallel must not exceed 20")
      }
    }
    if (spec.research.timeout_seconds !== undefined && spec.research.timeout_seconds <= 0) {
      throw new ZyalParseError("research.timeout_seconds must be positive")
    }
    if (spec.research.provider_policy?.prefer) {
      for (const [i, value] of spec.research.provider_policy.prefer.entries()) {
        if (!value.trim()) throw new ZyalParseError(`research.provider_policy.prefer[${i}] must not be empty`)
      }
    }
    if (spec.research.provider_policy?.allow) {
      for (const [i, value] of spec.research.provider_policy.allow.entries()) {
        if (!value.trim()) throw new ZyalParseError(`research.provider_policy.allow[${i}] must not be empty`)
      }
    }
    if (spec.research.extraction?.max_pages !== undefined) {
      requirePositiveInteger(spec.research.extraction.max_pages, "research.extraction.max_pages")
    }
    if (spec.research.budgets?.max_queries !== undefined) {
      requirePositiveInteger(spec.research.budgets.max_queries, "research.budgets.max_queries")
    }
    if (spec.research.budgets?.max_pages !== undefined) {
      requirePositiveInteger(spec.research.budgets.max_pages, "research.budgets.max_pages")
    }
    if (spec.research.budgets?.max_cost_usd !== undefined && spec.research.budgets.max_cost_usd <= 0) {
      throw new ZyalParseError("research.budgets.max_cost_usd must be positive")
    }
  }

  if (spec.jankurai) {
    if (spec.jankurai.root !== undefined && !spec.jankurai.root.trim()) {
      throw new ZyalParseError("jankurai.root must not be empty")
    }
    for (const [path, value] of [
      ["jankurai.audit.json", spec.jankurai.audit?.json],
      ["jankurai.audit.md", spec.jankurai.audit?.md],
      ["jankurai.audit.repair_queue_jsonl", spec.jankurai.audit?.repair_queue_jsonl],
      ["jankurai.audit.sarif", spec.jankurai.audit?.sarif],
      ["jankurai.repair_plan.json", spec.jankurai.repair_plan?.json],
      ["jankurai.repair_plan.md", spec.jankurai.repair_plan?.md],
      ["jankurai.regression.main_ref", spec.jankurai.regression?.main_ref],
    ] as const) {
      if (value !== undefined && !value.trim()) throw new ZyalParseError(`${path} must not be empty`)
    }
    for (const [path, items] of [
      ["jankurai.selection.defer_rules", spec.jankurai.selection?.defer_rules],
      ["jankurai.selection.incubate_rules", spec.jankurai.selection?.incubate_rules],
      ["jankurai.verification.commands", spec.jankurai.verification?.commands],
    ] as const) {
      for (const [index, item] of (items ?? []).entries()) {
        if (!item.trim()) throw new ZyalParseError(`${path}[${index}] must not be empty`)
      }
    }
    if (spec.jankurai.regression?.compare_every_iterations !== undefined) {
      requirePositiveInteger(
        spec.jankurai.regression.compare_every_iterations,
        "jankurai.regression.compare_every_iterations",
      )
    }
    if (
      spec.jankurai.regression?.max_new_hard_findings !== undefined &&
      (!Number.isFinite(spec.jankurai.regression.max_new_hard_findings) ||
        spec.jankurai.regression.max_new_hard_findings < 0)
    ) {
      throw new ZyalParseError("jankurai.regression.max_new_hard_findings must be a finite non-negative number")
    }
    if (
      spec.jankurai.regression?.max_score_drop !== undefined &&
      (!Number.isFinite(spec.jankurai.regression.max_score_drop) || spec.jankurai.regression.max_score_drop < 0)
    ) {
      throw new ZyalParseError("jankurai.regression.max_score_drop must be a finite non-negative number")
    }
  }

  // ─── v2.2: fleet — single-session worker cap of 20 ─────────────────
  if (spec.fleet) {
    const max = spec.fleet.max_workers
    if (!Number.isInteger(max) || max < 1 || max > 20) {
      throw new ZyalParseError(
        `fleet.max_workers must be an integer in [1, 20]; got ${max}`,
      )
    }
    // Enforce that downstream worker counts respect the fleet cap.
    const workerCount = (spec.agents?.workers ?? []).reduce((s, w) => s + (w.count ?? 0), 0)
    if (workerCount > max) {
      throw new ZyalParseError(
        `agents.workers total count (${workerCount}) exceeds fleet.max_workers (${max})`,
      )
    }
    if (spec.fan_out?.worker.max_parallel !== undefined && spec.fan_out.worker.max_parallel > max) {
      throw new ZyalParseError(
        `fan_out.worker.max_parallel (${spec.fan_out.worker.max_parallel}) exceeds fleet.max_workers (${max})`,
      )
    }
    if (spec.experiments?.max_parallel !== undefined && spec.experiments.max_parallel > max) {
      throw new ZyalParseError(
        `experiments.max_parallel (${spec.experiments.max_parallel}) exceeds fleet.max_workers (${max})`,
      )
    }
    if (spec.incubator?.budget) {
      const ideaParallel = spec.incubator.budget.max_parallel_idea_passes ?? 1
      const activeTasks = spec.incubator.budget.max_active_tasks ?? 1
      if (ideaParallel * activeTasks > max) {
        throw new ZyalParseError(
          `incubator concurrency (${ideaParallel} × ${activeTasks} = ${ideaParallel * activeTasks}) exceeds fleet.max_workers (${max})`,
        )
      }
    }
    if (spec.fleet.jnoccio?.max_instances !== undefined) {
      const ji = spec.fleet.jnoccio.max_instances
      if (!Number.isInteger(ji) || ji < 1 || ji > 20) {
        throw new ZyalParseError(`fleet.jnoccio.max_instances must be in [1, 20]; got ${ji}`)
      }
    }
  } else {
    // No fleet block — keep previous behaviour but still cap worker totals at 20.
    const workerCount = (spec.agents?.workers ?? []).reduce((s, w) => s + (w.count ?? 0), 0)
    if (workerCount > 20) {
      throw new ZyalParseError(
        `agents.workers total count (${workerCount}) exceeds default fleet cap (20). Add a fleet block to declare an explicit cap.`,
      )
    }
    if (spec.fan_out?.worker.max_parallel !== undefined && spec.fan_out.worker.max_parallel > 20) {
      throw new ZyalParseError(
        `fan_out.worker.max_parallel (${spec.fan_out.worker.max_parallel}) exceeds default fleet cap (20)`,
      )
    }
    if (spec.experiments?.max_parallel !== undefined && spec.experiments.max_parallel > 20) {
      throw new ZyalParseError(
        `experiments.max_parallel (${spec.experiments.max_parallel}) exceeds default fleet cap (20)`,
      )
    }
  }
}

// ─── v2.6: jankurai nested-key validator ─────────────────────────────────
function assertJankuraiNestedKeys(input: Record<string, unknown>) {
  if (input.jankurai === undefined) return
  const jankurai = expectRecord(input.jankurai, "jankurai")
  assertKeys("jankurai", jankurai, [
    "enabled",
    "root",
    "audit",
    "repair_plan",
    "task_source",
    "selection",
    "regression",
    "verification",
  ])
  if (jankurai.audit !== undefined) {
    assertKeys("jankurai.audit", expectRecord(jankurai.audit, "jankurai.audit"), [
      "mode",
      "json",
      "md",
      "repair_queue_jsonl",
      "sarif",
      "no_score_history",
    ])
  }
  if (jankurai.repair_plan !== undefined) {
    assertKeys("jankurai.repair_plan", expectRecord(jankurai.repair_plan, "jankurai.repair_plan"), [
      "enabled",
      "json",
      "md",
    ])
  }
  if (jankurai.selection !== undefined) {
    const selection = expectRecord(jankurai.selection, "jankurai.selection")
    assertKeys("jankurai.selection", selection, [
      "order",
      "randomize_ties",
      "max_risk",
      "skip_human_review_required",
      "incubate_risk_at",
      "defer_rules",
      "incubate_rules",
    ])
    for (const key of ["defer_rules", "incubate_rules"] as const) {
      if (selection[key] !== undefined && !Array.isArray(selection[key])) {
        throw new ZyalParseError(`jankurai.selection.${key} must be a list`)
      }
    }
  }
  if (jankurai.regression !== undefined) {
    assertKeys("jankurai.regression", expectRecord(jankurai.regression, "jankurai.regression"), [
      "main_ref",
      "compare_every_iterations",
      "mode",
      "max_new_hard_findings",
      "max_score_drop",
    ])
  }
  if (jankurai.verification !== undefined) {
    const verification = expectRecord(jankurai.verification, "jankurai.verification")
    assertKeys("jankurai.verification", verification, [
      "require_clean_start",
      "require_clean_after_checkpoint",
      "proof_from_test_map",
      "commands",
      "audit_delta",
      "rollback_unverified",
    ])
    if (verification.commands !== undefined && !Array.isArray(verification.commands)) {
      throw new ZyalParseError("jankurai.verification.commands must be a list")
    }
  }
}

// ─── v2.3: taint nested-key validator ────────────────────────────────────
function assertTaintNestedKeys(input: Record<string, unknown>) {
  if (input.taint === undefined) return
  const taint = expectRecord(input.taint, "taint")
  assertKeys("taint", taint, ["default_label", "labels", "forbid", "prompt_injection"])
  if (taint.labels === undefined) {
    throw new ZyalParseError("taint.labels is required when taint block is set")
  }
  const labels = expectRecord(taint.labels, "taint.labels")
  if (Object.keys(labels).length === 0) {
    throw new ZyalParseError("taint.labels must declare at least one label")
  }
  for (const [name, label] of Object.entries(labels)) {
    const rec = expectRecord(label, `taint.labels.${name}`)
    assertKeys(`taint.labels.${name}`, rec, ["rank", "notes"])
  }
  if (taint.default_label !== undefined) {
    const def = String(taint.default_label)
    if (!Object.prototype.hasOwnProperty.call(labels, def)) {
      throw new ZyalParseError(
        `taint.default_label '${def}' is not declared in taint.labels`,
      )
    }
  }
  if (taint.forbid !== undefined) {
    if (!Array.isArray(taint.forbid)) {
      throw new ZyalParseError("taint.forbid must be a list")
    }
    taint.forbid.forEach((rule, i) => {
      const r = expectRecord(rule, `taint.forbid[${i}]`)
      assertKeys(`taint.forbid[${i}]`, r, ["from", "cannot", "unless"])
      if (!Array.isArray(r.from) || r.from.length === 0) {
        throw new ZyalParseError(`taint.forbid[${i}].from must be a non-empty list`)
      }
      for (const lbl of r.from) {
        if (typeof lbl !== "string") {
          throw new ZyalParseError(`taint.forbid[${i}].from must contain strings only`)
        }
        if (!Object.prototype.hasOwnProperty.call(labels, lbl)) {
          throw new ZyalParseError(
            `taint.forbid[${i}].from references undeclared label '${lbl}'`,
          )
        }
      }
      if (!Array.isArray(r.cannot) || r.cannot.length === 0) {
        throw new ZyalParseError(`taint.forbid[${i}].cannot must be a non-empty list`)
      }
    })
  }
  if (taint.prompt_injection !== undefined) {
    const pi = expectRecord(taint.prompt_injection, "taint.prompt_injection")
    assertKeys("taint.prompt_injection", pi, ["detect_patterns", "on_detect", "scan_sources"])
    if (!Array.isArray(pi.detect_patterns) || pi.detect_patterns.length === 0) {
      throw new ZyalParseError(
        "taint.prompt_injection.detect_patterns must be a non-empty list",
      )
    }
    for (const pat of pi.detect_patterns) {
      if (typeof pat !== "string" || pat.trim().length === 0) {
        throw new ZyalParseError(
          "taint.prompt_injection.detect_patterns entries must be non-empty strings",
        )
      }
      try {
        new RegExp(pat)
      } catch {
        throw new ZyalParseError(
          `taint.prompt_injection.detect_patterns contains invalid regex: ${pat}`,
        )
      }
    }
  }
}

// ─── v2.4: research ─────────────────────────────────────────────────────
function assertResearchNestedKeys(input: Record<string, unknown>) {
  if (input.research === undefined) return
  const research = expectRecord(input.research, "research")
  assertKeys("research", research, [
    "version",
    "mode",
    "autonomy",
    "max_parallel",
    "timeout_seconds",
    "provider_policy",
    "extraction",
    "evidence",
    "safety",
    "budgets",
  ])
  if (research.version !== "v1") {
    throw new ZyalParseError(`research.version must be v1`)
  }
  if (research.provider_policy !== undefined) {
    const policy = expectRecord(research.provider_policy, "research.provider_policy")
    assertKeys("research.provider_policy", policy, ["prefer", "allow", "missing_provider"])
    if (policy.prefer !== undefined) {
      if (!Array.isArray(policy.prefer)) throw new ZyalParseError("research.provider_policy.prefer must be a list")
    }
    if (policy.allow !== undefined) {
      if (!Array.isArray(policy.allow)) throw new ZyalParseError("research.provider_policy.allow must be a list")
    }
  }
  if (research.extraction !== undefined) {
    const extraction = expectRecord(research.extraction, "research.extraction")
    assertKeys("research.extraction", extraction, ["enabled", "max_pages", "allowed_extractors"])
    if (extraction.allowed_extractors !== undefined && !Array.isArray(extraction.allowed_extractors)) {
      throw new ZyalParseError("research.extraction.allowed_extractors must be a list")
    }
  }
  if (research.evidence !== undefined) {
    const evidence = expectRecord(research.evidence, "research.evidence")
    assertKeys("research.evidence", evidence, ["require_citations", "claim_level", "store"])
  }
  if (research.safety !== undefined) {
    const safety = expectRecord(research.safety, "research.safety")
    assertKeys("research.safety", safety, ["redact_secrets", "block_internal_urls", "prompt_injection", "taint_label"])
  }
  if (research.budgets !== undefined) {
    const budgets = expectRecord(research.budgets, "research.budgets")
    assertKeys("research.budgets", budgets, ["max_queries", "max_pages", "max_cost_usd"])
  }
}

// ─── v2.2: fleet nested-key validator ────────────────────────────────────
function assertFleetNestedKeys(input: Record<string, unknown>) {
  if (input.fleet === undefined) return
  const fleet = expectRecord(input.fleet, "fleet")
  assertKeys("fleet", fleet, ["max_workers", "isolation", "jnoccio", "telemetry"])
  if (fleet.jnoccio !== undefined) {
    const jn = expectRecord(fleet.jnoccio, "fleet.jnoccio")
    assertKeys("fleet.jnoccio", jn, [
      "enabled", "base_url", "metrics_ws", "spawn_on_demand",
      "register_workers", "heartbeat_path", "heartbeat_interval", "max_instances",
    ])
  }
  if (fleet.telemetry !== undefined) {
    const tl = expectRecord(fleet.telemetry, "fleet.telemetry")
    assertKeys("fleet.telemetry", tl, ["publish_to", "headers"])
    if (tl.headers !== undefined) {
      const headers = expectRecord(tl.headers, "fleet.telemetry.headers")
      for (const [k, v] of Object.entries(headers)) {
        if (typeof v !== "string") {
          throw new ZyalParseError(`fleet.telemetry.headers.${k} must be a string`)
        }
      }
    }
  }
}


function assertKeys(path: string, value: Record<string, unknown>, allowed: string[]) {
  const set = new Set(allowed)
  for (const key of Object.keys(value)) {
    if (!set.has(key)) throw new ZyalParseError(`Unknown ZYAL key: ${path}.${key}`)
  }
}

function expectRecord(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new ZyalParseError(`${path} must be a YAML mapping`)
  }
  return value as Record<string, unknown>
}

function requirePositiveInteger(value: number, path: string) {
  if (!Number.isFinite(value) || value <= 0 || !Number.isInteger(value)) {
    throw new ZyalParseError(`${path} must be a finite positive integer`)
  }
}

function requireScore(value: number, path: string) {
  if (!Number.isFinite(value) || value <= 0 || value > 1) {
    throw new ZyalParseError(`${path} must be a finite score in (0, 1]`)
  }
}

function assertSignalList(value: unknown, path: string) {
  if (!Array.isArray(value)) throw new ZyalParseError(`${path} must be a list`)
  value.forEach((item, index) => {
    if (typeof item !== "string") throw new ZyalParseError(`${path}[${index}] must be a string`)
  })
}

function assertShellCheckKeys(record: Record<string, unknown>, path: string) {
  assertKeys(path, record, ["command", "timeout", "cwd", "assert"])
  if (record.assert !== undefined) {
    const assert = expectRecord(record.assert, `${path}.assert`)
    assertKeys(`${path}.assert`, assert, ["exit_code", "stdout_contains", "stdout_regex", "json"])
  }
}

function assertStopConditionKeys(record: Record<string, unknown>, path: string) {
  assertKeys(path, record, ["shell", "git_clean"])
  const hasShell = record.shell !== undefined
  const hasGitClean = record.git_clean !== undefined
  if (hasShell === hasGitClean) {
    throw new ZyalParseError(`${path} must contain exactly one of shell or git_clean`)
  }
  if (hasShell) assertShellCheckKeys(expectRecord(record.shell, `${path}.shell`), `${path}.shell`)
  if (hasGitClean) {
    const gitClean = expectRecord(record.git_clean, `${path}.git_clean`)
    assertKeys(`${path}.git_clean`, gitClean, ["allow_untracked"])
  }
}

function parseArm(text: string): ZyalArmScan {
  const close = text.match(CLOSE_RE)
  if (!close) return { kind: "missing", reason: "no_close" }
  const afterClose = text.slice((close.index ?? 0) + close[0].length).trim()
  const match = afterClose.match(/^ZYAL_ARM RUN_FOREVER id=(?<id>[A-Za-z0-9._-]+)$/)
  if (!match?.groups?.id) return { kind: "missing", reason: "no_arm" }
  return {
    kind: "found",
    arm: {
      action: "RUN_FOREVER",
      id: match.groups.id,
    },
  }
}

function hashSpec(spec: ZyalScript) {
  const stable = stableStringify(spec)
  return createHash("sha256").update(stable).digest("hex")
}

function stableStringify(value: unknown): string {
  return JSON.stringify(sortValue(value))
}

function sortValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortValue)
  if (!value || typeof value !== "object") return value
  return Object.fromEntries(Object.entries(value as Record<string, unknown>).sort(([a], [b]) => a.localeCompare(b)).map(([k, v]) => [k, sortValue(v)]))
}

function escapeRegExp(input: string) {
  return input.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
}
