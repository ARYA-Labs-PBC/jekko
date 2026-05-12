import { describe, expect, test } from "bun:test"
import type { Finding } from "../../src/session/daemon-finding-classifier"
import { DaemonTaskRouter } from "../../src/session/daemon-task-router"

function finding(rule: string, severity: Finding["severity"], paths: string[], cap?: string): Finding {
  return { ruleID: rule, fingerprint: rule, severity, paths, cap }
}

describe("DaemonTaskRouter.routeFindings", () => {
  test("low severity packs into parallel lane", () => {
    const { decisions, waves } = DaemonTaskRouter.routeFindings([
      finding("A", "low", ["src/a"]),
      finding("B", "low", ["src/b"]),
    ])
    expect(decisions.every((d) => d.lane === "parallel")).toBe(true)
    expect(waves.length).toBe(1)
    expect(waves[0].batches.length).toBe(2)
  })

  test("high severity escalates to incubator", () => {
    const { decisions } = DaemonTaskRouter.routeFindings([finding("X", "high", ["src/x"])])
    expect(decisions[0].lane).toBe("incubator")
    expect(decisions[0].reasons).toContain("hard_severity")
  })

  test("caps escalate to incubator regardless of severity", () => {
    const { decisions } = DaemonTaskRouter.routeFindings([
      finding("cap:c1", "critical", ["agent/proof-lanes.toml"], "c1"),
    ])
    expect(decisions[0].lane).toBe("incubator")
    expect(decisions[0].reasons).toContain("cap")
  })
})
