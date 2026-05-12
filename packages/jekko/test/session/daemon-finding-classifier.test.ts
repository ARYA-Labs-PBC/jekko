import { describe, expect, test } from "bun:test"
import { DaemonFindingClassifier } from "../../src/session/daemon-finding-classifier"

describe("DaemonFindingClassifier.classifyText", () => {
  test("empty findings returns zeroed totals", () => {
    const result = DaemonFindingClassifier.classifyText(`{"score": 95, "findings": []}`)
    expect(result.findings).toEqual([])
    expect(result.capsTotal).toBe(0)
    expect(result.hardTotal).toBe(0)
    expect(result.softTotal).toBe(0)
    expect(result.score).toBe(95)
  })

  test("severity ladder maps critical/high to hard, medium/low to soft", () => {
    const json = JSON.stringify({
      findings: [
        { rule_id: "A", fingerprint: "fa", severity: "critical", path: "src/a.rs" },
        { rule_id: "B", fingerprint: "fb", severity: "high", path: "src/b.rs" },
        { rule_id: "C", fingerprint: "fc", severity: "medium", path: "src/c.rs" },
        { rule_id: "D", fingerprint: "fd", severity: "low", path: "src/d.rs" },
        { rule_id: "E", fingerprint: "fe", severity: "info", path: "src/e.rs" },
      ],
    })
    const result = DaemonFindingClassifier.classifyText(json)
    expect(result.hardTotal).toBe(2)
    expect(result.softTotal).toBe(3)
    expect(result.capsTotal).toBe(0)
  })

  test("caps_applied become synthetic critical findings tagged with cap id", () => {
    const json = JSON.stringify({
      findings: [],
      caps_applied: [
        { id: "no-security-lane-on-high-risk-repo", affects: ["agent/proof-lanes.toml"] },
      ],
    })
    const result = DaemonFindingClassifier.classifyText(json)
    expect(result.capsTotal).toBe(1)
    const cap = result.findings.find((f) => f.cap !== undefined)
    expect(cap?.severity).toBe("critical")
    expect(cap?.ruleID).toBe("cap:no-security-lane-on-high-risk-repo")
    expect(cap?.paths).toEqual(["agent/proof-lanes.toml"])
  })

  test("collects paths from multiple fields and de-dupes", () => {
    const json = JSON.stringify({
      findings: [
        {
          rule_id: "X",
          severity: "low",
          paths: ["src/b", "src/a"],
          affected_files: ["src/a", "src/c"],
        },
      ],
    })
    const result = DaemonFindingClassifier.classifyText(json)
    expect(result.findings[0].paths).toEqual(["src/a", "src/b", "src/c"])
  })

  test("compareSeverity orders highest first", () => {
    expect(DaemonFindingClassifier.compareSeverity("critical", "low")).toBeLessThan(0)
    expect(DaemonFindingClassifier.compareSeverity("low", "critical")).toBeGreaterThan(0)
    expect(DaemonFindingClassifier.compareSeverity("high", "high")).toBe(0)
  })

  test("parseSeverity is case-insensitive and falls back to info", () => {
    expect(DaemonFindingClassifier.parseSeverity("CRITICAL")).toBe("critical")
    expect(DaemonFindingClassifier.parseSeverity("Med")).toBe("medium")
    expect(DaemonFindingClassifier.parseSeverity("xxx")).toBe("info")
    expect(DaemonFindingClassifier.parseSeverity(undefined)).toBe("info")
  })

  test("isHard tags critical and high only", () => {
    expect(DaemonFindingClassifier.isHard("critical")).toBe(true)
    expect(DaemonFindingClassifier.isHard("high")).toBe(true)
    expect(DaemonFindingClassifier.isHard("medium")).toBe(false)
    expect(DaemonFindingClassifier.isHard("low")).toBe(false)
    expect(DaemonFindingClassifier.isHard("info")).toBe(false)
  })

  test("invalid JSON throws", () => {
    expect(() => DaemonFindingClassifier.classifyText("{not json}")).toThrow()
  })

  test("non-object JSON throws", () => {
    expect(() => DaemonFindingClassifier.classifyText("[1, 2, 3]")).toThrow()
  })
})
