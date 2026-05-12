import { describe, expect, test } from "bun:test"
import {
  evaluateInputGuardrails,
  evaluateOutputPatternGuardrails,
  getIterationShellGuardrails,
  getOutputShellGuardrails,
} from "@/session/daemon-guardrails"
import type { ZyalGuardrails } from "@/agent-script/schema"

describe("daemon guardrails", () => {
  const guardrails: ZyalGuardrails = {
    input: [
      {
        name: "no-force-push",
        deny_patterns: ["git push --force", "git push -f"],
        action: "block",
      },
      {
        name: "no-drop-table",
        deny_patterns: ["DROP TABLE"],
        action: "block",
      },
    ],
    output: [
      {
        name: "no-secrets",
        deny_patterns: ["example-secret-[a-zA-Z0-9]{20,}"],
        scope: "tool_output",
        action: "block",
      },
      {
        name: "type-check",
        shell: "npx tsgo --noEmit",
        on_fail: "retry",
        max_retries: 2,
      },
    ],
    iteration: [
      {
        name: "diff-size",
        shell: "git diff --stat | wc -l",
        on_fail: "warn",
      },
    ],
  }

  test("input guardrail blocks matching command", () => {
    const result = evaluateInputGuardrails(guardrails, "git push --force origin main")
    expect(result.pass).toBe(false)
    if (!result.pass) {
      expect(result.name).toBe("no-force-push")
      expect(result.action).toBe("block")
    }
  })

  test("input guardrail passes clean commands", () => {
    const result = evaluateInputGuardrails(guardrails, "git push origin main")
    expect(result.pass).toBe(true)
  })

  test("input guardrail matches case-insensitively", () => {
    const result = evaluateInputGuardrails(guardrails, "drop table users")
    expect(result.pass).toBe(false)
  })

  test("output pattern guardrail blocks secret-like strings", () => {
    const result = evaluateOutputPatternGuardrails(
      guardrails,
      "API key: example-secret-abcdefghijklmnopqrstuvwxyz1234567890",
      "tool_output",
    )
    expect(result.pass).toBe(false)
    if (!result.pass) {
      expect(result.name).toBe("no-secrets")
    }
  })

  test("output pattern guardrail passes clean text", () => {
    const result = evaluateOutputPatternGuardrails(
      guardrails,
      "Everything compiled successfully",
      "tool_output",
    )
    expect(result.pass).toBe(true)
  })

  test("output pattern guardrail respects scope filter", () => {
    // The no-secrets guardrail has scope "tool_output" so it shouldn't match file_diff
    const result = evaluateOutputPatternGuardrails(
      guardrails,
      "example-secret-abcdefghijklmnopqrstuvwxyz1234567890",
      "file_diff",
    )
    expect(result.pass).toBe(true)
  })

  test("getIterationShellGuardrails returns iteration rails", () => {
    const rails = getIterationShellGuardrails(guardrails)
    expect(rails).toHaveLength(1)
    expect(rails[0].name).toBe("diff-size")
  })

  test("getOutputShellGuardrails filters to shell-type rails", () => {
    const rails = getOutputShellGuardrails(guardrails)
    expect(rails).toHaveLength(1)
    expect(rails[0].name).toBe("type-check")
  })

  test("returns pass when guardrails are undefined", () => {
    expect(evaluateInputGuardrails(undefined, "anything").pass).toBe(true)
    expect(evaluateOutputPatternGuardrails(undefined, "anything").pass).toBe(true)
    expect(getIterationShellGuardrails(undefined)).toHaveLength(0)
    expect(getOutputShellGuardrails(undefined)).toHaveLength(0)
  })
})
