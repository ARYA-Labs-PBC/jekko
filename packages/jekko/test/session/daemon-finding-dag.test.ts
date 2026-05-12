import { describe, expect, test } from "bun:test"
import type { Finding } from "../../src/session/daemon-finding-classifier"
import { DaemonFindingDag } from "../../src/session/daemon-finding-dag"

function finding(rule: string, severity: Finding["severity"], paths: string[]): Finding {
  return { ruleID: rule, fingerprint: rule, severity, paths, cap: undefined }
}

function cap(id: string, paths: string[]): Finding {
  return { ruleID: `cap:${id}`, fingerprint: `cap:${id}`, severity: "critical", paths, cap: id }
}

describe("DaemonFindingDag.schedule", () => {
  test("empty findings -> no waves", () => {
    expect(DaemonFindingDag.schedule([])).toEqual([])
  })

  test("disjoint findings pack into a single wave", () => {
    const waves = DaemonFindingDag.schedule([
      finding("A", "low", ["src/a"]),
      finding("B", "low", ["src/b"]),
      finding("C", "low", ["src/c"]),
    ])
    expect(waves.length).toBe(1)
    expect(waves[0].batches.length).toBe(3)
  })

  test("overlapping paths split across waves", () => {
    const waves = DaemonFindingDag.schedule([
      finding("A", "medium", ["src/shared"]),
      finding("B", "medium", ["src/shared"]),
      finding("C", "medium", ["src/other"]),
    ])
    expect(waves.length).toBe(2)
    expect(waves[0].batches.length).toBe(2) // A + C disjoint
    expect(waves[1].batches.length).toBe(1) // B requeued
    expect(waves[1].batches[0].findings[0].ruleID).toBe("B")
  })

  test("caps land alone in wave 0", () => {
    const waves = DaemonFindingDag.schedule([
      cap("c1", ["agent/proof-lanes.toml"]),
      cap("c2", ["agent/audit-policy.toml"]),
      finding("R", "low", ["src/x"]),
    ])
    expect(waves.length).toBe(2)
    expect(waves[0].batches.length).toBe(2)
    for (const batch of waves[0].batches) {
      expect(batch.findings.length).toBe(1)
      expect(batch.findings[0].cap).toBeDefined()
    }
    expect(waves[1].batches[0].findings[0].ruleID).toBe("R")
  })

  test("higher severity packs first inside a wave", () => {
    const waves = DaemonFindingDag.schedule([
      finding("Low", "low", ["src/shared"]),
      finding("High", "high", ["src/shared"]),
    ])
    expect(waves[0].batches[0].findings[0].ruleID).toBe("High")
    expect(waves[1].batches[0].findings[0].ruleID).toBe("Low")
  })

  test("touchedPaths sorts + dedupes", () => {
    const batch: { findings: Finding[] } = {
      findings: [
        finding("A", "low", ["src/b", "src/a"]),
        finding("B", "low", ["src/a"]),
      ],
    }
    expect(DaemonFindingDag.touchedPaths(batch)).toEqual(["src/a", "src/b"])
  })
})
