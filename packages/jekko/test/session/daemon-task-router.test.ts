import { describe, expect, test } from "bun:test"
import { Effect } from "effect"
import { getZyalExample } from "../../src/agent-script/examples"
import { parseZyal } from "../../src/agent-script/parser"
import { DaemonTaskRouter } from "../../src/session/daemon-task-router"

async function incubator() {
  return (await Effect.runPromise(parseZyal(getZyalExample("hard-task-incubator")!.text))).spec.incubator!
}

function task(patch: Record<string, unknown> = {}) {
  return {
    id: "task",
    title: "Task",
    attempt_count: 0,
    no_progress_count: 0,
    risk_score: 0.1,
    readiness_score: 0.9,
    implementation_confidence: 0.95,
    verification_confidence: 0.95,
    ...patch,
  } as any
}

describe("daemon task router", () => {
  test("high confidence low risk task stays normal", async () => {
    const route = DaemonTaskRouter.routeTask({ task: task(), incubator: await incubator() })
    expect(route.lane).toBe("normal")
  })

  test("repeated failure task routes to incubator", async () => {
    const route = DaemonTaskRouter.routeTask({ task: task({ attempt_count: 2 }), incubator: await incubator() })
    expect(route.lane).toBe("incubator")
    expect(route.reasons).toContain("repeated_attempts")
  })

  test("critical path task routes to incubator", async () => {
    const route = DaemonTaskRouter.routeTask({
      task: task(),
      incubator: await incubator(),
      touchedPaths: ["packages/jekko/src/session/daemon.ts"],
    })
    expect(route.lane).toBe("incubator")
    expect(route.reasons).toContain("critical_path")
  })

  test("no progress increases risk", async () => {
    const route = DaemonTaskRouter.routeTask({ task: task({ no_progress_count: 2 }), incubator: await incubator() })
    expect(route.lane).toBe("incubator")
    expect(route.riskScore).toBeGreaterThan(0.1)
  })

  test("model confidence alone cannot promote", () => {
    const score = DaemonTaskRouter.computeReadiness({
      implementationConfidence: 1,
      verificationConfidence: 1,
    })
    expect(score).toBeLessThan(0.1)
  })

  test("baseline score is reused when evidence is empty", () => {
    const score = DaemonTaskRouter.computeReadiness({
      baselineScore: 0.7,
    })
    expect(score).toBe(0.7)
  })

  test("exclude_when keeps low readiness tasks in the normal lane", async () => {
    const nextIncubator = {
      ...(await incubator()),
      exclude_when: {
        any: [{ readiness_score_lt: 0.95 }],
      },
    } as any
    const route = DaemonTaskRouter.routeTask({ task: task({ readiness_score: 0.5 }), incubator: nextIncubator })
    expect(route.lane).toBe("normal")
  })
})
