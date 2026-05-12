import { describe, test, expect } from "bun:test"
import {
  evaluateAutoApproval,
  checkApprovalTimeout,
  getEscalationTarget,
  shouldAutoEscalate,
  createApprovalRequest,
  recordDecision,
  getGateNames,
  hasRequiredEvidence,
  getAvailableDecisions,
} from "../../src/session/daemon-approvals"
import type { ZyalApprovals, ZyalApprovalGate } from "../../src/agent-script/schema"

const testGate: ZyalApprovalGate = {
  required_role: "tech_lead",
  timeout: "24h",
  on_timeout: "pause",
  decisions: ["approve", "reject", "edit"],
  require_evidence: ["test_results", "rollback_plan"],
  auto_approve_if: {
    risk_score_lt: 0.3,
    all_checks_pass: true,
  },
}

const testApprovals: ZyalApprovals = {
  gates: {
    plan_review: testGate,
    merge_review: {
      required_role: "code_owner",
    },
  },
  escalation: {
    chain: ["tech_lead", "staff_engineer", "director"],
    auto_escalate_after: "48h",
  },
}

describe("daemon approvals", () => {
  test("evaluateAutoApproval grants when conditions met", () => {
    const result = evaluateAutoApproval(testGate, { riskScore: 0.1, allChecksPassed: true })
    expect(result.granted).toBe(true)
    expect(result.autoApproved).toBe(true)
  })

  test("evaluateAutoApproval rejects when risk too high", () => {
    const result = evaluateAutoApproval(testGate, { riskScore: 0.5, allChecksPassed: true })
    expect(result.granted).toBe(false)
  })

  test("evaluateAutoApproval rejects when checks failed", () => {
    const result = evaluateAutoApproval(testGate, { riskScore: 0.1, allChecksPassed: false })
    expect(result.granted).toBe(false)
  })

  test("evaluateAutoApproval returns false when no auto_approve_if", () => {
    const result = evaluateAutoApproval({ required_role: "admin" }, { riskScore: 0.1 })
    expect(result.granted).toBe(false)
  })

  test("checkApprovalTimeout detects timeout", () => {
    const state = createApprovalRequest("plan_review", Date.now() - 25 * 60 * 60 * 1000) // 25h ago
    const result = checkApprovalTimeout(state, testGate, Date.now())
    expect(result.timedOut).toBe(true)
    expect(result.action).toBe("pause")
  })

  test("checkApprovalTimeout does not fire before timeout", () => {
    const state = createApprovalRequest("plan_review", Date.now() - 1000)
    const result = checkApprovalTimeout(state, testGate, Date.now())
    expect(result.timedOut).toBe(false)
  })

  test("checkApprovalTimeout skips non-pending", () => {
    const state = recordDecision(createApprovalRequest("plan_review"), "approve")
    const result = checkApprovalTimeout(state, testGate, Date.now())
    expect(result.timedOut).toBe(false)
  })

  test("getEscalationTarget returns chain members", () => {
    expect(getEscalationTarget(testApprovals, 0)).toEqual({ target: "tech_lead", level: 1 })
    expect(getEscalationTarget(testApprovals, 1)).toEqual({ target: "staff_engineer", level: 2 })
    expect(getEscalationTarget(testApprovals, 2)).toEqual({ target: "director", level: 3 })
    expect(getEscalationTarget(testApprovals, 3)).toEqual({ target: null, level: 3 })
  })

  test("shouldAutoEscalate triggers after duration", () => {
    const state = createApprovalRequest("plan_review", Date.now() - 49 * 60 * 60 * 1000) // 49h ago
    expect(shouldAutoEscalate(state, testApprovals, Date.now())).toBe(true)
  })

  test("shouldAutoEscalate does not trigger before duration", () => {
    const state = createApprovalRequest("plan_review")
    expect(shouldAutoEscalate(state, testApprovals, Date.now())).toBe(false)
  })

  test("createApprovalRequest creates pending state", () => {
    const state = createApprovalRequest("plan_review")
    expect(state.status).toBe("pending")
    expect(state.gateName).toBe("plan_review")
    expect(state.escalationLevel).toBe(0)
  })

  test("recordDecision sets approved status", () => {
    const state = recordDecision(createApprovalRequest("plan_review"), "approve", "alice")
    expect(state.status).toBe("approved")
    expect(state.decision).toBe("approve")
    expect(state.decidedBy).toBe("alice")
  })

  test("recordDecision sets rejected status", () => {
    const state = recordDecision(createApprovalRequest("plan_review"), "reject")
    expect(state.status).toBe("rejected")
  })

  test("recordDecision handles escalation", () => {
    const state = recordDecision(createApprovalRequest("plan_review"), "escalate")
    expect(state.status).toBe("escalated")
    expect(state.escalationLevel).toBe(1)
  })

  test("getGateNames returns gate names", () => {
    expect(getGateNames(testApprovals)).toEqual(["plan_review", "merge_review"])
    expect(getGateNames(undefined)).toEqual([])
  })

  test("hasRequiredEvidence checks evidence set", () => {
    expect(hasRequiredEvidence(testGate, new Set(["test_results", "rollback_plan"]))).toBe(true)
    expect(hasRequiredEvidence(testGate, new Set(["test_results"]))).toBe(false)
    expect(hasRequiredEvidence({ required_role: "admin" }, new Set())).toBe(true)
  })

  test("getAvailableDecisions returns configured decisions", () => {
    expect(getAvailableDecisions(testGate)).toEqual(["approve", "reject", "edit"])
    expect(getAvailableDecisions({ required_role: "admin" })).toEqual(["approve", "reject"])
  })
})
