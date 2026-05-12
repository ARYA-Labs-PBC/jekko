import { describe, expect, test } from "bun:test"
import {
  validateAssertions,
  canRetryAssertion,
} from "@/session/daemon-assertions"
import type { ZyalAssertions } from "@/agent-script/schema"

describe("daemon assertions", () => {
  const schema: ZyalAssertions = {
    require_structured_output: true,
    schema: {
      type: "object",
      properties: {
        files_changed: { type: "array" },
        confidence: { type: "number", minimum: 0, maximum: 1 },
        summary: { type: "string" },
      },
      required: ["files_changed", "confidence", "summary"],
    },
    on_invalid: "retry",
    max_retries: 2,
  }

  test("passes valid JSON matching schema", () => {
    const json = JSON.stringify({
      files_changed: ["a.ts", "b.ts"],
      confidence: 0.85,
      summary: "Updated auth module",
    })
    const result = validateAssertions(schema, json)
    expect(result.valid).toBe(true)
    if (result.valid) {
      expect(result.parsed.confidence).toBe(0.85)
    }
  })

  test("rejects non-JSON output", () => {
    const result = validateAssertions(schema, "Just some text without JSON")
    expect(result.valid).toBe(false)
    if (!result.valid) {
      expect(result.reason).toContain("does not contain valid JSON")
    }
  })

  test("rejects JSON missing required fields", () => {
    const json = JSON.stringify({
      files_changed: ["a.ts"],
      // missing confidence and summary
    })
    const result = validateAssertions(schema, json)
    expect(result.valid).toBe(false)
    if (!result.valid) {
      expect(result.reason).toContain("Missing required field")
    }
  })

  test("rejects wrong field types", () => {
    const json = JSON.stringify({
      files_changed: "not-an-array",
      confidence: 0.5,
      summary: "test",
    })
    const result = validateAssertions(schema, json)
    expect(result.valid).toBe(false)
    if (!result.valid) {
      expect(result.reason).toContain("expected type 'array'")
    }
  })

  test("rejects numeric value below minimum", () => {
    const json = JSON.stringify({
      files_changed: [],
      confidence: -0.5,
      summary: "test",
    })
    const result = validateAssertions(schema, json)
    expect(result.valid).toBe(false)
    if (!result.valid) {
      expect(result.reason).toContain("below minimum")
    }
  })

  test("rejects numeric value above maximum", () => {
    const json = JSON.stringify({
      files_changed: [],
      confidence: 1.5,
      summary: "test",
    })
    const result = validateAssertions(schema, json)
    expect(result.valid).toBe(false)
    if (!result.valid) {
      expect(result.reason).toContain("above maximum")
    }
  })

  test("extracts JSON from code blocks", () => {
    const text = `Here's my output:
\`\`\`json
{
  "files_changed": ["a.ts"],
  "confidence": 0.9,
  "summary": "done"
}
\`\`\`
That's all!`
    const result = validateAssertions(schema, text)
    expect(result.valid).toBe(true)
    if (result.valid) {
      expect(result.parsed.confidence).toBe(0.9)
    }
  })

  test("extracts JSON from inline braces", () => {
    const text = 'Output: {"files_changed": [], "confidence": 0.7, "summary": "nothing"} end'
    const result = validateAssertions(schema, text)
    expect(result.valid).toBe(true)
  })

  test("passes when assertions are disabled", () => {
    const result = validateAssertions(undefined, "anything")
    expect(result.valid).toBe(true)
  })

  test("passes when require_structured_output is false", () => {
    const result = validateAssertions({ require_structured_output: false }, "anything")
    expect(result.valid).toBe(true)
  })

  test("canRetryAssertion tracks attempts", () => {
    expect(canRetryAssertion(schema, 0)).toBe(true)
    expect(canRetryAssertion(schema, 1)).toBe(true)
    expect(canRetryAssertion(schema, 2)).toBe(false)
  })

  test("canRetryAssertion returns false when no assertions", () => {
    expect(canRetryAssertion(undefined, 0)).toBe(false)
  })
})
