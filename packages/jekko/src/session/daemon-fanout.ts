import type { ZyalFanOut, ZyalFanOutSplit, ZyalFanOutReduce } from "@/agent-script/schema"

/**
 * Fan-out orchestrator.
 * Splits work into N items, dispatches workers, collects results, and reduces.
 */

export type FanOutItem = {
  readonly id: string
  readonly index: number
}

export type FanOutWorkerResult = {
  readonly id: string
  readonly index: number
  readonly success: boolean
  readonly result?: Record<string, unknown>
  readonly error?: string
}

export type FanOutResult = {
  readonly items: readonly FanOutItem[]
  readonly results: readonly FanOutWorkerResult[]
  readonly reduced: Record<string, unknown> | null
  readonly partial_failures: number
  readonly action: "completed" | "aborted" | "paused"
}

/**
 * Resolve split items from the fan_out spec.
 * For static items, returns them directly.
 * For shell-based splits, the caller provides a shellRunner.
 */
export function resolveSplitItems(
  split: ZyalFanOutSplit,
  shellRunner?: (command: string) => string[],
): FanOutItem[] {
  if ("items" in split) {
    return split.items.map((id, index) => ({ id, index }))
  }
  if ("shell" in split && shellRunner) {
    const lines = shellRunner(split.shell)
    return lines.filter((l) => l.trim().length > 0).map((id, index) => ({ id: id.trim(), index }))
  }
  return []
}

/**
 * Compute the effective concurrency cap.
 */
export function effectiveParallelism(spec: ZyalFanOut, itemCount: number): number {
  const maxParallel = spec.worker.max_parallel ?? itemCount
  return Math.min(maxParallel, itemCount)
}

/**
 * Determine the fan-out action given partial failures and the on_partial_failure policy.
 */
export function resolvePartialFailureAction(
  spec: ZyalFanOut,
  results: readonly FanOutWorkerResult[],
): "completed" | "aborted" | "paused" {
  const failures = results.filter((r) => !r.success)
  if (failures.length === 0) return "completed"

  switch (spec.on_partial_failure) {
    case "abort":
      return "aborted"
    case "pause":
      return "paused"
    case "continue":
    default:
      return "completed"
  }
}
