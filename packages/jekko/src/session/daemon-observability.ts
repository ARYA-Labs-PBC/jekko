// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
import type { ZyalObservability } from "@/agent-script/schema"

/**
 * Observability engine for ZYAL v2.
 *
 * Collects spans, metrics, and cost data. Generates structured reports
 * for daemon runs with budget enforcement.
 */

export type Span = {
  readonly id: string
  readonly name: string
  readonly startedAt: number
  readonly endedAt?: number
  readonly status: "running" | "ok" | "error"
  readonly metadata?: Record<string, unknown>
  readonly parentId?: string
}

export type MetricValue = {
  readonly name: string
  readonly type: "counter" | "gauge" | "histogram"
  readonly value: number
  readonly timestamp: number
  readonly labels?: Record<string, string>
}

export type CostEntry = {
  readonly source: string
  readonly amount: number
  readonly currency: string
  readonly timestamp: number
}

export type ObservabilityState = {
  readonly spans: Span[]
  readonly metrics: MetricValue[]
  readonly costs: CostEntry[]
  readonly totalCost: number
  readonly budgetRemaining: number | null
  readonly budgetExceeded: boolean
}

/**
 * Initialize observability state.
 */
export function initializeObservability(config: ZyalObservability | undefined): ObservabilityState {
  return {
    spans: [],
    metrics: [],
    costs: [],
    totalCost: 0,
    budgetRemaining: config?.cost?.budget ?? null,
    budgetExceeded: false,
  }
}

/**
 * Start a new span.
 */
export function startSpan(
  state: ObservabilityState,
  name: string,
  parentId?: string,
): { state: ObservabilityState; spanId: string } {
  const id = `span_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`
  const span: Span = {
    id,
    name,
    startedAt: Date.now(),
    status: "running",
    parentId,
  }
  return { state: { ...state, spans: [...state.spans, span] }, spanId: id }
}

/**
 * End a span with a status.
 */
export function endSpan(
  state: ObservabilityState,
  spanId: string,
  status: "ok" | "error",
  metadata?: Record<string, unknown>,
): ObservabilityState {
  const spans = state.spans.map((s) =>
    s.id === spanId ? { ...s, endedAt: Date.now(), status, metadata: { ...s.metadata, ...metadata } } : s,
  )
  return { ...state, spans }
}

/**
 * Record a metric value.
 */
export function recordMetric(
  state: ObservabilityState,
  name: string,
  type: MetricValue["type"],
  value: number,
  labels?: Record<string, string>,
): ObservabilityState {
  const metric: MetricValue = { name, type, value, timestamp: Date.now(), labels }
  return { ...state, metrics: [...state.metrics, metric] }
}

/**
 * Increment a counter metric.
 */
export function incrementCounter(
  state: ObservabilityState,
  name: string,
  delta: number = 1,
): ObservabilityState {
  return recordMetric(state, name, "counter", delta)
}

/**
 * Set a gauge value.
 */
export function setGauge(
  state: ObservabilityState,
  name: string,
  value: number,
): ObservabilityState {
  return recordMetric(state, name, "gauge", value)
}

/**
 * Record a cost entry.
 */
export function recordCost(
  state: ObservabilityState,
  source: string,
  amount: number,
  currency: string = "USD",
): ObservabilityState {
  const entry: CostEntry = { source, amount, currency, timestamp: Date.now() }
  const totalCost = state.totalCost + amount
  const budgetRemaining = state.budgetRemaining !== null ? state.budgetRemaining - amount : null
  const budgetExceeded = budgetRemaining !== null && budgetRemaining <= 0
  return {
    ...state,
    costs: [...state.costs, entry],
    totalCost,
    budgetRemaining,
    budgetExceeded,
  }
}

/**
 * Check if a budget alert threshold has been reached.
 */
export function checkBudgetAlert(
  state: ObservabilityState,
  config: ZyalObservability | undefined,
): { alert: boolean; percent: number; action: string } {
  if (!config?.cost?.budget || !config.cost.alert_at_percent) {
    return { alert: false, percent: 0, action: "none" }
  }
  const percent = (state.totalCost / config.cost.budget) * 100
  if (percent >= config.cost.alert_at_percent) {
    return { alert: true, percent, action: config.cost.on_budget_exceeded ?? "warn" }
  }
  return { alert: false, percent, action: "none" }
}

/**
 * Filter spans based on emit config.
 */
export function filterSpans(
  state: ObservabilityState,
  config: ZyalObservability | undefined,
): Span[] {
  if (!config?.spans?.emit || config.spans.emit === "all") {
    return state.spans
  }
  if (config.spans.emit === "none") return []
  if (config.spans.emit === "errors_only") {
    return state.spans.filter((s) => s.status === "error")
  }
  return state.spans
}

/**
 * Get the latest value for a named metric.
 */
export function getLatestMetric(state: ObservabilityState, name: string): MetricValue | undefined {
  for (let i = state.metrics.length - 1; i >= 0; i--) {
    if (state.metrics[i].name === name) return state.metrics[i]
  }
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  return undefined
}

/**
 * Get aggregated counter value.
 */
export function getCounterTotal(state: ObservabilityState, name: string): number {
  return state.metrics
    .filter((m) => m.name === name && m.type === "counter")
    .reduce((sum, m) => sum + m.value, 0)
}

/**
 * Generate a structured report.
 */
export function generateReport(
  state: ObservabilityState,
  config: ZyalObservability | undefined,
): Record<string, unknown> {
  const report: Record<string, unknown> = {
    generated_at: new Date().toISOString(),
    format: config?.report?.format ?? "json",
  }

  const include = new Set(config?.report?.include ?? ["spans", "metrics", "costs", "summary"])

  if (include.has("spans")) {
    report.spans = {
      total: state.spans.length,
      errors: state.spans.filter((s) => s.status === "error").length,
      running: state.spans.filter((s) => s.status === "running").length,
    }
  }

  if (include.has("metrics")) {
    const grouped: Record<string, MetricValue[]> = {}
    for (const m of state.metrics) {
      if (!grouped[m.name]) grouped[m.name] = []
      grouped[m.name].push(m)
    }
    report.metrics = Object.entries(grouped).map(([name, values]) => ({
      name,
      type: values[0].type,
      count: values.length,
      latest: values[values.length - 1].value,
    }))
  }

  if (include.has("costs")) {
    report.costs = {
      total: state.totalCost,
      budget: config?.cost?.budget,
      remaining: state.budgetRemaining,
      exceeded: state.budgetExceeded,
      entries: state.costs.length,
    }
  }

  if (include.has("summary")) {
    report.summary = {
      span_count: state.spans.length,
      metric_count: state.metrics.length,
      cost_entries: state.costs.length,
      total_cost: state.totalCost,
      budget_exceeded: state.budgetExceeded,
    }
  }

  return report
}
