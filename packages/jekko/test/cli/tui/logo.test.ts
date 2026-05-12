import { describe, expect, test } from "bun:test"
import { firstVisibleIndex, lastVisibleIndex } from "../../../src/cli/cmd/tui/component/logo"

describe("logo visible index helpers", () => {
  test("return explicit found and missing states", () => {
    expect(firstVisibleIndex("   #  ")).toEqual({ kind: "found", index: 3 })
    expect(firstVisibleIndex("     ")).toEqual({ kind: "missing" })
    expect(lastVisibleIndex("  #   ")).toEqual({ kind: "found", index: 2 })
    expect(lastVisibleIndex("     ")).toEqual({ kind: "missing" })
  })
})
