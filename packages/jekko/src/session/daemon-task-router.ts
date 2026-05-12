import type { ZyalIncubator, ZyalIncubatorRouteCondition } from "@/agent-script/schema"
import type { DaemonStore } from "./daemon-store"
import type { Finding } from "./daemon-finding-classifier"
import type { Batch, Wave } from "./daemon-finding-dag"
import { schedule as scheduleWaves } from "./daemon-finding-dag"

export type ReadinessEvidence = {
  testsIdentified: number
  scopeBounded: number
  planReviewed: number
  prototypeValidated: number
  rollbackKnown: number
  affectedFilesKnown: number
  criticalObjectionsResolved: number
}

export type ReadinessPenalties = {
  unresolvedCritical: number
  noProgress: number
}

export type RouteInput = {
  task: Pick<
    DaemonStore.TaskInfo,
    | "id"
    | "title"
    | "attempt_count"
    | "no_progress_count"
    | "risk_score"
    | "readiness_score"
    | "implementation_confidence"
    | "verification_confidence"
  >
  incubator?: ZyalIncubator
  touchedPaths?: string[]
  evidence?: Partial<ReadinessEvidence>
  penalties?: Partial<ReadinessPenalties>
}

export type RouteResult = {
  lane: "normal" | "incubator" | "blocked" | "parallel"
  readinessScore: number
  riskScore: number
  reasons: string[]
}

/** Lane assignment for a finding routed through the PR3 worker pool. */
export type FindingLane = "parallel" | "incubator" | "human_required"

export type FindingRouteDecision = {
  finding: Finding
  lane: FindingLane
  /** Optional reason tag(s) for telemetry. */
  reasons: string[]
}

export type RouteFindingsResult = {
  waves: Wave[]
  decisions: FindingRouteDecision[]
}

/**
 * Plans parallel worker dispatch by combining the path-overlap DAG (Wave[])
 * with per-finding lane routing. Caps and high-severity findings are pulled
 * into the incubator lane regardless of which wave the DAG put them in;
 * everything else flows through `parallel` lanes one batch at a time.
 *
 * Workers consume `waves[w].batches[b]` in order; the daemon-side worker pool
 * (`daemon-worker-pool.ts`) gates concurrency so we never dispatch more than
 * the resolved pool size at once.
 */
export function routeFindings(findings: readonly Finding[]): RouteFindingsResult {
  const decisions: FindingRouteDecision[] = findings.map((finding) => {
    if (finding.cap !== undefined) {
      return { finding, lane: "incubator", reasons: ["cap"] }
    }
    if (finding.severity === "critical" || finding.severity === "high") {
      return { finding, lane: "incubator", reasons: ["hard_severity"] }
    }
    if (touchesCriticalPath(finding.paths)) {
      return { finding, lane: "incubator", reasons: ["critical_path"] }
    }
    return { finding, lane: "parallel", reasons: [] }
  })
  const waves = scheduleWaves(findings)
  return { waves, decisions }
}

export function laneForBatch(batch: Batch, decisions: readonly FindingRouteDecision[]): FindingLane {
  // A batch's lane is the worst lane of any finding in it. In practice a
  // batch holds one finding today (the DAG packs one finding per batch) but
  // we keep the rule conservative for future fan-in.
  const byFingerprint = new Map(decisions.map((d) => [d.finding.fingerprint, d]))
  let lane: FindingLane = "parallel"
  for (const finding of batch.findings) {
    const decision = byFingerprint.get(finding.fingerprint)
    if (!decision) continue
    if (decision.lane === "human_required") return "human_required"
    if (decision.lane === "incubator") lane = "incubator"
  }
  return lane
}

const DEFAULT_EVIDENCE: ReadinessEvidence = {
  testsIdentified: 0,
  scopeBounded: 0,
  planReviewed: 0,
  prototypeValidated: 0,
  rollbackKnown: 0,
  affectedFilesKnown: 0,
  criticalObjectionsResolved: 0,
}

const DEFAULT_PENALTIES: ReadinessPenalties = {
  unresolvedCritical: 0,
  noProgress: 0,
}

export function routeTask(input: RouteInput): RouteResult {
  const readinessScore = computeReadiness({
    evidence: input.evidence,
    penalties: {
      ...input.penalties,
      noProgress: input.penalties?.noProgress ?? input.task.no_progress_count,
    },
    implementationConfidence: input.task.implementation_confidence,
    verificationConfidence: input.task.verification_confidence,
    baselineScore: input.task.readiness_score,
  })
  const riskScore = computeRisk(input)
  if (!input.incubator?.enabled) return { lane: "normal", readinessScore, riskScore, reasons: [] }
  const excludeReasons = excludeReasonsFor({ ...input, readinessScore, riskScore })
  if (excludeReasons.length > 0) return { lane: "normal", readinessScore, riskScore, reasons: [] }
  const reasons = routeReasons({ ...input, readinessScore, riskScore })
  if (reasons.length === 0) return { lane: "normal", readinessScore, riskScore, reasons: [] }
  return { lane: "incubator", readinessScore, riskScore, reasons }
}

export function computeReadiness(input: {
  evidence?: Partial<ReadinessEvidence>
  penalties?: Partial<ReadinessPenalties>
  implementationConfidence?: number
  verificationConfidence?: number
  baselineScore?: number
}) {
  const evidence = { ...DEFAULT_EVIDENCE, ...(input.evidence ?? {}) }
  const penalties = { ...DEFAULT_PENALTIES, ...(input.penalties ?? {}) }
  if (Object.values(evidence).every((value) => value === 0) && input.baselineScore && input.baselineScore > 0) {
    return clamp(input.baselineScore, 0, 1)
  }
  const modelSignal = modelConfidenceCeiling(input)
  return clamp(
    evidence.testsIdentified * 0.18 +
      evidence.scopeBounded * 0.14 +
      evidence.planReviewed * 0.14 +
      evidence.prototypeValidated * 0.15 +
      evidence.rollbackKnown * 0.1 +
      evidence.affectedFilesKnown * 0.1 +
      evidence.criticalObjectionsResolved * 0.13 +
      modelSignal -
      penalties.unresolvedCritical * 0.25 -
      penalties.noProgress * 0.12,
    0,
    1,
  )
}

export function modelConfidenceCeiling(input: {
  implementationConfidence?: number
  verificationConfidence?: number
}) {
  return (
    0.03 * clamp(input.implementationConfidence ?? 0, 0, 1) +
    0.03 * clamp(input.verificationConfidence ?? 0, 0, 1)
  )
}

function computeRisk(input: RouteInput) {
  const base = clamp(input.task.risk_score ?? 0, 0, 1)
  const attempts = clamp(input.task.attempt_count / 4, 0, 0.35)
  const noProgress = clamp(input.task.no_progress_count / 4, 0, 0.35)
  const pathRisk = (input.touchedPaths ?? []).some(isCriticalPath) ? 0.25 : 0
  return clamp(Math.max(base, base + attempts + noProgress + pathRisk), 0, 1)
}

function routeReasons(input: RouteInput & { readinessScore: number; riskScore: number }) {
  const route = input.incubator?.route_when
  if (!route) return defaultReasons(input)
  return collectReasons(route, input)
}

function excludeReasonsFor(input: RouteInput & { readinessScore: number; riskScore: number }) {
  const route = input.incubator?.exclude_when
  if (!route) return []
  return collectReasons(route, input)
}

function collectReasons(
  route: {
    any?: readonly ZyalIncubatorRouteCondition[]
    all?: readonly ZyalIncubatorRouteCondition[]
  },
  input: RouteInput & { readinessScore: number; riskScore: number },
) {
  const reasons: string[] = []
  const any = route.any ?? []
  if (any.length > 0) {
    reasons.push(...any.flatMap((condition) => matchCondition(condition, input)))
  }
  const all = route.all ?? []
  if (all.length > 0) {
    const matched = all.flatMap((condition) => matchCondition(condition, input))
    if (matched.length === all.length) reasons.push(...matched)
  }
  return [...new Set(reasons)]
}

function defaultReasons(input: RouteInput & { readinessScore: number; riskScore: number }) {
  return [
    ...(input.task.attempt_count >= 2 ? ["repeated_attempts"] : []),
    ...(input.task.no_progress_count >= 2 ? ["no_progress"] : []),
    ...(input.riskScore >= 0.7 ? ["high_risk"] : []),
    ...(input.readinessScore > 0 && input.readinessScore < 0.62 ? ["low_readiness"] : []),
    ...((input.touchedPaths ?? []).some(isCriticalPath) ? ["critical_path"] : []),
  ]
}

function matchCondition(condition: ZyalIncubatorRouteCondition, input: RouteInput & { readinessScore: number; riskScore: number }) {
  const out: string[] = []
  if (condition.repeated_attempts_gte !== undefined && input.task.attempt_count >= condition.repeated_attempts_gte) {
    out.push("repeated_attempts")
  }
  if (
    condition.no_progress_iterations_gte !== undefined &&
    input.task.no_progress_count >= condition.no_progress_iterations_gte
  ) {
    out.push("no_progress")
  }
  if (condition.risk_score_gte !== undefined && input.riskScore >= condition.risk_score_gte) out.push("high_risk")
  if (condition.readiness_score_lt !== undefined && input.readinessScore < condition.readiness_score_lt) {
    out.push("low_readiness")
  }
  if (condition.touches_paths?.length && matchesAny(input.touchedPaths ?? [], condition.touches_paths)) {
    out.push("critical_path")
  }
  return out
}

function matchesAny(paths: readonly string[], patterns: readonly string[]) {
  return paths.some((target) => patterns.some((pattern) => globMatch(target, pattern)))
}

function isCriticalPath(value: string) {
  return [
    "packages/jekko/src/session/",
    "packages/jekko/src/server/",
    "packages/jekko/src/agent-script/",
    "db/migrations/",
  ].some((prefix) => value.startsWith(prefix))
}

/** Public surface for the finding router: any of these paths trip the
 *  incubator lane even at low severity. Mirrors `isCriticalPath` plus the
 *  jankurai canonical-config + migration dirs so cap-adjacent edits route
 *  through review. */
function touchesCriticalPath(paths: readonly string[]): boolean {
  return paths.some((p) =>
    isCriticalPath(p) ||
    p.startsWith("agent/") ||
    p.startsWith(".jekko/agent/") ||
    p.startsWith(".github/workflows/"),
  )
}

function globMatch(value: string, pattern: string) {
  if (!pattern.includes("*")) return value === pattern || value.startsWith(pattern.replace(/\/$/, "") + "/")
  const escaped = pattern
    .split("*")
    .map((part) => part.replace(/[.+?^${}()|[\]\\]/g, "\\$&"))
    .join(".*")
  return new RegExp(`^${escaped}$`).test(value)
}

function clamp(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) return min
  return Math.max(min, Math.min(max, value))
}

export * as DaemonTaskRouter from "./daemon-task-router"
