import { describe, expect, test } from "bun:test"
import {
  decodePromptHistoryLine,
  parsePromptHistoryText,
  type PromptInfo,
} from "../../../../../../src/cli/cmd/tui/component/prompt/history"

const validPrompt: PromptInfo = {
  input: "hello",
  mode: "shell",
  parts: [
    {
      type: "text",
      text: "hello world",
    },
  ],
}

describe("prompt history", () => {
  test("accepts valid prompt history lines", () => {
    const decoded = decodePromptHistoryLine(JSON.stringify(validPrompt))

    expect(decoded.kind).toBe("valid")
    if (decoded.kind !== "valid") return

    expect(decoded.value).toEqual(validPrompt)
  })

  test("rejects malformed prompt history lines", () => {
    expect(decodePromptHistoryLine("not json").kind).toBe("invalid")

    const decoded = decodePromptHistoryLine(JSON.stringify({ input: "hello", parts: [{ type: "text", text: 123 }] }))
    expect(decoded.kind).toBe("invalid")
  })

  test("drops invalid lines while keeping valid ones", () => {
    const parsed = parsePromptHistoryText([JSON.stringify(validPrompt), "not json"].join("\n"))

    expect(parsed.entries).toHaveLength(1)
    expect(parsed.entries[0]).toEqual(validPrompt)
    expect(parsed.dropped).toBe(1)
  })
})
