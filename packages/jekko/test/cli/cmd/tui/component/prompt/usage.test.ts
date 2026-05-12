import { describe, expect, test } from "bun:test"
import { computePromptUsage, type PromptUsageMessage } from "../../../../../../src/cli/cmd/tui/component/prompt/usage"

const assistantMessage: PromptUsageMessage = {
  role: "assistant",
  providerID: "openai",
  modelID: "gpt-4o",
  cost: 1.25,
  tokens: {
    input: 10,
    output: 5,
    reasoning: 2,
    cache: {
      read: 1,
      write: 0,
    },
  },
}

describe("prompt usage", () => {
  test("returns an explicit state when no session is selected", () => {
    const usage = computePromptUsage(undefined, [], [])

    expect(usage.kind).toBe("missing-session")
    if (usage.kind !== "missing-session") return

    expect(usage.reason).toContain("Open a session")
  })

  test("returns ready usage for assistant token totals", () => {
    const usage = computePromptUsage("session-1", [assistantMessage], [
      {
        id: "openai",
        models: {
          "gpt-4o": {
            limit: {
              context: 8192,
            },
          },
        },
      },
    ])

    expect(usage.kind).toBe("ready")
    if (usage.kind !== "ready") return

    expect(usage.tokens).toBe(18)
    expect(usage.cost).toBe(1.25)
    expect(usage.contextLimit).toBe(8192)
  })

  test("returns an explicit state until assistant usage is available", () => {
    const usage = computePromptUsage("session-1", [{ role: "user" }], [])

    expect(usage.kind).toBe("missing-assistant")
    if (usage.kind !== "missing-assistant") return

    expect(usage.reason).toContain("assistant reply")
  })
})
