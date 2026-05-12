import { describe, expect, test } from "bun:test"
import {
  evaluateConstraint,
  evaluateAllConstraints,
  captureBaselines,
  type ConstraintBaselines,
} from "@/session/daemon-constraints"
import type { ZyalConstraint } from "@/agent-script/schema"

describe("daemon constraints", () => {
  const makeConstraint = (overrides: Partial<ZyalConstraint> = {}): ZyalConstraint => ({
    name: "test-constraint",
    check: { shell: "echo 5" },
    invariant: "gte_baseline",
    on_violation: "pause",
    ...overrides,
  })

  test("gte_baseline passes when current >= baseline", () => {
    const c = makeConstraint({ invariant: "gte_baseline" })
    const baselines: ConstraintBaselines = { "test-constraint": 5 }
    expect(evaluateConstraint(c, 5, baselines).pass).toBe(true)
    expect(evaluateConstraint(c, 6, baselines).pass).toBe(true)
  })

  test("gte_baseline fails when current < baseline", () => {
    const c = makeConstraint({ invariant: "gte_baseline" })
    const baselines: ConstraintBaselines = { "test-constraint": 5 }
    const result = evaluateConstraint(c, 4, baselines)
    expect(result.pass).toBe(false)
    if (!result.pass) {
      expect(result.name).toBe("test-constraint")
      expect(result.action).toBe("pause")
    }
  })

  test("lte_baseline passes when current <= baseline", () => {
    const c = makeConstraint({ invariant: "lte_baseline" })
    const baselines: ConstraintBaselines = { "test-constraint": 10 }
    expect(evaluateConstraint(c, 10, baselines).pass).toBe(true)
    expect(evaluateConstraint(c, 9, baselines).pass).toBe(true)
  })

  test("lte_baseline fails when current > baseline", () => {
    const c = makeConstraint({ invariant: "lte_baseline" })
    const baselines: ConstraintBaselines = { "test-constraint": 10 }
    expect(evaluateConstraint(c, 11, baselines).pass).toBe(false)
  })

  test("equals_zero passes when value is 0", () => {
    const c = makeConstraint({ invariant: "equals_zero" })
    expect(evaluateConstraint(c, 0, {}).pass).toBe(true)
  })

  test("equals_zero fails when value is non-zero", () => {
    const c = makeConstraint({ invariant: "equals_zero" })
    expect(evaluateConstraint(c, 1, {}).pass).toBe(false)
  })

  test("non_zero passes when value is non-zero", () => {
    const c = makeConstraint({ invariant: "non_zero" })
    expect(evaluateConstraint(c, 42, {}).pass).toBe(true)
  })

  test("non_zero fails when value is 0", () => {
    const c = makeConstraint({ invariant: "non_zero" })
    expect(evaluateConstraint(c, 0, {}).pass).toBe(false)
  })

  test("equals_baseline passes when current equals baseline", () => {
    const c = makeConstraint({ invariant: "equals_baseline" })
    const baselines: ConstraintBaselines = { "test-constraint": 7 }
    expect(evaluateConstraint(c, 7, baselines).pass).toBe(true)
    expect(evaluateConstraint(c, 8, baselines).pass).toBe(false)
  })

  test("captureBaselines captures values from shellRunner", () => {
    const constraints: ZyalConstraint[] = [
      makeConstraint({ name: "a", baseline: "capture_on_start" }),
      makeConstraint({ name: "b", baseline: "capture_on_checkpoint" }),
      makeConstraint({ name: "c", baseline: "capture_on_start" }),
    ]
    const values: Record<string, number> = { "echo 5": 5 }
    const baselines = captureBaselines(
      constraints,
      (cmd) => values[cmd] ?? 0,
      "capture_on_start",
    )
    expect(baselines.a).toBe(5)
    expect(baselines.b).toBeUndefined()
    expect(baselines.c).toBe(5)
  })

  test("evaluateAllConstraints returns first violation", () => {
    const constraints: ZyalConstraint[] = [
      makeConstraint({ name: "ok", invariant: "equals_zero" }),
      makeConstraint({ name: "bad", invariant: "equals_zero" }),
    ]
    const values = [0, 3]
    let idx = 0
    const result = evaluateAllConstraints(
      constraints,
      () => values[idx++],
      {},
    )
    expect(result.pass).toBe(false)
    if (!result.pass) {
      expect(result.name).toBe("bad")
    }
  })

  test("evaluateAllConstraints passes when all constraints pass", () => {
    const constraints: ZyalConstraint[] = [
      makeConstraint({ name: "a", invariant: "equals_zero" }),
      makeConstraint({ name: "b", invariant: "equals_zero" }),
    ]
    const result = evaluateAllConstraints(constraints, () => 0, {})
    expect(result.pass).toBe(true)
  })
})
