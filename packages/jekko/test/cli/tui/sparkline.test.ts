import { describe, expect, test } from "bun:test"
import { sparkline, SPARKLINE_GLYPHS, SPARKLINE_BLANK_GLYPH } from "../../../src/cli/cmd/tui/feature-plugins/jankurai/sparkline"

describe("sparkline", () => {
  test("empty values render blank glyphs for the full width", () => {
    expect(sparkline([], 6)).toBe(SPARKLINE_BLANK_GLYPH.repeat(6))
  })

  test("zero width is empty string", () => {
    expect(sparkline([1, 2, 3], 0)).toBe("")
  })

  test("constant values render the middle glyph for every column", () => {
    const out = sparkline([5, 5, 5, 5], 4)
    expect(out.length).toBe(4)
    for (const ch of out) {
      expect(ch).toBe(SPARKLINE_GLYPHS[Math.floor(SPARKLINE_GLYPHS.length / 2)]!)
    }
  })

  test("min and max map to first and last glyphs", () => {
    const out = sparkline([0, 100], 2)
    expect(out[0]).toBe(SPARKLINE_GLYPHS[0]!)
    expect(out[1]).toBe(SPARKLINE_GLYPHS[SPARKLINE_GLYPHS.length - 1]!)
  })

  test("width-clip keeps the most-recent samples", () => {
    const out = sparkline([1, 1, 1, 1, 1, 1, 1, 1, 1, 100], 3)
    expect(out.length).toBe(3)
    // The clipped tail's last char should be the peak (the 100).
    expect(out[out.length - 1]).toBe(SPARKLINE_GLYPHS[SPARKLINE_GLYPHS.length - 1]!)
  })

  test("left-pad with blank glyphs when fewer samples than width", () => {
    const out = sparkline([10, 20], 5)
    expect(out.length).toBe(5)
    expect(out.slice(0, 3)).toBe(SPARKLINE_BLANK_GLYPH.repeat(3))
  })

  test("non-finite values render as blank glyphs inline", () => {
    const out = sparkline([1, Number.NaN, 5], 3)
    expect(out.length).toBe(3)
    expect(out[1]).toBe(SPARKLINE_BLANK_GLYPH)
  })
})
