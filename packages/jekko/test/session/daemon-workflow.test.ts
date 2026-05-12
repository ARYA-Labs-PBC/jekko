import { describe, test, expect } from "bun:test"
import {
  evaluateWorkflowTransitions,
  reachableStates,
  detectCycles,
  hasRequiredEvidence,
  getTerminalStates,
} from "../../src/session/daemon-workflow"
import type { ZyalWorkflow } from "../../src/agent-script/schema"

const basicWorkflow: ZyalWorkflow = {
  type: "state_machine",
  initial: "discover",
  states: {
    discover: {
      agent: "plan",
      writes: "scratch_only",
      produces: ["impact_map"],
      transitions: [
        { to: "plan", when: { evidence_exists: "impact_map" } },
        { to: "incubate", when: { risk_score_gte: 0.7 } },
      ],
    },
    plan: {
      agent: "plan",
      writes: "scratch_only",
      requires: ["impact_map"],
      approval: "plan_review",
      transitions: [
        { to: "implement", when: { approval_granted: "plan_review" } },
      ],
    },
    implement: {
      agent: "build",
      writes: "isolated_worktree",
      transitions: [
        { to: "done", when: { all_checks_pass: true } },
        { to: "plan", when: { checks_failed: true } },
      ],
    },
    incubate: {
      transitions: [{ to: "plan", when: { evidence_exists: "analysis" } }],
    },
    done: { terminal: true },
  },
}

describe("daemon workflow", () => {
  test("advances on evidence", () => {
    const result = evaluateWorkflowTransitions({
      workflow: basicWorkflow,
      currentState: "discover",
      evidence: { impact_map: { files: ["a.ts"] } },
      approvals: {},
      constraintViolations: [],
      checksPassed: false,
    })
    expect(result.nextState).toBe("plan")
    expect(result.blocked).toBe(false)
  })

  test("blocks on missing approval", () => {
    const result = evaluateWorkflowTransitions({
      workflow: basicWorkflow,
      currentState: "plan",
      evidence: { impact_map: {} },
      approvals: {},
      constraintViolations: [],
      checksPassed: false,
    })
    expect(result.nextState).toBeNull()
    expect(result.blocked).toBe(true)
    expect(result.blockedBy).toBe("plan_review")
  })

  test("advances when approval granted", () => {
    const result = evaluateWorkflowTransitions({
      workflow: basicWorkflow,
      currentState: "plan",
      evidence: { impact_map: {} },
      approvals: { plan_review: { granted: true, decision: "approve" } },
      constraintViolations: [],
      checksPassed: false,
    })
    expect(result.nextState).toBe("implement")
    expect(result.blocked).toBe(false)
  })

  test("transitions on all_checks_pass", () => {
    const result = evaluateWorkflowTransitions({
      workflow: basicWorkflow,
      currentState: "implement",
      evidence: {},
      approvals: {},
      constraintViolations: [],
      checksPassed: true,
    })
    expect(result.nextState).toBe("done")
  })

  test("transitions on checks_failed", () => {
    const result = evaluateWorkflowTransitions({
      workflow: basicWorkflow,
      currentState: "implement",
      evidence: {},
      approvals: {},
      constraintViolations: [],
      checksPassed: false,
    })
    expect(result.nextState).toBe("plan")
  })

  test("terminal state returns null", () => {
    const result = evaluateWorkflowTransitions({
      workflow: basicWorkflow,
      currentState: "done",
      evidence: {},
      approvals: {},
      constraintViolations: [],
      checksPassed: false,
    })
    expect(result.nextState).toBeNull()
    expect(result.reason).toBe("terminal state")
  })

  test("risk_score_gte triggers incubation", () => {
    const result = evaluateWorkflowTransitions({
      workflow: basicWorkflow,
      currentState: "discover",
      evidence: { risk_score: 0.8 },
      approvals: {},
      constraintViolations: [],
      checksPassed: false,
    })
    expect(result.nextState).toBe("incubate")
  })

  test("constraint_violated triggers transition", () => {
    const wf: ZyalWorkflow = {
      type: "state_machine",
      initial: "a",
      states: {
        a: {
          transitions: [
            { to: "b", when: { constraint_violated: true } },
          ],
        },
        b: { terminal: true },
      },
    }
    const result = evaluateWorkflowTransitions({
      workflow: wf,
      currentState: "a",
      evidence: {},
      approvals: {},
      constraintViolations: ["test_count"],
      checksPassed: false,
    })
    expect(result.nextState).toBe("b")
  })

  test("reachableStates finds all reachable states", () => {
    const states = reachableStates(basicWorkflow, "discover")
    expect(states).toContain("plan")
    expect(states).toContain("implement")
    expect(states).toContain("done")
    expect(states).toContain("incubate")
  })

  test("detectCycles finds cycles", () => {
    // implement -> plan -> implement is a cycle
    const cycles = detectCycles(basicWorkflow)
    expect(cycles.length).toBeGreaterThan(0)
  })

  test("detectCycles returns empty for acyclic graph", () => {
    const dag: ZyalWorkflow = {
      type: "dag",
      initial: "a",
      states: {
        a: { transitions: [{ to: "b", when: { all_checks_pass: true } }] },
        b: { transitions: [{ to: "c", when: { all_checks_pass: true } }] },
        c: { terminal: true },
      },
    }
    const cycles = detectCycles(dag)
    expect(cycles).toHaveLength(0)
  })

  test("hasRequiredEvidence checks requirements", () => {
    const state = basicWorkflow.states.plan!
    expect(hasRequiredEvidence(state, { impact_map: {} })).toBe(true)
    expect(hasRequiredEvidence(state, {})).toBe(false)
  })

  test("getTerminalStates returns terminal states", () => {
    const terminals = getTerminalStates(basicWorkflow)
    expect(terminals).toEqual(["done"])
  })

  test("unknown state returns null", () => {
    const result = evaluateWorkflowTransitions({
      workflow: basicWorkflow,
      currentState: "nonexistent",
      evidence: {},
      approvals: {},
      constraintViolations: [],
      checksPassed: false,
    })
    expect(result.nextState).toBeNull()
    expect(result.reason).toBe("unknown state")
  })

  test("no transitions returns null", () => {
    const wf: ZyalWorkflow = {
      type: "state_machine",
      initial: "a",
      states: {
        a: {},
        b: { terminal: true },
      },
    }
    const result = evaluateWorkflowTransitions({
      workflow: wf,
      currentState: "a",
      evidence: {},
      approvals: {},
      constraintViolations: [],
      checksPassed: false,
    })
    expect(result.nextState).toBeNull()
    expect(result.reason).toBe("no transitions")
  })
})
