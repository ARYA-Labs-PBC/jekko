import type { ZyalIncubator, ZyalIncubatorRouteCondition } from "@/agent-script/schema"
import type { DaemonStore } from "./daemon-store"

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
  lane: "normal" | "incubator" | "blocked"
  readinessScore: number
  riskScore: number
  reasons: string[]
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
