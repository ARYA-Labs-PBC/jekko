import type { IterationInfo, RunInfo } from "./daemon-store"

export type DaemonEventLike = {
  readonly event_type?: string
  readonly eventType?: string
  readonly payload_json?: unknown
  readonly payload?: unknown
  readonly iteration?: number
  readonly time_created?: number
}

export type DaemonProgressTone = "info" | "success" | "warning" | "danger" | "muted"

export type DaemonProgressLine = {
  readonly tone: DaemonProgressTone
  readonly text: string
  readonly stage?: string
  readonly reason?: string
}

export type DaemonWorkerWaveSummary = {
  readonly started: number
  readonly verified: number
  readonly blocked: number
  readonly reason?: string
}

export type DaemonProgressSnapshot = {
  readonly lastSuccessfulStage?: string
  readonly recentStages: readonly DaemonProgressLine[]
  readonly blockedReasons: readonly string[]
  readonly seededArtifacts: "missing" | "reused" | "invalid" | "regenerated"
  readonly workerWave?: DaemonWorkerWaveSummary
  readonly checkpoint?: { readonly ok: boolean; readonly reason?: string }
  readonly audit?: { readonly ok: boolean; readonly reason?: string; readonly score?: number; readonly findingCount?: number }
  readonly repairPlan?: { readonly ok: boolean; readonly reason?: string; readonly packetCount?: number }
}

function record(input: DaemonEventLike): Record<string, unknown> {
  return isRecord(input.payload_json) ? input.payload_json : isRecord(input.payload) ? input.payload : {}
}

function eventType(input: DaemonEventLike) {
  return input.event_type ?? input.eventType ?? "event"
}

function stringFrom(value: unknown): string | undefined {
  return typeof value === "string" && value.length > 0 ? value : undefined
}

function numberFrom(value: unknown): number | undefined {
  const next = Number(value)
  return Number.isFinite(next) ? next : undefined
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function eventLabel(input: DaemonEventLike) {
  const type = eventType(input)
  const payload = record(input)

  switch (type) {
    case "run.created":
      return line("info", "run created", "preflight")
    case "run.previewed":
      return line("info", `preflight preview ${previewLabel(payload)}`, "preflight")
    case "daemon.model_profile.rerouted":
      return line("info", "model profile rerouted", "preflight")
    case "jankurai.preflight":
      return line("info", `preflight ${compactReason(payload)}`, "preflight")
    case "jankurai.bootstrap.completed":
      return line("success", `bootstrap completed${compactReasonSuffix(payload)}`, "bootstrap")
    case "jankurai.bootstrap.blocked":
      return line("danger", `bootstrap blocked${compactReasonSuffix(payload)}`, "bootstrap", reason(payload))
    case "jankurai.integration_branch.ready":
      return line("success", `integration_branch.ready${compactReasonSuffix(payload, "branch")}`, "integration_branch.ready")
    case "jankurai.integration_branch.blocked":
      return line("danger", `integration_branch blocked${compactReasonSuffix(payload)}`, "integration_branch.ready", reason(payload))
    case "jankurai.seeded_artifacts.reused":
      return line("success", `seeded_artifacts.reused${seedPathsSuffix(payload)}`, "seeded_artifacts.reused")
    case "jankurai.seeded_artifacts.invalid":
      return line("warning", `seeded_artifacts.invalid${compactReasonSuffix(payload)}`, "seeded_artifacts.invalid", reason(payload))
    case "jankurai.audit.started":
      return line("info", `audit.started${compactReasonSuffix(payload, "command")}`, "audit.started")
    case "jankurai.audit.completed":
      return line(
        payload.ok === false || reason(payload) ? "danger" : "success",
        `audit.completed${auditSuffix(payload)}`,
        "audit.completed",
        reason(payload),
      )
    case "jankurai.repair_plan.completed":
      return line(
        payload.ok === false || reason(payload) ? "danger" : "success",
        `repair_plan.completed${repairPlanSuffix(payload)}`,
        "repair_plan.completed",
        reason(payload),
      )
    case "jankurai.worker_wave.started":
      return line("info", `worker_wave.started${workerWaveSuffix(payload)}`, "worker_wave.started")
    case "jankurai.worker_wave.completed":
      return line(
        ((numberFrom(payload.verified) ?? 0) > 0 ? "success" : reason(payload) ? "warning" : "info"),
        `worker_wave.completed${workerWaveSuffix(payload)}`,
        "worker_wave.completed",
        reason(payload),
      )
    case "jankurai.checkpoint.started":
      return line("info", "checkpoint.started", "checkpoint.started")
    case "jankurai.checkpoint.completed":
      return line(
        payload.ok === false || reason(payload) ? "danger" : "success",
        `checkpoint.completed${checkpointSuffix(payload)}`,
        "checkpoint.completed",
        reason(payload),
      )
    case "jankurai.sleeping":
      return line("muted", `sleeping${sleepSuffix(payload)}`, "sleeping")
    case "stop.evaluated":
      return line("info", `stop.evaluated ${payload.satisfied === true ? "satisfied" : "pending"}`, "stop.evaluated")
    case "iteration.finished":
      return line("info", `iteration finished${compactReasonSuffix(payload, "terminal")}`, "iteration.finished")
    case "iteration.error":
      return line("danger", `iteration error${compactReasonSuffix(payload, "error")}`, "iteration.error", reason(payload))
    case "run.supervisor_restarting":
      return line("warning", "supervisor restarting", "run.supervisor_restarting")
    case "run.crashed":
      return line("danger", `run crashed${compactReasonSuffix(payload, "cause")}`, "run.crashed", reason(payload))
    case "checkpoint.failed_continued":
      return line("warning", `checkpoint failed continued${compactReasonSuffix(payload)}`, "checkpoint.failed_continued", reason(payload))
    case "jankurai.rollback.applied":
      return line("warning", `rollback applied${compactReasonSuffix(payload)}`, "jankurai.rollback.applied", reason(payload))
    case "jankurai.task.leased":
      return line("info", `task leased${compactReasonSuffix(payload, "taskID")}`, "jankurai.task.leased")
    case "jankurai.worker.started":
      return line("info", `worker started${compactReasonSuffix(payload, "workerID")}`, "jankurai.worker.started")
    case "jankurai.worker.verified":
      return line("success", `worker verified${compactReasonSuffix(payload, "taskID")}`, "jankurai.worker.verified")
    case "jankurai.worker.blocked":
      return line("warning", `worker blocked${compactReasonSuffix(payload)}`, "jankurai.worker.blocked", reason(payload))
    default:
      if (type.startsWith("jankurai.")) {
        return line("info", shortStage(type), shortStage(type))
      }
      return line("info", `${type}${compactReasonSuffix(payload)}`)
  }
}

function line(
  tone: DaemonProgressTone,
  text: string,
  stage?: string,
  reason?: string,
): DaemonProgressLine {
  return { tone, text, stage, reason }
}

function shortStage(value: string) {
  return value.replace(/^jankurai\./, "")
}

function compactReason(payload: Record<string, unknown>) {
  return reason(payload) ?? stringFrom(payload.branch) ?? stringFrom(payload.command) ?? stringFrom(payload.reportPath) ?? ""
}

function compactReasonSuffix(payload: Record<string, unknown>, field?: string) {
  const value = field ? payload[field] : undefined
  const direct = stringFrom(value) ?? reason(payload)
  return direct ? ` ${truncate(direct)}` : ""
}

function reason(payload: Record<string, unknown>) {
  return stringFrom(payload.reason) ?? stringFrom(payload.blockedReason) ?? stringFrom(payload.error) ?? stringFrom(payload.cause) ?? stringFrom(payload.message)
}

function previewLabel(payload: Record<string, unknown>) {
  const spec = isRecord(payload.preview) ? payload.preview : undefined
  const job = isRecord(spec?.job) ? spec.job : undefined
  return stringFrom(job?.name) ?? stringFrom(payload.id) ?? "(preview)"
}

function seedPathsSuffix(payload: Record<string, unknown>) {
  const reportPath = stringFrom(payload.reportPath)
  const repairPlanPath = stringFrom(payload.repairPlanPath)
  const parts = [reportPath, repairPlanPath].filter(Boolean).join(", ")
  return parts ? ` ${parts}` : ""
}

function auditSuffix(payload: Record<string, unknown>) {
  const summary = isRecord(payload.summary) ? payload.summary : undefined
  if (!summary) return compactReasonSuffix(payload)
  const parts = [
    numberFrom(summary.score) !== undefined ? `score ${summary.score}` : undefined,
    numberFrom(summary.finding_count) !== undefined ? `findings ${summary.finding_count}` : undefined,
    numberFrom(summary.hard_findings) !== undefined ? `hard ${summary.hard_findings}` : undefined,
    numberFrom(summary.soft_findings) !== undefined ? `soft ${summary.soft_findings}` : undefined,
  ].filter(Boolean)
  return parts.length ? ` ${parts.join(" ")}` : compactReasonSuffix(payload)
}

function repairPlanSuffix(payload: Record<string, unknown>) {
  const packetCount = numberFrom(payload.packet_count)
  return packetCount !== undefined ? ` packets ${packetCount}` : compactReasonSuffix(payload)
}

function workerWaveSuffix(payload: Record<string, unknown>) {
  const parts = [
    numberFrom(payload.workers) !== undefined ? `workers ${payload.workers}` : undefined,
    numberFrom(payload.started) !== undefined ? `started ${payload.started}` : undefined,
    numberFrom(payload.verified) !== undefined ? `verified ${payload.verified}` : undefined,
    numberFrom(payload.blocked) !== undefined ? `blocked ${payload.blocked}` : undefined,
  ].filter(Boolean)
  const suffix = parts.length ? ` ${parts.join(" ")}` : ""
  return `${suffix}${compactReasonSuffix(payload)}`
}

function checkpointSuffix(payload: Record<string, unknown>) {
  const parts = [payload.ok === true ? "ok" : payload.ok === false ? "blocked" : undefined]
  const sha = stringFrom(payload.checkpointSha)
  if (sha) parts.push(`sha ${sha}`)
  const suffix = parts.filter(Boolean).join(" ")
  return suffix ? ` ${suffix}${compactReasonSuffix(payload)}` : compactReasonSuffix(payload)
}

function sleepSuffix(payload: Record<string, unknown>) {
  const delay = stringFrom(payload.sleep) ?? stringFrom(payload.delay) ?? stringFrom(payload.delay_ms)
  return delay ? ` ${delay}` : ""
}

function truncate(value: string, limit = 120) {
  return value.length <= limit ? value : `${value.slice(0, limit - 1)}…`
}

export function collectDaemonProgress(events: readonly DaemonEventLike[]): DaemonProgressSnapshot {
  const recentStages: DaemonProgressLine[] = []
  const blockedReasons: string[] = []
  let lastSuccessfulStage: string | undefined
  let seededArtifacts: DaemonProgressSnapshot["seededArtifacts"] = "missing"
  let workerWave: DaemonWorkerWaveSummary | undefined
  let checkpoint: DaemonProgressSnapshot["checkpoint"]
  let audit: DaemonProgressSnapshot["audit"]
  let repairPlan: DaemonProgressSnapshot["repairPlan"]

  for (const event of events) {
    const line = eventLabel(event)
    if (!line) continue
    if (line.stage) {
      recentStages.push(line)
      if (line.tone !== "danger") lastSuccessfulStage = line.stage
      if (recentStages.length > 10) recentStages.shift()
    }
    if (line.reason && !blockedReasons.includes(line.reason)) {
      blockedReasons.push(line.reason)
      if (blockedReasons.length > 5) blockedReasons.shift()
    }

    const payload = record(event)
    switch (eventType(event)) {
      case "jankurai.seeded_artifacts.reused":
        seededArtifacts = "reused"
        break
      case "jankurai.seeded_artifacts.invalid":
        seededArtifacts = "invalid"
        break
      case "jankurai.audit.started":
      case "jankurai.audit.completed":
        if (seededArtifacts !== "reused") seededArtifacts = "regenerated"
        if (eventType(event) === "jankurai.audit.completed") {
          audit = {
            ok: payload.ok !== false && !reason(payload),
            reason: reason(payload),
            score: numberFrom(isRecord(payload.summary) ? payload.summary.score : undefined),
            findingCount: numberFrom(isRecord(payload.summary) ? payload.summary.finding_count : undefined),
          }
        }
        break
      case "jankurai.repair_plan.completed":
        repairPlan = {
          ok: payload.ok !== false && !reason(payload),
          reason: reason(payload),
          packetCount: numberFrom(payload.packet_count),
        }
        break
      case "jankurai.worker_wave.completed":
        workerWave = {
          started: numberFrom(payload.started) ?? 0,
          verified: numberFrom(payload.verified) ?? 0,
          blocked: numberFrom(payload.blocked) ?? 0,
          reason: reason(payload),
        }
        break
      case "jankurai.checkpoint.completed":
        checkpoint = {
          ok: payload.ok !== false && !reason(payload),
          reason: reason(payload),
        }
        break
    }
  }

  return {
    lastSuccessfulStage,
    recentStages,
    blockedReasons,
    seededArtifacts,
    workerWave,
    checkpoint,
    audit,
    repairPlan,
  }
}

export function formatDaemonProgressTrace(progress: DaemonProgressSnapshot) {
  return progress.recentStages.length ? progress.recentStages.map((line) => line.text).join(" -> ") : "(none)"
}

export function formatDaemonRunStatusLine(input: {
  run: Pick<RunInfo, "id" | "status" | "phase" | "iteration" | "epoch" | "last_error">
  taskCount?: number
  progress?: DaemonProgressSnapshot
}) {
  const parts = [
    `run ${input.run.id}`,
    `status ${input.run.status}`,
    `phase ${input.run.phase}`,
    `iter ${input.run.iteration}`,
    `epoch ${input.run.epoch}`,
    input.taskCount !== undefined ? `tasks ${input.taskCount}` : undefined,
    input.progress?.lastSuccessfulStage ? `stage ${input.progress.lastSuccessfulStage}` : undefined,
    input.progress?.seededArtifacts && input.progress.seededArtifacts !== "missing" ? `seed ${input.progress.seededArtifacts}` : undefined,
    input.progress?.blockedReasons?.length ? `blocked ${input.progress.blockedReasons[0]}` : undefined,
    input.run.last_error ? `error ${input.run.last_error}` : undefined,
  ]
    .filter(Boolean)
    .join(" · ")
  return parts
}

export function formatDaemonHeartbeatLine(input: {
  run: Pick<RunInfo, "id" | "status" | "phase" | "iteration" | "epoch" | "last_error">
  progress?: DaemonProgressSnapshot
}) {
  return [
    "daemon heartbeat",
    formatDaemonRunStatusLine({ run: input.run, progress: input.progress }),
  ].join(" · ")
}

export function formatDaemonEventLine(event: DaemonEventLike) {
  return eventLabel(event)
}

export function isTerminalDaemonStatus(status: string | undefined) {
  return status === "satisfied" || status === "aborted" || status === "failed" || status === "paused"
}

export function readDaemonProgressEvents(input: readonly DaemonEventLike[]) {
  return input.map((event) => eventLabel(event)).filter((line): line is DaemonProgressLine => line !== undefined)
}
