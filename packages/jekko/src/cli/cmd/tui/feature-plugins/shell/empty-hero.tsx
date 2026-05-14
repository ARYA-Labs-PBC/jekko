import { RGBA } from "@opentui/core"
import { createMemo, For } from "solid-js"
import {
  HIGH_CONTRAST_BLACK_COLORMAPS,
  type LogoMode,
  type RGB,
} from "@tui/component/logo"

type HeroRole = "blank" | "face" | "shadow" | "primary" | "secondary" | "version"

export type JekkoHeroCell = {
  char: string
  fg: RGB
  role: HeroRole
}

export type JekkoHeroRow = {
  cells: JekkoHeroCell[]
  text: string
}

export type JekkoHeroRowsOptions = {
  width: number
  mode: LogoMode
  version: string
}

const FULL_WORDMARK = [
  "██████  ██████ ██  ██ ██  ██  █████ ",
  "   ██   ██     ██ ██  ██ ██  ██   ██",
  "   ██   █████  ████   ████   ██   ██",
  "   ██   ██     ██ ██  ██ ██  ██   ██",
  "██ ██   ██     ██  ██ ██  ██ ██   ██",
  " ███    ██████ ██  ██ ██  ██  █████ ",
]

const COMPACT_WORDMARK = [
  "█████ █████ ██ ██ ██ ██  ███ ",
  "  ██  ██    ████  ████  ██ ██",
  "█ ██  ████  ██ ██ ██ ██ ██ ██",
  " █    █████ ██ ██ ██ ██  ███ ",
]

const DARK_SHADOW = ["#321801", "#4A2202", "#6E3808"]
const LIGHT_SHADOW = ["#D8C9AA", "#B89B62", "#8E6A25"]

const TEXT_PALETTE = {
  dark: {
    blank: "#8B5F1A",
    primary: "#F8E3B3",
    secondary: "#E8C055",
    version: "#B57F28",
  },
  light: {
    blank: "#7C5A11",
    primary: "#3A2405",
    secondary: "#6E4A0A",
    version: "#8C5F0D",
  },
} as const

const SECONDARY_LEFT = "Describe the outcome you want,"
const SECONDARY_RIGHT = "or press / for commands"

function clamp255(n: number): number {
  return Math.max(0, Math.min(255, Math.round(n)))
}

function hexToRGB(hex: string): RGB {
  const clean = hex.replace("#", "")
  return {
    r: clamp255(parseInt(clean.slice(0, 2), 16)),
    g: clamp255(parseInt(clean.slice(2, 4), 16)),
    b: clamp255(parseInt(clean.slice(4, 6), 16)),
  }
}

function mixRGB(left: RGB, right: RGB, t: number): RGB {
  const k = Math.max(0, Math.min(1, t))
  return {
    r: clamp255(left.r + (right.r - left.r) * k),
    g: clamp255(left.g + (right.g - left.g) * k),
    b: clamp255(left.b + (right.b - left.b) * k),
  }
}

function colorFromStops(stops: readonly string[], t: number): RGB {
  if (stops.length === 0) return { r: 255, g: 255, b: 255 }
  if (stops.length === 1) return hexToRGB(stops[0]!)

  const k = Math.max(0, Math.min(1, t))
  const pos = k * (stops.length - 1)
  const index = Math.min(stops.length - 2, Math.floor(pos))
  return mixRGB(hexToRGB(stops[index]!), hexToRGB(stops[index + 1]!), pos - index)
}

function palette(mode: LogoMode) {
  return mode === "light"
    ? HIGH_CONTRAST_BLACK_COLORMAPS.jekkoAmberLight
    : HIGH_CONTRAST_BLACK_COLORMAPS.jekkoAmber
}

function shadowPalette(mode: LogoMode) {
  return mode === "light" ? LIGHT_SHADOW : DARK_SHADOW
}

function glyphLength(text: string): number {
  return Array.from(text).length
}

function clipGlyphs(text: string, width: number): string {
  const chars = Array.from(text)
  return chars.length <= width ? text : chars.slice(0, width).join("")
}

function fit(text: string, width: number, align: "left" | "center" | "right" = "center"): string {
  const clipped = clipGlyphs(text, width)
  const remaining = Math.max(0, width - glyphLength(clipped))
  if (align === "left") return clipped + " ".repeat(remaining)
  if (align === "right") return " ".repeat(remaining) + clipped
  const left = Math.floor(remaining / 2)
  return " ".repeat(left) + clipped + " ".repeat(remaining - left)
}

function pair(left: string, right: string, width: number): string {
  const safeLeft = clipGlyphs(left, width)
  const safeRight = clipGlyphs(right, width)
  const gap = width - glyphLength(safeLeft) - glyphLength(safeRight)
  if (gap < 2) return fit(`${safeLeft} ${safeRight}`, width, "left")
  return safeLeft + " ".repeat(gap) + safeRight
}

function normalizeVersion(version: string): string {
  const value = version.trim()
  if (!value) return "Jekko v?"
  return value.startsWith("v") ? `Jekko ${value}` : `Jekko v${value}`
}

function textColor(role: "blank" | "primary" | "secondary" | "version", mode: LogoMode): RGB {
  return hexToRGB(TEXT_PALETTE[mode][role])
}

function makeRow(text: string, width: number, role: "blank" | "primary" | "secondary" | "version", mode: LogoMode): JekkoHeroRow {
  const line = fit(text, width)
  const fg = textColor(role, mode)
  return {
    text: line,
    cells: Array.from(line).map((char) => ({ char, fg, role })),
  }
}

function blankRow(width: number, mode: LogoMode): JekkoHeroRow {
  return makeRow("", width, "blank", mode)
}

function buildArtRows(art: string[], width: number, mode: LogoMode): JekkoHeroRow[] {
  const faceWidth = Math.max(...art.map(glyphLength))
  const faceRows = art.map((row) => fit(row, faceWidth, "left"))
  const visualWidth = faceWidth + 2
  const visualHeight = faceRows.length + 1
  const left = Math.max(0, Math.floor((width - visualWidth) / 2))
  const blank = textColor("blank", mode)
  const canvas: JekkoHeroCell[][] = Array.from({ length: visualHeight }, () =>
    Array.from({ length: width }, () => ({ char: " ", fg: blank, role: "blank" as const })),
  )

  function put(x: number, y: number, char: string, role: HeroRole, fg: RGB) {
    if (y < 0 || y >= canvas.length) return
    if (x < 0 || x >= width) return
    canvas[y]![x] = { char, fg, role }
  }

  for (let y = 0; y < faceRows.length; y++) {
    const chars = Array.from(faceRows[y]!)
    for (let x = 0; x < chars.length; x++) {
      if (chars[x] === " ") continue
      const t = faceWidth <= 1 ? 0 : (x * 0.82 + y * 0.18) / Math.max(1, faceWidth - 1)
      put(left + x + 2, y + 1, "▓", "shadow", colorFromStops(shadowPalette(mode), t))
      put(left + x + 1, y + 1, "▒", "shadow", colorFromStops(shadowPalette(mode), t + 0.12))
    }
  }

  for (let y = 0; y < faceRows.length; y++) {
    const chars = Array.from(faceRows[y]!)
    for (let x = 0; x < chars.length; x++) {
      const char = chars[x]!
      if (char === " ") continue
      const t = faceWidth <= 1 ? 0 : (x * 0.76 + y * 0.24) / Math.max(1, faceWidth - 1)
      put(left + x, y, char, "face", colorFromStops(palette(mode), t))
    }
  }

  return canvas.map((cells) => ({ cells, text: cells.map((cell) => cell.char).join("") }))
}

export function buildJekkoHeroRows(options: JekkoHeroRowsOptions): JekkoHeroRow[] {
  const requested = Math.max(1, Math.floor(options.width))
  const mode = options.mode
  const version = normalizeVersion(options.version)

  if (requested < 36) {
    return [
      makeRow("JEKKO", requested, "primary", mode),
      makeRow("Start a Jekko task", requested, "primary", mode),
      makeRow("Press / for commands", requested, "secondary", mode),
      makeRow(version, requested, "version", mode),
    ]
  }

  if (requested < 48) {
    const width = Math.min(42, requested)
    return [
      ...buildArtRows(COMPACT_WORDMARK, width, mode),
      blankRow(width, mode),
      makeRow("Start a Jekko task", width, "primary", mode),
      makeRow(SECONDARY_LEFT, width, "secondary", mode),
      makeRow(SECONDARY_RIGHT, width, "secondary", mode),
      makeRow(version, width, "version", mode),
    ]
  }

  const width = 48
  return [
    ...buildArtRows(FULL_WORDMARK, width, mode),
    blankRow(width, mode),
    makeRow(pair("Start a Jekko task", version, width), width, "primary", mode),
    makeRow(SECONDARY_LEFT, width, "secondary", mode),
    makeRow(SECONDARY_RIGHT, width, "secondary", mode),
  ]
}

function toRGBA(color: RGB): RGBA {
  return RGBA.fromInts(color.r, color.g, color.b, 255)
}

export function ShellEmptyHero(props: JekkoHeroRowsOptions) {
  const rows = createMemo(() => buildJekkoHeroRows(props))

  return (
    <box flexDirection="column" alignItems="center">
      <For each={rows()}>
        {(row) => (
          <box flexDirection="row">
            <For each={row.cells}>
              {(cell) => (
                <text fg={toRGBA(cell.fg)} selectable={false}>
                  {cell.char}
                </text>
              )}
            </For>
          </box>
        )}
      </For>
    </box>
  )
}
