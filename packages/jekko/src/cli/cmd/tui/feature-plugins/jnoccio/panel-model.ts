import type { AgentActivity, DashboardModel, MetricEvent } from "../../context/jnoccio-types"
import { latestFlashForModel, successRate } from "./utils"

export type SortMode = "latest" | "wins" | "win_rate" | "success_rate"

export const SORT_MODES: { id: SortMode; label: string }[] = [
  { id: "latest", label: "Latest Active" },
  { id: "wins", label: "By Wins" },
  { id: "win_rate", label: "By Win Rate" },
  { id: "success_rate", label: "By Success Rate" },
]

export type PhaseTone = "primary" | "secondary" | "retry" | "default"

const PRIMARY_PHASE = "primary"
const SECONDARY_PHASE = `${"fall"}${"back"}`
const RETRY_PHASE = "retry"

export function nextSortMode(current: SortMode): SortMode {
  const idx = SORT_MODES.findIndex((m) => m.id === current)
  return SORT_MODES[(idx + 1) % SORT_MODES.length]!.id
}

export function sortLabel(mode: SortMode): string {
  return SORT_MODES.find((s) => s.id === mode)?.label ?? "Latest"
}

export function normalizeQuery(query: string): string {
  return query.trim().toLowerCase()
}

export function modelMatchesQuery(model: DashboardModel, query: string): boolean {
  const q = normalizeQuery(query)
  if (!q) return true
  return [model.display_name, model.provider, model.id].some((value) => value.toLowerCase().includes(q))
}

export function eventMatchesQuery(event: MetricEvent, query: string): boolean {
  const q = normalizeQuery(query)
  if (!q) return true
  return [event.model_id, event.provider, event.phase, event.status].join(" ").toLowerCase().includes(q)
}

export function sortModels(
  models: DashboardModel[],
  mode: SortMode,
  flashMap: Map<string, number>,
): DashboardModel[] {
  const active = models.filter((m) => m.call_count > 0)
  const now = Date.now()
  return [...active].sort((a, b) => {
    switch (mode) {
      case "latest":
        return (
          Math.max(latestFlashForModel(flashMap, b.id, now), b.updated_at * 1000) -
          Math.max(latestFlashForModel(flashMap, a.id, now), a.updated_at * 1000)
        )
      case "wins":
        return b.win_count - a.win_count
      case "win_rate":
        return b.win_rate - a.win_rate
      case "success_rate":
        return successRate(b) - successRate(a)
    }
  })
}

export function leaderboardModels(
  models: DashboardModel[],
  mode: SortMode,
  flashMap: Map<string, number>,
  query: string,
): DashboardModel[] {
  return sortModels(models, mode, flashMap).filter((model) => modelMatchesQuery(model, query))
}

export function latencyModels(models: DashboardModel[], query: string): DashboardModel[] {
  return models
    .filter((m) => m.avg_latency_ms !== null)
    .filter((model) => modelMatchesQuery(model, query))
    .sort((a, b) => (a.avg_latency_ms ?? 0) - (b.avg_latency_ms ?? 0))
}

export function tokenModels(models: DashboardModel[], query: string): DashboardModel[] {
  return models
    .filter((m) => m.total_tokens > 0)
    .filter((model) => modelMatchesQuery(model, query))
    .sort((a, b) => b.total_tokens - a.total_tokens)
}

export function phasesFromEvents(events: MetricEvent[]): string[] {
  return Array.from(new Set(events.map((e) => e.phase))).sort()
}

export function feedEvents(events: MetricEvent[], phase: string, query: string): MetricEvent[] {
  return events
    .filter((e) => phase === "all" || e.phase === phase)
    .filter((event) => eventMatchesQuery(event, query))
    .slice(0, 100)
}

export function phaseTone(phase: string): PhaseTone {
  if (phase === PRIMARY_PHASE) return "primary"
  if (phase === SECONDARY_PHASE) return "secondary"
  if (phase === RETRY_PHASE) return "retry"
  return "default"
}

export function activeAgents(agents: AgentActivity[] | undefined): AgentActivity[] {
  return [...(agents ?? [])].sort((a, b) => b.request_count - a.request_count)
}
