import type { TuiPluginApi } from "@jekko-ai/plugin/tui"
import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js"
import {
  formatZyalRuntime,
  useZyalExit,
  useZyalFlash,
  useZyalMetrics,
  zyalFleetHardCap,
} from "../../../context/zyal-flash"
import { NEON, compactFmt, latencyFmt, moneyFmt, numberFmt } from "./constants"

export function ZyalSidebarView(props: { api: TuiPluginApi }) {
  const theme = () => props.api.theme.current
  const flash = useZyalFlash()
  const metrics = useZyalMetrics()
  const exit = useZyalExit()
  const visible = createMemo(() => flash().size > 0 || exit() !== null)

  const [tick, setTick] = createSignal(Date.now())
  onMount(() => {
    const handle = setInterval(() => setTick(Date.now()), 1000)
    onCleanup(() => clearInterval(handle))
  })

  const runtime = createMemo(() => formatZyalRuntime(tick()))

  const totalTokens = createMemo(() => {
    const m = metrics()
    if (m.jnoccioConnected && m.jnoccioTotalTokens != null) return m.jnoccioTotalTokens
    return m.totalTokens
  })
  const tokensLine = createMemo(() => numberFmt.format(totalTokens()))

  const tokenBreakdownParts = createMemo<{ label: string; value: string; color: string }[]>(() => {
    const m = metrics()
    const out: { label: string; value: string; color: string }[] = []
    if (m.jnoccioConnected && m.jnoccioPromptTokens != null) {
      if (m.jnoccioPromptTokens) out.push({ label: "in", value: compactFmt.format(m.jnoccioPromptTokens), color: NEON.tokensIn })
      if (m.jnoccioCompletionTokens != null && m.jnoccioCompletionTokens > 0) {
        out.push({ label: "out", value: compactFmt.format(m.jnoccioCompletionTokens), color: NEON.tokensOut })
      }
    } else {
      if (m.inputTokens) out.push({ label: "in", value: compactFmt.format(m.inputTokens), color: NEON.tokensIn })
      if (m.outputTokens) out.push({ label: "out", value: compactFmt.format(m.outputTokens), color: NEON.tokensOut })
      if (m.cacheTokens) out.push({ label: "cache", value: compactFmt.format(m.cacheTokens), color: NEON.cache })
    }
    return out
  })

  const loopsLine = createMemo(() => {
    const m = metrics()
    if (m.tasksCompleted || m.tasksIncubated) {
      return `${m.loopsCompleted} (${m.tasksCompleted}✓ ${m.tasksIncubated}🜨)`
    }
    return numberFmt.format(m.loopsCompleted)
  })

  const statusLine = createMemo(() => metrics().status ?? "active")
  const showJankurai = createMemo(() => metrics().jankuraiFindings !== null)
  const showCost = createMemo(() => metrics().costUsd > 0)

  const jnoccioLine = createMemo<"hidden" | "visible">(() => {
    const m = metrics()
    return m.jnoccioConnected || m.jnoccioInstances !== null ? "visible" : "hidden"
  })

  const jnoccioInstancesActive = createMemo(() => numberFmt.format(metrics().jnoccioInstances ?? 0))
  const jnoccioInstancesMax = createMemo(() => numberFmt.format(metrics().jnoccioMaxInstances ?? zyalFleetHardCap()))

  const trafficCounts = createMemo<{ state: "empty" } | { state: "ready"; calls: number; wins: number; failures: number }>(() => {
    const m = metrics()
    if (!m.jnoccioConnected) return { state: "empty" }
    const calls = m.jnoccioCalls ?? 0
    const wins = m.jnoccioWins ?? 0
    const failures = m.jnoccioFailures ?? 0
    if (!calls && !wins && !failures) return { state: "empty" }
    return { state: "ready", calls, wins, failures }
  })

  const jnoccioLatencyMs = createMemo<{ state: "missing" } | { state: "present"; value: number }>(() => {
    const m = metrics()
    if (!m.jnoccioConnected || m.jnoccioAvgLatencyMs == null) return { state: "missing" }
    return { state: "present", value: m.jnoccioAvgLatencyMs }
  })

  const heartbeatStatus = createMemo<
    | { state: "missing" }
    | { state: "live"; color: string; value: string; suffix: string }
    | { state: "aging"; color: string; value: string; suffix: string }
  >(() => {
    const m = metrics()
    if (!m.jnoccioConnected || !m.jnoccioLastHeartbeat) return { state: "missing" }
    const ageMs = Math.max(0, tick() - m.jnoccioLastHeartbeat)
    if (ageMs < 2_000) {
      return { state: "live", color: NEON.heartbeatLive, value: "live", suffix: "" }
    }
    const sec = Math.floor(ageMs / 1000)
    const aging = ageMs > 60_000
    return {
      state: aging ? "aging" : "live",
      color: aging ? NEON.heartbeatStale : NEON.heartbeatLive,
      value: `${sec}s`,
      suffix: " ago",
    }
  })

  const runIdShort = createMemo<{ state: "empty" } | { state: "present"; id: string }>(() => {
    const id = metrics().runId
    if (!id) return { state: "empty" }
    return { state: "present", id: id.length > 12 ? id.slice(0, 12) + "…" : id }
  })

  const commitsLine = createMemo<{ state: "empty" } | { state: "ready"; completed: number; incubated: number }>(() => {
    const m = metrics()
    if (!m.tasksCompleted && !m.tasksIncubated) return { state: "empty" }
    return { state: "ready", completed: m.tasksCompleted, incubated: m.tasksIncubated }
  })

  const EXIT_TONE = {
    success: { fg: "#00FF87", label: "ZYAL SATISFIED" },
    warning: { fg: "#FFD000", label: "ZYAL PAUSED" },
    error: { fg: "#FF4060", label: "ZYAL EXITED" },
  }

  return (
    <Show when={visible()}>
      <box gap={0}>
        <Show when={exit()}>
          {(record) => {
            const tone = EXIT_TONE[record().tone] ?? EXIT_TONE.warning
            return (
              <>
                <text fg={tone.fg}>
                  <b>{tone.label}</b>
                </text>
                <text fg={theme().textMuted}>
                  <span style={{ fg: tone.fg, bold: true }}>●</span>{" "}
                  <span style={{ fg: tone.fg }}>{record().status}</span>
                </text>
                <text fg={theme().text}>{record().reason}</text>
                <text fg={NEON.separator}>{"─".repeat(38)}</text>
              </>
            )
          }}
        </Show>
        <text fg={theme().warning}>
          <b>∞ ZYAL MODE</b>
        </text>
        <Show when={runIdShort().state === "present"}>
          <text fg={theme().textMuted}>run {runIdShort().id}</text>
        </Show>
        <text fg={theme().textMuted}>
          <span style={{ fg: statusLine() === "active" ? theme().success : theme().warning }}>●</span>{" "}
          {statusLine()}
        </text>
        <text fg={NEON.separator}>{"─".repeat(38)}</text>
        <text fg={theme().textMuted}>
          Loops    <span style={{ fg: NEON.loops, bold: true }}>{loopsLine()}</span>
        </text>
        <text fg={theme().textMuted}>
          Tokens   <span style={{ fg: NEON.tokensTotal, bold: true }}>{tokensLine()}</span>
        </text>
        <Show when={tokenBreakdownParts().length > 0}>
          <text fg={theme().textMuted}>
            {"         "}
            <For each={tokenBreakdownParts()}>
              {(part, i) => (
                <>
                  <Show when={i() > 0}>
                    <span style={{ fg: theme().textMuted }}> · </span>
                  </Show>
                  <span style={{ fg: theme().textMuted }}>{part.label} </span>
                  <span style={{ fg: part.color, bold: true }}>{part.value}</span>
                </>
              )}
            </For>
          </text>
        </Show>
        <text fg={theme().textMuted}>
          Workers{" "}
          <span style={{ fg: NEON.workersActive, bold: true }}>
            {numberFmt.format(
              metrics().jnoccioConnected && metrics().jnoccioActiveAgents != null
                ? metrics().jnoccioActiveAgents!
                : metrics().workersActive,
            )}
          </span>
          <span style={{ fg: theme().textMuted }}> / </span>
          <span style={{ fg: NEON.workersMax }}>{numberFmt.format(metrics().workersMax || zyalFleetHardCap())}</span>
        </text>
        <Show when={runtime()}>
          <text fg={theme().textMuted}>
            Uptime   <span style={{ fg: NEON.uptime, bold: true }}>{runtime()}</span>
          </text>
        </Show>
        <Show when={showCost()}>
          <text fg={theme().textMuted}>
            Cost     <span style={{ fg: NEON.cost, bold: true }}>{moneyFmt.format(metrics().costUsd)}</span>
          </text>
        </Show>
        <Show when={commitsLine().state === "ready"}>
          {(counts) => (
            <text fg={theme().textMuted}>
              {counts().completed}✓ completed · {counts().incubated}🜨 incubated
            </text>
          )}
        </Show>
        <Show when={showJankurai()}>
          <text fg={theme().textMuted}>
            jankurai: <span style={{ fg: NEON.fails, bold: true }}>{metrics().jankuraiFindings}</span> open
          </text>
        </Show>
        <Show when={jnoccioLine() === "visible"}>
          <text fg={NEON.separator}>{"─".repeat(38)}</text>
          <text fg={metrics().jnoccioConnected ? theme().success : theme().textMuted}>
            <b>Jnoccio</b>
            <span style={{ fg: theme().textMuted }}> </span>
            <span style={{ fg: NEON.workersActive, bold: true }}>{jnoccioInstancesActive()}</span>
            <span style={{ fg: theme().textMuted }}> / </span>
            <span style={{ fg: NEON.workersMax }}>{jnoccioInstancesMax()}</span>
          </text>
        </Show>
        <Show when={trafficCounts().state === "ready"}>
          {(counts) => (
            <text fg={theme().textMuted}>
              <span style={{ fg: NEON.calls, bold: true }}>{compactFmt.format(counts().calls)}</span>
              <span style={{ fg: theme().textMuted }}> calls · </span>
              <span style={{ fg: NEON.wins, bold: true }}>{compactFmt.format(counts().wins)}</span>
              <span style={{ fg: theme().textMuted }}> wins · </span>
              <span style={{ fg: NEON.fails, bold: true }}>{compactFmt.format(counts().failures)}</span>
              <span style={{ fg: theme().textMuted }}> fail</span>
            </text>
          )}
        </Show>
        <Show when={jnoccioLatencyMs().state === "present"}>
          {(ms) => (
            <text fg={theme().textMuted}>
              avg <span style={{ fg: NEON.latency, bold: true }}>{latencyFmt.format(ms().value)}ms</span>
            </text>
          )}
        </Show>
        <Show when={heartbeatStatus().state !== "missing"}>
          {(beat) => (
            <text fg={theme().textMuted}>
              {beat().state === "live" ? "" : "last beat "}
              <span style={{ fg: beat().color, bold: true }}>{beat().value}</span>
              {beat().suffix}
            </text>
          )}
        </Show>
        <text fg={NEON.separator}>{"─".repeat(38)}</text>
      </box>
    </Show>
  )
}
