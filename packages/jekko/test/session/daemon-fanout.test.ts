import { describe, expect, test } from "bun:test"
import {
  resolveSplitItems,
  effectiveParallelism,
  resolvePartialFailureAction,
  type FanOutWorkerResult,
} from "@/session/daemon-fanout"
import type { ZyalFanOut } from "@/agent-script/schema"

describe("daemon fanout", () => {
  const makeFanOut = (overrides: Partial<ZyalFanOut> = {}): ZyalFanOut => ({
    split: { items: ["a", "b", "c"] },
    worker: { max_parallel: 3 },
    reduce: { strategy: "merge_all" },
    ...overrides,
  })

  test("resolveSplitItems with static items", () => {
    const items = resolveSplitItems({ items: ["auth", "database", "api"] })
    expect(items).toHaveLength(3)
    expect(items[0].id).toBe("auth")
    expect(items[0].index).toBe(0)
    expect(items[2].id).toBe("api")
    expect(items[2].index).toBe(2)
  })

  test("resolveSplitItems with shell runner", () => {
    const items = resolveSplitItems(
      { shell: "echo items" },
      () => ["lint", "typecheck", "test"],
    )
    expect(items).toHaveLength(3)
    expect(items[0].id).toBe("lint")
  })

  test("resolveSplitItems filters empty lines", () => {
    const items = resolveSplitItems(
      { shell: "echo items" },
      () => ["lint", "", "  ", "test"],
    )
    expect(items).toHaveLength(2)
    expect(items[0].id).toBe("lint")
    expect(items[1].id).toBe("test")
  })

  test("effectiveParallelism caps at item count", () => {
    const spec = makeFanOut({ worker: { max_parallel: 10 } })
    expect(effectiveParallelism(spec, 3)).toBe(3)
  })

  test("effectiveParallelism caps at max_parallel", () => {
    const spec = makeFanOut({ worker: { max_parallel: 2 } })
    expect(effectiveParallelism(spec, 5)).toBe(2)
  })

  test("effectiveParallelism defaults to item count when unset", () => {
    const spec = makeFanOut({ worker: {} })
    expect(effectiveParallelism(spec, 4)).toBe(4)
  })

  test("resolvePartialFailureAction returns completed with no failures", () => {
    const spec = makeFanOut()
    const results: FanOutWorkerResult[] = [
      { id: "a", index: 0, success: true },
      { id: "b", index: 1, success: true },
    ]
    expect(resolvePartialFailureAction(spec, results)).toBe("completed")
  })

  test("resolvePartialFailureAction returns aborted when policy is abort", () => {
    const spec = makeFanOut({ on_partial_failure: "abort" })
    const results: FanOutWorkerResult[] = [
      { id: "a", index: 0, success: true },
      { id: "b", index: 1, success: false, error: "timeout" },
    ]
    expect(resolvePartialFailureAction(spec, results)).toBe("aborted")
  })

  test("resolvePartialFailureAction returns paused when policy is pause", () => {
    const spec = makeFanOut({ on_partial_failure: "pause" })
    const results: FanOutWorkerResult[] = [
      { id: "a", index: 0, success: false, error: "crash" },
    ]
    expect(resolvePartialFailureAction(spec, results)).toBe("paused")
  })

  test("resolvePartialFailureAction returns completed when policy is continue", () => {
    const spec = makeFanOut({ on_partial_failure: "continue" })
    const results: FanOutWorkerResult[] = [
      { id: "a", index: 0, success: false, error: "fail" },
      { id: "b", index: 1, success: true },
    ]
    expect(resolvePartialFailureAction(spec, results)).toBe("completed")
  })
})
