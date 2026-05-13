// jankurai:allow HLT-000-SCORE-DIMENSION reason=large-structured-logo-renderer-with-parallel-pixel-font-patterns expires=2027-01-01
import { RGBA } from "@opentui/core"
import { createMemo, For } from "solid-js"
import { useTheme } from "@tui/context/theme"

export type Align = "left" | "center" | "right"
export type RGB = { r: number; g: number; b: number }
type VisibleIndexResult = { kind: "found"; index: number } | { kind: "missing" }
type WordmarkLayer = "wordmark" | "wordmarkShadowNear" | "wordmarkShadowMid" | "wordmarkShadowFar"
type WordmarkSource = {
  layer: WordmarkLayer
  sourceX: number
  sourceY: number
  sourceWidth: number
  sourceHeight: number
}

export type CellLayer =
  | "global"
  | "wordmark"
  | "wordmarkShadowNear"
  | "wordmarkShadowMid"
  | "wordmarkShadowFar"

export type LogoCell = {
  char: string
  layer: CellLayer
  strong?: boolean
  dim?: boolean
  sourceX?: number
  sourceY?: number
  sourceWidth?: number
  sourceHeight?: number
}

export type LogoRow = {
  cells: LogoCell[]
  strong?: boolean
  dim?: boolean
}

export type LogoProps = {
  shape?: unknown
  ink?: RGBA
  idle?: boolean
  support?: string
  status?: string
}

export type GoLogoProps = {
  idle?: boolean
}

export type SvgPreviewOptions = {
  cellWidth?: number
  cellHeight?: number
  fontSize?: number
  baseline?: number
  paddingX?: number
  paddingY?: number
  background?: string
  title?: string
}

// 78 inner columns fills a standard 80-col terminal and gives the scaleX=2
// pixel font + subtle shadow room to breathe without clipping.
export const INNER_WIDTH = 78
export const OUTER_WIDTH = INNER_WIDTH + 2
const GRADIENT_STEPS = 512
const BLACK = { r: 0, g: 0, b: 0 } satisfies RGB
const WHITE = { r: 255, g: 255, b: 255 } satisfies RGB

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

function clamp01(n: number): number {
  return Math.max(0, Math.min(1, n))
}

function clamp255(n: number): number {
  return Math.max(0, Math.min(255, Math.round(n)))
}

function rgb(r: number, g: number, b: number): RGB {
  return { r: clamp255(r), g: clamp255(g), b: clamp255(b) }
}

function toRGBA(color: RGB): RGBA {
  return RGBA.fromInts(color.r, color.g, color.b, 255)
}

function hexToRGB(hex: string): RGB {
  const clean = hex.replace("#", "").trim()

  if (clean.length !== 6) {
    return rgb(255, 0, 184)
  }

  return rgb(
    parseInt(clean.slice(0, 2), 16),
    parseInt(clean.slice(2, 4), 16),
    parseInt(clean.slice(4, 6), 16),
  )
}

export function colorToCss(color: RGB): string {
  return `rgb(${color.r},${color.g},${color.b})`
}

function mixRGB(left: RGB, right: RGB, t: number): RGB {
  const k = clamp01(t)
  return rgb(
    left.r + (right.r - left.r) * k,
    left.g + (right.g - left.g) * k,
    left.b + (right.b - left.b) * k,
  )
}

function smoothstep(t: number): number {
  const x = clamp01(t)
  return x * x * (3 - 2 * x)
}

function srgbToLinear(value: number): number {
  const x = clamp01(value / 255)
  return x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4
}

function relativeLuminance(color: RGB): number {
  return (
    0.2126 * srgbToLinear(color.r) +
    0.7152 * srgbToLinear(color.g) +
    0.0722 * srgbToLinear(color.b)
  )
}

export function contrastOnBlack(color: RGB): number {
  return (relativeLuminance(color) + 0.05) / 0.05
}

function liftContrastOnBlack(color: RGB, minimumContrast: number): RGB {
  if (contrastOnBlack(color) >= minimumContrast) {
    return color
  }

  let lo = 0
  let hi = 1
  let best = color

  for (let i = 0; i < 18; i++) {
    const mid = (lo + hi) / 2
    const candidate = mixRGB(color, WHITE, mid)

    if (contrastOnBlack(candidate) >= minimumContrast) {
      best = candidate
      hi = mid
    } else {
      lo = mid
    }
  }

  return best
}

function dimReadable(color: RGB, amount: number, minimumContrast: number): RGB {
  return liftContrastOnBlack(mixRGB(color, BLACK, amount), minimumContrast)
}

// ---------------------------------------------------------------------------
// Jekko amber palette
// ---------------------------------------------------------------------------
// Single-hue amber gradient, deep umber → cream highlight. No rainbow, no
// cyan/magenta. Anchor stop `#D4A843` is the canonical JEKKO gold used as the
// brand accent across the TUI. Stops are monotonic in luminance so the
// gradient reads as embossed metal rather than tie-dye.

export const HIGH_CONTRAST_BLACK_COLORMAPS = {
  jekkoAmber: [
    "#3D2606",
    "#5C3A0E",
    "#8B5F1A",
    "#B57F28",
    "#D4A843",
    "#E8C055",
    "#EFD17A",
    "#F8E3B3",
  ],
  // Light-mode amber stops: all medium-dark to dark, monotone luminance.
  // Reads as embossed bronze on warm-cream `#FBF7EE` canvas. No contrast
  // lift is applied (`minimumContrast: 0`) — these stops already contrast.
  jekkoAmberLight: [
    "#2D1A04",
    "#4F3408",
    "#553707",
    "#6E4A0A",
    "#7C5A11",
    "#8C5F0D",
    "#A67817",
    "#B98828",
  ],
} as const

const JEKKO_AMBER_STOPS = HIGH_CONTRAST_BLACK_COLORMAPS.jekkoAmber.map(hexToRGB)
const WORDMARK_AMBER_STOPS = HIGH_CONTRAST_BLACK_COLORMAPS.jekkoAmber
  .slice(0, 7)
  .map(hexToRGB)
const JEKKO_AMBER_LIGHT_STOPS = HIGH_CONTRAST_BLACK_COLORMAPS.jekkoAmberLight.map(hexToRGB)
const WORDMARK_AMBER_LIGHT_STOPS = HIGH_CONTRAST_BLACK_COLORMAPS.jekkoAmberLight
  .slice(0, 7)
  .map(hexToRGB)

function buildGradientLUT(
  stops: RGB[],
  steps: number,
  minimumContrast: number,
): RGB[] {
  if (stops.length < 2) {
    return [liftContrastOnBlack(stops[0] ?? rgb(0, 245, 255), minimumContrast)]
  }

  const out: RGB[] = []
  const segments = stops.length - 1

  for (let i = 0; i < steps; i++) {
    const t = i / (steps - 1)
    const pos = t * segments
    const segment = Math.min(segments - 1, Math.floor(pos))
    const localT = pos - segment
    const eased = smoothstep(localT)

    const mixed = mixRGB(stops[segment]!, stops[segment + 1]!, eased)
    out.push(liftContrastOnBlack(mixed, minimumContrast))
  }

  return out
}

const GLOBAL_AMBER_LUT = buildGradientLUT(JEKKO_AMBER_STOPS, GRADIENT_STEPS, 8.0)
const WORDMARK_AMBER_LUT = buildGradientLUT(WORDMARK_AMBER_STOPS, GRADIENT_STEPS, 8.8)
const GLOBAL_AMBER_LUT_LIGHT = buildGradientLUT(JEKKO_AMBER_LIGHT_STOPS, GRADIENT_STEPS, 0)
const WORDMARK_AMBER_LUT_LIGHT = buildGradientLUT(WORDMARK_AMBER_LIGHT_STOPS, GRADIENT_STEPS, 0)

export type LogoMode = "dark" | "light"

function colorFromLUT(lut: RGB[], t: number): RGB {
  const idx = Math.max(
    0,
    Math.min(lut.length - 1, Math.round(clamp01(t) * (lut.length - 1))),
  )

  return lut[idx]!
}

// ---------------------------------------------------------------------------
// Global and wordmark gradients
// ---------------------------------------------------------------------------

function globalDiagonalT(
  x: number,
  y: number,
  width: number,
  height: number,
): number {
  const tx = width <= 1 ? 0 : x / (width - 1)
  const ty = height <= 1 ? 0 : y / (height - 1)

  return clamp01(tx * 0.74 + ty * 0.26)
}

function globalGradientColor(
  x: number,
  y: number,
  width: number,
  height: number,
  mode: LogoMode,
  dim = false,
): RGB {
  const lut = mode === "light" ? GLOBAL_AMBER_LUT_LIGHT : GLOBAL_AMBER_LUT
  const base = colorFromLUT(lut, globalDiagonalT(x, y, width, height))
  return dim ? dimReadable(base, 0.24, 5.8) : base
}

function wordmarkT(
  sourceX: number,
  sourceY: number,
  width: number,
  height: number,
): number {
  const tx = width <= 1 ? 0 : clamp01(sourceX / (width - 1))
  const ty = height <= 1 ? 0 : clamp01(sourceY / (height - 1))

  // Mostly horizontal for readability, with a slight diagonal sweep so the
  // top-left and bottom-right do not look flat.
  return clamp01(tx * 0.84 + ty * 0.16)
}

function wordmarkColor(
  sourceX: number,
  sourceY: number,
  width: number,
  height: number,
  mode: LogoMode,
): RGB {
  const lut = mode === "light" ? WORDMARK_AMBER_LUT_LIGHT : WORDMARK_AMBER_LUT
  return colorFromLUT(lut, wordmarkT(sourceX, sourceY, width, height))
}

function wordmarkShadowColor(
  sourceX: number,
  sourceY: number,
  width: number,
  height: number,
  layer: "near" | "mid" | "far",
  mode: LogoMode,
): RGB {
  const baseT = wordmarkT(sourceX, sourceY, width, height)
  const shifted = clamp01(
    baseT + (layer === "far" ? 0.16 : layer === "mid" ? 0.11 : 0.06),
  )
  const lut = mode === "light" ? WORDMARK_AMBER_LUT_LIGHT : WORDMARK_AMBER_LUT
  const base = colorFromLUT(lut, shifted)

  // Shadow colors are dimmed aggressively (70-80% toward black) so the bright
  // face pixels are clearly distinguishable from the depth extrusion.
  if (layer === "far") return dimReadable(base, 0.85, 1.5)
  if (layer === "mid") return dimReadable(base, 0.80, 1.5)
  return dimReadable(base, 0.70, 2.0)
}

function resolveWordmarkSource(
  cell: LogoCell,
  x: number,
  y: number,
  totalWidth: number,
  totalRows: number,
): WordmarkSource | undefined {
  if (
    cell.layer !== "wordmark" &&
    cell.layer !== "wordmarkShadowNear" &&
    cell.layer !== "wordmarkShadowMid" &&
    cell.layer !== "wordmarkShadowFar"
  ) {
    return
  }

  return {
    layer: cell.layer,
    sourceX: cell.sourceX ?? x,
    sourceY: cell.sourceY ?? y,
    sourceWidth: cell.sourceWidth ?? totalWidth,
    sourceHeight: cell.sourceHeight ?? totalRows,
  }
}

// ---------------------------------------------------------------------------
// Text layout helpers
// ---------------------------------------------------------------------------

export function glyphLength(text: string): number {
  return Array.from(text).length
}

function padGlyphs(text: string, width: number): string {
  const remaining = Math.max(0, width - glyphLength(text))
  return text + " ".repeat(remaining)
}

function clipGlyphs(text: string, width: number): string {
  const chars = Array.from(text)
  return chars.length <= width ? text : chars.slice(0, width).join("")
}

function fit(text: string, width: number, align: Align = "center"): string {
  const clipped = clipGlyphs(text, width)
  const remaining = Math.max(0, width - glyphLength(clipped))

  if (align === "left") {
    return clipped + " ".repeat(remaining)
  }

  if (align === "right") {
    return " ".repeat(remaining) + clipped
  }

  const left = Math.floor(remaining / 2)
  const right = remaining - left

  return " ".repeat(left) + clipped + " ".repeat(right)
}

function pair(left: string, right: string, width = INNER_WIDTH): string {
  const safeLeft = clipGlyphs(left, width)
  const safeRight = clipGlyphs(right, width)
  const gap = width - glyphLength(safeLeft) - glyphLength(safeRight)

  if (gap < 1) {
    return fit(`${safeLeft} ${safeRight}`, width, "left")
  }

  return safeLeft + " ".repeat(gap) + safeRight
}

function framed(content = "", align: Align = "center"): string {
  return `│${fit(content, INNER_WIDTH, align)}│`
}

function framedPair(left: string, right: string): string {
  return `│${pair(left, right)}│`
}

function topBorder(): string {
  return `╭${"─".repeat(OUTER_WIDTH - 2)}╮`
}

function divider(): string {
  return `├${"─".repeat(OUTER_WIDTH - 2)}┤`
}

function bottomBorder(): string {
  return `╰${"─".repeat(OUTER_WIDTH - 2)}╯`
}

// ---------------------------------------------------------------------------
// Cell helpers
// ---------------------------------------------------------------------------

function globalCell(
  char: string,
  options: { strong?: boolean; dim?: boolean } = {},
): LogoCell {
  return {
    char,
    layer: "global",
    strong: options.strong,
    dim: options.dim,
  }
}

function emptyGlobalCell(): LogoCell {
  return globalCell(" ")
}

function textRow(
  text: string,
  options: { strong?: boolean; dim?: boolean } = {},
): LogoRow {
  return {
    strong: options.strong,
    dim: options.dim,
    cells: Array.from(text).map((char) => globalCell(char, options)),
  }
}

function frameCells(cells: LogoCell[]): LogoRow {
  const inner = cells.slice(0, INNER_WIDTH)

  while (inner.length < INNER_WIDTH) {
    inner.push(emptyGlobalCell())
  }

  return {
    cells: [globalCell("│"), ...inner, globalCell("│")],
  }
}

function isInk(char: string): boolean {
  return char !== " "
}

// ---------------------------------------------------------------------------
// Crisp half-block wordmarks
// ---------------------------------------------------------------------------
// A compact 5x7 pixel face rendered with Unicode half-blocks. Each terminal
// cell represents two pixel rows, so the wordmark gets square-ish pixels and
// a crisp arcade silhouette without the blobby stretched look.

type PixelFont = Record<string, string[]>

const PIXEL_FONT_5X7: PixelFont = {
  J: [
    "11111",
    "00010",
    "00010",
    "00010",
    "00010",
    "10010",
    "01100",
  ],
  E: [
    "11111",
    "10000",
    "10000",
    "11110",
    "10000",
    "10000",
    "11111",
  ],
  K: [
    "10001",
    "10010",
    "10100",
    "11000",
    "10100",
    "10010",
    "10001",
  ],
  O: [
    "01110",
    "10001",
    "10001",
    "10001",
    "10001",
    "10001",
    "01110",
  ],
  G: [
    "01110",
    "10001",
    "10000",
    "10111",
    "10001",
    "10001",
    "01110",
  ],
  " ": [
    "00000",
    "00000",
    "00000",
    "00000",
    "00000",
    "00000",
    "00000",
  ],
}

function renderPixelWord(
  text: string,
  options: { scaleX?: number; gap?: number } = {},
): string[] {
  const scaleX = options.scaleX ?? 1
  const gap = options.gap ?? 1
  const glyphs = Array.from(text.toUpperCase()).map(
    (letter) => PIXEL_FONT_5X7[letter] ?? PIXEL_FONT_5X7[" "]!,
  )
  const pixelHeight = Math.max(...glyphs.map((glyph) => glyph.length))
  const terminalRows = Math.ceil(pixelHeight / 2)
  const rows: string[] = []

  for (let tr = 0; tr < terminalRows; tr++) {
    const topRow = 2 * tr
    const botRow = 2 * tr + 1
    const pieces: string[] = []

    for (const glyph of glyphs) {
      const top = glyph[topRow] ?? ""
      const bot = glyph[botRow] ?? ""
      const cols = Math.max(top.length, bot.length, 5)
      let piece = ""

      for (let c = 0; c < cols; c++) {
        const t = top[c] === "1"
        const b = bot[c] === "1"
        const ch = t && b ? "█" : t ? "▀" : b ? "▄" : " "
        piece += ch.repeat(scaleX)
      }

      pieces.push(piece)
    }

    rows.push(pieces.join(" ".repeat(gap)))
  }

  return rows
}

const JEKKO_WORDMARK_CRISP = renderPixelWord("JEKKO", { scaleX: 2, gap: 2 })
const GO_WORDMARK_CRISP = renderPixelWord("GO", { scaleX: 2, gap: 2 })

type ArtMetrics = {
  lines: string[]
  artWidth: number
  artHeight: number
  gradientLeft: number
  gradientRight: number
  gradientWidth: number
  gradientHeight: number
}

function normalizeArt(art: string[]): string[] {
  const width = Math.max(...art.map(glyphLength))
  return art.map((line) => padGlyphs(line, width))
}

export function firstVisibleIndex(line: string): VisibleIndexResult {
  const chars = Array.from(line)
  const idx = chars.findIndex(isInk)
  return idx >= 0 ? { kind: "found", index: idx } : { kind: "missing" }
}

export function lastVisibleIndex(line: string): VisibleIndexResult {
  const chars = Array.from(line)

  for (let i = chars.length - 1; i >= 0; i--) {
    if (isInk(chars[i]!)) return { kind: "found", index: i }
  }

  return { kind: "missing" }
}

function artMetrics(art: string[]): ArtMetrics {
  const lines = normalizeArt(art)
  const artWidth = Math.max(...lines.map(glyphLength))
  const artHeight = lines.length

  const allVisibleXs: number[] = []

  for (const line of lines) {
    const chars = Array.from(line)

    chars.forEach((char, x) => {
      if (isInk(char)) allVisibleXs.push(x)
    })
  }

  const minVisibleX = allVisibleXs.length > 0 ? Math.min(...allVisibleXs) : 0
  const maxVisibleX = allVisibleXs.length > 0 ? Math.max(...allVisibleXs) : artWidth - 1

  return {
    lines,
    artWidth,
    artHeight,
    gradientLeft: minVisibleX,
    gradientRight: maxVisibleX,
    gradientWidth: Math.max(1, maxVisibleX - minVisibleX + 1),
    gradientHeight: Math.max(1, artHeight),
  }
}

const WORDMARK_SHADOW_LAYERS: Array<{
  dx: number
  dy: number
  layer: "wordmarkShadowNear" | "wordmarkShadowMid" | "wordmarkShadowFar"
}> = [
  { dx: 1, dy: 1, layer: "wordmarkShadowNear" },
  { dx: 2, dy: 1, layer: "wordmarkShadowMid" },
]

function maxShadowDx(): number {
  return WORDMARK_SHADOW_LAYERS.length === 0
    ? 0
    : Math.max(...WORDMARK_SHADOW_LAYERS.map((layer) => layer.dx))
}

function maxShadowDy(): number {
  return WORDMARK_SHADOW_LAYERS.length === 0
    ? 0
    : Math.max(...WORDMARK_SHADOW_LAYERS.map((layer) => layer.dy))
}

function putCell(
  canvas: LogoCell[][],
  x: number,
  y: number,
  cell: LogoCell,
): void {
  if (y < 0 || y >= canvas.length) return
  if (x < 0 || x >= canvas[y]!.length) return

  canvas[y]![x] = cell
}

function buildShadowedArtRows(
  art: string[],
  targetWidth: number,
  options: {
    framed?: boolean
    dim?: boolean
  } = {},
): LogoRow[] {
  const metrics = artMetrics(art)
  const shadowDx = maxShadowDx()
  const shadowDy = maxShadowDy()
  const visualWidth = metrics.artWidth + shadowDx
  const visualHeight = metrics.artHeight + shadowDy
  const left = Math.max(0, Math.floor((targetWidth - visualWidth) / 2))

  const canvas: LogoCell[][] = Array.from({ length: visualHeight }, () =>
    Array.from({ length: targetWidth }, emptyGlobalCell),
  )

  function makeArtCell(
    char: string,
    layer: CellLayer,
    x: number,
    y: number,
  ): LogoCell {
    return {
      char,
      layer,
      dim: options.dim,
      sourceX: x - metrics.gradientLeft,
      sourceY: y,
      sourceWidth: metrics.gradientWidth,
      sourceHeight: metrics.gradientHeight,
    }
  }

  // Draw extrusion back-to-front, then draw the crisp face last.
  const reversedLayers = [...WORDMARK_SHADOW_LAYERS].reverse()

  for (const shadow of reversedLayers) {
    for (let y = 0; y < metrics.lines.length; y++) {
      const chars = Array.from(metrics.lines[y]!)

      for (let x = 0; x < chars.length; x++) {
        const char = chars[x]!

        if (!isInk(char)) continue

        putCell(
          canvas,
          left + x + shadow.dx,
          y + shadow.dy,
          makeArtCell("█", shadow.layer, x, y),
        )
      }
    }
  }

  for (let y = 0; y < metrics.lines.length; y++) {
    const chars = Array.from(metrics.lines[y]!)

    for (let x = 0; x < chars.length; x++) {
      const char = chars[x]!

      if (!isInk(char)) continue

      putCell(
        canvas,
        left + x,
        y,
        makeArtCell(char, "wordmark", x, y),
      )
    }
  }

  const rows = canvas.map((cells) => ({ dim: options.dim, cells }))

  return options.framed ? rows.map((row) => frameCells(row.cells)) : rows
}

// ---------------------------------------------------------------------------
// Cell color selection
// ---------------------------------------------------------------------------

export function cellColor(
  cell: LogoCell,
  row: LogoRow,
  x: number,
  y: number,
  totalWidth: number,
  totalRows: number,
  mode: LogoMode = "dark",
): RGB {
  const dim = Boolean(cell.dim ?? row.dim)

  const wordmarkSource = resolveWordmarkSource(cell, x, y, totalWidth, totalRows)
  if (wordmarkSource) {
    if (wordmarkSource.layer === "wordmark") {
      const color = wordmarkColor(
        wordmarkSource.sourceX,
        wordmarkSource.sourceY,
        wordmarkSource.sourceWidth,
        wordmarkSource.sourceHeight,
        mode,
      )

      return dim ? dimReadable(color, 0.2, 6.2) : color
    }

    return wordmarkShadowColor(
      wordmarkSource.sourceX,
      wordmarkSource.sourceY,
      wordmarkSource.sourceWidth,
      wordmarkSource.sourceHeight,
      wordmarkSource.layer === "wordmarkShadowFar"
        ? "far"
        : wordmarkSource.layer === "wordmarkShadowMid"
          ? "mid"
          : "near",
      mode,
    )
  }

  return globalGradientColor(x, y, totalWidth, totalRows, mode, dim)
}

export function logoWidth(rows: LogoRow[], minimum = OUTER_WIDTH): number {
  return Math.max(minimum, ...rows.map((row) => row.cells.length))
}

// ---------------------------------------------------------------------------
// Logo builders
// ---------------------------------------------------------------------------

export function buildLogoRows(props: LogoProps = {}): LogoRow[] {
  const support = props.support ?? "ZYAL"

  const status =
    props.status ??
    (props.idle
      ? "camouflage idle • watching the wall"
      : "safe autonomous coding ready")

  const headerRight = props.idle
    ? "gecko mode idle   ● ● ●"
    : "gecko mode active ● ● ●"

  return [
    textRow(topBorder()),
    textRow(framedPair(" ›_ JEKKO", headerRight), { strong: true }),
    textRow(divider()),

    textRow(framed()),

    ...buildShadowedArtRows(JEKKO_WORDMARK_CRISP, INNER_WIDTH, {
      framed: true,
      dim: false,
    }),

    textRow(
      framed(`AI coding gecko • ${support} support • climbs hard problems`),
    ),
    textRow(framed(`gecko:// ${status}`), {
      dim: props.idle,
    }),

    textRow(bottomBorder()),
  ]
}

export function buildGoLogoRows(props: GoLogoProps = {}): LogoRow[] {
  return buildShadowedArtRows(GO_WORDMARK_CRISP, 26, {
    framed: false,
    dim: props.idle,
  })
}

// ---------------------------------------------------------------------------
// OpenTUI / Solid components
// ---------------------------------------------------------------------------

function GradientRow(props: {
  row: LogoRow
  y: number
  totalRows: number
  totalWidth: number
  mode: LogoMode
}) {
  return (
    <box flexDirection="row">
      <For each={props.row.cells}>
        {(cell: LogoCell, x: () => number) => {
          const strong = Boolean(cell.strong ?? props.row.strong)
          const attrs = strong ? 1 : undefined

          const color = cellColor(
            cell,
            props.row,
            x(),
            props.y,
            props.totalWidth,
            props.totalRows,
            props.mode,
          )

          return (
            <text
              fg={toRGBA(color)}
              attributes={attrs}
              selectable={false}
            >
              {cell.char}
            </text>
          )
        }}
      </For>
    </box>
  )
}

export function Logo(props: LogoProps = {}) {
  const { mode } = useTheme()
  const rows = createMemo(() => buildLogoRows(props))
  const totalWidth = createMemo(() => logoWidth(rows(), OUTER_WIDTH))

  return (
    <box flexDirection="column">
      <For each={rows()}>
        {(row: LogoRow, y: () => number) => (
          <GradientRow
            row={row}
            y={y()}
            totalRows={rows().length}
            totalWidth={totalWidth()}
            mode={mode() as LogoMode}
          />
        )}
      </For>
    </box>
  )
}

export function GoLogo(props: GoLogoProps = {}) {
  const { mode } = useTheme()
  const rows = createMemo(() => buildGoLogoRows(props))
  const totalWidth = createMemo(() => logoWidth(rows(), 0))

  return (
    <box flexDirection="column">
      <For each={rows()}>
        {(row: LogoRow, y: () => number) => (
          <GradientRow
            row={row}
            y={y()}
            totalRows={rows().length}
            totalWidth={totalWidth()}
            mode={mode() as LogoMode}
          />
        )}
      </For>
    </box>
  )
}

// ---------------------------------------------------------------------------
// Code-derived preview helpers
// ---------------------------------------------------------------------------
// These render from the same row builder and color engine as <Logo />. Use the
// ANSI output for terminal screenshots, or the SVG output for deterministic
// visual snapshots without involving a separate image generator.

function colorToAnsiFg(color: RGB): string {
  return `\x1b[38;2;${color.r};${color.g};${color.b}m`
}

export function getLogoPlainText(props: LogoProps = {}): string {
  return buildLogoRows(props)
    .map((row) => row.cells.map((cell) => cell.char).join(""))
    .join("\n")
}

export function getGoLogoPlainText(props: GoLogoProps = {}): string {
  return buildGoLogoRows(props)
    .map((row) => row.cells.map((cell) => cell.char).join(""))
    .join("\n")
}

export function renderLogoToAnsi(props: LogoProps = {}): string {
  const rows = buildLogoRows(props)
  const totalWidth = logoWidth(rows, OUTER_WIDTH)
  const totalRows = rows.length
  const reset = "\x1b[0m"
  const strong = "\x1b[1m"

  return rows
    .map((row, y) =>
      row.cells
        .map((cell, x) => {
          const color = cellColor(cell, row, x, y, totalWidth, totalRows)
          const attrs = Boolean(cell.strong ?? row.strong) ? strong : ""
          return `${attrs}${colorToAnsiFg(color)}${cell.char}${reset}`
        })
        .join(""),
    )
    .join("\n")
}

export function renderGoLogoToAnsi(props: GoLogoProps = {}): string {
  const rows = buildGoLogoRows(props)
  const totalWidth = logoWidth(rows, 0)
  const totalRows = rows.length
  const reset = "\x1b[0m"
  const strong = "\x1b[1m"

  return rows
    .map((row, y) =>
      row.cells
        .map((cell, x) => {
          const color = cellColor(cell, row, x, y, totalWidth, totalRows)
          const attrs = Boolean(cell.strong ?? row.strong) ? strong : ""
          return `${attrs}${colorToAnsiFg(color)}${cell.char}${reset}`
        })
        .join(""),
    )
    .join("\n")
}

function escapeXml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
}

function renderSvgCell(
  cell: LogoCell,
  row: LogoRow,
  x: number,
  y: number,
  totalWidth: number,
  totalRows: number,
  opts: Required<SvgPreviewOptions>,
): string {
  if (cell.char === " ") return ""

  const color = colorToCss(cellColor(cell, row, x, y, totalWidth, totalRows))
  const px = opts.paddingX + x * opts.cellWidth
  const py = opts.paddingY + y * opts.cellHeight
  const halfH = opts.cellHeight / 2

  if (cell.char === "█") {
    return `<rect x="${px}" y="${py}" width="${opts.cellWidth}" height="${opts.cellHeight}" fill="${color}" shape-rendering="crispEdges"/>`
  }

  if (cell.char === "▀") {
    return `<rect x="${px}" y="${py}" width="${opts.cellWidth}" height="${halfH}" fill="${color}" shape-rendering="crispEdges"/>`
  }

  if (cell.char === "▄") {
    return `<rect x="${px}" y="${py + halfH}" width="${opts.cellWidth}" height="${halfH}" fill="${color}" shape-rendering="crispEdges"/>`
  }

  const weight = Boolean(cell.strong ?? row.strong) ? 800 : 500
  return `<text x="${px}" y="${py + opts.baseline}" font-family="JetBrains Mono, DejaVu Sans Mono, Menlo, Consolas, monospace" font-size="${opts.fontSize}" font-weight="${weight}" fill="${color}" xml:space="preserve">${escapeXml(cell.char)}</text>`
}

export function renderLogoToSvg(
  props: LogoProps = { idle: false },
  options: SvgPreviewOptions = {},
): string {
  const opts: Required<SvgPreviewOptions> = {
    cellWidth: options.cellWidth ?? 7,
    cellHeight: options.cellHeight ?? 14,
    fontSize: options.fontSize ?? 11,
    baseline: options.baseline ?? 11,
    paddingX: options.paddingX ?? 8,
    paddingY: options.paddingY ?? 7,
    background: options.background ?? "#050607",
    title: options.title ?? "JEKKO logo preview",
  }

  const rows = buildLogoRows(props)
  const totalWidth = logoWidth(rows, OUTER_WIDTH)
  const totalRows = rows.length
  const width = opts.paddingX * 2 + totalWidth * opts.cellWidth
  const height = opts.paddingY * 2 + totalRows * opts.cellHeight

  const cells: string[] = []

  rows.forEach((row, y) => {
    row.cells.forEach((cell, x) => {
      const rendered = renderSvgCell(
        cell,
        row,
        x,
        y,
        totalWidth,
        totalRows,
        opts,
      )

      if (rendered) cells.push(rendered)
    })
  })

  return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
  <title>${escapeXml(opts.title)}</title>
  <rect width="100%" height="100%" fill="${opts.background}"/>
  <g text-rendering="geometricPrecision">
    ${cells.join("\n    ")}
  </g>
</svg>
`
}
