// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
import { incrementJnoccioCounters, setZyalFlashSource, updateZyalMetrics } from "./zyal-flash-state"
import { createStore, produce, reconcile } from "solid-js/store"
import type { DashboardSnapshot, DashboardModel, MetricEvent } from "./jnoccio-types"
import { emptySnapshot, normalizeSnapshot, totalsFromModels } from "./jnoccio-snapshot"

/**
 * Jnoccio Fusion live-metrics WebSocket subscriber.
 *
 * When a daemon run declares `fleet.jnoccio.enabled` plus a `base_url`, the
 * TUI opens a single WebSocket to `<base_url>/v1/jnoccio/metrics/ws` and
 * funnels DashboardSnapshot / RequestEvent / Heartbeat messages directly
 * into the ZYAL fleet metrics signal so the right-pane panel reflects
 * cross-instance totals without round-tripping through the daemon endpoint.
 *
 * The wire format is the canonical Jnoccio one (see jnoccio-fusion/src/fusion.rs):
 *
 *   {type: "snapshot",      snapshot: DashboardSnapshot}
 *   {type: "request_event", event:    MetricEvent}
 *   {type: "model_updated", model:    DashboardModel}
 *   {type: "heartbeat",     timestamp: i64}
 *
 * The connector is keyed by (base_url, runId) so flipping runs reuses or
 * tears down the active socket — there is never more than one open.
 */

const SOURCE_ID = "jnoccio:ws"
const RECONNECT_INITIAL_MS = 1_000
const RECONNECT_MAX_MS = 30_000
const HEARTBEAT_TIMEOUT_MS = 60_000
const MAX_DASHBOARD_EVENTS = 300

// ── Full Dashboard Snapshot Store ──────────────────────────────────────────
// Provides the complete snapshot data for the Jnoccio TUI plugin.
// Uses lazy init to avoid running createStore at module import time
// before SolidJS root is ready (which would crash the TUI).

let _dashboardStore: DashboardSnapshot | null = null
let _setDashboardStore: ((fn: any) => void) | null = null
let _lastHbTs: { ts: number | null } | null = null
let _setLastHbTs: ((key: string, val: any) => void) | null = null

function ensureDashboardStore() {
  if (_dashboardStore) return
  const [store, setStore] = createStore<DashboardSnapshot>(emptySnapshot())
  _dashboardStore = store
  _setDashboardStore = setStore as any
  const [hb, setHb] = createStore<{ ts: number | null }>({ ts: null })
  _lastHbTs = hb
  _setLastHbTs = setHb as any
}

/** Reactive accessor for the full Jnoccio dashboard snapshot. */
export function useJnoccioSnapshot(): DashboardSnapshot {
  ensureDashboardStore()
  return _dashboardStore!
}

/** Reactive accessor for the last heartbeat timestamp. */
export function useJnoccioLastHeartbeat(): number | null {
  ensureDashboardStore()
  return _lastHbTs!.ts
}

type ConnectInput = {
  baseUrl: string
  metricsWsPath?: string
  runId?: string | null
}

type ActiveConnection = {
  key: string
  baseUrl: string
  metricsWsPath: string
  runId: string | null
  socket: WebSocket | null
  reconnectMs: number
  reconnectTimer: ReturnType<typeof setTimeout> | null
  heartbeatTimer: ReturnType<typeof setTimeout> | null
  closed: boolean
}

let active: ActiveConnection | null = null

export function buildJnoccioWsUrl(baseUrl: string, path: string): string | null {
  try {
    const trimmed = baseUrl.trim().replace(/\/+$/, "")
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    if (!trimmed) return null
    const u = new URL(trimmed)
    u.pathname = `${u.pathname.replace(/\/+$/, "")}${path.startsWith("/") ? path : `/${path}`}`
    if (u.protocol === "http:") u.protocol = "ws:"
    else if (u.protocol === "https:") u.protocol = "wss:"
    return u.toString()
  } catch {
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    return null
  }
}

function cancelReconnect(conn: ActiveConnection) {
  if (conn.reconnectTimer) {
    clearTimeout(conn.reconnectTimer)
    conn.reconnectTimer = null
  }
}

function cancelHeartbeat(conn: ActiveConnection) {
  if (conn.heartbeatTimer) {
    clearTimeout(conn.heartbeatTimer)
    conn.heartbeatTimer = null
  }
}

function armHeartbeatWatchdog(conn: ActiveConnection) {
  cancelHeartbeat(conn)
  conn.heartbeatTimer = setTimeout(() => {
    // Server heartbeats every 15s; if we miss 4× we treat it as dead.
    const sock = conn.socket
    if (sock && sock.readyState === sock.OPEN) {
      try {
        sock.close(4000, "heartbeat-timeout")
      } catch {}
    }
  }, HEARTBEAT_TIMEOUT_MS)
}

export function applyJnoccioSnapshot(snapshot: any) {
  if (!snapshot || typeof snapshot !== "object") return
  const totals = snapshot.totals ?? {}
  const promptTokens = Number(totals.prompt_tokens ?? 0)
  const completionTokens = Number(totals.completion_tokens ?? 0)
  const totalTokens = Number(totals.total_tokens ?? 0) || promptTokens + completionTokens
  updateZyalMetrics({
    jnoccioConnected: true,
    jnoccioInstances: Number(snapshot.instance_count ?? 0),
    jnoccioMaxInstances: Number(snapshot.max_instances ?? 0),
    jnoccioActiveAgents: Number(snapshot.agent_count ?? snapshot.active_agents?.length ?? 0),
    jnoccioPromptTokens: promptTokens,
    jnoccioCompletionTokens: completionTokens,
    jnoccioTotalTokens: totalTokens,
    jnoccioCalls: Number(totals.calls ?? 0),
    jnoccioWins: Number(totals.wins ?? 0),
    jnoccioFailures: Number(totals.failures ?? 0),
    jnoccioAvgLatencyMs: totals.average_latency_ms != null ? Number(totals.average_latency_ms) : null,
    jnoccioWorkerThreads: snapshot.worker_threads != null ? Number(snapshot.worker_threads) : null,
    jnoccioInstanceRole: typeof snapshot.instance_role === "string" ? snapshot.instance_role : null,
  })

  // Update full dashboard store for the TUI plugin
  try {
    ensureDashboardStore()
    _setDashboardStore!(reconcile(normalizeSnapshot(snapshot as DashboardSnapshot)))
  } catch {
    // Tolerate malformed snapshots — ZYAL metrics already applied above
  }
}

export function applyJnoccioRequestEvent(event: any) {
  // Per-event tokens are already rolled into the next snapshot, so we only
  // bump the running counters by the delta to keep the UI feeling live
  // between snapshot polls (5s). Snapshots overwrite when they arrive.
  //
  // Delegates to `incrementJnoccioCounters` which uses Solid's functional
  // setter to keep the read+merge+write atomic; without that, a snapshot
  // landing between the read and write would have its authoritative reset
  // overwritten by previous-baseline-plus-delta.
  if (!event || typeof event !== "object") return
  const dPrompt = Number(event.prompt_tokens ?? 0)
  const dCompletion = Number(event.completion_tokens ?? 0)
  const dTotal = Number(event.total_tokens ?? 0) || dPrompt + dCompletion
  const status = typeof event.status === "string" ? event.status : ""
  const callsDelta = status === "success" || status === "failure" ? 1 : 0
  const winsDelta = status === "winner" || event.winner_model_id ? 1 : 0
  const failuresDelta = status === "failure" ? 1 : 0
  const latencyMs = event.latency_ms == null ? null : Number(event.latency_ms)
  incrementJnoccioCounters({
    promptTokens: dPrompt || undefined,
    completionTokens: dCompletion || undefined,
    totalTokens: dTotal || undefined,
    calls: callsDelta || undefined,
    wins: winsDelta || undefined,
    failures: failuresDelta || undefined,
    avgLatencyMs: Number.isFinite(latencyMs) ? (latencyMs as number) : undefined,
  })

  // Prepend to dashboard store recent_events for the TUI plugin
  try {
    ensureDashboardStore()
    const me = event as MetricEvent
    _setDashboardStore!(produce((draft: DashboardSnapshot) => {
      if (draft.recent_events.some((e) => e.id === me.id)) return
      draft.recent_events.unshift(me)
      if (draft.recent_events.length > MAX_DASHBOARD_EVENTS) {
        draft.recent_events.length = MAX_DASHBOARD_EVENTS
      }
    }))
  } catch {
    // Tolerate type mismatches
  }
}

function applyHeartbeat(timestamp: number) {
  updateZyalMetrics({
    jnoccioConnected: true,
    jnoccioLastHeartbeat: Number.isFinite(timestamp) ? timestamp * 1000 : Date.now(),
  })
  try {
    ensureDashboardStore()
    _setLastHbTs!("ts", Number.isFinite(timestamp) ? timestamp : null)
  } catch {
    // Tolerate init failures
  }
}

function clearJnoccioMetrics() {
  updateZyalMetrics({
    jnoccioConnected: false,
    jnoccioInstances: null,
    jnoccioMaxInstances: null,
    jnoccioActiveAgents: null,
    jnoccioPromptTokens: null,
    jnoccioCompletionTokens: null,
    jnoccioTotalTokens: null,
    jnoccioCalls: null,
    jnoccioWins: null,
    jnoccioFailures: null,
    jnoccioAvgLatencyMs: null,
    jnoccioWorkerThreads: null,
    jnoccioInstanceRole: null,
    jnoccioLastHeartbeat: null,
  })
}

function open(conn: ActiveConnection) {
  if (conn.closed) return
  const url = buildJnoccioWsUrl(conn.baseUrl, conn.metricsWsPath)
  if (!url) {
    // Bad config — give up and drop the source flag.
    setZyalFlashSource(SOURCE_ID, false)
    return
  }

  let socket: WebSocket
  try {
    socket = new WebSocket(url)
  } catch {
    scheduleReconnect(conn)
    return
  }
  conn.socket = socket

  socket.addEventListener("open", () => {
    if (conn.closed) {
      try {
        socket.close()
      } catch {}
      return
    }
    conn.reconnectMs = RECONNECT_INITIAL_MS
    setZyalFlashSource(SOURCE_ID, true)
    armHeartbeatWatchdog(conn)
  })

  socket.addEventListener("message", (evt: MessageEvent) => {
    const raw = typeof evt.data === "string" ? evt.data : null
    if (!raw) return
    let parsed: any
    try {
      parsed = JSON.parse(raw)
    } catch {
      return
    }
    if (!parsed || typeof parsed !== "object") return
    armHeartbeatWatchdog(conn)
    switch (parsed.type) {
      case "snapshot":
        applyJnoccioSnapshot(parsed.snapshot)
        break
      case "request_event":
        applyJnoccioRequestEvent(parsed.event)
        break
      case "heartbeat":
        applyHeartbeat(Number(parsed.timestamp ?? 0))
        break
      default:
        // Ignore model_updated — snapshot already covers totals.
        break
    }
  })

  socket.addEventListener("close", () => {
    cancelHeartbeat(conn)
    conn.socket = null
    if (conn.closed) return
    scheduleReconnect(conn)
  })

  socket.addEventListener("error", () => {
    // Let close handle reconnect.
    try {
      socket.close()
    } catch {}
  })
}

function scheduleReconnect(conn: ActiveConnection) {
  if (conn.closed) return
  const delay = conn.reconnectMs
  conn.reconnectMs = Math.min(RECONNECT_MAX_MS, conn.reconnectMs * 2)
  setZyalFlashSource(SOURCE_ID, false)
  // Don't wipe metrics on transient drop — keep last-known until reconnect.
  cancelReconnect(conn)
  conn.reconnectTimer = setTimeout(() => {
    conn.reconnectTimer = null
    open(conn)
  }, delay)
}

function teardown(conn: ActiveConnection) {
  // Invariant — must run in this order:
  //   1. cancel any pending timers BEFORE flipping `closed`. Both timers
  //      check `conn.closed` inside their callbacks; cancelling first
  //      removes the queued tick entirely so even a re-entrant scheduler
  //      cannot re-arm us.
  //   2. flip `closed = true` so any in-flight socket event listeners
  //      (open / message / close / error) bail out early.
  //   3. close the socket and null the reference so further sends fail.
  //   4. drop the gold-flash source flag.
  //   5. clear cached jnoccio metrics so the panel resets visibly.
  cancelReconnect(conn)
  cancelHeartbeat(conn)
  conn.closed = true
  if (conn.socket) {
    try {
      conn.socket.close(1000, "tui-teardown")
    } catch {}
    conn.socket = null
  }
  setZyalFlashSource(SOURCE_ID, false)
  clearJnoccioMetrics()
}

function makeKey(input: ConnectInput): string {
  const path = input.metricsWsPath ?? "/v1/jnoccio/metrics/ws"
  return `${input.baseUrl}::${path}::${input.runId ?? ""}`
}

/**
 * Connect (or reconnect) the jnoccio metrics WebSocket. Idempotent — calling
 * with the same key while a connection is open is a no-op. Calling with a
 * different key tears down the previous connection first.
 */
export function connectJnoccio(input: ConnectInput): void {
  const key = makeKey(input)
  if (active && active.key === key) return
  if (active) teardown(active)
  if (!buildJnoccioWsUrl(input.baseUrl, input.metricsWsPath ?? "/v1/jnoccio/metrics/ws")) {
    active = null
    setZyalFlashSource(SOURCE_ID, false)
    clearJnoccioMetrics()
    return
  }
  const conn: ActiveConnection = {
    key,
    baseUrl: input.baseUrl,
    metricsWsPath: input.metricsWsPath ?? "/v1/jnoccio/metrics/ws",
    runId: input.runId ?? null,
    socket: null,
    reconnectMs: RECONNECT_INITIAL_MS,
    reconnectTimer: null,
    heartbeatTimer: null,
    closed: false,
  }
  active = conn
  open(conn)
}

/** Tear down the active jnoccio WebSocket connection. */
export function disconnectJnoccio(): void {
  if (!active) return
  const conn = active
  active = null
  teardown(conn)
}

/** Test helper: peek at the current key. */
export function jnoccioConnectionKey(): string | null {
  return active?.key ?? null
}
