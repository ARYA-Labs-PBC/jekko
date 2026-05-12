import { EOL } from "os"

function fg(r: number, g: number, b: number) {
  return `\x1b[38;2;${r};${g};${b}m`
}

const RESET = "\x1b[0m"
const BOLD = "\x1b[1m"
const DIM = "\x1b[2m"

export const Style = {
  TEXT_NORMAL: RESET,
  TEXT_NORMAL_BOLD: BOLD,
  TEXT_DIM: DIM,
  TEXT_DIM_BOLD: DIM + BOLD,
  TEXT_DANGER_BOLD: "\x1b[31;1m",
  TEXT_SUCCESS: "\x1b[32m",
  TEXT_INFO_BOLD: "\x1b[36;1m",
  TEXT_SUCCESS_BOLD: "\x1b[32;1m",
  TEXT_WARNING_BOLD: "\x1b[33;1m",
  TEXT_HIGHLIGHT_BOLD: "\x1b[35;1m",
} as const

export class CancelledError extends Error {
  constructor() {
    super("Cancelled")
    this.name = "CancelledError"
  }
}

export function println(...args: string[]) {
  process.stdout.write(args.join(" ") + EOL)
}

export function empty() {
  println()
}

function paint(text: string, rgb: readonly [number, number, number], opts: { bold?: boolean; dim?: boolean } = {}) {
  let res = ""
  if (opts.bold) res += BOLD
  if (opts.dim) res += DIM
  res += fg(...rgb) + text + RESET
  return res
}

// ---------------------------------------------------------------------------
// Acid Gecko Bloom — 512-step unified diagonal gradient (sync with logo.tsx)
// ---------------------------------------------------------------------------

const GRADIENT_STEPS = 512

function clamp01(n: number): number {
  return Math.max(0, Math.min(1, n))
}

function clamp255(n: number): number {
  return Math.max(0, Math.min(255, Math.round(n)))
}

function hexToRGB(hex: string): [number, number, number] {
  const clean = hex.replace("#", "")
  return [
    parseInt(clean.slice(0, 2), 16),
    parseInt(clean.slice(2, 4), 16),
    parseInt(clean.slice(4, 6), 16),
  ]
}

function rgbToHSV(r: number, g: number, b: number): [number, number, number] {
  const rn = r / 255, gn = g / 255, bn = b / 255
  const max = Math.max(rn, gn, bn)
  const min = Math.min(rn, gn, bn)
  const delta = max - min

  let h = 0
  if (delta !== 0) {
    if (max === rn) h = 60 * (((gn - bn) / delta) % 6)
    else if (max === gn) h = 60 * ((bn - rn) / delta + 2)
    else h = 60 * ((rn - gn) / delta + 4)
  }
  if (h < 0) h += 360

  return [h, max === 0 ? 0 : delta / max, max]
}

function hsvToRGB(h: number, s: number, v: number): [number, number, number] {
  const hue = ((h % 360) + 360) % 360
  const sat = clamp01(s)
  const val = clamp01(v)
  const c = val * sat
  const x = c * (1 - Math.abs(((hue / 60) % 2) - 1))
  const m = val - c

  let r = 0, g = 0, b = 0
  if (hue < 60)       { r = c; g = x }
  else if (hue < 120) { r = x; g = c }
  else if (hue < 180) { g = c; b = x }
  else if (hue < 240) { g = x; b = c }
  else if (hue < 300) { r = x; b = c }
  else                { r = c; b = x }

  return [clamp255((r + m) * 255), clamp255((g + m) * 255), clamp255((b + m) * 255)]
}

function mixHue(a: number, b: number, t: number): number {
  const delta = ((b - a + 540) % 360) - 180
  return (a + delta * t + 360) % 360
}

function smoothstep(t: number): number {
  const x = clamp01(t)
  return x * x * (3 - 2 * x)
}

function forceNeon(rgb: [number, number, number]): [number, number, number] {
  const [h, _s, _v] = rgbToHSV(rgb[0], rgb[1], rgb[2])
  return hsvToRGB(h, 1, 1)
}

const BLOOM_STOPS = [
  "#00FF16",  // acid green
  "#66FF00",  // electric lime
  "#00FFCC",  // neon cyan
  "#00BBFF",  // electric blue
  "#6600FF",  // deep violet
  "#CC00FF",  // vivid magenta
  "#FF00B8",  // hot pink
].map(hexToRGB)

function buildGradientLUT(stops: [number, number, number][], steps: number): [number, number, number][] {
  const out: [number, number, number][] = []
  const segments = stops.length - 1

  for (let i = 0; i < steps; i++) {
    const t = i / (steps - 1)
    const pos = t * segments
    const seg = Math.min(segments - 1, Math.floor(pos))
    const localT = smoothstep(pos - seg)

    const lo = rgbToHSV(...stops[seg]!)
    const hi = rgbToHSV(...stops[seg + 1]!)

    const h = mixHue(lo[0], hi[0], localT)
    const s = lo[1] + (hi[1] - lo[1]) * localT
    const v = lo[2] + (hi[2] - lo[2]) * localT

    out.push(forceNeon(hsvToRGB(h, s, v)))
  }

  return out
}

const ACID_GECKO_BLOOM = buildGradientLUT(BLOOM_STOPS, GRADIENT_STEPS)

function bloomColor(
  x: number,
  y: number,
  width: number,
  height: number,
): [number, number, number] {
  const tx = width <= 1 ? 0 : x / (width - 1)
  const ty = height <= 1 ? 0 : y / (height - 1)

  const t = clamp01((tx + ty) / 2)
  const idx = Math.max(0, Math.min(GRADIENT_STEPS - 1, Math.round(t * (GRADIENT_STEPS - 1))))
  return ACID_GECKO_BLOOM[idx]!
}

// Physical X position — whole box is one unified diagonal gradient.
function gradientLine(
  text: string,
  y: number,
  totalRows: number,
  totalWidth: number,
  opts: { bold?: boolean } = {},
): string {
  const chars = Array.from(text)
  let out = ""
  for (let x = 0; x < chars.length; x++) {
    const rgb = bloomColor(x, y, totalWidth, totalRows)
    out += paint(chars[x]!, rgb, { bold: opts.bold })
  }
  return out
}

const INNER_WIDTH = 78

function fit(text: string, width: number): string {
  const remaining = Math.max(0, width - text.length)
  const left = Math.floor(remaining / 2)
  const right = remaining - left
  return " ".repeat(left) + text + " ".repeat(right)
}

function pairStr(left: string, right: string, width = INNER_WIDTH): string {
  const gap = Math.max(1, width - left.length - right.length)
  return left + " ".repeat(gap) + right
}

const JEKKO_WORDMARK = [
  "     ██╗███████╗██╗  ██╗██╗  ██╗ ██████╗ ",
  "     ██║██╔════╝██║ ██╔╝██║ ██╔╝██╔═══██╗",
  "     ██║█████╗  █████╔╝ █████╔╝ ██║   ██║",
  "██   ██║██╔══╝  ██╔═██╗ ██╔═██╗ ██║   ██║",
  "╚█████╔╝███████╗██║  ██╗██║  ██╗╚██████╔╝",
  " ╚════╝ ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝ ",
]

export function logo(pad?: string) {
  if (!process.stdout.isTTY && !process.stderr.isTTY) {
    const result = []
    for (const row of JEKKO_WORDMARK) {
      if (pad) result.push(pad)
      result.push(row)
      result.push(EOL)
    }
    return result.join("").trimEnd()
  }

  const top = `╭${"─".repeat(INNER_WIDTH)}╮`
  const bottom = `╰${"─".repeat(INNER_WIDTH)}╯`
  const sep = `├${"─".repeat(INNER_WIDTH)}┤`

  type Row = { text: string; bold?: boolean }
  const rows: Row[] = []

  rows.push({ text: top, bold: true })
  rows.push({
    text: `│${pairStr(" ›_ JEKKO", "gecko mode active  ● ● ● ")}│`,
    bold: true,
  })
  rows.push({ text: sep, bold: true })
  rows.push({ text: `│${fit("", INNER_WIDTH)}│`, bold: true })

  for (const line of JEKKO_WORDMARK) {
    rows.push({ text: `│${fit(line, INNER_WIDTH)}│`, bold: true })
  }

  rows.push({ text: `│${fit("", INNER_WIDTH)}│`, bold: true })
  rows.push({
    text: `│${fit("AI coding gecko • ZYAL support • climbs hard problems", INNER_WIDTH)}│`,
    bold: true,
  })
  rows.push({
    text: `│${fit("gecko:// safe autonomous coding ready", INNER_WIDTH)}│`,
    bold: true,
  })
  rows.push({ text: bottom, bold: true })

  const totalRows = rows.length
  const totalWidth = Math.max(...rows.map((r) => r.text.length))
  const rendered = rows.map((row, y) =>
    (pad || "") + gradientLine(row.text, y, totalRows, totalWidth, { bold: row.bold }),
  )

  return rendered.join(EOL)
}

export async function input(prompt: string): Promise<string> {
  const readline = require("readline")
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  })

  return new Promise((resolve) => {
    rl.question(prompt, (answer: string) => {
      rl.close()
      resolve(answer.trim())
    })
  })
}

export function error(message: string) {
  if (message.startsWith("Error: ")) {
    message = message.slice("Error: ".length)
  }
  println(Style.TEXT_DANGER_BOLD + "Error: " + Style.TEXT_NORMAL + message)
}

export function markdown(text: string): string {
  return text
}

export * as UI from "./ui"
