import { createHash } from "crypto"
import { mkdir, mkdtemp, readFile, rm, writeFile } from "fs/promises"
import os from "os"
import path from "path"
import { Effect } from "effect"
import { ulid } from "ulid"
import type { ZyalJankurai, ZyalJankuraiAuditDelta, ZyalJankuraiAuditMode, ZyalJankuraiRisk, ZyalScript } from "@/agent-script/schema"
import type { Worktree } from "@/worktree"
import type { Session } from "./session"
import type { SessionPrompt } from "./prompt"
import { SessionID } from "./schema"
import type { DaemonChecks, ShellCheckResult } from "./daemon-checks"
import type { DaemonStore } from "./daemon-store"

type JsonRecord = Record<string, unknown>

export type JankuraiConfig = {
  readonly enabled: boolean
  readonly root: string
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
    readonly order: "quick_wins_first" | "severity_first" | "random"
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
  return {
    enabled: block.enabled === true,
    root: block.root ?? ".",
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
  const text = [
    ruleID,
    pathValue,
    stringFrom(input.packet.reason),
    stringFrom(input.packet.problem),
    stringFrom(input.packet.why),
    stringFrom(input.finding?.problem),
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
    if (config.verification.require_clean_start) {
      const clean = yield* input.checks.gitClean({ cwd: input.cwd, allowUntracked: false })
      if (!clean.clean) {
        const reason = `jankurai requires a clean start; dirty paths: ${clean.dirty.join(", ")}`
        yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.checkpoint.blocked", { reason })
        return { ok: false, enabled: true as const, reason }
      }
    }
    return { ok: true, enabled: true as const, config }
  })
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
    yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.audit.completed", {
      matched: result.matched,
      exitCode: result.exitCode,
      summary: report ? summarizeReport(report) : undefined,
    })
    return { result, report }
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
    yield* appendOptionalEvent(input.store, input.runID, input.iteration ?? 0, "jankurai.repair_plan.completed", {
      matched: result.matched,
      exitCode: result.exitCode,
      packet_count: extractRepairPackets(plan).length,
    })
    return { result, plan }
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
    const worktree = yield* input.worktree.create({ name: `jankurai-${input.task.id.slice(-12)}` })
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
      yield* input.prompt.prompt({
        sessionID: session.id,
        parts: [{ type: "text", text: buildWorkerPrompt(input.task, input.config) } as any],
      })
      const workerVerification = yield* verifyCandidate({
        cwd: worktree.directory,
        config: input.config,
        checks: input.checks,
        task: input.task,
        beforeReport: input.beforeReport,
      })
      const taskDir = path.join(input.cwd, ".jekko", "daemon", input.run.id, "tasks", input.task.id)
      yield* Effect.promise(() => mkdir(taskDir, { recursive: true }))
      const patchPath = path.join(taskDir, "worker.patch")
      const patch = yield* input.checks.runShellCheck({
        cwd: worktree.directory,
        command: `git diff --binary HEAD > ${shellQuote(patchPath)}`,
        timeout: "1 minute",
      })
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
    const workerCount = Math.max(1, Math.min(10, Math.floor(input.maxWorkers)))
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
}) {
  const report = input.report ? summarizeReport(input.report) : undefined
  const stats = jankuraiTaskStats(input.tasks)
  const activeWorkers = input.workers.filter((worker) => ["active", "running", "leased"].includes(String(worker.status))).length
  return [
    report
      ? `Jankurai: score ${report.score}, findings ${report.finding_count}, hard ${report.hard_findings}, soft ${report.soft_findings}`
      : `Jankurai: no audit report loaded`,
    `Jankurai tasks: queued ${stats.queued}, leased ${stats.leased}, blocked ${stats.blocked}, incubating ${stats.incubating}, done ${stats.done}`,
    `Jankurai workers: active ${activeWorkers}/${input.config.selection.order === "quick_wins_first" ? "quick-wins" : input.config.selection.order}`,
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

function buildWorkerPrompt(task: DaemonStore.TaskInfo, config: JankuraiConfig) {
  const body = asRecord(task.body_json)
  const packet = asRecord(body.packet)
  const finding = asRecord(body.finding)
  return [
    `You are a bounded Jankurai repair worker.`,
    `Task fingerprint: ${task.external_id ?? task.id}`,
    `Rule: ${stringFrom(body.rule_id) ?? stringFrom(packet.rule_id) ?? stringFrom(finding.rule_id) ?? "(unknown)"}`,
    `Allowed paths: ${stringArray(body.allowed_paths).join(", ") || "(none declared)"}`,
    `Forbidden paths: ${stringArray(body.forbidden_paths).join(", ") || "(none declared)"}`,
    `Locked paths: ${stringArray(body.locked_paths).join(", ") || "(none declared)"}`,
    `Proof commands: ${proofCommandsForPacket(packet, finding, config).join("; ") || "(configured verification only)"}`,
    ``,
    `Repair exactly this finding. Do not include secret evidence in logs or comments. Stop and report blocked if the fix needs human review, a migration, generated-output hand edits, public API redesign, or security authority changes.`,
    ``,
    `Finding packet:`,
    JSON.stringify({ packet, finding }, null, 2),
  ].join("\n")
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
