import type { ZyalFanOutReduce } from "@/agent-script/schema"
import type { FanOutWorkerResult } from "./daemon-fanout"
import { walkJsonPath } from "./daemon-checks"

/**
 * Result reducers for fan-out operations.
 * Takes N worker results and produces a single aggregated output.
 */

export type ReduceResult = {
  readonly strategy: string
  readonly output: Record<string, unknown>
  readonly selected_index?: number
}

/**
 * Apply the configured reduce strategy to a set of worker results.
 */
export function reduceResults(
  spec: ZyalFanOutReduce,
  results: readonly FanOutWorkerResult[],
  shellRunner?: (command: string) => string,
): ReduceResult {
  const successful = results.filter((r) => r.success && r.result)

  switch (spec.strategy) {
    case "merge_all":
      return reduceMergeAll(successful)
    case "best_score":
      return reduceBestScore(successful, spec.score_key ?? "$.score")
    case "vote":
      return reduceVote(successful)
    case "custom_shell":
      return reduceCustomShell(successful, spec.command ?? "echo '{}'", shellRunner)
    default:
      return reduceMergeAll(successful)
  }
}

/**
 * Merge all results into a single array.
 */
function reduceMergeAll(results: readonly FanOutWorkerResult[]): ReduceResult {
  return {
    strategy: "merge_all",
    output: {
      items: results.map((r) => ({
        id: r.id,
        index: r.index,
        ...r.result,
      })),
      count: results.length,
    },
  }
}

/**
 * Pick the result with the highest score at the given JSON path.
 */
function reduceBestScore(results: readonly FanOutWorkerResult[], scoreKey: string): ReduceResult {
  let bestIndex = 0
  let bestScore = -Infinity

  for (let i = 0; i < results.length; i++) {
    const result = results[i]
    if (!result.result) continue
    const score = walkJsonPath(result.result, scoreKey)
    const numScore = typeof score === "number" ? score : parseFloat(String(score))
    if (!isNaN(numScore) && numScore > bestScore) {
      bestScore = numScore
      bestIndex = i
    }
  }

  const best = results[bestIndex]
  return {
    strategy: "best_score",
    output: {
      id: best?.id,
      index: best?.index,
      score: bestScore,
      ...best?.result,
    },
    selected_index: bestIndex,
  }
}

/**
 * Vote: pick the most common result by counting identical outputs.
 */
function reduceVote(results: readonly FanOutWorkerResult[]): ReduceResult {
  const counts = new Map<string, { count: number; result: FanOutWorkerResult }>()

  for (const r of results) {
    const key = JSON.stringify(r.result ?? {})
    const existing = counts.get(key)
    if (existing) {
      existing.count++
    } else {
      counts.set(key, { count: 1, result: r })
    }
  }

  let best: { count: number; result: FanOutWorkerResult } | undefined
  for (const entry of counts.values()) {
    if (!best || entry.count > best.count) {
      best = entry
    }
  }

  return {
    strategy: "vote",
    output: {
      winner_id: best?.result.id,
      winner_index: best?.result.index,
      vote_count: best?.count ?? 0,
      total_votes: results.length,
      ...best?.result.result,
    },
    selected_index: best?.result.index,
  }
}

/**
 * Custom shell: pass all results as JSON to a shell command and return its output.
 */
function reduceCustomShell(
  results: readonly FanOutWorkerResult[],
  command: string,
  shellRunner?: (command: string) => string,
): ReduceResult {
  if (!shellRunner) {
    return {
      strategy: "custom_shell",
      output: {
        error: "No shell runner available for custom_shell reduce strategy",
        items: results.map((r) => ({ id: r.id, ...r.result })),
      },
    }
  }

  try {
    const output = shellRunner(command)
    const parsed = JSON.parse(output)
    return {
      strategy: "custom_shell",
      output: typeof parsed === "object" && parsed !== null ? parsed : { result: parsed },
    }
  } catch {
    return {
      strategy: "custom_shell",
      output: {
        error: "Failed to parse custom_shell output as JSON",
        items: results.map((r) => ({ id: r.id, ...r.result })),
      },
    }
  }
}
