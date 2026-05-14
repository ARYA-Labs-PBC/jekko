import { expect, test } from "bun:test"

const { DEFAULT_THEMES, allThemes, addTheme, hasTheme, resolveTheme } = await import(
  "../../../src/cli/cmd/tui/context/theme"
)

function luminance(color: { r: number; g: number; b: number }): number {
  const channel = (value: number) => {
    const x = value > 1 ? value / 255 : value
    return x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4
  }

  return 0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

function contrast(a: { r: number; g: number; b: number }, b: { r: number; g: number; b: number }): number {
  const [one, two] = [luminance(a), luminance(b)].sort((x, y) => y - x)
  return (one + 0.05) / (two + 0.05)
}

test("addTheme writes into module theme store", () => {
  const name = `plugin-theme-${Date.now()}`
  expect(addTheme(name, DEFAULT_THEMES.jekko)).toBe(true)

  expect(allThemes()[name]).toBeDefined()
})

test("addTheme keeps first theme for duplicate names", () => {
  const name = `plugin-theme-keep-${Date.now()}`
  const one = structuredClone(DEFAULT_THEMES.jekko)
  const two = structuredClone(DEFAULT_THEMES.jekko)
  one.theme.primary = "#101010"
  two.theme.primary = "#fefefe"

  expect(addTheme(name, one)).toBe(true)
  expect(addTheme(name, two)).toBe(false)

  expect(allThemes()[name]).toBeDefined()
  expect(allThemes()[name]!.theme.primary).toBe("#101010")
})

test("addTheme ignores entries without a theme object", () => {
  const name = `plugin-theme-invalid-${Date.now()}`
  expect(addTheme(name, { defs: { a: "#ffffff" } })).toBe(false)
  expect(allThemes()[name]).toBeUndefined()
})

test("hasTheme checks theme presence", () => {
  const name = `plugin-theme-has-${Date.now()}`
  expect(hasTheme(name)).toBe(false)
  expect(addTheme(name, DEFAULT_THEMES.jekko)).toBe(true)
  expect(hasTheme(name)).toBe(true)
})

test("default theme registry is Jekko-branded and keeps light alias compatibility", () => {
  expect(Object.keys(DEFAULT_THEMES).sort()).toEqual(["jekko", "jekko-gold", "jekko-light"])
})

test("jekko preset resolves to distinct dark and light palettes", () => {
  const dark = resolveTheme(DEFAULT_THEMES.jekko, "dark")
  const light = resolveTheme(DEFAULT_THEMES.jekko, "light")

  expect(dark.background.r).toBeLessThan(0.1)
  expect(light.background.r).toBeGreaterThan(0.9)
  expect(dark.background.r).not.toBe(light.background.r)
  expect(light.primary.r).toBeLessThan(light.background.r)
})

test("jekko dark theme layers surfaces and keeps muted text readable", () => {
  const dark = resolveTheme(DEFAULT_THEMES.jekko, "dark")

  expect(luminance(dark.background)).toBeLessThan(luminance(dark.backgroundPanel))
  expect(luminance(dark.backgroundPanel)).toBeLessThan(luminance(dark.backgroundElement))
  expect(luminance(dark.backgroundElement)).toBeLessThan(luminance(dark.backgroundMenu))
  expect(contrast(dark.text, dark.background)).toBeGreaterThanOrEqual(7)
  expect(contrast(dark.textMuted, dark.background)).toBeGreaterThanOrEqual(4.5)
})

test("jekko light theme layers surfaces and keeps muted text readable", () => {
  const light = resolveTheme(DEFAULT_THEMES.jekko, "light")

  expect(luminance(light.background)).toBeGreaterThan(luminance(light.backgroundPanel))
  expect(luminance(light.backgroundPanel)).toBeGreaterThan(luminance(light.backgroundElement))
  expect(luminance(light.backgroundElement)).toBeGreaterThan(luminance(light.backgroundMenu))
  expect(contrast(light.text, light.background)).toBeGreaterThanOrEqual(7)
  expect(contrast(light.textMuted, light.background)).toBeGreaterThanOrEqual(4.5)
})

test("resolveTheme rejects circular color refs", () => {
  const item = structuredClone(DEFAULT_THEMES.jekko)
  item.defs = {
    ...item.defs,
    one: "two",
    two: "one",
  }
  item.theme.primary = "one"

  expect(() => resolveTheme(item, "dark")).toThrow("Circular color reference")
})
