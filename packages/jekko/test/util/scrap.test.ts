import { describe, expect, test } from "bun:test"
import { formatLabel, hasMeaningfulText, normalizeLabel } from "../../src/util/scrap"

describe("util.scrap", () => {
  test("normalizes label text", () => {
    expect(normalizeLabel("  alpha   beta  ")).toBe("alpha beta")
  })

  test("detects meaningful text", () => {
    expect(hasMeaningfulText("  ")).toBe(false)
    expect(hasMeaningfulText("value")).toBe(true)
  })

  test("formats labeled values", () => {
    expect(formatLabel("  status  ", "  ready now ")).toBe("status: ready now")
    expect(formatLabel("   ", "  ready now ")).toBe("ready now")
  })
})
