import { describe, expect, test } from "bun:test"
import { parseBaselineJson } from "../../../src/cli/cmd/tui/context/jankurai-baseline"

describe("parseBaselineJson", () => {
  test("null on non-object input", () => {
    expect(parseBaselineJson("[1,2,3]")).toBeNull()
    expect(parseBaselineJson("null")).toBeNull()
    expect(parseBaselineJson("not json")).toBeNull()
  })

  test("null when score is missing or not numeric", () => {
    expect(parseBaselineJson(JSON.stringify({ caps_applied: [] }))).toBeNull()
    expect(parseBaselineJson(JSON.stringify({ score: "high" }))).toBeNull()
  })

  test("returns baseline shape with decision substructure flattened", () => {
    const json = JSON.stringify({
      score: 60,
      decision: { hard_findings: 5, soft_findings: 18 },
      caps_applied: [{ id: "x" }, { id: "y" }],
      observed_conformance_level: "B",
      standard_version: "0.8.0",
    })
    const out = parseBaselineJson(json)
    expect(out).toEqual({
      score: 60,
      hardFindings: 5,
      softFindings: 18,
      capsApplied: 2,
      conformanceLevel: "B",
      standardVersion: "0.8.0",
    })
  })

  test("falls back to claimed_conformance_level when observed missing", () => {
    const json = JSON.stringify({ score: 50, claimed_conformance_level: "C" })
    expect(parseBaselineJson(json)?.conformanceLevel).toBe("C")
  })

  test("defaults missing hard/soft to 0", () => {
    const json = JSON.stringify({ score: 80 })
    const out = parseBaselineJson(json)
    expect(out?.hardFindings).toBe(0)
    expect(out?.softFindings).toBe(0)
    expect(out?.capsApplied).toBe(0)
  })
})
