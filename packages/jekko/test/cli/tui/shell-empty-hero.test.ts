import { describe, expect, test } from "bun:test"
import { buildJekkoHeroRows } from "../../../src/cli/cmd/tui/feature-plugins/shell/empty-hero"

function glyphLength(text: string): number {
  return Array.from(text).length
}

function heroText(rows: ReturnType<typeof buildJekkoHeroRows>): string {
  return rows.map((row) => row.text).join("\n")
}

describe("shell empty hero", () => {
  test("full hero fits within 48 columns and includes onboarding copy", () => {
    const rows = buildJekkoHeroRows({ width: 80, mode: "dark", version: "1.2.3" })
    const text = heroText(rows)

    expect(rows.length).toBeLessThanOrEqual(12)
    expect(Math.max(...rows.map((row) => glyphLength(row.text)))).toBeLessThanOrEqual(48)
    expect(text).toContain("Start a Jekko task")
    expect(text).toContain("Describe the outcome you want")
    expect(text).toContain("/ for commands")
    expect(text).toContain("Jekko v1.2.3")
  })

  test("compact hero fits within 36-47 columns", () => {
    const rows = buildJekkoHeroRows({ width: 42, mode: "dark", version: "1.2.3" })
    const text = heroText(rows)

    expect(Math.max(...rows.map((row) => glyphLength(row.text)))).toBeLessThanOrEqual(42)
    expect(text).toContain("Start a Jekko task")
    expect(text).toContain("Describe the outcome you want")
    expect(text).toContain("/ for commands")
    expect(text).toContain("Jekko v1.2.3")
  })

  test("minimal fallback fits under 36 columns", () => {
    const rows = buildJekkoHeroRows({ width: 32, mode: "dark", version: "1.2.3" })
    const text = heroText(rows)

    expect(Math.max(...rows.map((row) => glyphLength(row.text)))).toBeLessThanOrEqual(32)
    expect(text).toContain("Start a Jekko task")
    expect(text).toContain("/ for commands")
    expect(text).toContain("Jekko v1.2.3")
  })

  test("dark and light modes choose different face and shadow palettes", () => {
    const dark = buildJekkoHeroRows({ width: 48, mode: "dark", version: "1.2.3" }).flatMap((row) => row.cells)
    const light = buildJekkoHeroRows({ width: 48, mode: "light", version: "1.2.3" }).flatMap((row) => row.cells)

    expect(dark.find((cell) => cell.role === "face")?.fg).not.toEqual(
      light.find((cell) => cell.role === "face")?.fg,
    )
    expect(dark.find((cell) => cell.role === "shadow")?.fg).not.toEqual(
      light.find((cell) => cell.role === "shadow")?.fg,
    )
  })
})
