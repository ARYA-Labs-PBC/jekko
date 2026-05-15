import { describe, expect, test } from "bun:test"
import { parseCapabilityJson } from "../../../src/cli/cmd/tui/context/capability"

describe("parseCapabilityJson", () => {
  test("reports malformed JSON", () => {
    const out = parseCapabilityJson("{bad")

    expect(out.ok).toBe(false)
    if (!out.ok) {
      expect(out.message).toContain("not valid JSON")
      expect(out.repairHint).toContain("jankurai audit")
    }
  })

  test("reports non-object input", () => {
    const out = parseCapabilityJson("[1,2,3]")

    expect(out.ok).toBe(false)
    if (!out.ok) {
      expect(out.message).toContain("JSON object")
    }
  })

  test("reports missing score", () => {
    const out = parseCapabilityJson(JSON.stringify({ caps_applied: [] }))

    expect(out.ok).toBe(false)
    if (!out.ok) {
      expect(out.message).toContain("numeric score")
    }
  })

  test("parses a valid minimal score record", () => {
    const out = parseCapabilityJson(JSON.stringify({ score: 85 }))

    expect(out.ok).toBe(true)
    if (out.ok) {
      expect(out.state).toMatchObject({
        score: 85,
        decision: "unknown",
        hardFindings: 0,
        softFindings: 0,
        capsApplied: 0,
        loaded: true,
        error: undefined,
      })
    }
  })
})
