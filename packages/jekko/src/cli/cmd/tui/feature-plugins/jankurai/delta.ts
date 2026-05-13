// Delta-vs-baseline math + glyph chooser for the audit-live panel. The Δ
// columns in the panel render `value (+/-N glyph)` and `glyph` depends on
// whether higher-or-lower is better for the metric.

export type DeltaDirection = "improving" | "worsening" | "neutral" | "unknown"
export type DeltaMetric = "score" | "findings" | "caps" | "hard" | "soft" | "level"

export type DeltaResult = {
  /** Current minus baseline. `undefined` when either side is missing. */
  delta: number | undefined
  direction: DeltaDirection
  /** Single glyph the panel paints next to the delta number. */
  glyph: string
}

export function delta(current: number | undefined, baseline: number | undefined, metric: DeltaMetric): DeltaResult {
  if (current === undefined || baseline === undefined) {
    return { delta: undefined, direction: "unknown", glyph: "—" }
  }
  const diff = current - baseline
  if (diff === 0) {
    return { delta: 0, direction: "neutral", glyph: "=" }
  }
  // For score, higher is better. For finding counters, lower is better.
  const lowerIsBetter = metric !== "score" && metric !== "level"
  const improving = lowerIsBetter ? diff < 0 : diff > 0
  const direction: DeltaDirection = improving ? "improving" : "worsening"
  const magnitude = Math.abs(diff)
  // Double-arrow glyph above a large delta — purely cosmetic; the renderer
  // doesn't care about magnitude, but the panel uses ≥10 (score) or ≥10
  // (finding count) as the threshold.
  const big = magnitude >= 10
  const glyph = improving ? (big ? "▲▲" : "▲") : big ? "▼▼" : "▼"
  return { delta: diff, direction, glyph }
}

/** Format the delta for the panel as `±N glyph`. */
export function formatDelta(result: DeltaResult): string {
  if (result.delta === undefined) return "—"
  if (result.delta === 0) return "= 0"
  const sign = result.delta > 0 ? "+" : ""
  return `${sign}${result.delta} ${result.glyph}`
}
