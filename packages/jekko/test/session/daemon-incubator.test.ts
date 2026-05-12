import { describe, expect, test } from "bun:test"
import { DaemonTaskPromote } from "../../src/session/daemon-task-promote"

const task = {
  id: "task",
  readiness_score: 0.8,
} as any

const promotion = {
  promote_at: 0.78,
  require: ["problem_statement", "current_best_plan", "verification_strategy", "risk_review"],
  block_on: { unresolved_critical_objections_gte: 1 },
  on_promote: "move_to_ready_queue",
  on_exhausted: "park_with_summary",
} as any

describe("daemon incubator promotion gate", () => {
  test("promotes only when required evidence exists", () => {
    const decision = DaemonTaskPromote.evaluatePromotion({
      task,
      promotion,
      maxPasses: 7,
      passes: [],
      memories: [
        { kind: "problem_statement" },
        { kind: "current_best_plan" },
        { kind: "verification_strategy" },
        { kind: "risk_review" },
      ] as any,
    })
    expect(decision.promote).toBe(true)
  })

  test("blocks high score with unresolved critical objection", () => {
    const decision = DaemonTaskPromote.evaluatePromotion({
      task,
      promotion,
      maxPasses: 7,
      passes: [],
      memories: [
        { kind: "problem_statement" },
        { kind: "current_best_plan" },
        { kind: "verification_strategy" },
        { kind: "risk_review" },
        { kind: "critical_objection", payload_json: { severity: "high" } },
      ] as any,
    })
    expect(decision.promote).toBe(false)
    expect(decision.blockers).toContain("unresolved_critical_objections")
  })
})

