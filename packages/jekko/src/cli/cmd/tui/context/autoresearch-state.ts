/**
 * AutoResearch reactive state store.
 *
 * Tracks live benchmark scores from ZYAL AutoResearch runs.
 * Populated by the daemon poll loop when a run with AutoResearch artifacts is detected.
 * Consumed by the sidebar widget and the full-screen research dashboard.
 */
import path from "path"
import { createSignal } from "solid-js"
import type { BestState, LaneScore } from "./autoresearch-parser"
import { computeLeaders } from "./autoresearch-parser"

export type GoalDirection = "maximize" | "minimize"

export type AutoResearchSnapshot = {
  readonly active: boolean
  readonly runId: string | null
  readonly daemonDir: string | null
  readonly jobName: string | null
  readonly goalDirection: GoalDirection
  readonly startScore: number | null
  readonly currentBest: number | null
  readonly delta: number | null
  readonly deltaIsGood: boolean
  readonly leaderLaneId: string | null
  readonly iteration: number
  readonly totalLanes: number
  readonly promotedLanes: number
  readonly activeLanes: number
  readonly allScores: LaneScore[]
  readonly leaderEnvelope: LaneScore[]
  readonly recentRuns: LaneScore[]
}

const EMPTY: AutoResearchSnapshot = {
  active: false,
  runId: null,
  daemonDir: null,
  jobName: null,
  goalDirection: "maximize",
  startScore: null,
  currentBest: null,
  delta: null,
  deltaIsGood: false,
  leaderLaneId: null,
  iteration: 0,
  totalLanes: 0,
  promotedLanes: 0,
  activeLanes: 0,
  allScores: [],
  leaderEnvelope: [],
  recentRuns: [],
}

const [snapshot, setSnapshot] = createSignal<AutoResearchSnapshot>(EMPTY)

export function useAutoResearch() {
  return snapshot
}

export function useAutoResearchActive() {
  return () => snapshot().active
}

export function activateAutoResearch(input: {
  runId: string
  daemonDir: string
  jobName: string
  goalDirection: GoalDirection
  totalLanes: number
}) {
  setSnapshot({
    ...EMPTY,
    active: true,
    runId: input.runId,
    daemonDir: input.daemonDir,
    jobName: input.jobName,
    goalDirection: input.goalDirection,
    totalLanes: input.totalLanes,
  })
}

export function deactivateAutoResearch() {
  setSnapshot(EMPTY)
}

export function updateAutoResearchScores(input: {
  scores: LaneScore[]
  bestState: BestState | null
  iteration: number
}) {
  setSnapshot((prev) => {
    const direction = prev.goalDirection
    const { withLeaders, leaderEnvelope } = computeLeaders(input.scores, direction)
    const startScore = resolveStartScore(prev.startScore, withLeaders, direction)
    const currentBest = resolveCurrentBest(input.bestState, withLeaders, direction)
    const delta = startScore !== null && currentBest !== null ? currentBest - startScore : null
    const deltaIsGood = delta !== null ? (direction === "maximize" ? delta > 0 : delta < 0) : false
    const leaderLaneId = input.bestState?.laneId ?? leaderEnvelope.at(-1)?.laneId ?? null
    const promotedLanes = new Set(withLeaders.filter((score) => score.promoted).map((score) => score.laneId)).size
    const activeLanes = new Set(withLeaders.map((score) => score.laneId)).size
    const recentRuns = [...withLeaders].sort((a, b) => b.timestamp - a.timestamp).slice(0, 50)

    return {
      ...prev,
      allScores: withLeaders,
      leaderEnvelope,
      recentRuns,
      startScore,
      currentBest,
      delta,
      deltaIsGood,
      leaderLaneId,
      iteration: input.iteration,
      promotedLanes,
      activeLanes,
    }
  })
}

function resolveStartScore(
  current: number | null,
  scores: LaneScore[],
  direction: GoalDirection,
): number | null {
  if (current !== null) return current
  if (scores.length === 0) return null
  const firstIteration = Math.min(...scores.map((score) => score.iteration))
  const firstScores = scores.filter((score) => score.iteration === firstIteration)
  return direction === "maximize"
    ? Math.max(...firstScores.map((score) => score.score))
    : Math.min(...firstScores.map((score) => score.score))
}

function resolveCurrentBest(
  bestState: BestState | null,
  scores: LaneScore[],
  direction: GoalDirection,
): number | null {
  if (bestState) return bestState.score
  if (scores.length === 0) return null
  return direction === "maximize"
    ? Math.max(...scores.map((score) => score.score))
    : Math.min(...scores.map((score) => score.score))
}

export function detectAutoResearch(run: Record<string, any>): {
  runId: string
  daemonDir: string
  jobName: string
  goalDirection: GoalDirection
  totalLanes: number
} | null {
  const spec = run.spec_json ?? run.spec ?? {}
  const experiments = spec.experiments ?? {}
  const fanOut = spec.fan_out ?? {}

  const totalLanes = deriveLaneCount(experiments, fanOut)
  const hasAutoResearch = totalLanes > 0 || hasScoring(experiments) || hasFanOut(fanOut)
  if (!hasAutoResearch) return null

  const runId = String(run.id ?? run.run_id ?? "")
  if (!runId) return null

  const daemonDir =
    deriveDaemonDirFromRun(run) ??
    deriveDaemonDirFromSpec(spec, runId)

  const jobName = String(spec.job?.name ?? run.name ?? "AutoResearch")
  const goalDirection: GoalDirection = experiments?.scoring?.goal_direction === "minimize" ? "minimize" : "maximize"

  return { runId, daemonDir, jobName, goalDirection, totalLanes }
}

export function deriveDaemonDirFromRun(run: Record<string, any>): string | null {
  for (const candidate of [
    run.daemon_dir,
    run.daemonDir,
    run.artifact_root,
    run.artifactRoot,
    run.paths?.daemon_dir,
    run.paths?.daemonDir,
    run.paths?.artifact_root,
    run.paths?.artifactRoot,
  ]) {
    const resolved = normalizeDaemonDir(candidate)
    if (resolved) return resolved
  }

  const spec = run.spec_json ?? run.spec ?? {}
  return deriveDaemonDirFromSpec(spec, String(run.id ?? run.run_id ?? ""))
}

function deriveDaemonDirFromSpec(spec: Record<string, any>, runId: string): string | null {
  const commandCandidates = [
    spec.experiments?.scoring?.command,
    spec.fan_out?.reduce?.command,
    spec.fan_out?.split?.shell,
  ]

  for (const command of commandCandidates) {
    const fromCommand = deriveDaemonDirFromCommand(command)
    if (fromCommand) return fromCommand
  }

  return runId ? path.posix.join(".jekko", "daemon", runId) : null
}

function deriveDaemonDirFromCommand(command: unknown): string | null {
  if (typeof command !== "string" || command.trim().length === 0) return null
  const candidate = command.match(/(?:^|[\s"'`])((?:\.?\/?[^ \t\n\r"'`]*\.jekko\/daemon\/[^ \t\n\r"'`]+))/)
  if (candidate?.[1]) {
    return normalizeDaemonDir(candidate[1])
  }

  const args = [...command.matchAll(/--(?:out|scoreboard|best-state|promotion-decision|negative-memory|best-patch|curriculum|current-best-state|lanes|population|baseline|exec)\s+([^\s"'`]+)/g)]
  for (const match of args) {
    const resolved = normalizeDaemonDir(match[1])
    if (resolved) return resolved
  }

  return null
}

function normalizeDaemonDir(value: unknown): string | null {
  if (typeof value !== "string") return null
  const trimmed = value.trim()
  if (!trimmed) return null
  if (trimmed.includes(".jekko/daemon/")) {
    const match = trimmed.match(/(?:^|[\\/])(\.jekko[\\/]+daemon[\\/]+[^\\/]+)(?:[\\/].*)?$/)
    if (match?.[1]) return match[1].replace(/\\/g, "/")
  }
  if (trimmed.startsWith(".jekko/daemon/")) {
    const parts = trimmed.split("/").slice(0, 3)
    return parts.join("/")
  }
  return null
}

function deriveLaneCount(experiments: Record<string, any>, fanOut: Record<string, any>): number {
  const laneCount = Array.isArray(experiments.lanes) ? experiments.lanes.length : 0
  if (laneCount > 0) return laneCount
  const maxParallel = Number(fanOut?.worker?.max_parallel ?? fanOut?.max_parallel ?? experiments?.max_parallel ?? 0)
  return Number.isFinite(maxParallel) && maxParallel > 0 ? Math.trunc(maxParallel) : 0
}

function hasScoring(experiments: Record<string, any>): boolean {
  return typeof experiments?.scoring?.command === "string" && experiments.scoring.command.trim().length > 0
}

function hasFanOut(fanOut: Record<string, any>): boolean {
  return !!fanOut && typeof fanOut === "object" && (
    typeof fanOut?.reduce?.command === "string" ||
    typeof fanOut?.split?.shell === "string" ||
    Number.isFinite(Number(fanOut?.worker?.max_parallel ?? fanOut?.max_parallel))
  )
}
