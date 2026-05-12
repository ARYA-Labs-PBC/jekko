import { describe, expect, test } from "bun:test"
import { parseJavaMajorVersion } from "@/lsp/java-version"

describe("lsp.server", () => {
  test("parses modern java version output", () => {
    expect(parseJavaMajorVersion('openjdk version "21.0.2" 2024-01-16\n')).toBe(21)
  })

  test("parses java 8 style version output", () => {
    expect(parseJavaMajorVersion('java version "1.8.0_402"\n')).toBe(8)
  })

  test("rejects malformed output", () => {
    expect(parseJavaMajorVersion("java version 21")).toBeUndefined()
  })
})
