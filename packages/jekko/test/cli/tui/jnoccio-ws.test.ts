import { afterEach, beforeEach, describe, expect, test } from "bun:test"
import {
  applyJnoccioRequestEvent,
  applyJnoccioSnapshot,
  buildJnoccioWsUrl,
  connectJnoccio,
  disconnectJnoccio,
  jnoccioConnectionKey,
} from "../../../src/cli/cmd/tui/context/jnoccio-ws"
import { resetZyalMetrics, useZyalMetrics } from "../../../src/cli/cmd/tui/context/zyal-flash"

class FakeWebSocket {
  static instances: FakeWebSocket[] = []
  readonly OPEN = 1
  readonly CLOSED = 3
  readyState = 0
  closedWith: { code?: number; reason?: string } | null = null
  private listeners = new Map<string, Set<(evt?: any) => void>>()

  constructor(readonly url: string) {
    FakeWebSocket.instances.push(this)
  }

  addEventListener(type: string, listener: (evt?: any) => void) {
    const listeners = this.listeners.get(type) ?? new Set()
    listeners.add(listener)
    this.listeners.set(type, listeners)
  }

  close(code?: number, reason?: string) {
    this.readyState = this.CLOSED
    this.closedWith = { code, reason }
    this.emit("close", {})
  }

  emit(type: string, evt?: any) {
    for (const listener of this.listeners.get(type) ?? []) listener(evt)
  }
}

describe("jnoccio websocket metrics", () => {
  const originalWebSocket = globalThis.WebSocket

  beforeEach(() => {
    resetZyalMetrics()
    disconnectJnoccio()
    FakeWebSocket.instances = []
    ;(globalThis as any).WebSocket = FakeWebSocket
  })

  afterEach(() => {
    disconnectJnoccio()
    resetZyalMetrics()
    ;(globalThis as any).WebSocket = originalWebSocket
  })

  test("builds websocket URLs from http and https base URLs", () => {
    expect(buildJnoccioWsUrl("http://127.0.0.1:4317", "/v1/jnoccio/metrics/ws")).toBe(
      "ws://127.0.0.1:4317/v1/jnoccio/metrics/ws",
    )
    expect(buildJnoccioWsUrl("https://fusion.example/base", "metrics/ws")).toBe(
      "wss://fusion.example/base/metrics/ws",
    )
    expect(buildJnoccioWsUrl("not a url", "/ws")).toBeNull()
  })

  test("keeps one socket per connection key and switches runs cleanly", () => {
    connectJnoccio({ baseUrl: "http://127.0.0.1:4317", runId: "run_1" })
    expect(jnoccioConnectionKey()).toBe("http://127.0.0.1:4317::/v1/jnoccio/metrics/ws::run_1")
    expect(FakeWebSocket.instances).toHaveLength(1)

    connectJnoccio({ baseUrl: "http://127.0.0.1:4317", runId: "run_1" })
    expect(FakeWebSocket.instances).toHaveLength(1)

    const first = FakeWebSocket.instances[0]!
    connectJnoccio({ baseUrl: "http://127.0.0.1:4317", runId: "run_2" })
    expect(FakeWebSocket.instances).toHaveLength(2)
    expect(first.closedWith?.reason).toBe("tui-teardown")
    expect(jnoccioConnectionKey()).toBe("http://127.0.0.1:4317::/v1/jnoccio/metrics/ws::run_2")

    connectJnoccio({ baseUrl: "not a url", runId: "bad" })
    expect(jnoccioConnectionKey()).toBeNull()
    expect(FakeWebSocket.instances).toHaveLength(2)
  })

  test("maps snapshots and request_event deltas into live counters", () => {
    applyJnoccioSnapshot({
      instance_count: 2,
      max_instances: 4,
      agent_count: 3,
      worker_threads: 8,
      instance_role: "leader",
      totals: {
        calls: 10,
        wins: 2,
        failures: 1,
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
        average_latency_ms: 250,
      },
    })

    applyJnoccioRequestEvent({
      status: "success",
      prompt_tokens: 11,
      completion_tokens: 7,
      total_tokens: 18,
      latency_ms: 123,
    })
    applyJnoccioRequestEvent({ status: "winner", winner_model_id: "model-a" })
    applyJnoccioRequestEvent({ status: "failure" })

    const metrics = useZyalMetrics()()
    expect(metrics.jnoccioConnected).toBe(true)
    expect(metrics.jnoccioInstances).toBe(2)
    expect(metrics.jnoccioMaxInstances).toBe(4)
    expect(metrics.jnoccioActiveAgents).toBe(3)
    expect(metrics.jnoccioPromptTokens).toBe(111)
    expect(metrics.jnoccioCompletionTokens).toBe(57)
    expect(metrics.jnoccioTotalTokens).toBe(168)
    expect(metrics.jnoccioCalls).toBe(12)
    expect(metrics.jnoccioWins).toBe(3)
    expect(metrics.jnoccioFailures).toBe(2)
    expect(metrics.jnoccioAvgLatencyMs).toBe(123)
    expect(metrics.jnoccioWorkerThreads).toBe(8)
    expect(metrics.jnoccioInstanceRole).toBe("leader")
  })
})
