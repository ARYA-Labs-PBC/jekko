/**
 * AutoResearch score artifact parsers.
 *
 * Reads and parses the daemon's filesystem artifacts:
 * - scoreboard.tsv → LaneScore[]
 * - best-state.json → BestState
 * - exec-score.json → ExecScore
 */

export type LaneScore = {
  readonly laneId: string
  readonly hypothesis: string
  readonly score: number
  readonly rank: number
  readonly iteration: number
  readonly timestamp: number
  readonly isLeader: boolean
  readonly promoted: boolean
  readonly status: "pass" | "fail" | "pending" | "unknown"
  readonly source?: string
  readonly ci95Low?: number
  readonly total?: number
  readonly stressTotal?: number
  readonly gateCount?: number
  readonly costUsd?: number
  readonly delta?: number
}

export type BestState = {
  readonly score: number
  readonly laneId: string
  readonly iteration: number
  readonly timestamp: number
  readonly source?: "winner" | "selected" | "current"
}

export type ExecScore = {
  readonly total: number
  readonly breakdown: Record<string, number>
}

export function parseScoreboard(tsv: string): LaneScore[] {
  const lines = tsv
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .filter((line) => line.trim().length > 0)

  if (lines.length < 2) return []

  const header = lines[0]!.split("\t").map((value) => normalizeHeader(value))
  const rows: LaneScore[] = []

  for (let lineIndex = 1; lineIndex < lines.length; lineIndex++) {
    const cols = lines[lineIndex]!.split("\t")
    const row = readScoreboardRow(header, cols, lineIndex - 1)
    if (row) rows.push(row)
  }

  return rows
}

function readScoreboardRow(header: readonly string[], cols: readonly string[], fallbackRank: number): LaneScore | null {
  const laneId =
    readString(cols, header, "name") ??
    readString(cols, header, "lane_id") ??
    readString(cols, header, "lane") ??
    `lane_${fallbackRank + 1}`

  const score =
    readNumber(cols, header, "total") ??
    readNumber(cols, header, "score") ??
    readNumber(cols, header, "best_score") ??
    0

  const rank = readInt(cols, header, "rank") ?? fallbackRank + 1
  const iteration = readInt(cols, header, "iteration") ?? Math.max(0, rank - 1)
  const timestamp = readInt(cols, header, "timestamp") ?? Date.now()
  const status = parseStatus(readString(cols, header, "status"))

  return {
    laneId,
    hypothesis: readString(cols, header, "hypothesis") ?? readString(cols, header, "source") ?? "",
    score,
    rank,
    iteration,
    timestamp,
    isLeader: false,
    promoted: parseBoolean(readString(cols, header, "promoted") ?? readString(cols, header, "promotion")),
    status,
    source: readString(cols, header, "source") ?? undefined,
    ci95Low: readNumber(cols, header, "ci95_low") ?? undefined,
    total: readNumber(cols, header, "total") ?? undefined,
    stressTotal: readNumber(cols, header, "stress_total") ?? undefined,
    gateCount: readInt(cols, header, "gate_count") ?? undefined,
    costUsd: readNumber(cols, header, "cost_usd") ?? undefined,
    delta: readNumber(cols, header, "delta") ?? undefined,
  }
}

function normalizeHeader(value: string): string {
  return value.trim().toLowerCase().replace(/[^a-z0-9]+/g, "_")
}

function readString(cols: readonly string[], header: readonly string[], key: string): string | undefined {
  const idx = header.indexOf(key)
  if (idx === -1) return undefined
  const value = cols[idx]
  if (value === undefined) return undefined
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : undefined
}

function readNumber(cols: readonly string[], header: readonly string[], key: string): number | undefined {
  const value = readString(cols, header, key)
  if (value === undefined) return undefined
  const next = Number(value)
  return Number.isFinite(next) ? next : undefined
}

function readInt(cols: readonly string[], header: readonly string[], key: string): number | undefined {
  const next = readNumber(cols, header, key)
  return next === undefined ? undefined : Math.trunc(next)
}

function parseBoolean(value: string | undefined): boolean {
  if (!value) return false
  const lower = value.trim().toLowerCase()
  return lower === "1" || lower === "true" || lower === "yes" || lower === "y" || lower === "promoted" || lower === "winner"
}

function parseStatus(raw: string | undefined): LaneScore["status"] {
  const s = raw?.trim().toLowerCase() ?? ""
  if (s === "pass" || s === "passed" || s === "promoted" || s === "winner") return "pass"
  if (s === "fail" || s === "failed" || s === "error") return "fail"
  if (s === "pending" || s === "running" || s === "active") return "pending"
  return "unknown"
}

export function parseBestState(json: string): BestState | null {
  try {
    const obj = JSON.parse(json)
    return parseBestStateValue(obj)
  } catch {
    return null
  }
}

function parseBestStateValue(value: unknown, source?: BestState["source"]): BestState | null {
  if (!value || typeof value !== "object") return null
  const record = value as Record<string, unknown>

  const direct = readBestStateCandidate(record, source)
  if (direct) return direct

  const nestedSources: BestState["source"][] = ["winner", "selected", "current"]
  for (const nestedSource of nestedSources) {
    const nested = record[nestedSource]
    const parsed = parseBestStateValue(nested, nestedSource)
    if (parsed) return parsed
  }

  return null
}

function readBestStateCandidate(record: Record<string, unknown>, source?: BestState["source"]): BestState | null {
  const score = Number(
    record.score ??
      record.total ??
      record.best_score ??
      record.current_best ??
      record.currentScore ??
      record.value,
  )
  if (!Number.isFinite(score)) return null

  const laneId = String(
    record.lane_id ??
      record.laneId ??
      record.lane ??
      record.name ??
      record.id ??
      "unknown",
  )
  const iteration = Number(record.iteration ?? record.iter ?? 0)
  const timestamp = Number(record.timestamp ?? record.ts ?? Date.now())

  return {
    score,
    laneId,
    iteration: Number.isFinite(iteration) ? Math.trunc(iteration) : 0,
    timestamp: Number.isFinite(timestamp) ? Math.trunc(timestamp) : Date.now(),
    source,
  }
}

export function parseExecScore(json: string): ExecScore | null {
  try {
    const obj = JSON.parse(json)
    if (!obj || typeof obj !== "object") return null
    const total = Number(obj.total ?? obj.score ?? obj.total_score)
    if (!Number.isFinite(total)) return null
    const breakdown: Record<string, number> = {}
    const raw = obj.breakdown ?? obj.scores ?? obj.dimensions ?? {}
    if (raw && typeof raw === "object") {
      for (const [key, val] of Object.entries(raw)) {
        const n = Number(val)
        if (Number.isFinite(n)) breakdown[key] = n
      }
    }
    return { total, breakdown }
  } catch {
    return null
  }
}

export function computeLeaders(
  scores: LaneScore[],
  direction: "maximize" | "minimize",
): { withLeaders: LaneScore[]; leaderEnvelope: LaneScore[] } {
  if (scores.length === 0) return { withLeaders: [], leaderEnvelope: [] }

  const sorted = [...scores].sort((a, b) => {
    if (a.iteration !== b.iteration) return a.iteration - b.iteration
    if (a.rank !== b.rank) return a.rank - b.rank
    return direction === "maximize" ? b.score - a.score : a.score - b.score
  })

  const bestScore = direction === "maximize"
    ? Math.max(...sorted.map((score) => score.score))
    : Math.min(...sorted.map((score) => score.score))

  const withLeaders = sorted.map((score) => ({
    ...score,
    isLeader: score.score === bestScore,
  }))

  const leaderEnvelope: LaneScore[] = []
  let runningBest = direction === "maximize" ? -Infinity : Infinity
  for (const score of sorted) {
    const better = direction === "maximize" ? score.score > runningBest : score.score < runningBest
    if (better) {
      runningBest = score.score
      leaderEnvelope.push({ ...score, isLeader: true })
    }
  }

  return { withLeaders, leaderEnvelope }
}
