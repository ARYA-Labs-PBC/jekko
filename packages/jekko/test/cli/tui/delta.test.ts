import { describe, expect, test } from "bun:test"
import { delta, formatDelta } from "../../../src/cli/cmd/tui/feature-plugins/jankurai/delta"

describe("delta", () => {
  test("unknown when either side is missing", () => {
    expect(delta(undefined, 5, "score").direction).toBe("unknown")
    expect(delta(5, undefined, "score").direction).toBe("unknown")
    expect(delta(undefined, undefined, "score").glyph).toBe("—")
  })

  test("score: higher than baseline is improving", () => {
    const result = delta(78, 60, "score")
    expect(result.delta).toBe(18)
    expect(result.direction).toBe("improving")
    expect(result.glyph).toBe("▲▲") // |Δ|=18 >= 10
  })

  test("score: lower than baseline is worsening", () => {
    const result = delta(54, 60, "score")
    expect(result.direction).toBe("worsening")
    expect(result.glyph).toBe("▼") // |Δ|=6 < 10 -> single glyph
  })

  test("score: large drop uses double glyph", () => {
    const result = delta(40, 60, "score")
    expect(result.direction).toBe("worsening")
    expect(result.glyph).toBe("▼▼") // |Δ|=20 -> big
  })

  test("findings: lower than baseline is improving", () => {
    const result = delta(3, 5, "caps")
    expect(result.delta).toBe(-2)
    expect(result.direction).toBe("improving")
    expect(result.glyph).toBe("▲")
  })

  test("findings: higher than baseline is worsening", () => {
    const result = delta(20, 5, "hard")
    expect(result.direction).toBe("worsening")
    expect(result.glyph).toBe("▼▼") // |Δ|=15 >= 10
  })

  test("zero delta is neutral", () => {
    const result = delta(5, 5, "soft")
    expect(result.delta).toBe(0)
    expect(result.direction).toBe("neutral")
    expect(result.glyph).toBe("=")
  })

  test("formatDelta renders sign + glyph", () => {
    expect(formatDelta(delta(78, 60, "score"))).toBe("+18 ▲▲")
    expect(formatDelta(delta(3, 5, "caps"))).toBe("-2 ▲")
    expect(formatDelta(delta(5, 5, "soft"))).toBe("= 0")
    expect(formatDelta(delta(undefined, 5, "score"))).toBe("—")
  })
})
