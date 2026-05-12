import { describe, test, expect } from "bun:test"
import {
  initializeObservability,
  startSpan,
  endSpan,
  recordMetric,
  incrementCounter,
  setGauge,
  recordCost,
  checkBudgetAlert,
  filterSpans,
  getLatestMetric,
  getCounterTotal,
  generateReport,
} from "../../src/session/daemon-observability"
import type { ZyalObservability } from "../../src/agent-script/schema"

const testConfig: ZyalObservability = {
  spans: {
    emit: "all",
    include_tool_calls: true,
    include_model_calls: true,
  },
  metrics: [
    { name: "tool_calls", type: "counter", source: "runtime" },
    { name: "risk_score", type: "gauge", source: "analysis" },
  ],
  cost: {
    budget: 10.0,
    currency: "USD",
    alert_at_percent: 80,
    on_budget_exceeded: "pause",
  },
  report: {
    format: "json",
    on_complete: true,
    include: ["spans", "metrics", "costs", "summary"],
  },
}

describe("daemon observability", () => {
  test("initializeObservability creates empty state", () => {
    const state = initializeObservability(testConfig)
    expect(state.spans).toHaveLength(0)
    expect(state.metrics).toHaveLength(0)
    expect(state.costs).toHaveLength(0)
    expect(state.totalCost).toBe(0)
    expect(state.budgetRemaining).toBe(10.0)
  })

  test("initializeObservability handles no config", () => {
    const state = initializeObservability(undefined)
    expect(state.budgetRemaining).toBeNull()
  })

  test("startSpan creates a running span", () => {
    let state = initializeObservability(testConfig)
    const result = startSpan(state, "test_operation")
    state = result.state
    expect(state.spans).toHaveLength(1)
    expect(state.spans[0].name).toBe("test_operation")
    expect(state.spans[0].status).toBe("running")
  })

  test("startSpan supports parent spans", () => {
    let state = initializeObservability(testConfig)
    const parent = startSpan(state, "parent")
    state = parent.state
    const child = startSpan(state, "child", parent.spanId)
    state = child.state
    expect(state.spans[1].parentId).toBe(parent.spanId)
  })

  test("endSpan sets status and timestamp", () => {
    let state = initializeObservability(testConfig)
    const { state: s1, spanId } = startSpan(state, "op")
    state = endSpan(s1, spanId, "ok", { result: "success" })
    expect(state.spans[0].status).toBe("ok")
    expect(state.spans[0].endedAt).toBeDefined()
  })

  test("recordMetric adds metric value", () => {
    let state = initializeObservability(testConfig)
    state = recordMetric(state, "tool_calls", "counter", 1)
    expect(state.metrics).toHaveLength(1)
    expect(state.metrics[0].name).toBe("tool_calls")
  })

  test("incrementCounter adds delta", () => {
    let state = initializeObservability(testConfig)
    state = incrementCounter(state, "tool_calls")
    state = incrementCounter(state, "tool_calls", 5)
    expect(getCounterTotal(state, "tool_calls")).toBe(6)
  })

  test("setGauge records value", () => {
    let state = initializeObservability(testConfig)
    state = setGauge(state, "risk_score", 0.4)
    state = setGauge(state, "risk_score", 0.6)
    expect(getLatestMetric(state, "risk_score")!.value).toBe(0.6)
  })

  test("recordCost tracks expenses and budget", () => {
    let state = initializeObservability(testConfig)
    state = recordCost(state, "model_call", 3.0)
    expect(state.totalCost).toBe(3.0)
    expect(state.budgetRemaining).toBe(7.0)
    expect(state.budgetExceeded).toBe(false)
  })

  test("recordCost detects budget exceeded", () => {
    let state = initializeObservability(testConfig)
    state = recordCost(state, "call1", 6.0)
    state = recordCost(state, "call2", 5.0)
    expect(state.totalCost).toBe(11.0)
    expect(state.budgetExceeded).toBe(true)
  })

  test("checkBudgetAlert fires at threshold", () => {
    let state = initializeObservability(testConfig)
    state = recordCost(state, "call", 8.5) // 85% of $10
    const result = checkBudgetAlert(state, testConfig)
    expect(result.alert).toBe(true)
    expect(result.percent).toBe(85)
    expect(result.action).toBe("pause")
  })

  test("checkBudgetAlert does not fire below threshold", () => {
    let state = initializeObservability(testConfig)
    state = recordCost(state, "call", 2.0) // 20%
    const result = checkBudgetAlert(state, testConfig)
    expect(result.alert).toBe(false)
  })

  test("filterSpans returns all for emit=all", () => {
    let state = initializeObservability(testConfig)
    const { state: s1, spanId: id1 } = startSpan(state, "ok_span")
    state = endSpan(s1, id1, "ok")
    const { state: s2, spanId: id2 } = startSpan(state, "err_span")
    state = endSpan(s2, id2, "error")
    expect(filterSpans(state, testConfig)).toHaveLength(2)
  })

  test("filterSpans returns only errors for errors_only", () => {
    const config: ZyalObservability = { spans: { emit: "errors_only" } }
    let state = initializeObservability(config)
    const { state: s1, spanId: id1 } = startSpan(state, "ok")
    state = endSpan(s1, id1, "ok")
    const { state: s2, spanId: id2 } = startSpan(state, "err")
    state = endSpan(s2, id2, "error")
    expect(filterSpans(state, config)).toHaveLength(1)
  })

  test("filterSpans returns empty for emit=none", () => {
    const config: ZyalObservability = { spans: { emit: "none" } }
    let state = initializeObservability(config)
    const { state: s1 } = startSpan(state, "test")
    state = s1
    expect(filterSpans(state, config)).toHaveLength(0)
  })

  test("getLatestMetric returns most recent", () => {
    let state = initializeObservability(testConfig)
    state = recordMetric(state, "x", "gauge", 1)
    state = recordMetric(state, "x", "gauge", 2)
    state = recordMetric(state, "x", "gauge", 3)
    expect(getLatestMetric(state, "x")!.value).toBe(3)
  })

  test("getLatestMetric returns undefined for unknown", () => {
    const state = initializeObservability(testConfig)
    expect(getLatestMetric(state, "nonexistent")).toBeUndefined()
  })

  test("generateReport produces structured output", () => {
    let state = initializeObservability(testConfig)
    const { state: s1, spanId } = startSpan(state, "op")
    state = endSpan(s1, spanId, "ok")
    state = incrementCounter(state, "tool_calls", 5)
    state = recordCost(state, "model", 3.0)

    const report = generateReport(state, testConfig)
    expect(report.format).toBe("json")
    expect(report.spans).toBeDefined()
    expect(report.metrics).toBeDefined()
    expect(report.costs).toBeDefined()
    expect(report.summary).toBeDefined()
  })
})
