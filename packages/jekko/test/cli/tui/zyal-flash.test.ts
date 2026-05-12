import { describe, expect, test } from "bun:test"
import {
  daemonRunJnoccioConfig,
  daemonRunToZyalMetrics,
  isZyalTerminalStatus,
  textHasZyalSentinel,
} from "../../../src/cli/cmd/tui/context/zyal-flash"
import {
  daemonPollResultForSession,
  selectDaemonRunForSession,
  shouldPreserveZyalStateOnDaemonPollError,
} from "../../../src/cli/cmd/tui/routes/session/daemon-poll"

const ZYAL_FLASH_STATE_URL = new URL("../../../src/cli/cmd/tui/context/zyal-flash-state.ts", import.meta.url)

async function loadZyalFlashState() {
  const url = new URL(ZYAL_FLASH_STATE_URL)
  url.searchParams.set("fresh", `${Date.now()}-${Math.random()}`)
  return await import(url.href)
}

describe("zyal flash metrics", () => {
  test("normalizes daemon API stats and spec_json fleet", async () => {
    const { resetZyalMetrics, updateZyalMetrics, useZyalMetrics } = await loadZyalFlashState()
    resetZyalMetrics()
    const run = {
      id: "run_1",
      status: "running",
      iteration: 2,
      spec: { fleet: { max_workers: 99 } },
      spec_json: {
        fleet: {
          max_workers: 7,
          jnoccio: {
            enabled: true,
            base_url: "http://127.0.0.1:4317",
            metrics_ws: "/metrics/ws",
          },
        },
      },
      _stats: {
        iteration_count: 5,
        input_tokens: 100,
        output_tokens: 50,
        cache_tokens: 25,
        total_tokens: 175,
        cost_usd: 1.25,
        active_workers: 3,
        completed_tasks: 2,
        incubated_tasks: 1,
      },
    }

    updateZyalMetrics(daemonRunToZyalMetrics(run, "ses_1"))
    const metrics = useZyalMetrics()()
    expect(metrics.runId).toBe("run_1")
    expect(metrics.workersActive).toBe(3)
    expect(metrics.workersMax).toBe(7)
    expect(metrics.totalTokens).toBe(175)
    expect(metrics.inputTokens).toBe(100)
    expect(metrics.outputTokens).toBe(50)
    expect(metrics.cacheTokens).toBe(25)
    expect(metrics.loopsCompleted).toBe(5)
    expect(metrics.tasksCompleted).toBe(2)
    expect(metrics.tasksIncubated).toBe(1)
    expect(metrics.costUsd).toBe(1.25)
  })

  test("derives jnoccio config from spec_json fleet", async () => {
    const config = daemonRunJnoccioConfig({
      id: "run_2",
      spec_json: {
        fleet: {
          jnoccio: {
            enabled: true,
            base_url: "https://fusion.example",
            metrics_ws: "custom/ws",
          },
        },
      },
    })
    expect(config).toEqual({
      baseUrl: "https://fusion.example",
      metricsWsPath: "custom/ws",
      runId: "run_2",
    })
  })
})

describe("textHasZyalSentinel", () => {
  test("matches the open sentinel anywhere in the buffer", () => {
    expect(textHasZyalSentinel("<<<ZYAL v1:daemon id=test>>>")).toBe(true)
    expect(textHasZyalSentinel("# preamble\n<<<ZYAL v1:daemon id=foo-bar>>>\n")).toBe(true)
    expect(textHasZyalSentinel("here you go: <<<ZYAL v1:daemon id=t1>>>")).toBe(true)
    expect(textHasZyalSentinel("[<<<ZYAL v1:daemon id=wrap>>>]")).toBe(true)
  })

  test("rejects buffers without the sentinel", () => {
    expect(textHasZyalSentinel("")).toBe(false)
    expect(textHasZyalSentinel(null)).toBe(false)
    expect(textHasZyalSentinel(undefined)).toBe(false)
    expect(textHasZyalSentinel("<<<ZYAL v1:daemon>>>")).toBe(false)
    expect(textHasZyalSentinel("not zyal at all")).toBe(false)
  })
})

describe("zyal exit record", () => {
  test("isZyalTerminalStatus recognises every transition the daemon can take", () => {
    expect(isZyalTerminalStatus("satisfied")).toBe(true)
    expect(isZyalTerminalStatus("paused")).toBe(true)
    expect(isZyalTerminalStatus("aborted")).toBe(true)
    expect(isZyalTerminalStatus("failed")).toBe(true)
    expect(isZyalTerminalStatus("running")).toBe(false)
    expect(isZyalTerminalStatus("running_iteration")).toBe(false)
    expect(isZyalTerminalStatus(undefined)).toBe(false)
    expect(isZyalTerminalStatus(null)).toBe(false)
  })

})

describe("daemon poll helpers", () => {
  test("prefers live daemon runs over older terminal runs for a session", () => {
    const priorRun = { id: "run_1", status: "satisfied", root_session_id: "ses_1" }
    const live = { id: "new", status: "running", active_session_id: "ses_1" }

    expect(selectDaemonRunForSession([priorRun, live], "ses_1")).toBe(live)
    expect(daemonPollResultForSession([priorRun], "ses_1")).toMatchObject({
      run: priorRun,
      found: true,
      live: false,
      terminal: true,
    })
  })

  test("preserves gold state on transient poll errors while submit or live run is pending", () => {
    expect(
      shouldPreserveZyalStateOnDaemonPollError({
        promptSubmitted: true,
        currentRun: undefined,
      }),
    ).toBe(true)
    expect(
      shouldPreserveZyalStateOnDaemonPollError({
        promptSubmitted: false,
        currentRun: { status: "running" },
      }),
    ).toBe(true)
    expect(
      shouldPreserveZyalStateOnDaemonPollError({
        promptSubmitted: false,
        currentRun: { status: "failed" },
      }),
    ).toBe(false)
  })
})

describe("incrementJnoccioCounters atomic merge", () => {
  test("monotone counters survive interleaved snapshot resets", async () => {
    const { incrementJnoccioCounters, resetZyalMetrics, updateZyalMetrics, useZyalMetrics } = await loadZyalFlashState()
    resetZyalMetrics()
    // Authoritative snapshot establishes a baseline.
    updateZyalMetrics({
      jnoccioConnected: true,
      jnoccioPromptTokens: 100,
      jnoccioCompletionTokens: 50,
      jnoccioTotalTokens: 150,
      jnoccioCalls: 10,
      jnoccioWins: 2,
      jnoccioFailures: 1,
    })
    incrementJnoccioCounters({
      promptTokens: 11,
      completionTokens: 7,
      totalTokens: 18,
      calls: 1,
      avgLatencyMs: 123,
    })
    incrementJnoccioCounters({ wins: 1 })
    incrementJnoccioCounters({ calls: 1, failures: 1 })
    const m = useZyalMetrics()()
    expect(m.jnoccioConnected).toBe(true)
    expect(m.jnoccioPromptTokens).toBe(111)
    expect(m.jnoccioCompletionTokens).toBe(57)
    expect(m.jnoccioTotalTokens).toBe(168)
    expect(m.jnoccioCalls).toBe(12)
    expect(m.jnoccioWins).toBe(3)
    expect(m.jnoccioFailures).toBe(2)
    expect(m.jnoccioAvgLatencyMs).toBe(123)
  })

  test("starts from null baseline without throwing", async () => {
    const { incrementJnoccioCounters, resetZyalMetrics, useZyalMetrics } = await loadZyalFlashState()
    resetZyalMetrics()
    incrementJnoccioCounters({
      promptTokens: 5,
      completionTokens: 3,
      totalTokens: 8,
      calls: 1,
    })
    const m = useZyalMetrics()()
    expect(m.jnoccioConnected).toBe(true)
    expect(m.jnoccioPromptTokens).toBe(5)
    expect(m.jnoccioCompletionTokens).toBe(3)
    expect(m.jnoccioTotalTokens).toBe(8)
    expect(m.jnoccioCalls).toBe(1)
  })

  test("ignores zero / undefined deltas without resetting counters", async () => {
    const { incrementJnoccioCounters, resetZyalMetrics, updateZyalMetrics, useZyalMetrics } = await loadZyalFlashState()
    resetZyalMetrics()
    updateZyalMetrics({ jnoccioPromptTokens: 42, jnoccioConnected: true })
    incrementJnoccioCounters({ promptTokens: 0, completionTokens: undefined })
    expect(useZyalMetrics()().jnoccioPromptTokens).toBe(42)
  })

  test("avgLatencyMs is replaced not summed", async () => {
    const { incrementJnoccioCounters, resetZyalMetrics, updateZyalMetrics, useZyalMetrics } = await loadZyalFlashState()
    resetZyalMetrics()
    updateZyalMetrics({ jnoccioAvgLatencyMs: 200, jnoccioConnected: true })
    incrementJnoccioCounters({ avgLatencyMs: 50 })
    expect(useZyalMetrics()().jnoccioAvgLatencyMs).toBe(50)
  })

  test("avgLatencyMs null clears the field", async () => {
    const { incrementJnoccioCounters, resetZyalMetrics, updateZyalMetrics, useZyalMetrics } = await loadZyalFlashState()
    resetZyalMetrics()
    updateZyalMetrics({ jnoccioAvgLatencyMs: 200, jnoccioConnected: true })
    incrementJnoccioCounters({ avgLatencyMs: null })
    expect(useZyalMetrics()().jnoccioAvgLatencyMs).toBe(null)
  })
})
