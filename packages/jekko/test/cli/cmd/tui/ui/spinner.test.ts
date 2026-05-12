import { describe, expect, test } from "bun:test"
import { RGBA } from "@opentui/core"
import {
  createColors,
  createFrames,
  resolveKnightRiderTrailConfig,
} from "../../../../../src/cli/cmd/tui/ui/spinner"

describe("resolveKnightRiderTrailConfig", () => {
  test("builds one shared palette for both frame and color generators", () => {
    const config = resolveKnightRiderTrailConfig({
      color: "#112233",
      trailSteps: 3,
      holdStart: 2,
      holdEnd: 4,
      inactiveFactor: 0.5,
    })

    expect(config.colors).toHaveLength(3)
    expect(config.trailOptions.colors).toBe(config.colors)
    expect(config.trailOptions.trailLength).toBe(3)
    expect(config.trailOptions.holdFrames).toEqual({ start: 2, end: 4 })
    expect(config.defaultColor.a).toBeCloseTo(0.5, 2)
    expect(config.defaultColor).toBe(config.trailOptions.defaultColor)
  })
})

describe("createFrames and createColors", () => {
  test("use the shared resolver output without throwing", () => {
    const options = {
      color: "#112233",
      trailSteps: 2,
      width: 2,
      holdStart: 1,
      holdEnd: 1,
    }

    const frames = createFrames(options)
    const colors = createColors(options)

    expect(frames).toHaveLength(5)
    expect(frames[0]).toHaveLength(2)
    expect(colors(0, 0, 5, 2)).toBeInstanceOf(RGBA)
  })
})
