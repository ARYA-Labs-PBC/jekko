/**
 * Jnoccio TUI formatting and display utilities.
 *
 * Formatting and model-display helpers for the TUI-native dashboard.
 */
import type { DashboardModel } from "../../context/jnoccio-types"

// ── Number Formatting ─────────────────────────────────────────────────

/** Compact number: 1234 → "1.2K", 1234567 → "1.2M" */
export function fmtN(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n)) return "—"
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B`
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 10_000) return `${(n / 1_000).toFixed(1)}K`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return n.toLocaleString()
}

/** Millions formatter: 1.23 → "1.2" */
export function fmtM(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n)) return "0.0"
  return n.toFixed(1)
}

/** Latency: null → "—", 123 → "123ms", 1234 → "1.2s", 15400 → "15.4s" */
export function fmtMs(ms: number | null | undefined): string {
  if (ms == null || !Number.isFinite(ms)) return "—"
  if (ms < 1000) return `${Math.round(ms)}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

/** Percentage: 0.85 → "85%", 0.854 → "85.4%" */
export function fmtPct(ratio: number | null | undefined): string {
  if (ratio == null || !Number.isFinite(ratio)) return "—"
  const pct = ratio * 100
  if (pct === 0) return "0%"
  if (pct >= 10) return `${Math.round(pct)}%`
  return `${pct.toFixed(1)}%`
}

/** Relative time: seconds ago → "3s ago", "2m ago", "1h ago" */
export function fmtTime(epochSecs: number): string {
  const now = Date.now() / 1000
  const diff = Math.max(0, now - epochSecs)
  if (diff < 60) return `${Math.floor(diff)}s ago`
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`
  return `${Math.floor(diff / 86400)}d ago`
}

/** Truncate text: "Hello world foo" → "Hello wor…" */
export function truncate(s: string, max: number): string {
  if (s.length <= max) return s
  return s.slice(0, max - 1) + "…"
}

// ── Model Metrics ─────────────────────────────────────────────────────

/** Success rate as a 0-1 ratio. */
export function successRate(m: DashboardModel): number {
  if (m.call_count === 0) return 0
  return m.success_count / m.call_count
}

/** Max configured context window across models. */
export function maxConfigured(models: DashboardModel[]): number {
  return Math.max(0, ...models.map((m) => m.configured_context_window))
}

// ── Agent Color ────────────────────────────────────────────────────────

/** Deterministic color from an agent ID string. Returns a hex color. */
const AGENT_PALETTE = [
  "#FF6B6B", "#4ECDC4", "#45B7D1", "#F7DC6F", "#BB8FCE",
  "#82E0AA", "#F1948A", "#85C1E9", "#F8C471", "#73C6B6",
  "#D7BDE2", "#A3E4D7", "#FAD7A0", "#AED6F1", "#F5B7B1",
]

export function agentColor(agentId: string | null | undefined): string {
  if (!agentId) return "#888"
  let hash = 0
  for (let i = 0; i < agentId.length; i++) {
    hash = ((hash << 5) - hash + agentId.charCodeAt(i)) | 0
  }
  return AGENT_PALETTE[Math.abs(hash) % AGENT_PALETTE.length]!
}

// ── Flash Map ──────────────────────────────────────────────────────────

const FLASH_DURATION_MS = 3_000

export function flashKey(modelId: string, agentId?: string | null): string {
  return agentId ? `${modelId}::${agentId}` : modelId
}

export type FlashEntry = { agentId: string; ts: number }

/**
 * Returns active flash entries for a model.
 * Each entry has { agentId, ts } where ts is the flash timestamp.
 */
export function flashesForModel(
  flashMap: Map<string, number>,
  modelId: string,
  now: number,
): FlashEntry[] {
  const result: FlashEntry[] = []
  for (const [key, ts] of flashMap) {
    if (now - ts > FLASH_DURATION_MS) continue
    if (key === modelId) {
      result.push({ agentId: "", ts })
    } else if (key.startsWith(`${modelId}::`)) {
      result.push({ agentId: key.slice(modelId.length + 2), ts })
    }
  }
  return result
}

/** Latest flash timestamp for a model, or 0 if none active. */
export function latestFlashForModel(
  flashMap: Map<string, number>,
  modelId: string,
  now: number,
): number {
  const flashes = flashesForModel(flashMap, modelId, now)
  if (flashes.length === 0) return 0
  return Math.max(...flashes.map((f) => f.ts))
}

// ── Text Bar Rendering ────────────────────────────────────────────────

/** Create a text-based bar: "████░░░░░░" */
export function textBar(value: number, max: number, width: number): string {
  if (max <= 0) return "░".repeat(width)
  const filled = Math.max(0, Math.min(width, Math.round((value / max) * width)))
  return "█".repeat(filled) + "░".repeat(width - filled)
}

// ── Latency Tier ──────────────────────────────────────────────────────

export type LatencyTier = "fast" | "normal" | "slow" | "critical"

export function latencyTier(ms: number | null | undefined): LatencyTier {
  if (ms == null) return "normal"
  if (ms < 1000) return "fast"
  if (ms < 5000) return "normal"
  if (ms < 15000) return "slow"
  return "critical"
}

export function latencyIcon(tier: LatencyTier): string {
  switch (tier) {
    case "fast":
      return "⚡"
    case "normal":
      return "●"
    case "slow":
      return "▲"
    case "critical":
      return "✖"
  }
}

// ── Status Colors ──────────────────────────────────────────────────────

/** Status indicator character: ready→●, cooldown→○, error→✖ */
export function statusDot(status: string): string {
  if (status === "ready" || status === "active") return "●"
  if (status === "cooldown") return "○"
  if (status === "disabled") return "◌"
  if (status === "error" || status === "failed") return "✖"
  return "·"
}

// ── Connection Label ──────────────────────────────────────────────────

export function connLabel(
  connection: string,
  lastHeartbeat: number | null,
): string {
  if (connection === "live") {
    if (lastHeartbeat) {
      const ago = Math.round((Date.now() / 1000 - lastHeartbeat))
      if (ago < 120) return `Live · ${ago}s`
    }
    return "Live"
  }
  if (connection === "connecting") return "Connecting…"
  if (connection === "reconnecting") return "Reconnecting…"
  if (connection === "error") return "Error"
  return "Loading…"
}
