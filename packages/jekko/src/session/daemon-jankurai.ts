import { createHash } from "crypto"
import { mkdir, mkdtemp, readFile, rm, stat, writeFile } from "fs/promises"
import os from "os"
import path from "path"
import { Effect } from "effect"
import { ulid } from "ulid"
import type { ZyalJankurai, ZyalJankuraiAuditDelta, ZyalJankuraiAuditMode, ZyalJankuraiRisk, ZyalScript } from "@/agent-script/schema"
import { detectCanonical } from "@/cli/cmd/jankurai/detect"
import type { Worktree } from "@/worktree"
import type { Session } from "./session"
import type { SessionPrompt } from "./prompt"
import { SessionID } from "./schema"
import type { DaemonChecks, ShellCheckResult } from "./daemon-checks"
import type { DaemonStore } from "./daemon-store"
import type { DaemonProgressSnapshot } from "./daemon-progress"

type JsonRecord = Record<string, unknown>

export type JankuraiConfig = {
  readonly enabled: boolean
  readonly root: string
  readonly pool?: {
    readonly size: number
    readonly hard_cap: number
    readonly branch_prefix: string
    readonly integration_branch?: string
    readonly commit_on_green: boolean
  }
  readonly bootstrap?: {
    readonly run_update_on_start: boolean
    readonly ensure_init: boolean
    readonly ensure_canonical: boolean
    readonly yes: boolean
    readonly strict: boolean
    readonly dry_run: boolean
  }
  readonly audit: {
    readonly mode: ZyalJankuraiAuditMode
    readonly json: string
    readonly md: string
    readonly repair_queue_jsonl?: string
    readonly sarif?: string
    readonly no_score_history: boolean
  }
  readonly repair_plan: {
    readonly enabled: boolean
    readonly json: string
    readonly md: string
  }
  readonly task_source: "repair_plan" | "findings" | "agent_fix_queue" | "repair_queue_jsonl"
  readonly selection: {
    readonly order: "quick_wins_first" | "blocker_first" | "random"
    readonly randomize_ties: boolean
    readonly max_risk: ZyalJankuraiRisk
    readonly skip_human_review_required: boolean
    readonly incubate_risk_at?: ZyalJankuraiRisk
    readonly defer_rules: readonly string[]
    readonly incubate_rules: readonly string[]
  }
  readonly regression: {
    readonly main_ref: string
    readonly compare_every_iterations: number
    readonly mode: ZyalJankuraiAuditMode
    readonly max_new_hard_findings: number
    readonly max_score_drop: number
  }
  readonly verification: {
    readonly require_clean_start: boolean
    readonly require_clean_after_checkpoint: boolean
    readonly proof_from_test_map: boolean
    readonly commands: readonly string[]
    readonly audit_delta: ZyalJankuraiAuditDelta
    readonly rollback_unverified: boolean
  }
}

export type JankuraiReportSummary = {
  readonly score: number
  readonly raw_score?: number
  readonly finding_count: number
  readonly hard_findings: number
  readonly soft_findings: number
  readonly fingerprints: readonly string[]
}

export type JankuraiComparison = {
  readonly ok: boolean
  readonly score_before: number
  readonly score_after: number
  readonly score_drop: number
  readonly new_findings: readonly string[]
  readonly new_hard_findings: readonly string[]
  readonly removed_findings: readonly string[]
  readonly reason?: string
}

export type JankuraiTaskRoute = {
  readonly status: "queued" | "incubating" | "blocked"
  readonly lane: "normal" | "incubator" | "blocked"
  readonly phase: string
  readonly priority: number
  readonly riskScore: number
  readonly blockedReason?: string
}

export type JankuraiIngestResult = {
  readonly upserted: number
  readonly queued: number
  readonly incubating: number
  readonly blocked: number
  readonly tasks: readonly DaemonStore.TaskInfo[]
}

export type JankuraiVerificationResult = {
  readonly ok: boolean
  readonly commands: readonly ShellCheckResult[]
  readonly comparison?: JankuraiComparison
  readonly reason?: string
}

const RISK_ORDER: Record<ZyalJankuraiRisk, number> = {
  low: 1,
  medium: 2,
  high: 3,
  critical: 4,
}

const RISK_SCORE: Record<ZyalJankuraiRisk, number> = {
  low: 0.2,
  medium: 0.5,
  high: 0.8,
  critical: 1,
}

const SEVERITY_RISK: Record<string, ZyalJankuraiRisk> = {
  info: "low",
  low: "low",
  medium: "medium",
  med: "medium",
  high: "high",
  critical: "critical",
}

const NEVER_AUTO_RULES = new Set([
  "HLT-010-SECRET-SPRAWL",
  "HLT-021-DESTRUCTIVE-MIGRATION",
])

export function resolveJankuraiConfig(spec: ZyalScript): JankuraiConfig | undefined {
  const block = spec.jankurai
  if (!block) return undefined
  const auditMode = block.audit?.mode ?? "advisory"
  const pool = block.pool
  return {
    enabled: block.enabled === true,
    root: block.root ?? ".",
    pool: pool
      ? {
          size: Math.max(1, Math.floor(pool.size ?? 5)),
          hard_cap: Math.max(1, Math.floor(pool.hard_cap ?? 20)),
          branch_prefix: pool.branch_prefix ?? "zyal/jankurai-port",
          integration_branch: pool.integration_branch,
          commit_on_green: pool.commit_on_green ?? true,
        }
      : undefined,
    bootstrap: block.bootstrap
      ? {
          run_update_on_start: block.bootstrap.run_update_on_start ?? false,
          ensure_init: block.bootstrap.ensure_init ?? false,
          ensure_canonical: block.bootstrap.ensure_canonical ?? false,
          yes: block.bootstrap.yes ?? false,
          strict: block.bootstrap.strict ?? false,
          dry_run: block.bootstrap.dry_run ?? false,
        }
      : undefined,
    audit: {
      mode: auditMode,
      json: block.audit?.json ?? "target/jankurai/repo-score.json",
      md: block.audit?.md ?? "target/jankurai/repo-score.md",
      repair_queue_jsonl: block.audit?.repair_queue_jsonl,
      sarif: block.audit?.sarif,
      no_score_history: block.audit?.no_score_history ?? true,
    },
    repair_plan: {
      enabled: block.repair_plan?.enabled ?? true,
      json: block.repair_plan?.json ?? "target/jankurai/repair-plan.json",
      md: block.repair_plan?.md ?? "target/jankurai/repair-plan.md",
    },
    task_source: block.task_source ?? "repair_plan",
    selection: {
      order: block.selection?.order ?? "quick_wins_first",
      randomize_ties: block.selection?.randomize_ties ?? true,
      max_risk: block.selection?.max_risk ?? "low",
      skip_human_review_required: block.selection?.skip_human_review_required ?? true,
      incubate_risk_at: block.selection?.incubate_risk_at,
      defer_rules: block.selection?.defer_rules ?? [],
      incubate_rules: block.selection?.incubate_rules ?? [],
    },
    regression: {
      main_ref: block.regression?.main_ref ?? "origin/main",
      compare_every_iterations: block.regression?.compare_every_iterations ?? 5,
      mode: block.regression?.mode ?? auditMode,
      max_new_hard_findings: block.regression?.max_new_hard_findings ?? 0,
      max_score_drop: block.regression?.max_score_drop ?? 0,
    },
    verification: {
      require_clean_start: block.verification?.require_clean_start ?? true,
      require_clean_after_checkpoint: block.verification?.require_clean_after_checkpoint ?? true,
      proof_from_test_map: block.verification?.proof_from_test_map ?? true,
      commands: block.verification?.commands ?? [],
      audit_delta: block.verification?.audit_delta ?? "no_new_findings",
      rollback_unverified: block.verification?.rollback_unverified ?? true,
    },
  }
}

export function isJankuraiEnabled(spec: ZyalScript) {
  return resolveJankuraiConfig(spec)?.enabled === true
}

export function jankuraiTaskID(fingerprint: string) {
  const hash = createHash("sha256").update(fingerprint).digest("hex").replace(/[^A-Za-z0-9]/g, "-")
  return `jankurai-${hash}`
}

export function summarizeReport(report: unknown): JankuraiReportSummary {
  const record = asRecord(report)
  const findings = extractFindings(report)
  const score = numberFrom(record.score) ?? numberFrom(asRecord(record.decision).score) ?? 0
  const rawScore = numberFrom(record.raw_score)
  const hard = numberFrom(record.hard_findings) ?? findings.filter(isHardFinding).length
  const soft = numberFrom(record.soft_findings) ?? Math.max(0, findings.length - hard)
  return {
    score,
    raw_score: rawScore,
    finding_count: numberFrom(record.finding_count) ?? findings.length,
    hard_findings: hard,
    soft_findings: soft,
    fingerprints: findings.flatMap((finding) => stringFrom(finding.fingerprint) ? [stringFrom(finding.fingerprint)!] : []),
  }
}

export function compareReports(input: {
  before: unknown
  after: unknown
  maxNewHardFindings?: number
  maxScoreDrop?: number
  targetFingerprint?: string
  auditDelta?: ZyalJankuraiAuditDelta
}): JankuraiComparison {
  const before = summarizeReport(input.before)
  const after = summarizeReport(input.after)
  const beforeFindings = new Set(before.fingerprints)
  const afterFindings = new Set(after.fingerprints)
  const newFindings = [...afterFindings].filter((fingerprint) => !beforeFindings.has(fingerprint))
  const removedFindings = [...beforeFindings].filter((fingerprint) => !afterFindings.has(fingerprint))
  const afterByFingerprint = new Map(extractFindings(input.after).map((finding) => [stringFrom(finding.fingerprint), finding]))
  const newHardFindings = newFindings.filter((fingerprint) => {
    const finding = afterByFingerprint.get(fingerprint)
    return finding ? isHardFinding(finding) : false
  })
  const scoreDrop = Math.max(0, before.score - after.score)
  const maxNewHardFindings = input.maxNewHardFindings ?? 0
  const maxScoreDrop = input.maxScoreDrop ?? 0
  const delta = input.auditDelta ?? "no_new_findings"
  let reason: string | undefined
  if (newHardFindings.length > maxNewHardFindings) {
    reason = `new hard findings ${newHardFindings.length} exceeds ${maxNewHardFindings}`
  } else if (scoreDrop > maxScoreDrop) {
    reason = `score drop ${scoreDrop} exceeds ${maxScoreDrop}`
  } else if (delta === "no_new_findings" && newFindings.length > 0) {
    reason = `new findings ${newFindings.length} exceeds 0`
  } else if (delta === "no_score_drop" && scoreDrop > 0) {
    reason = `score dropped by ${scoreDrop}`
  } else if (delta === "target_fingerprint_removed" && input.targetFingerprint && afterFindings.has(input.targetFingerprint)) {
    reason = `target fingerprint ${input.targetFingerprint} is still present`
  }
  return {
    ok: reason === undefined,
    score_before: before.score,
    score_after: after.score,
    score_drop: scoreDrop,
    new_findings: newFindings,
    new_hard_findings: newHardFindings,
    removed_findings: removedFindings,
    reason,
  }
}

export function taskRoute(input: {
  config: JankuraiConfig
  packet: JsonRecord
  finding?: JsonRecord
}): JankuraiTaskRoute {
  const ruleID = stringFrom(input.packet.rule_id) ?? stringFrom(input.finding?.rule_id) ?? ""
  const risk = riskForPacket(input.packet, input.finding)
  const eligibility = stringFrom(input.packet.repair_eligibility) ?? "agent-assisted"
  const humanReview = booleanFrom(input.packet.human_review_required) ?? eligibility === "human-required"
  const pathValue = stringFrom(input.packet.finding_path) ?? stringFrom(input.packet.path) ?? stringFrom(input.finding?.path) ?? ""
  const allowedPaths = stringArray(input.packet.allowed_paths)
  const uncoveredRequiredPaths = requiredFixPaths(input.packet, input.finding).filter(
    (requiredPath) => !isCoveredByAllowedPaths(requiredPath, allowedPaths),
  )
  const text = [
    ruleID,
    pathValue,
    stringFrom(input.packet.reason),
    stringFrom(input.packet.problem),
    stringFrom(input.packet.why),
    stringFrom(input.packet.agent_fix),
    stringFrom(input.finding?.problem),
    stringFrom(input.finding?.agent_fix),
  ].filter(Boolean).join(" ").toLowerCase()

  const neverAuto =
    NEVER_AUTO_RULES.has(ruleID) ||
    eligibility === "never-auto" ||
    risk === "critical" ||
    text.includes("secret") ||
    text.includes("credential") ||
    text.includes("db/migrations") ||
    text.includes("migration") ||
    text.includes("public api") ||
    text.includes("auth") ||
    text.includes("security authority") ||
    (Array.isArray(input.packet.forbidden_paths) && pathValue && input.packet.forbidden_paths.includes(pathValue))
  const configuredDefer = input.config.selection.defer_rules.includes(ruleID)
  if (neverAuto || configuredDefer || (humanReview && input.config.selection.skip_human_review_required)) {
    return {
      status: "blocked",
      lane: "blocked",
      phase: "blocked",
      priority: priorityFor(input.config, risk, eligibility, true),
      riskScore: RISK_SCORE[risk],
      blockedReason: neverAuto
        ? "jankurai policy blocks automatic repair"
        : configuredDefer
        ? `rule ${ruleID} deferred by jankurai.selection.defer_rules`
          : "human review required",
    }
  }

  if (uncoveredRequiredPaths.length > 0) {
    return {
      status: "blocked",
      lane: "blocked",
      phase: "blocked",
      priority: priorityFor(input.config, risk, eligibility, true),
      riskScore: RISK_SCORE[risk],
      blockedReason: `required fix path outside allowed_paths: ${uncoveredRequiredPaths.slice(0, 3).join(", ")}`,
    }
  }

  const incubateAt = input.config.selection.incubate_risk_at
  const shouldIncubate =
    input.config.selection.incubate_rules.includes(ruleID) ||
    (incubateAt !== undefined && RISK_ORDER[risk] >= RISK_ORDER[incubateAt])
  if (shouldIncubate) {
    return {
      status: "incubating",
      lane: "incubator",
      phase: "routing_tasks",
      priority: priorityFor(input.config, risk, eligibility, false),
      riskScore: RISK_SCORE[risk],
    }
  }

  if (RISK_ORDER[risk] > RISK_ORDER[input.config.selection.max_risk]) {
    return {
      status: "blocked",
      lane: "blocked",
      phase: "blocked",
      priority: priorityFor(input.config, risk, eligibility, true),
      riskScore: RISK_SCORE[risk],
      blockedReason: `risk ${risk} exceeds jankurai.selection.max_risk ${input.config.selection.max_risk}`,
    }
  }

  return {
    status: "queued",
    lane: "normal",
    phase: "queued",
    priority: priorityFor(input.config, risk, eligibility, false),
    riskScore: RISK_SCORE[risk],
  }
}

export function preflight(input: {
  cwd: string
  spec: ZyalScript
  checks: DaemonChecks.Interface
  store?: DaemonStore.Interface
  runID?: string
  iteration?: number
}) {
  return Effect.gen(function* () {
    const config = resolveJankuraiConfig(input.spec)
    if (!config?.enabled) return { ok: true, enabled: false as const }
    yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.preflight", {
      root: config.root,
      require_clean_start: config.verification.require_clean_start,
    })
    let bootstrapReceipt: Awaited<ReturnType<typeof runBootstrap>> | undefined
    if (config.verification.require_clean_start) {
      const clean = yield* input.checks.gitClean({ cwd: input.cwd, allowUntracked: false })
      if (!clean.clean) {
        const reason = `jankurai requires a clean start; dirty paths: ${clean.dirty.join(", ")}`
        yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.checkpoint.blocked", { reason })
        return { ok: false, enabled: true as const, reason }
      }
    }
    if (config.bootstrap) {
      bootstrapReceipt = yield* runBootstrap({
        cwd: input.cwd,
        config,
        checks: input.checks,
        store: input.store,
        runID: input.runID,
        iteration: input.iteration ?? 0,
      })
      if (!bootstrapReceipt.ok) {
        return { ok: false, enabled: true as const, reason: bootstrapReceipt.reason }
      }
    }
    if (config.pool?.integration_branch) {
      const branchResult = yield* bootstrapIntegrationBranch({
        cwd: input.cwd,
        config,
        checks: input.checks,
        store: input.store,
        runID: input.runID,
        iteration: input.iteration ?? 0,
      })
      if (!branchResult.ok) {
        return { ok: false, enabled: true as const, reason: branchResult.reason }
      }
    }
    return { ok: true, enabled: true as const, config, bootstrap: bootstrapReceipt }
  })
}

export function resolveWorkerPoolSize(input: {
  config: JankuraiConfig
  fleetMaxWorkers?: number
}) {
  const requested = Math.max(1, Math.floor(input.config.pool?.size ?? input.fleetMaxWorkers ?? 1))
  const hardCap = Math.max(1, Math.floor(input.config.pool?.hard_cap ?? 20))
  const fleetCap = Math.max(1, Math.floor(input.fleetMaxWorkers ?? requested))
  return Math.max(1, Math.min(requested, hardCap, fleetCap, 20))
}

export function runAudit(input: {
  cwd: string
  config: JankuraiConfig
  checks: DaemonChecks.Interface
  store?: DaemonStore.Interface
  runID?: string
  iteration?: number
}) {
  return Effect.gen(function* () {
    const command = auditCommand(input.config, input.config.audit.mode)
    yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.audit.started", {
      command,
      json: input.config.audit.json,
    })
    const result = yield* input.checks.runShellCheck({ cwd: input.cwd, command, timeout: "15 minutes" })
    const report = yield* readJsonFile(path.resolve(input.cwd, input.config.audit.json))
    const ok = isAuditReportShape(report)
    const reason = ok ? undefined : "audit report missing or invalid"
    yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.audit.completed", {
      matched: result.matched,
      exitCode: result.exitCode,
      ok,
      reason,
      summary: ok ? summarizeReport(report) : undefined,
    })
    return { result, report: ok ? report : undefined, ok, reason }
  })
}

export function runRepairPlan(input: {
  cwd: string
  config: JankuraiConfig
  checks: DaemonChecks.Interface
  store?: DaemonStore.Interface
  runID?: string
  iteration?: number
}) {
  return Effect.gen(function* () {
    if (!input.config.repair_plan.enabled) return { result: undefined, plan: undefined }
    const command = [
      "jankurai",
      "repair-plan",
      shellQuote(input.config.root),
      "--from",
      shellQuote(input.config.audit.json),
      "--out",
      shellQuote(input.config.repair_plan.json),
      "--md",
      shellQuote(input.config.repair_plan.md),
    ].join(" ")
    const result = yield* input.checks.runShellCheck({ cwd: input.cwd, command, timeout: "15 minutes" })
    const plan = yield* readJsonFile(path.resolve(input.cwd, input.config.repair_plan.json))
    const ok = isRepairPlanShape(plan)
    const reason = ok ? undefined : "repair plan missing or invalid"
    yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.repair_plan.completed", {
      matched: result.matched,
      exitCode: result.exitCode,
      ok,
      reason,
      packet_count: ok ? extractRepairPackets(plan).length : 0,
    })
    return { result, plan: ok ? plan : undefined, ok, reason }
  })
}

export function ingestTasks(input: {
  runID: string
  config: JankuraiConfig
  store: DaemonStore.Interface
  report?: unknown
  repairPlan?: unknown
  repairQueue?: readonly JsonRecord[]
}) {
  return Effect.gen(function* () {
    const findings = extractFindings(input.report)
    const findingsByFingerprint = new Map(findings.map((finding) => [stringFrom(finding.fingerprint), finding]))
    const packets = packetsForSource({
      source: input.config.task_source,
      report: input.report,
      repairPlan: input.repairPlan,
      repairQueue: input.repairQueue,
    })
    const tasks: DaemonStore.TaskInfo[] = []
    for (const packet of packets) {
      const fingerprint = packetFingerprint(packet)
      if (!fingerprint) continue
      const finding = findingsByFingerprint.get(fingerprint)
      const route = taskRoute({ config: input.config, packet, finding })
      const lockedPaths = lockedPathsForPacket(packet, finding)
      const title = taskTitle(packet, finding)
      const body = {
        source: input.config.task_source,
        fingerprint,
        rule_id: stringFrom(packet.rule_id) ?? stringFrom(finding?.rule_id),
        risk: riskForPacket(packet, finding),
        allowed_paths: stringArray(packet.allowed_paths),
        forbidden_paths: stringArray(packet.forbidden_paths),
        locked_paths: lockedPaths,
        proof_commands: proofCommandsForPacket(packet, finding, input.config),
        packet,
        finding,
      }
      const task = yield* input.store.upsertTask({
        id: jankuraiTaskID(fingerprint),
        run_id: input.runID,
        external_id: fingerprint,
        title,
        body_json: body,
        status: route.status,
        lane: route.lane,
        phase: route.phase,
        difficulty_score: RISK_SCORE[riskForPacket(packet, finding)],
        risk_score: route.riskScore,
        readiness_score: route.status === "queued" ? 0.6 : 0.2,
        implementation_confidence: route.status === "queued" ? 0.65 : 0.1,
        verification_confidence: route.status === "queued" ? 0.65 : 0.1,
        attempt_count: 0,
        no_progress_count: 0,
        incubator_round: 0,
        incubator_status: route.lane === "incubator" ? "queued" : "none",
        accepted_artifact_id: null,
        last_assessment_json: { route, ingested_at: Date.now() },
        promotion_result_json: null,
        blocked_reason: route.blockedReason ?? null,
        priority: route.priority,
        lease_worker_id: null,
        lease_expires_at: null,
        locked_paths_json: lockedPaths,
        evidence_json: null,
      } as any)
      yield* input.store.appendEvent({
        runID: input.runID,
        iteration: 0,
        eventType: "jankurai.task.upserted",
        payload: {
          taskID: task.id,
          fingerprint,
          status: task.status,
          lane: task.lane,
          priority: task.priority,
          locked_paths: lockedPaths,
        },
      })
      tasks.push(task)
    }
    return {
      upserted: tasks.length,
      queued: tasks.filter((task) => task.status === "queued").length,
      incubating: tasks.filter((task) => task.lane === "incubator" || task.status === "incubating").length,
      blocked: tasks.filter((task) => task.status === "blocked").length,
      tasks,
    } satisfies JankuraiIngestResult
  })
}

export function leaseConflictFreeTask(input: {
  runID: string
  workerID: string
  config: JankuraiConfig
  store: DaemonStore.Interface
  ttlMs?: number
}) {
  return Effect.gen(function* () {
    const tasks = yield* input.store.listTasks(input.runID)
    const activeLocks = tasks
      .filter((task) => task.status === "leased" && task.lease_expires_at !== null && task.lease_expires_at > Date.now())
      .flatMap((task) => stringArray(task.locked_paths_json))
    for (const task of tasks) {
      if (task.status !== "queued" || task.lane !== "normal") continue
      const body = asRecord(task.body_json)
      const risk = riskValue(stringFrom(body.risk)) ?? "low"
      if (RISK_ORDER[risk] > RISK_ORDER[input.config.selection.max_risk]) continue
      const locks = stringArray(task.locked_paths_json)
      if (locksOverlap(activeLocks, locks)) continue
      const leased = yield* input.store.leaseSpecificTask({
        runID: input.runID,
        taskID: task.id,
        workerID: input.workerID,
        ttlMs: input.ttlMs ?? 15 * 60 * 1000,
        lockedPaths: locks,
      })
      if (leased) {
        yield* input.store.appendEvent({
          runID: input.runID,
          iteration: 0,
          eventType: "jankurai.task.leased",
          payload: { taskID: leased.id, workerID: input.workerID, locked_paths: locks },
        })
        return leased
      }
    }
    return undefined
  })
}

export function runWorkerTask(input: {
  cwd: string
  run: DaemonStore.RunInfo
  task: DaemonStore.TaskInfo
  workerID: string
  config: JankuraiConfig
  beforeReport?: unknown
  sessions: Session.Interface
  prompt: SessionPrompt.Interface
  store: DaemonStore.Interface
  checks: DaemonChecks.Interface
  worktree: Worktree.Interface
}) {
  return Effect.gen(function* () {
    yield* input.store.upsertWorker({
      id: input.workerID,
      run_id: input.run.id,
      role: "jankurai",
      session_id: null,
      worktree_path: null,
      branch: null,
      status: "starting",
      lease_task_id: input.task.id,
      last_heartbeat_at: Date.now(),
    } as any)
    yield* input.store.appendEvent({
      runID: input.run.id,
      iteration: input.run.iteration,
      eventType: "jankurai.worker.started",
      payload: { workerID: input.workerID, taskID: input.task.id },
    })
  const worktree = yield* input.worktree.create({
      name: `jankurai-${input.run.id.slice(-8)}-${input.workerID.slice(-8)}-${input.task.id.slice(-12)}`,
      branchPrefix: input.config.pool?.branch_prefix ?? "jekko",
    })
    try {
      const session = yield* input.sessions.create({
        parentID: SessionID.make(input.run.active_session_id),
        title: `Jankurai ${input.task.external_id ?? input.task.id}`,
        directory: worktree.directory,
      })
      yield* input.store.upsertWorker({
        id: input.workerID,
        run_id: input.run.id,
        role: "jankurai",
        session_id: session.id,
        worktree_path: worktree.directory,
        branch: worktree.branch,
        status: "running",
        lease_task_id: input.task.id,
        last_heartbeat_at: Date.now(),
      } as any)
      const memories = yield* input.store.listTaskMemory({ runID: input.run.id, taskID: input.task.id })
      yield* input.prompt.prompt({
        sessionID: session.id,
        parts: [
          {
            type: "text",
            text: buildWorkerPrompt({
              task: input.task,
              config: input.config,
              memories,
              beforeReport: input.beforeReport,
              workerID: input.workerID,
            }),
          } as any,
        ],
      })
      const workerVerification = yield* verifyCandidate({
        cwd: worktree.directory,
        config: input.config,
        checks: input.checks,
        task: input.task,
        beforeReport: input.beforeReport,
      })
      const workerStatus = yield* input.checks.runShellCheck({
        cwd: worktree.directory,
        command: "git status --porcelain",
        timeout: "1 minute",
      })
      const workerStatusLines = workerStatus.stdout
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter(Boolean)
      if (!workerStatus.matched) {
        const reason = workerStatus.error ?? "worker status check failed"
        yield* input.store.appendTaskMemory({
          runID: input.run.id,
          taskID: input.task.id,
          kind: "risk_review",
          title: `${input.task.title}: blocked`,
          summary: reason,
          payload: {
            reason,
            workerVerification,
            workerStatus,
            patchPath: null,
          },
          importance: 0.7,
          confidence: 0.6,
        })
        yield* input.store.blockTask({
          taskID: input.task.id,
          evidence: { reason, workerVerification, workerStatus, worktree: worktree.directory },
        })
        yield* input.store.appendEvent({
          runID: input.run.id,
          iteration: input.run.iteration,
          eventType: "jankurai.worker.blocked",
          payload: { workerID: input.workerID, taskID: input.task.id, reason, statusLines: workerStatusLines },
        })
        return { ok: false as const, reason, patchPath: null }
      }
      if (workerVerification.ok && workerStatusLines.length === 0) {
        const reason = "worker produced no diff"
        yield* input.store.appendTaskMemory({
          runID: input.run.id,
          taskID: input.task.id,
          kind: "risk_review",
          title: `${input.task.title}: blocked`,
          summary: reason,
          payload: {
            reason,
            workerVerification,
            workerStatus,
            patchPath: null,
          },
          importance: 0.7,
          confidence: 0.6,
        })
        yield* input.store.blockTask({
          taskID: input.task.id,
          evidence: { reason, workerVerification, workerStatus, worktree: worktree.directory },
        })
        yield* input.store.appendEvent({
          runID: input.run.id,
          iteration: input.run.iteration,
          eventType: "jankurai.worker.blocked",
          payload: { workerID: input.workerID, taskID: input.task.id, reason, statusLines: workerStatusLines },
        })
        return { ok: false as const, reason, patchPath: null }
      }
      const taskDir = path.join(input.cwd, ".jekko", "daemon", input.run.id, "tasks", input.task.id)
      yield* Effect.promise(() => mkdir(taskDir, { recursive: true }))
      const patchPath = path.join(taskDir, "worker.patch")
      const patch = yield* input.checks.runShellCheck({
        cwd: worktree.directory,
        command: `git add -N . && git diff --binary HEAD > ${shellQuote(patchPath)}`,
        timeout: "1 minute",
      })
      const patchSize = yield* Effect.promise(() => stat(patchPath).then((file) => file.size).catch(() => 0))
      if (workerVerification.ok && patchSize === 0) {
        const reason = `worker patch empty despite dirty worktree: ${workerStatusLines.slice(0, 5).join(" | ")}`
        yield* input.store.appendTaskMemory({
          runID: input.run.id,
          taskID: input.task.id,
          kind: "risk_review",
          title: `${input.task.title}: blocked`,
          summary: reason,
          payload: {
            reason,
            workerVerification,
            workerStatus,
            patch,
            patchSize,
            statusLines: workerStatusLines,
          },
          importance: 0.7,
          confidence: 0.6,
        })
        yield* input.store.blockTask({
          taskID: input.task.id,
          evidence: { reason, workerVerification, workerStatus, patch, patchSize, statusLines: workerStatusLines },
        })
        yield* input.store.appendEvent({
          runID: input.run.id,
          iteration: input.run.iteration,
          eventType: "jankurai.worker.blocked",
          payload: {
            workerID: input.workerID,
            taskID: input.task.id,
            reason,
            patchPath,
            statusLines: workerStatusLines,
          },
        })
        return { ok: false as const, reason, patchPath }
      }
      const primaryClean = yield* input.checks.gitClean({ cwd: input.cwd, allowUntracked: false })
      const applyCheck = workerVerification.ok && primaryClean.clean
        ? yield* input.checks.runShellCheck({
            cwd: input.cwd,
            command: `git apply --check ${shellQuote(patchPath)}`,
            timeout: "1 minute",
          })
        : undefined
      if (!workerVerification.ok || !primaryClean.clean || applyCheck?.matched === false || patch.matched === false) {
        const reason =
          workerVerification.reason ??
          (!primaryClean.clean ? `primary checkout dirty: ${primaryClean.dirty.join(", ")}` : undefined) ??
          applyCheck?.error ??
          patch.error ??
          "worker patch rejected"
        yield* input.store.appendTaskMemory({
          runID: input.run.id,
          taskID: input.task.id,
          kind: "risk_review",
          title: `${input.task.title}: blocked`,
          summary: reason,
          payload: {
            reason,
            patchPath,
            workerVerification,
            primaryClean,
            applyCheck,
          },
          importance: 0.7,
          confidence: 0.6,
        })
        yield* input.store.blockTask({
          taskID: input.task.id,
          evidence: { reason, workerVerification, patchPath, worktree: worktree.directory },
        })
        yield* input.store.appendEvent({
          runID: input.run.id,
          iteration: input.run.iteration,
          eventType: "jankurai.worker.blocked",
          payload: { workerID: input.workerID, taskID: input.task.id, reason, patchPath },
        })
        return { ok: false as const, reason, patchPath }
      }
      const applied = yield* input.checks.runShellCheck({
        cwd: input.cwd,
        command: `git apply ${shellQuote(patchPath)}`,
        timeout: "1 minute",
      })
      if (!applied.matched) {
        yield* input.store.appendTaskMemory({
          runID: input.run.id,
          taskID: input.task.id,
          kind: "rollback_known",
          title: `${input.task.title}: rollback`,
          summary: applied.error ?? "git apply failed",
          payload: { patchPath, error: applied.error ?? "git apply failed" },
          importance: 0.8,
          confidence: 0.7,
        })
        yield* input.store.blockTask({ taskID: input.task.id, evidence: { reason: applied.error, patchPath } })
        return { ok: false as const, reason: applied.error ?? "git apply failed", patchPath }
      }
      const primaryVerification = yield* verifyCandidate({
        cwd: input.cwd,
        config: input.config,
        checks: input.checks,
        task: input.task,
        beforeReport: input.beforeReport,
      })
      if (!primaryVerification.ok) {
        const rollback = input.config.verification.rollback_unverified
          ? yield* rollbackCandidate({ cwd: input.cwd, patchPath, checks: input.checks })
          : undefined
        yield* input.store.appendTaskMemory({
          runID: input.run.id,
          taskID: input.task.id,
          kind: primaryVerification.comparison ? "regression_fail" : rollback?.ok ? "rollback_known" : "risk_review",
          title: `${input.task.title}: verification failed`,
          summary: primaryVerification.reason ?? "primary verification failed",
          payload: {
            reason: primaryVerification.reason,
            patchPath,
            rollback,
            comparison: primaryVerification.comparison,
          },
          importance: 0.9,
          confidence: 0.75,
        })
        yield* input.store.blockTask({
          taskID: input.task.id,
          evidence: { reason: primaryVerification.reason, patchPath, rollback },
        })
        yield* input.store.appendEvent({
          runID: input.run.id,
          iteration: input.run.iteration,
          eventType: "jankurai.rollback.applied",
          payload: { taskID: input.task.id, rollback },
        })
        return { ok: false as const, reason: primaryVerification.reason ?? "primary verification failed", patchPath }
      }
      yield* input.store.appendTaskMemory({
        runID: input.run.id,
        taskID: input.task.id,
        kind: primaryVerification.comparison ? "regression_pass" : "verification_strategy",
        title: `${input.task.title}: verified`,
        summary: primaryVerification.comparison
          ? `Regression comparison ok: ${primaryVerification.comparison.score_before} -> ${primaryVerification.comparison.score_after}`
          : `Verified ${patchPath}`,
        payload: {
          patchPath,
          workerVerification,
          primaryVerification,
        },
        importance: 0.85,
        confidence: 0.8,
      })
      const artifact = yield* input.store.upsertArtifact({
        id: ulid(),
        run_id: input.run.id,
        task_id: input.task.id,
        pass_id: null,
        kind: "jankurai_worker_patch",
        path_or_ref: patchPath,
        sha: yield* fileSha256(patchPath),
        payload_json: { workerID: input.workerID, worktree: worktree.directory, verification: primaryVerification },
      } as any)
      yield* input.store.completeTask({
        taskID: input.task.id,
        evidence: { patchArtifactID: artifact.id, patchPath, verification: primaryVerification },
      })
      yield* input.store.appendEvent({
        runID: input.run.id,
        iteration: input.run.iteration,
        eventType: "jankurai.worker.verified",
        payload: { workerID: input.workerID, taskID: input.task.id, patchPath, artifactID: artifact.id },
      })
      return { ok: true as const, patchPath, artifactID: artifact.id }
    } finally {
      yield* input.worktree.remove({ directory: worktree.directory }).pipe(Effect.ignore)
      yield* input.store.upsertWorker({
        id: input.workerID,
        run_id: input.run.id,
        role: "jankurai",
        session_id: null,
        worktree_path: worktree.directory,
        branch: worktree.branch,
        status: "idle",
        lease_task_id: null,
        last_heartbeat_at: Date.now(),
      } as any).pipe(Effect.ignore)
    }
  })
}

export function runWorkerPool(input: {
  cwd: string
  run: DaemonStore.RunInfo
  maxWorkers: number
  config: JankuraiConfig
  beforeReport?: unknown
  sessions: Session.Interface
  prompt: SessionPrompt.Interface
  store: DaemonStore.Interface
  checks: DaemonChecks.Interface
  worktree: Worktree.Interface
}) {
  return Effect.gen(function* () {
    const workerCount = resolveWorkerPoolSize({ config: input.config, fleetMaxWorkers: input.maxWorkers })
    const slots = Array.from({ length: workerCount }, (_, index) => index + 1)
    const results = yield* Effect.forEach(
      slots,
      (slot) =>
        Effect.gen(function* () {
          const workerID = `${input.run.id}:jankurai:${slot}`
          yield* input.store.upsertWorker({
            id: workerID,
            run_id: input.run.id,
            role: "jankurai",
            session_id: null,
            worktree_path: null,
            branch: null,
            status: "idle",
            lease_task_id: null,
            last_heartbeat_at: Date.now(),
          } as any)
          const task = yield* leaseConflictFreeTask({
            runID: input.run.id,
            workerID,
            config: input.config,
            store: input.store,
          })
          if (!task) return { workerID, skipped: true as const, reason: "no conflict-free task" }
          return yield* runWorkerTask({
            cwd: input.cwd,
            run: input.run,
            task,
            workerID,
            config: input.config,
            beforeReport: input.beforeReport,
            sessions: input.sessions,
            prompt: input.prompt,
            store: input.store,
            checks: input.checks,
            worktree: input.worktree,
          })
        }).pipe(
          Effect.catch((error) =>
            input.store
              .appendEvent({
                runID: input.run.id,
                iteration: input.run.iteration,
                eventType: "jankurai.worker.blocked",
                payload: { workerSlot: slot, error: String(error) },
              })
              .pipe(Effect.as({ ok: false as const, reason: String(error) })),
          ),
        ),
      { concurrency: workerCount },
    )
    return {
      workers: workerCount,
      results,
      started: results.filter((result) => !("skipped" in result)).length,
      verified: results.filter((result) => "ok" in result && result.ok === true).length,
      blocked: results.filter((result) => "ok" in result && result.ok === false).length,
      reason:
        results.filter((result) => !("skipped" in result)).length === 0
          ? results.find((result) => "skipped" in result)?.reason ?? "no conflict-free task"
          : results.find((result) => "ok" in result && result.ok === false)?.reason,
    }
  })
}

export function verifyCandidate(input: {
  cwd: string
  config: JankuraiConfig
  checks: DaemonChecks.Interface
  task?: DaemonStore.TaskInfo
  beforeReport?: unknown
}) {
  return Effect.gen(function* () {
    const taskBody = asRecord(input.task?.body_json)
    const commands = [...input.config.verification.commands]
    if (input.config.verification.proof_from_test_map) {
      commands.push(...(yield* proofCommandsFromTestMap(input.cwd, stringArray(taskBody.locked_paths))))
    }
    commands.push(...stringArray(taskBody.proof_commands))
    const uniqueCommands = [...new Set(commands.filter((command) => command.trim()))]
    const results: ShellCheckResult[] = []
    for (const command of uniqueCommands) {
      const result = yield* input.checks.runShellCheck({ cwd: input.cwd, command, timeout: "20 minutes" })
      results.push(result)
      if (!result.matched) return { ok: false, commands: results, reason: result.error ?? `command failed: ${command}` }
    }
    const diffCheck = yield* input.checks.runShellCheck({ cwd: input.cwd, command: "git diff --check", timeout: "1 minute" })
    results.push(diffCheck)
    if (!diffCheck.matched) return { ok: false, commands: results, reason: diffCheck.error ?? "git diff --check failed" }
    const audit = yield* runAudit({ cwd: input.cwd, config: input.config, checks: input.checks })
    if (!audit.report) return { ok: false, commands: results, reason: "jankurai audit did not write a JSON report" }
    if (input.beforeReport) {
      const comparison = compareReports({
        before: input.beforeReport,
        after: audit.report,
        maxNewHardFindings: input.config.regression.max_new_hard_findings,
        maxScoreDrop: input.config.regression.max_score_drop,
        targetFingerprint: stringFrom(input.task?.external_id),
        auditDelta: input.config.verification.audit_delta,
      })
      if (!comparison.ok) return { ok: false, commands: results, comparison, reason: comparison.reason }
      return { ok: true, commands: results, comparison }
    }
    return { ok: true, commands: results }
  })
}

export function runMainRegressionAudit(input: {
  cwd: string
  runID: string
  iteration: number
  config: JankuraiConfig
  checks: DaemonChecks.Interface
  store?: DaemonStore.Interface
  branchReport?: unknown
}) {
  return Effect.gen(function* () {
    if (input.iteration % input.config.regression.compare_every_iterations !== 0) {
      return { skipped: true as const, reason: "not scheduled" }
    }
    const receiptRoot = path.resolve(input.cwd, "target", "jankurai", "zyal", input.runID)
    yield* Effect.promise(() => mkdir(receiptRoot, { recursive: true }))
    const tempRoot = yield* Effect.promise(() => mkdtemp(path.join(os.tmpdir(), "jekko-jankurai-main-")))
    const mainReportPath = path.join(receiptRoot, `main-${input.iteration}.json`)
    const mainMdPath = path.join(receiptRoot, `main-${input.iteration}.md`)
    const branchReport = input.branchReport ?? (yield* readJsonFile(path.resolve(input.cwd, input.config.audit.json)))
    const mainRef = input.config.regression.main_ref
    const auditMode = input.config.regression.mode
    const command = [
      `git fetch origin`,
      `git worktree add --detach ${shellQuote(tempRoot)} ${shellQuote(mainRef)}`,
      `jankurai audit ${shellQuote(tempRoot)} --mode ${shellQuote(auditMode)} --json ${shellQuote(mainReportPath)} --md ${shellQuote(mainMdPath)} --no-score-history`,
    ].join(" && ")
    try {
      const result = yield* input.checks.runShellCheck({ cwd: input.cwd, command, timeout: "20 minutes" })
      const mainReport = yield* readJsonFile(mainReportPath)
      const comparison = branchReport && mainReport
        ? compareReports({
            before: mainReport,
            after: branchReport,
            maxNewHardFindings: input.config.regression.max_new_hard_findings,
            maxScoreDrop: input.config.regression.max_score_drop,
            auditDelta: "no_new_findings",
          })
        : undefined
      yield* appendOptionalEvent(input.store, input.runID, input.iteration, comparison?.ok === false ? "jankurai.regression.fail" : "jankurai.regression.pass", {
        command,
        exitCode: result.exitCode,
        comparison,
      })
      return { skipped: false as const, result, comparison, mainReportPath }
    } finally {
      yield* input.checks.runShellCheck({ cwd: input.cwd, command: `git worktree remove --force ${shellQuote(tempRoot)}`, timeout: "1 minute" }).pipe(Effect.ignore)
      yield* Effect.promise(() => rm(tempRoot, { recursive: true, force: true })).pipe(Effect.ignore)
    }
  })
}

export function rollbackCandidate(input: {
  cwd: string
  patchPath: string
  checks: DaemonChecks.Interface
}) {
  return Effect.gen(function* () {
    const result = yield* input.checks.runShellCheck({
      cwd: input.cwd,
      command: `git apply -R ${shellQuote(input.patchPath)}`,
      timeout: "1 minute",
    })
    const clean = yield* input.checks.gitClean({ cwd: input.cwd, allowUntracked: false })
    return { ok: result.matched && clean.clean, result, clean }
  })
}

export function readRepairQueueJsonl(cwd: string, config: JankuraiConfig) {
  return Effect.gen(function* () {
    if (!config.audit.repair_queue_jsonl) return []
    const file = path.resolve(cwd, config.audit.repair_queue_jsonl)
    const text = yield* Effect.promise(() => readFile(file, "utf8")).pipe(Effect.catch(() => Effect.succeed("")))
    return text
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .flatMap((line) => {
        try {
          const parsed = JSON.parse(line)
          return isRecord(parsed) ? [parsed] : []
        } catch {
          return []
        }
      })
  })
}

export function jankuraiTaskStats(tasks: readonly DaemonStore.TaskInfo[]) {
  return {
    queued: tasks.filter((task) => task.status === "queued").length,
    leased: tasks.filter((task) => task.status === "leased").length,
    blocked: tasks.filter((task) => task.status === "blocked").length,
    incubating: tasks.filter((task) => task.status === "incubating" || task.lane === "incubator").length,
    done: tasks.filter((task) => task.status === "done").length,
  }
}

export function promptSummaryLines(input: {
  config: JankuraiConfig
  report?: unknown
  tasks: readonly DaemonStore.TaskInfo[]
  workers: readonly DaemonStore.WorkerInfo[]
  currentTask?: DaemonStore.TaskInfo
  regression?: unknown
  bootstrap?: unknown
  progress?: DaemonProgressSnapshot
}) {
  const report = input.report ? summarizeReport(input.report) : undefined
  const stats = jankuraiTaskStats(input.tasks)
  const activeWorkers = input.workers.filter((worker) => ["active", "running", "leased"].includes(String(worker.status))).length
  return [
    input.progress
      ? `Jankurai stage: ${input.progress.lastSuccessfulStage ?? "(none)"}`
      : `Jankurai stage: (unknown)`,
    input.progress?.recentStages.length
      ? `Jankurai trace: ${input.progress.recentStages.map((line) => line.text).join(" -> ")}`
      : `Jankurai trace: (none)`,
    input.progress?.blockedReasons.length
      ? `Jankurai blocked: ${input.progress.blockedReasons.slice(-3).join(" | ")}`
      : `Jankurai blocked: (none)`,
    input.progress ? `Jankurai seed: ${input.progress.seededArtifacts}` : `Jankurai seed: (unknown)`,
    input.progress?.workerWave
      ? `Jankurai worker wave: verified ${input.progress.workerWave.verified > 0 ? "yes" : "no"} (${input.progress.workerWave.started} started, ${input.progress.workerWave.blocked} blocked${input.progress.workerWave.reason ? `, ${input.progress.workerWave.reason}` : ""})`
      : `Jankurai worker wave: (none)`,
    report
      ? `Jankurai: score ${report.score}, findings ${report.finding_count}, hard ${report.hard_findings}, soft ${report.soft_findings}`
      : `Jankurai: no audit report loaded`,
    `Jankurai tasks: queued ${stats.queued}, leased ${stats.leased}, blocked ${stats.blocked}, incubating ${stats.incubating}, done ${stats.done}`,
    `Jankurai workers: active ${activeWorkers}/${input.config.selection.order === "quick_wins_first" ? "quick-wins" : input.config.selection.order}`,
    input.config.pool
      ? `Jankurai pool: size ${input.config.pool.size}, hard cap ${input.config.pool.hard_cap}, branch prefix ${input.config.pool.branch_prefix}, commit_on_green ${input.config.pool.commit_on_green ? "on" : "off"}`
      : `Jankurai pool: (default worker namespace)`,
    input.config.bootstrap
      ? `Jankurai bootstrap: ${[
          input.config.bootstrap.run_update_on_start ? "update_on_start" : null,
          input.config.bootstrap.ensure_init ? "ensure_init" : null,
          input.config.bootstrap.ensure_canonical ? "ensure_canonical" : null,
          input.config.bootstrap.yes ? "yes" : null,
          input.config.bootstrap.strict ? "strict" : null,
          input.config.bootstrap.dry_run ? "dry_run" : null,
        ]
          .filter(Boolean)
          .join(" ")}`
      : `Jankurai bootstrap: (not configured)`,
    input.bootstrap ? `Jankurai bootstrap receipt: ${JSON.stringify(input.bootstrap)}` : `Jankurai bootstrap receipt: (none)`,
    input.currentTask ? `Jankurai current task: ${input.currentTask.external_id ?? input.currentTask.id}` : `Jankurai current task: (none)`,
    `Jankurai proof: ${input.config.verification.commands.join("; ") || "(configured audit only)"}`,
    input.regression ? `Jankurai regression: ${JSON.stringify(input.regression)}` : `Jankurai regression: (none)`,
  ]
}

function auditCommand(config: JankuraiConfig, mode: ZyalJankuraiAuditMode) {
  return [
    "jankurai",
    "audit",
    shellQuote(config.root),
    "--mode",
    shellQuote(mode),
    "--json",
    shellQuote(config.audit.json),
    "--md",
    shellQuote(config.audit.md),
    config.audit.repair_queue_jsonl ? `--repair-queue-jsonl ${shellQuote(config.audit.repair_queue_jsonl)}` : null,
    config.audit.sarif ? `--sarif ${shellQuote(config.audit.sarif)}` : null,
    config.audit.no_score_history ? "--no-score-history" : null,
  ].filter(Boolean).join(" ")
}

function packetsForSource(input: {
  source: JankuraiConfig["task_source"]
  report?: unknown
  repairPlan?: unknown
  repairQueue?: readonly JsonRecord[]
}) {
  if (input.source === "repair_plan") {
    const packets = extractRepairPackets(input.repairPlan)
    if (packets.length > 0) return packets
  }
  if (input.source === "agent_fix_queue") return extractAgentFixQueue(input.report)
  if (input.source === "repair_queue_jsonl") return [...(input.repairQueue ?? [])]
  return extractFindings(input.report)
}

function extractRepairPackets(plan: unknown): JsonRecord[] {
  const record = asRecord(plan)
  return stringRecordArray(record.packets)
}

function extractFindings(report: unknown): JsonRecord[] {
  const record = asRecord(report)
  return stringRecordArray(record.findings)
}

function extractAgentFixQueue(report: unknown): JsonRecord[] {
  const record = asRecord(report)
  return stringRecordArray(record.agent_fix_queue)
}

function packetFingerprint(packet: JsonRecord) {
  return stringFrom(packet.finding_fingerprint) ?? stringFrom(packet.fingerprint) ?? stringFrom(packet.id)
}

function taskTitle(packet: JsonRecord, finding?: JsonRecord) {
  const rule = stringFrom(packet.rule_id) ?? stringFrom(finding?.rule_id) ?? "jankurai"
  const pathValue = stringFrom(packet.finding_path) ?? stringFrom(packet.path) ?? stringFrom(finding?.path) ?? "."
  const problem = stringFrom(packet.problem) ?? stringFrom(packet.why) ?? stringFrom(finding?.problem) ?? "audit finding"
  return `${rule} ${pathValue}: ${problem}`.slice(0, 240)
}

function riskForPacket(packet: JsonRecord, finding?: JsonRecord): ZyalJankuraiRisk {
  return riskValue(stringFrom(packet.risk_level)) ??
    riskValue(stringFrom(packet.risk)) ??
    riskValue(stringFrom(packet.priority)) ??
    riskValue(stringFrom(packet.severity)) ??
    riskValue(stringFrom(finding?.severity)) ??
    "medium"
}

function riskValue(value: string | undefined): ZyalJankuraiRisk | undefined {
  if (!value) return undefined
  return SEVERITY_RISK[value.toLowerCase()]
}

function lockedPathsForPacket(packet: JsonRecord, finding?: JsonRecord) {
  const paths = [
    ...stringArray(packet.locked_paths),
    ...stringArray(packet.allowed_paths).filter((item) => !item.endsWith("/")),
    stringFrom(packet.finding_path),
    stringFrom(packet.path),
    stringFrom(finding?.path),
  ].filter((item): item is string => typeof item === "string" && item.length > 0)
  return [...new Set(paths)]
}

function proofCommandsForPacket(packet: JsonRecord, finding: JsonRecord | undefined, config: JankuraiConfig) {
  return [
    ...stringArray(packet.required_proof),
    stringFrom(finding?.rerun_command),
    ...config.verification.commands,
  ].filter((item): item is string => typeof item === "string" && item.trim().length > 0)
}

function requiredFixPaths(packet: JsonRecord, finding: JsonRecord | undefined) {
  const sources = [
    stringFrom(packet.agent_fix),
    stringFrom(packet.problem),
    stringFrom(packet.why),
    stringFrom(finding?.agent_fix),
    stringFrom(finding?.problem),
    stringFrom(finding?.reason),
    stringFrom(finding?.rerun_command),
  ].filter((value): value is string => typeof value === "string" && value.length > 0)
  const paths = new Set<string>()
  const pathPattern = /(?:[A-Za-z0-9_.-]+(?:\/[A-Za-z0-9_.*?-]+)+|(?:\.\/)?[A-Za-z0-9_-]+\.[A-Za-z0-9._-]+)/g
  for (const source of sources) {
    for (const match of source.match(pathPattern) ?? []) {
      const normalized = match.replace(/^[("'`]+|[)"'`,.;:]+$/g, "")
      if (normalized) paths.add(normalized)
    }
  }
  return [...paths]
}

function isCoveredByAllowedPaths(candidatePath: string, allowedPaths: readonly string[]) {
  const candidate = normalizeTaskPath(candidatePath)
  if (!candidate) return true
  for (const allowedPath of allowedPaths) {
    const allowed = normalizeTaskPath(allowedPath)
    if (!allowed) continue
    if (allowed.includes("*")) {
      const prefix = allowed.slice(0, allowed.indexOf("*"))
      if (prefix && candidate.startsWith(prefix)) return true
      continue
    }
    if (allowed.endsWith("/")) {
      if (candidate.startsWith(allowed)) return true
      continue
    }
    if (candidate === allowed || candidate.startsWith(`${allowed}/`)) return true
  }
  return false
}

function normalizeTaskPath(value: string) {
  return value.trim().replace(/^\.\//, "").replace(/^\/+/, "")
}

function priorityFor(config: JankuraiConfig, risk: ZyalJankuraiRisk, eligibility: string, blocked: boolean) {
  const quickWin = config.selection.order === "quick_wins_first"
  const riskBase = quickWin ? 5000 - RISK_ORDER[risk] * 1000 : RISK_ORDER[risk] * 1000
  const assisted = eligibility === "agent-assisted" ? 500 : 0
  const blockPenalty = blocked ? -10_000 : 0
  const jitter = config.selection.randomize_ties ? Math.floor(Math.random() * 20) : 0
  return riskBase + assisted + blockPenalty + jitter
}

function isHardFinding(finding: JsonRecord) {
  const hardness = stringFrom(finding.hardness)?.toLowerCase()
  if (hardness === "hard") return true
  const severity = stringFrom(finding.severity)?.toLowerCase()
  return severity === "high" || severity === "critical"
}

function isAuditReportShape(value: unknown): value is JsonRecord {
  const record = asRecord(value)
  return Array.isArray(record.findings)
}

function isRepairPlanShape(value: unknown): value is JsonRecord {
  const record = asRecord(value)
  return Array.isArray(record.packets)
}

function buildWorkerPrompt(input: {
  task: DaemonStore.TaskInfo
  config: JankuraiConfig
  memories: readonly DaemonStore.TaskMemoryInfo[]
  beforeReport?: unknown
  workerID?: string
}) {
  const body = asRecord(input.task.body_json)
  const packet = asRecord(body.packet)
  const finding = asRecord(body.finding)
  const lockedPaths = [...new Set([...stringArray(input.task.locked_paths_json), ...stringArray(body.locked_paths)])]
  const recentMemories = [...input.memories]
    .slice(-8)
    .map((item) => `- ${item.kind}: ${item.title} — ${item.summary}`)
    .join("\n")
  return [
    `You are a bounded Jankurai repair worker.`,
    input.workerID ? `Worker ID: ${input.workerID}` : undefined,
    `Task fingerprint: ${input.task.external_id ?? input.task.id}`,
    `Rule: ${stringFrom(body.rule_id) ?? stringFrom(packet.rule_id) ?? stringFrom(finding.rule_id) ?? "(unknown)"}`,
    `Allowed paths: ${stringArray(body.allowed_paths).join(", ") || "(none declared)"}`,
    `Forbidden paths: ${stringArray(body.forbidden_paths).join(", ") || "(none declared)"}`,
    `Locked paths: ${lockedPaths.join(", ") || "(none declared)"}`,
    `Proof commands: ${proofCommandsForPacket(packet, finding, input.config).join("; ") || "(configured verification only)"}`,
    `Blocked reason: ${stringFrom(input.task.blocked_reason) ?? "(none)"}`,
    input.beforeReport ? `Regression status: ${JSON.stringify(summarizeReport(input.beforeReport))}` : `Regression status: (none)`,
    ``,
    `Recent task memory:`,
    recentMemories ? recentMemories : `- (none)`,
    ``,
    `If you have a verified, low-risk, and reversible slice, commit it to ${input.config.pool?.integration_branch ?? "the integration branch"} before widening scope. Prefer frequent small commits over one large batch. If a change is broken but safely revertible, commit the last known-good slice and let another attempt continue from there.`,
    ``,
    `Repair exactly this finding. Do not include secret evidence in logs or comments. Stop and report blocked if the fix needs human review, a migration, generated-output hand edits, public API redesign, or security authority changes.`,
    ``,
    `Finding packet:`,
    JSON.stringify({ packet, finding }, null, 2),
  ]
    .filter((line) => line !== undefined)
    .join("\n")
}

function bootstrapIntegrationBranch(input: {
  cwd: string
  config: JankuraiConfig
  checks: DaemonChecks.Interface
  store?: DaemonStore.Interface
  runID?: string
  iteration?: number
}) {
  return Effect.gen(function* () {
    const branch = input.config.pool?.integration_branch?.trim()
    if (!branch) return { ok: true as const, skipped: true as const }
    const branchRef = `refs/heads/${branch}`
    const exists = yield* input.checks.runShellCheck({
      cwd: input.cwd,
      command: `git show-ref --verify --quiet ${shellQuote(branchRef)}`,
      timeout: "1 minute",
    })
    const command = exists.exitCode === 0
      ? `git switch ${shellQuote(branch)}`
      : `git switch -c ${shellQuote(branch)}`
    const result = yield* input.checks.runShellCheck({
      cwd: input.cwd,
      command,
      timeout: "1 minute",
    })
    if (!result.matched) {
      const reason = result.error ?? `failed to bootstrap integration branch ${branch}`
      yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.integration_branch.blocked", {
        branch,
        command,
        reason,
      })
      return { ok: false as const, reason }
    }
    yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.integration_branch.ready", {
      branch,
      command,
    })
    return { ok: true as const, branch, command }
  })
}

function runBootstrap(input: {
  cwd: string
  config: JankuraiConfig
  checks: DaemonChecks.Interface
  store?: DaemonStore.Interface
  runID?: string
  iteration?: number
}) {
  return Effect.gen(function* () {
    const bootstrap = input.config.bootstrap
    if (!bootstrap) return { ok: true as const, skipped: true as const }

    const detectionBefore = detectCanonical(input.cwd)
    const commands: string[] = []

    if (bootstrap.run_update_on_start) {
      const updateCommand = "jankurai update --client-start --quiet"
      commands.push(updateCommand)
      const update = yield* input.checks.runShellCheck({
        cwd: input.cwd,
        command: updateCommand,
        timeout: "5 minutes",
      })
      if (!update.matched) {
        const reason = update.error ?? "jankurai update failed"
        yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.bootstrap.blocked", {
          step: "update",
          reason,
          commands,
        })
        return { ok: false as const, reason }
      }
    }

    if (bootstrap.ensure_canonical) {
      const canonicalCommand = bootstrap.dry_run
        ? "jankurai init --dry-run"
        : "jankurai init --yes"
      commands.push(canonicalCommand)
      const canonical = yield* input.checks.runShellCheck({
        cwd: input.cwd,
        command: canonicalCommand,
        timeout: "10 minutes",
      })
      if (!canonical.matched) {
        const reason = canonical.error ?? "jankurai canonical bootstrap failed"
        yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.bootstrap.blocked", {
          step: "canonical",
          reason,
          commands,
        })
        return { ok: false as const, reason }
      }
    }

    const detectionAfter = detectCanonical(input.cwd)
    if ((bootstrap.ensure_init || bootstrap.ensure_canonical) && detectionAfter.missingRequired.length > 0) {
      const reason = `jankurai init incomplete; missing required files: ${detectionAfter.missingRequired.join(", ")}`
      yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.bootstrap.blocked", {
        step: "verify",
        reason,
        commands,
      })
      return { ok: false as const, reason }
    }

    yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.bootstrap.completed", {
      commands,
      missing_required_before: detectionBefore.missingRequired,
      missing_optional_before: detectionBefore.missingOptional,
      missing_required_after: detectionAfter.missingRequired,
      missing_optional_after: detectionAfter.missingOptional,
    })
    return { ok: true as const, commands, detectionBefore, detectionAfter }
  })
}

function proofCommandsFromTestMap(cwd: string, paths: readonly string[]) {
  return Effect.gen(function* () {
    if (paths.length === 0) return []
    const mapPath = path.resolve(cwd, "agent", "test-map.json")
    const map = yield* readJsonFile(mapPath)
    const tests = asRecord(asRecord(map).tests)
    const commands = new Set<string>()
    for (const pathValue of paths) {
      for (const [key, value] of Object.entries(tests)) {
        if (!pathMatches(pathValue, key)) continue
        const command = stringFrom(asRecord(value).command)
        if (command) commands.add(command)
      }
    }
    return [...commands]
  })
}

function pathMatches(file: string, pattern: string) {
  if (pattern.endsWith("/")) return file.startsWith(pattern)
  if (pattern.endsWith("/**")) return file.startsWith(pattern.slice(0, -3))
  return file === pattern || file.startsWith(`${pattern}/`)
}

function locksOverlap(left: readonly string[], right: readonly string[]) {
  for (const a of left) {
    for (const b of right) {
      if (a === b || a.startsWith(`${b}/`) || b.startsWith(`${a}/`)) return true
    }
  }
  return false
}

function readJsonFile(file: string) {
  return Effect.promise(async () => {
    try {
      return JSON.parse(await readFile(file, "utf8")) as unknown
    } catch {
      return undefined
    }
  })
}

function fileSha256(file: string) {
  return Effect.promise(async () => {
    try {
      return createHash("sha256").update(await readFile(file)).digest("hex")
    } catch {
      return null
    }
  })
}

function appendOptionalEvent(
  store: DaemonStore.Interface | undefined,
  runID: string | undefined,
  iteration: number,
  eventType: string,
  payload: Record<string, unknown>,
) {
  if (!store || !runID) return Effect.void
  return store.appendEvent({ runID, iteration, eventType, payload }).pipe(Effect.asVoid)
}

function shellQuote(value: string) {
  return `'${value.replace(/'/g, `'\\''`)}'`
}

function stringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  return value.filter((item): item is string => typeof item === "string")
}

function stringRecordArray(value: unknown): JsonRecord[] {
  if (!Array.isArray(value)) return []
  return value.filter(isRecord)
}

function asRecord(value: unknown): JsonRecord {
  return isRecord(value) ? value : {}
}

function isRecord(value: unknown): value is JsonRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function stringFrom(value: unknown) {
  return typeof value === "string" ? value : undefined
}

function numberFrom(value: unknown) {
  const number = Number(value)
  return Number.isFinite(number) ? number : undefined
}

function booleanFrom(value: unknown) {
  return typeof value === "boolean" ? value : undefined
}

export * as DaemonJankurai from "./daemon-jankurai"
