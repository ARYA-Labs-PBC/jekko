import { describe, expect, test } from "bun:test"
import { reduceResults, type ReduceResult } from "@/session/daemon-reduce"
import type { FanOutWorkerResult } from "@/session/daemon-fanout"

describe("daemon reduce", () => {
  const results: FanOutWorkerResult[] = [
    { id: "auth", index: 0, success: true, result: { score: 0.8, files: 3 } },
    { id: "db", index: 1, success: true, result: { score: 0.95, files: 5 } },
    { id: "api", index: 2, success: true, result: { score: 0.6, files: 2 } },
  ]

  test("merge_all collects all results", () => {
    const reduced = reduceResults({ strategy: "merge_all" }, results)
    expect(reduced.strategy).toBe("merge_all")
    expect((reduced.output.items as unknown[]).length).toBe(3)
    expect(reduced.output.count).toBe(3)
  })

  test("best_score picks highest score", () => {
    const reduced = reduceResults(
      { strategy: "best_score", score_key: "$.score" },
      results,
    )
    expect(reduced.strategy).toBe("best_score")
    expect(reduced.output.id).toBe("db")
    expect(reduced.selected_index).toBe(1)
    expect(reduced.output.score).toBe(0.95)
  })

  test("best_score falls back to first when no scores found", () => {
    const noScoreResults: FanOutWorkerResult[] = [
      { id: "a", index: 0, success: true, result: { name: "test" } },
    ]
    const reduced = reduceResults(
      { strategy: "best_score", score_key: "$.nonexistent" },
      noScoreResults,
    )
    expect(reduced.strategy).toBe("best_score")
    expect(reduced.selected_index).toBe(0)
  })

  test("vote picks majority result", () => {
    const votable: FanOutWorkerResult[] = [
      { id: "w1", index: 0, success: true, result: { answer: "yes" } },
      { id: "w2", index: 1, success: true, result: { answer: "yes" } },
      { id: "w3", index: 2, success: true, result: { answer: "no" } },
    ]
    const reduced = reduceResults({ strategy: "vote" }, votable)
    expect(reduced.strategy).toBe("vote")
    expect(reduced.output.vote_count).toBe(2)
    expect(reduced.output.total_votes).toBe(3)
    expect(reduced.output.answer).toBe("yes")
  })

  test("custom_shell uses shellRunner", () => {
    const reduced = reduceResults(
      { strategy: "custom_shell", command: "merge-script" },
      results,
      () => JSON.stringify({ merged: true, total: 3 }),
    )
    expect(reduced.strategy).toBe("custom_shell")
    expect(reduced.output.merged).toBe(true)
    expect(reduced.output.total).toBe(3)
  })

  test("custom_shell handles missing shellRunner gracefully", () => {
    const reduced = reduceResults(
      { strategy: "custom_shell", command: "merge-script" },
      results,
    )
    expect(reduced.strategy).toBe("custom_shell")
    expect(reduced.output.error).toBeDefined()
  })

  test("merge_all filters out failed results", () => {
    const mixed: FanOutWorkerResult[] = [
      { id: "ok", index: 0, success: true, result: { data: 1 } },
      { id: "fail", index: 1, success: false, error: "timeout" },
    ]
    const reduced = reduceResults({ strategy: "merge_all" }, mixed)
    expect(reduced.output.count).toBe(1)
  })
})
