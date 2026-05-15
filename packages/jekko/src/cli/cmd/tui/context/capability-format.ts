/**
 * Render an epoch-ms timestamp as "Xs ago" / "Xm ago" / "Xh ago" / "Xd ago".
 * Returns "just now" for ages under a minute and "—" when `generatedAt` is
 * undefined.
 */
export function formatCapabilityAge(generatedAtMs: number | undefined, nowMs: number): string {
  if (!generatedAtMs) return "—"
  const ageSec = Math.max(0, Math.floor((nowMs - generatedAtMs) / 1000))
  if (ageSec < 60) return "just now"
  if (ageSec < 3600) return `${Math.floor(ageSec / 60)}m ago`
  if (ageSec < 86400) return `${Math.floor(ageSec / 3600)}h ago`
  return `${Math.floor(ageSec / 86400)}d ago`
}

/**
 * Strip the "HL" prefix from a conformance level (e.g. "HL3" -> "L3"). When
 * the input doesn't start with "HL" the original value is returned
 * unchanged. Empty/missing input falls back to "—".
 */
export function formatConformanceLevel(level: string): string {
  if (!level) return "—"
  if (level.startsWith("HL")) return "L" + level.slice(2)
  return level
}
