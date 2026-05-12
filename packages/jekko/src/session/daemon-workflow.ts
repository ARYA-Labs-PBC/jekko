import type { ZyalWorkflow, ZyalWorkflowState, ZyalWorkflowTransitionCondition } from "@/agent-script/schema"

/**
 * Workflow state machine evaluator for ZYAL v2.
 *
 * Evaluates transition conditions against an evidence set and resolves
 * the next state in a durable workflow graph.
 */

export type EvidenceSet = Record<string, unknown>
export type ApprovalSet = Record<string, { granted: boolean; decision?: string }>

export type WorkflowEvalResult = {
  readonly currentState: string
  readonly nextState: string | null
  readonly reason: string
  readonly blocked: boolean
  readonly blockedBy?: string
}

/**
 * Evaluate transitions from the current state and return the next state.
 */
export function evaluateWorkflowTransitions(input: {
  workflow: ZyalWorkflow
  currentState: string
  evidence: EvidenceSet
  approvals: ApprovalSet
  constraintViolations: string[]
  checksPassed: boolean
}): WorkflowEvalResult {
  const state = input.workflow.states[input.currentState]
  if (!state) {
    return { currentState: input.currentState, nextState: null, reason: "unknown state", blocked: false }
  }

  if (state.terminal) {
    return { currentState: input.currentState, nextState: null, reason: "terminal state", blocked: false }
  }

  // Check approval gate
  if (state.approval) {
    const approval = input.approvals[state.approval]
    if (!approval?.granted) {
      return {
        currentState: input.currentState,
        nextState: null,
        reason: `waiting for approval: ${state.approval}`,
        blocked: true,
        blockedBy: state.approval,
      }
    }
  }

  // Evaluate transitions in order
  if (!state.transitions?.length) {
    return { currentState: input.currentState, nextState: null, reason: "no transitions", blocked: false }
  }

  for (const transition of state.transitions) {
    if (evaluateCondition(transition.when, input.evidence, input.approvals, input.constraintViolations, input.checksPassed)) {
      return {
        currentState: input.currentState,
        nextState: transition.to,
        reason: describeCondition(transition.when),
        blocked: false,
      }
    }
  }

  return { currentState: input.currentState, nextState: null, reason: "no transition conditions met", blocked: false }
}

/**
 * Get all states reachable from a given state (BFS).
 */
export function reachableStates(workflow: ZyalWorkflow, from: string): string[] {
  const visited = new Set<string>()
  const queue = [from]
  while (queue.length > 0) {
    const current = queue.shift()!
    if (visited.has(current)) continue
    visited.add(current)
    const state = workflow.states[current]
    if (state?.transitions) {
      for (const t of state.transitions) {
        if (!visited.has(t.to)) queue.push(t.to)
      }
    }
  }
  visited.delete(from)
  return Array.from(visited)
}

/**
 * Detect cycles in the workflow graph (for DAG mode validation).
 */
export function detectCycles(workflow: ZyalWorkflow): string[][] {
  const cycles: string[][] = []
  const visited = new Set<string>()
  const stack = new Set<string>()

  function dfs(node: string, path: string[]) {
    if (stack.has(node)) {
      const cycleStart = path.indexOf(node)
      cycles.push(path.slice(cycleStart).concat(node))
      return
    }
    if (visited.has(node)) return
    visited.add(node)
    stack.add(node)
    const state = workflow.states[node]
    if (state?.transitions) {
      for (const t of state.transitions) {
        dfs(t.to, [...path, node])
      }
    }
    stack.delete(node)
  }

  for (const name of Object.keys(workflow.states)) {
    dfs(name, [])
  }
  return cycles
}

/**
 * Get the list of required evidence for a workflow state.
 */
export function getStateRequirements(state: ZyalWorkflowState): string[] {
  return [...(state.requires ?? [])]
}

/**
 * Check if all required evidence exists for a state.
 */
export function hasRequiredEvidence(state: ZyalWorkflowState, evidence: EvidenceSet): boolean {
  for (const req of state.requires ?? []) {
    if (evidence[req] === undefined) return false
  }
  return true
}

/**
 * Get terminal states in a workflow.
 */
export function getTerminalStates(workflow: ZyalWorkflow): string[] {
  return Object.entries(workflow.states)
    .filter(([, state]) => state.terminal === true)
    .map(([name]) => name)
}

function evaluateCondition(
  condition: ZyalWorkflowTransitionCondition,
  evidence: EvidenceSet,
  approvals: ApprovalSet,
  constraintViolations: string[],
  checksPassed: boolean,
): boolean {
  if (condition.evidence_exists !== undefined) {
    if (evidence[condition.evidence_exists] === undefined) return false
  }
  if (condition.approval_granted !== undefined) {
    if (!approvals[condition.approval_granted]?.granted) return false
  }
  if (condition.all_checks_pass !== undefined) {
    if (condition.all_checks_pass !== checksPassed) return false
  }
  if (condition.checks_failed !== undefined) {
    if (condition.checks_failed !== !checksPassed) return false
  }
  if (condition.constraint_violated !== undefined) {
    if (condition.constraint_violated !== (constraintViolations.length > 0)) return false
  }
  if (condition.risk_score_gte !== undefined) {
    const riskScore = typeof evidence.risk_score === "number" ? evidence.risk_score : 0
    if (riskScore < condition.risk_score_gte) return false
  }
  return true
}

function describeCondition(condition: ZyalWorkflowTransitionCondition): string {
  const parts: string[] = []
  if (condition.evidence_exists) parts.push(`evidence:${condition.evidence_exists}`)
  if (condition.approval_granted) parts.push(`approval:${condition.approval_granted}`)
  if (condition.all_checks_pass) parts.push("all_checks_pass")
  if (condition.checks_failed) parts.push("checks_failed")
  if (condition.constraint_violated) parts.push("constraint_violated")
  if (condition.risk_score_gte !== undefined) parts.push(`risk≥${condition.risk_score_gte}`)
  return parts.join(", ") || "unconditional"
}
