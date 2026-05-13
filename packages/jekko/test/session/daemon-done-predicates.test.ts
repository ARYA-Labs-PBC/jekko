import { describe, expect, test } from "bun:test"
import {
  DaemonDonePredicates,
  jankuraiCapsZero,
  noNewHardFindings,
  regressionFreeCycles,
  scoreHistorySlopeFlat24h,
  evaluateDoneRequire,
  resolveDonePredicate,
} from "../../src/session/daemon-done-predicates"

const baseInput = {
  score: null,
  history: [],
  regressionStatuses: [],
} as Parameters<typeof evaluateDoneRequire>[1]

describe("jankuraiCapsZero", () => {
  test("nil score -> unsatisfied", () => {
    expect(jankuraiCapsZero(baseInput).satisfied).toBe(false)
  })
  test("caps==0 -> satisfied", () => {
    expect(jankuraiCapsZero({ ...baseInput, score: { capsApplied: 0, hardFindings: 0, softFindings: 0, score: 95 } }).satisfied).toBe(true)
  })
  test("caps>0 -> unsatisfied", () => {
    const result = jankuraiCapsZero({ ...baseInput, score: { capsApplied: 2, hardFindings: 0, softFindings: 0, score: 75 } })
    expect(result.satisfied).toBe(false)
    expect(result.reason).toContain("caps=2")
  })
})

describe("noNewHardFindings", () => {
  test("history shorter than N -> unsatisfied", () => {
    expect(noNewHardFindings(3)({ ...baseInput, history: [{ ts: 1, score: 80, hardFindings: 2 }] }).satisfied).toBe(false)
  })
  test("non-increasing tail -> satisfied", () => {
    const result = noNewHardFindings(3)({
      ...baseInput,
      history: [
        { ts: 1, score: 70, hardFindings: 10 },
        { ts: 2, score: 75, hardFindings: 8 },
        { ts: 3, score: 80, hardFindings: 8 },
        { ts: 4, score: 85, hardFindings: 5 },
      ],
    })
    expect(result.satisfied).toBe(true)
  })
  test("growth anywhere in tail -> unsatisfied", () => {
    const result = noNewHardFindings(3)({
      ...baseInput,
      history: [
        { ts: 1, score: 70, hardFindings: 8 },
        { ts: 2, score: 75, hardFindings: 9 },
        { ts: 3, score: 80, hardFindings: 7 },
      ],
    })
    expect(result.satisfied).toBe(false)
    expect(result.reason).toContain("hard grew 8->9")
  })
  test("missing hardFindings -> unsatisfied", () => {
    const result = noNewHardFindings(2)({
      ...baseInput,
      history: [
        { ts: 1, score: 70 },
        { ts: 2, score: 75 },
      ],
    })
    expect(result.satisfied).toBe(false)
    expect(result.reason).toContain("missing hard count")
  })
})

describe("regressionFreeCycles", () => {
  test("fewer recorded cycles than N -> unsatisfied", () => {
    expect(regressionFreeCycles(5)({ ...baseInput, regressionStatuses: ["pass", "pass"] }).satisfied).toBe(false)
  })
  test("all N pass -> satisfied", () => {
    expect(regressionFreeCycles(3)({ ...baseInput, regressionStatuses: ["pass", "pass", "pass"] }).satisfied).toBe(true)
  })
  test("any non-pass in tail -> unsatisfied", () => {
    const result = regressionFreeCycles(3)({
      ...baseInput,
      regressionStatuses: ["pass", "regression", "pass"],
    })
    expect(result.satisfied).toBe(false)
    expect(result.reason).toContain("cycle 1:regression")
  })
})

describe("scoreHistorySlopeFlat24h", () => {
  test("fewer than 2 points -> unsatisfied", () => {
    expect(scoreHistorySlopeFlat24h({ ...baseInput, history: [{ ts: 1, score: 50 }] }).satisfied).toBe(false)
  })
  test("flat slope within tolerance -> satisfied", () => {
    const now = 1_700_000_000
    const result = scoreHistorySlopeFlat24h({
      ...baseInput,
      history: [
        { ts: now - 23 * 3600, score: 85 },
        { ts: now - 12 * 3600, score: 85.5 },
        { ts: now, score: 85.2 },
      ],
    })
    expect(result.satisfied).toBe(true)
  })
  test("steep slope -> unsatisfied", () => {
    const now = 1_700_000_000
    const result = scoreHistorySlopeFlat24h({
      ...baseInput,
      history: [
        { ts: now - 24 * 3600, score: 30 },
        { ts: now, score: 95 },
      ],
    })
    expect(result.satisfied).toBe(false)
  })
})

describe("resolveDonePredicate + evaluateDoneRequire", () => {
  test("registered predicates resolve by name", () => {
    expect(resolveDonePredicate("jankurai_caps_zero")).toBeDefined()
    expect(resolveDonePredicate("score_history_slope_flat_24h")).toBeDefined()
  })

  test("pattern predicates resolve with N parsed from name", () => {
    expect(resolveDonePredicate("regression_free_cycles_gte_10")).toBeDefined()
    expect(resolveDonePredicate("no_new_hard_findings_3_runs")).toBeDefined()
    expect(resolveDonePredicate("nonsense")).toBeUndefined()
  })

  test("evaluateDoneRequire collects unresolved names", () => {
    const result = evaluateDoneRequire(["jankurai_caps_zero", "nonsense"], {
      ...baseInput,
      score: { capsApplied: 0, hardFindings: 0, softFindings: 0, score: 95 },
    })
    expect(result.satisfied).toBe(false)
    expect(result.unresolved).toEqual(["nonsense"])
    expect(result.perPredicate["jankurai_caps_zero"]?.satisfied).toBe(true)
  })

  test("all satisfied -> satisfied=true", () => {
    const now = 1_700_000_000
    const result = evaluateDoneRequire(
      ["jankurai_caps_zero", "regression_free_cycles_gte_3", "score_history_slope_flat_24h"],
      {
        ...baseInput,
        score: { capsApplied: 0, hardFindings: 0, softFindings: 0, score: 95 },
        regressionStatuses: ["pass", "pass", "pass"],
        history: [
          { ts: now - 23 * 3600, score: 95 },
          { ts: now - 12 * 3600, score: 95.1 },
          { ts: now, score: 94.9 },
        ],
      },
    )
    expect(result.satisfied).toBe(true)
    expect(result.unresolved).toEqual([])
  })

  test("namespace export mirrors top-level functions", () => {
    expect(DaemonDonePredicates.jankuraiCapsZero).toBe(jankuraiCapsZero)
  })
})
