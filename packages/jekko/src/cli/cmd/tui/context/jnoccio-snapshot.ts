import type { DashboardModel, DashboardSnapshot, DashboardTotals } from "./jnoccio-types"

/** Default empty snapshot for initial state. */
export function emptySnapshot(): DashboardSnapshot {
  return {
    totals: {
      total_models: 0,
      enabled_models: 0,
      calls: 0,
      successes: 0,
      failures: 0,
      wins: 0,
      prompt_tokens: 0,
      completion_tokens: 0,
      total_tokens: 0,
      average_latency_ms: null,
    },
    token_rate: {
      median_m_tokens_per_24h: 0,
      max_m_tokens_per_24h: 0,
      sample_minutes: 0,
      window_minutes: 1440,
      smoothing_minutes: 10,
    },
    capacity: {
      known_limit_per_hour: 0,
      known_used: 0,
      known_remaining: 0,
      percent_used: 0,
      models: [],
      unknown_models: [],
    },
    context: {
      estimates: [],
      histogram: [],
      recent_events: [],
    },
    models: [],
    recent_events: [],
    agent_count: 0,
    max_agents: 20,
    active_agents: [],
    instance_count: 1,
    max_instances: 20,
    available_instance_slots: 19,
    instance_role: "main",
    worker_threads: 0,
  }
}

/** Normalize a snapshot with bounded defaults for missing or non-finite fields. */
export function normalizeSnapshot(snapshot: DashboardSnapshot): DashboardSnapshot {
  if (!snapshot.max_agents) snapshot.max_agents = 20
  if (!snapshot.max_instances) snapshot.max_instances = 20
  if (!snapshot.instance_count) snapshot.instance_count = 1
  if (!snapshot.instance_role) snapshot.instance_role = "main"
  if (!snapshot.agent_count && snapshot.active_agents?.length) {
    snapshot.agent_count = snapshot.active_agents.length
  }
  if (!snapshot.available_instance_slots && snapshot.max_instances > snapshot.instance_count) {
    snapshot.available_instance_slots = snapshot.max_instances - snapshot.instance_count
  }

  const t = snapshot.totals ?? ({} as Partial<DashboardTotals>)
  snapshot.totals = {
    total_models: finiteNumberOr(t.total_models, 0),
    enabled_models: finiteNumberOr(t.enabled_models, 0),
    calls: finiteNumberOr(t.calls, 0),
    successes: finiteNumberOr(t.successes, 0),
    failures: finiteNumberOr(t.failures, 0),
    wins: finiteNumberOr(t.wins, 0),
    prompt_tokens: finiteNumberOr(t.prompt_tokens, 0),
    completion_tokens: finiteNumberOr(t.completion_tokens, 0),
    total_tokens: finiteNumberOr(t.total_tokens, 0),
    average_latency_ms: finiteNumberOrNull(t.average_latency_ms),
  }

  if (snapshot.capacity) {
    snapshot.capacity.percent_used = finiteNumberOr(snapshot.capacity.percent_used, 0)
  }

  if (snapshot.token_rate) {
    snapshot.token_rate.median_m_tokens_per_24h = finiteNumberOr(snapshot.token_rate.median_m_tokens_per_24h, 0)
  }

  const seen = new Set<number>()
  snapshot.recent_events = (snapshot.recent_events ?? []).filter((event) => {
    if (seen.has(event.id)) return false
    seen.add(event.id)
    return true
  })
  snapshot.recent_events.sort((a, b) => b.created_at - a.created_at || b.id - a.id)
  snapshot.recent_events = snapshot.recent_events.slice(0, 300)

  return snapshot
}

function finiteNumberOr(value: number | null | undefined, defaultValue: number): number {
  return Number.isFinite(value) ? (value as number) : defaultValue
}

function finiteNumberOrNull(value: number | null | undefined): number | null {
  return Number.isFinite(value) ? (value as number) : null
}

/** Recompute totals from model rows. */
export function totalsFromModels(models: DashboardModel[]): DashboardTotals {
  let latencyTotal = 0
  let latencyCount = 0

  for (const model of models) {
    if (model.avg_latency_ms != null) {
      const weight = Math.max(model.call_count, 1)
      latencyTotal += model.avg_latency_ms * weight
      latencyCount += weight
    }
  }

  return {
    total_models: models.length,
    enabled_models: models.filter((model) => model.enabled).length,
    calls: models.reduce((sum, model) => sum + model.call_count, 0),
    successes: models.reduce((sum, model) => sum + model.success_count, 0),
    failures: models.reduce((sum, model) => sum + model.failure_count, 0),
    wins: models.reduce((sum, model) => sum + model.win_count, 0),
    prompt_tokens: models.reduce((sum, model) => sum + model.prompt_tokens, 0),
    completion_tokens: models.reduce((sum, model) => sum + model.completion_tokens, 0),
    total_tokens: models.reduce((sum, model) => sum + model.total_tokens, 0),
    average_latency_ms: latencyCount === 0 ? null : latencyTotal / latencyCount,
  }
}
