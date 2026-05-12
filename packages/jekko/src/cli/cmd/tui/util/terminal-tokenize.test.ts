import { describe, expect, test } from "bun:test"
import { tokenizeTerminal } from "./terminal-tokenize"

describe("tokenizeTerminal", () => {
  test("bounds the scan length", () => {
    const prefix = "$ echo hello"
    const text = `${prefix}\n${"x".repeat(20_000)}`

    const tokens = tokenizeTerminal(text)

    expect(tokens.some((token) => token.scope === "command")).toBe(true)
    expect(tokens.every((token) => token.end <= 10_000)).toBe(true)
  })
})
