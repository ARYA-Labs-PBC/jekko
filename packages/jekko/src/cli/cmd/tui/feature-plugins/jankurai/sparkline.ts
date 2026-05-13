// Pure unicode-block sparkline renderer. Used by the audit-live panel to show
// score history in a single line of width N. Width-clip from the tail so the
// most-recent samples always appear; resolution maps (max - min) onto eight
// glyphs.

const GLYPHS = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"] as const
const PLACEHOLDER = "·"

export function sparkline(values: readonly number[], width: number): string {
  if (width <= 0) return ""
  if (values.length === 0) return PLACEHOLDER.repeat(width)
  const tail = values.length > width ? values.slice(values.length - width) : values
  // Min / max over finite values only — otherwise a NaN poisons the bounds
  // and every glyph collapses to a placeholder.
  const finite = tail.filter((v) => Number.isFinite(v))
  if (finite.length === 0) {
    return PLACEHOLDER.repeat(width)
  }
  const min = Math.min(...finite)
  const max = Math.max(...finite)
  const span = max - min
  const glyphsForTail = tail.map((value) => {
    if (!Number.isFinite(value)) return PLACEHOLDER
    if (span === 0) return GLYPHS[Math.floor(GLYPHS.length / 2)]
    const idx = Math.min(
      GLYPHS.length - 1,
      Math.max(0, Math.floor(((value - min) / span) * (GLYPHS.length - 1) + 0.5)),
    )
    return GLYPHS[idx]
  })
  if (glyphsForTail.length >= width) return glyphsForTail.join("")
  // Left-pad with placeholder so the sparkline lines up at the right edge.
  return PLACEHOLDER.repeat(width - glyphsForTail.length) + glyphsForTail.join("")
}

export const SPARKLINE_GLYPHS = GLYPHS
export const SPARKLINE_PLACEHOLDER = PLACEHOLDER
