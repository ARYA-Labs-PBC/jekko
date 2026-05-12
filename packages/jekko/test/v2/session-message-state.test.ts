import { expect, test } from "bun:test"
import {
  activeAssistant,
  activeCompaction,
  activeShell,
  latestReasoning,
  latestText,
  latestTool,
} from "../../src/v2/session-message-state"

test("shared session message selectors pick the latest active entries", () => {
  const assistantMessages = [
    {
      type: "assistant",
      time: { completed: "done" },
      content: [],
    },
    {
      type: "assistant",
      time: {},
      content: [
        { type: "text", text: "first" },
        { type: "tool", id: "call-a", state: { status: "pending" } },
        { type: "tool", id: "call-b", state: { status: "pending" } },
        { type: "reasoning", id: "reasoning-a", text: "why" },
      ],
    },
  ] as const

  const assistant = activeAssistant(assistantMessages)
  expect(assistant?.content).toHaveLength(4)
  expect(latestText(assistant)?.text).toBe("first")
  expect(latestTool(assistant, "call-b")?.id).toBe("call-b")
  expect(latestReasoning(assistant, "reasoning-a")?.text).toBe("why")

  const compactions = [
    { type: "compaction", summary: "prior" },
    { type: "compaction", summary: "new" },
  ] as const

  expect(activeCompaction(compactions)?.summary).toBe("new")

  const shells = [
    { type: "shell", callID: "a", output: "one" },
    { type: "shell", callID: "b", output: "two" },
  ] as const

  expect(activeShell(shells, "b")?.output).toBe("two")
})
