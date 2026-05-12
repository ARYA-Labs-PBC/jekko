import { updateZyalMetrics, useZyalMetrics, zyalFleetHardCap } from "./zyal-flash-state"

function numberFrom(...values: unknown[]): number | undefined {
  for (const value of values) {
    if (value === undefined || value === null || value === "") continue
    const next = Number(value)
    if (Number.isFinite(next)) return next
  }
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  return undefined
}

function daemonTokens(run: Record<string, any>) {
  const stats = (run._stats ?? {}) as Record<string, any>
  const tokens = (run.token_usage ?? run.tokens ?? {}) as Record<string, any>
  const cache = tokens.cache ?? {}
  const input = numberFrom(stats.input_tokens, stats.inputTokens, tokens.input, tokens.inputTokens) ?? 0
  const output = numberFrom(stats.output_tokens, stats.outputTokens, tokens.output, tokens.outputTokens) ?? 0
  const cacheTokens =
    numberFrom(stats.cache_tokens, stats.cacheTokens) ??
    (numberFrom(cache.read) ?? 0) + (numberFrom(cache.write) ?? 0) + (numberFrom(tokens.reasoning) ?? 0)
  const total = numberFrom(stats.total_tokens, stats.totalTokens, tokens.total, tokens.totalTokens) ?? input + output + cacheTokens
  return { input, output, cache: cacheTokens, total }
}

export function zyalDaemonFleet(run: Record<string, any> | undefined | null): Record<string, any> | undefined {
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!run) return undefined
  return run.spec_json?.fleet ?? run.spec?.fleet ?? run.fleet
}

export function daemonRunToZyalMetrics(
  run: Record<string, any>,
  sessionIdIfMissing?: string,
): Partial<import("./zyal-flash-state").ZyalFleetMetrics> {
  const stats = (run._stats ?? {}) as Record<string, any>
  const fleet = zyalDaemonFleet(run) ?? {}
  const fleetMaxRaw = numberFrom(fleet.max_workers)
  const tokens = daemonTokens(run)
  return {
    runId: String(run.id ?? run.run_id ?? sessionIdIfMissing ?? ""),
    status: String(run.status ?? "active"),
    workersActive: numberFrom(stats.active_workers, stats.workers_active, run.workers_active, run.active_workers) ?? 0,
    workersMax: fleetMaxRaw && fleetMaxRaw > 0 ? Math.min(zyalFleetHardCap(), fleetMaxRaw) : undefined,
    inputTokens: tokens.input,
    outputTokens: tokens.output,
    cacheTokens: tokens.cache,
    totalTokens: tokens.total,
    costUsd: numberFrom(stats.cost_usd, stats.total_cost, run.cost_usd, run.cost) ?? 0,
    loopsCompleted: numberFrom(stats.iteration_count, run.iteration_count, run.iterations, run.iteration) ?? 0,
    tasksCompleted: numberFrom(stats.completed_tasks, stats.tasks_completed, run.tasks_completed) ?? 0,
    tasksIncubated: numberFrom(stats.incubated_tasks, stats.tasks_incubated, run.tasks_incubated) ?? 0,
    jankuraiFindings: numberFrom(stats.jankurai_findings, run.jankurai_findings) ?? undefined,
  }
}

export function daemonRunJnoccioConfig(run: Record<string, any> | undefined | null):
  | { baseUrl: string; metricsWsPath?: string; runId?: string }
  | undefined {
  const fleet = zyalDaemonFleet(run)
  const jnoccio = fleet?.jnoccio
  const baseUrl = typeof jnoccio?.base_url === "string" ? jnoccio.base_url.trim() : ""
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!run || !baseUrl || jnoccio?.enabled === false) return undefined
  return {
    baseUrl,
    metricsWsPath: typeof jnoccio?.metrics_ws === "string" ? jnoccio.metrics_ws : undefined,
    runId: String(run.id ?? run.run_id ?? ""),
  }
}

/**
 * Compute the elapsed wall-clock running time as a "Hh Mm Ss" string.
 * Returns null when no run is active.
 */
export function formatZyalRuntime(now: number = Date.now()): string | null {
  const m = useZyalMetrics()()
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!m.startedAt) return null
  const elapsedMs = Math.max(0, now - m.startedAt)
  const totalSec = Math.floor(elapsedMs / 1000)
  const h = Math.floor(totalSec / 3600)
  const min = Math.floor((totalSec % 3600) / 60)
  const sec = totalSec % 60
  if (h > 0) return `${h}h ${min}m ${sec}s`
  if (min > 0) return `${min}m ${sec}s`
  return `${sec}s`
}
