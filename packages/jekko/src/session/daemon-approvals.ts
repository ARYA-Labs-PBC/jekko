import type { ZyalApprovals, ZyalApprovalGate } from "@/agent-script/schema"
import { parseDuration } from "./daemon-retry"

/**
 * Approval gate evaluator for ZYAL v2.
 *
 * Manages human-in-the-loop approval gates with roles, timeouts,
 * auto-approval conditions, and escalation chains.
 */

export type ApprovalState = {
  readonly gateName: string
  readonly status: "pending" | "approved" | "rejected" | "escalated" | "timed_out"
  readonly requestedAt: number
  readonly decidedAt?: number
  readonly decision?: string
  readonly decidedBy?: string
  readonly escalationLevel: number
}

export type ApprovalEvalResult = {
  readonly granted: boolean
  readonly reason: string
  readonly autoApproved: boolean
}

/**
 * Evaluate whether an approval gate should auto-approve.
 */
export function evaluateAutoApproval(
  gate: ZyalApprovalGate,
  context: { riskScore?: number; allChecksPassed?: boolean },
): ApprovalEvalResult {
  if (!gate.auto_approve_if) {
    return { granted: false, reason: "no auto-approve conditions", autoApproved: false }
  }

  let pass = true

  if (gate.auto_approve_if.risk_score_lt !== undefined) {
    const score = context.riskScore ?? 1
    if (score >= gate.auto_approve_if.risk_score_lt) pass = false
  }

  if (gate.auto_approve_if.all_checks_pass !== undefined) {
    if (gate.auto_approve_if.all_checks_pass !== (context.allChecksPassed ?? false)) pass = false
  }

  if (pass) {
    return { granted: true, reason: "auto-approved by conditions", autoApproved: true }
  }

  return { granted: false, reason: "auto-approve conditions not met", autoApproved: false }
}

/**
 * Check if a pending approval has timed out.
 */
export function checkApprovalTimeout(
  state: ApprovalState,
  gate: ZyalApprovalGate,
  now: number,
): { timedOut: boolean; action: string } {
  if (state.status !== "pending") return { timedOut: false, action: "none" }
  if (!gate.timeout) return { timedOut: false, action: "none" }

  const timeoutMs = parseDuration(gate.timeout)
  if (now - state.requestedAt >= timeoutMs) {
    return { timedOut: true, action: gate.on_timeout ?? "pause" }
  }
  return { timedOut: false, action: "none" }
}

/**
 * Get the next escalation target from the chain.
 */
export function getEscalationTarget(
  approvals: ZyalApprovals,
  currentLevel: number,
): { target: string | null; level: number } {
  const chain = approvals.escalation?.chain
  if (!chain?.length) return { target: null, level: currentLevel }
  if (currentLevel >= chain.length) return { target: null, level: currentLevel }
  return { target: chain[currentLevel], level: currentLevel + 1 }
}

/**
 * Check if auto-escalation should trigger.
 */
export function shouldAutoEscalate(
  state: ApprovalState,
  approvals: ZyalApprovals,
  now: number,
): boolean {
  if (state.status !== "pending") return false
  if (!approvals.escalation?.auto_escalate_after) return false
  const escalateMs = parseDuration(approvals.escalation.auto_escalate_after)
  return now - state.requestedAt >= escalateMs
}

/**
 * Create a new pending approval state.
 */
export function createApprovalRequest(gateName: string, now?: number): ApprovalState {
  return {
    gateName,
    status: "pending",
    requestedAt: now ?? Date.now(),
    escalationLevel: 0,
  }
}

/**
 * Record a human decision on an approval.
 */
export function recordDecision(
  state: ApprovalState,
  decision: string,
  decidedBy?: string,
): ApprovalState {
  const isApproved = decision === "approve"
  const isEscalated = decision === "escalate"
  return {
    ...state,
    status: isApproved ? "approved" : isEscalated ? "escalated" : "rejected",
    decision,
    decidedBy,
    decidedAt: Date.now(),
    escalationLevel: isEscalated ? state.escalationLevel + 1 : state.escalationLevel,
  }
}

/**
 * Get all gate names defined in approvals config.
 */
export function getGateNames(approvals: ZyalApprovals | undefined): string[] {
  if (!approvals?.gates) return []
  return Object.keys(approvals.gates)
}

/**
 * Check if required evidence exists for a gate.
 */
export function hasRequiredEvidence(
  gate: ZyalApprovalGate,
  evidence: Set<string>,
): boolean {
  if (!gate.require_evidence?.length) return true
  return gate.require_evidence.every((e) => evidence.has(e))
}

/**
 * Get available decisions for a gate.
 */
export function getAvailableDecisions(gate: ZyalApprovalGate): string[] {
  return gate.decisions?.map(String) ?? ["approve", "reject"]
}
