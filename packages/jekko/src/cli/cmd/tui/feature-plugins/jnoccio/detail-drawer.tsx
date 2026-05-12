/**
 * Jnoccio model detail drawer.
 *
 * Shown when the user presses Enter on a selected model.
 * Renders as a side panel or dialog depending on terminal width.
 */
import { createMemo, For, Show } from "solid-js"
import type { TuiPluginApi } from "@jekko-ai/plugin/tui"
import type { DashboardModel, DashboardSnapshot, MetricEvent } from "../../context/jnoccio-types"
import { fmtN, fmtMs, fmtPct, fmtTime, truncate, statusDot, latencyTier, latencyIcon } from "./utils"
import { RGBA } from "@opentui/core"

const GOLD = RGBA.fromHex("#F5A623")
const TEAL = RGBA.fromHex("#36D7B7")
const RED = RGBA.fromHex("#FF4757")
const PURPLE = RGBA.fromHex("#A855F7")

function StatRow(props: {
  label: string
  value: string
  api: TuiPluginApi
  valueColor?: RGBA
}) {
  const theme = () => props.api.theme.current
  return (
    <box flexDirection="row" justifyContent="space-between" width="100%">
      <text fg={theme().textMuted}>{props.label}</text>
      <text fg={props.valueColor ?? theme().text}>{props.value}</text>
    </box>
  )
}

export function DetailDrawer(props: {
  api: TuiPluginApi
  model: DashboardModel
  snapshot: DashboardSnapshot
  onClose: () => void
}) {
  const theme = () => props.api.theme.current
  const m = () => props.model

  const modelEvents = createMemo(() =>
    props.snapshot.recent_events
      .filter((e) => e.model_id === m().id)
      .slice(0, 20),
  )

  const contextEstimate = createMemo(() =>
    props.snapshot.context.estimates.find((e) => e.model_id === m().id),
  )

  const statusColor = () => {
    const s = m().status
    if (s === "ready" || s === "active") return theme().success
    if (s === "cooldown") return GOLD
    if (s === "disabled") return theme().textMuted
    return RED
  }

  return (
    <box
      flexDirection="column"
      width="100%"
      paddingLeft={2}
      paddingRight={2}
      paddingTop={1}
      paddingBottom={1}
      gap={1}
    >
      {/* Header */}
      <box flexDirection="row" justifyContent="space-between">
        <text fg={theme().text}>
          <b>{m().display_name}</b>
        </text>
        <text fg={theme().textMuted}>Esc to close</text>
      </box>

      <box flexDirection="row" gap={2}>
        <text fg={statusColor()}>
          {statusDot(m().status)} {m().status}
        </text>
        <text fg={theme().textMuted}>{m().provider}</text>
        <text fg={theme().textMuted}>{m().upstream_model}</text>
      </box>

      {/* Roles */}
      <box flexDirection="row" gap={1}>
        <text fg={theme().textMuted}>Roles:</text>
        <text fg={theme().text}>{m().roles.join(", ") || "—"}</text>
      </box>

      {/* Stats sections */}
      <text fg={GOLD}>
        <b>─── Performance ───</b>
      </text>
      <StatRow api={props.api} label="Calls" value={fmtN(m().call_count)} />
      <StatRow api={props.api} label="Successes" value={fmtN(m().success_count)} valueColor={theme().success} />
      <StatRow api={props.api} label="Failures" value={fmtN(m().failure_count)} valueColor={m().failure_count > 0 ? RED : theme().text} />
      <StatRow api={props.api} label="Wins" value={fmtN(m().win_count)} valueColor={GOLD} />
      <StatRow api={props.api} label="Win Rate" value={fmtPct(m().win_rate)} valueColor={GOLD} />

      <text fg={TEAL}>
        <b>─── Latency ───</b>
      </text>
      <StatRow api={props.api} label="Average" value={fmtMs(m().avg_latency_ms)} />
      <StatRow api={props.api} label="Last" value={fmtMs(m().last_latency_ms)} />
      <StatRow api={props.api} label="Min" value={fmtMs(m().min_latency_ms)} />
      <StatRow api={props.api} label="Max" value={fmtMs(m().max_latency_ms)} />

      <text fg={PURPLE}>
        <b>─── Tokens ───</b>
      </text>
      <StatRow api={props.api} label="Prompt" value={fmtN(m().prompt_tokens)} />
      <StatRow api={props.api} label="Completion" value={fmtN(m().completion_tokens)} />
      <StatRow api={props.api} label="Total" value={fmtN(m().total_tokens)} valueColor={PURPLE} />

      {/* Capacity */}
      <text fg={theme().text}>
        <b>─── Capacity ───</b>
      </text>
      <StatRow api={props.api} label="Hourly Capacity" value={m().hourly_capacity ? fmtN(m().hourly_capacity) : "unknown"} />
      <StatRow api={props.api} label="Hourly Used" value={fmtN(m().hourly_used)} />
      <StatRow api={props.api} label="Context Window" value={fmtN(m().configured_context_window)} />
      <StatRow api={props.api} label="Safe Window" value={fmtN(m().safe_context_window)} />
      <Show when={m().learned_context_window}>
        <StatRow api={props.api} label="Learned Window" value={fmtN(m().learned_context_window)} valueColor={TEAL} />
      </Show>
      <Show when={m().context_overrun_count > 0}>
        <StatRow api={props.api} label="Context Overruns" value={fmtN(m().context_overrun_count)} valueColor={RED} />
      </Show>

      {/* Errors */}
      <Show when={m().last_error_kind}>
        <text fg={RED}>
          <b>─── Last Error ───</b>
        </text>
        <text fg={RED}>{m().last_error_kind}</text>
        <text fg={theme().textMuted}>{truncate(m().last_error_message ?? "", 60)}</text>
      </Show>

      {/* Recent events for this model */}
      <Show when={modelEvents().length > 0}>
        <text fg={theme().text}>
          <b>─── Recent Events ───</b>
        </text>
        <For each={modelEvents()}>
          {(event) => {
            const statusColor = event.status === "success" || event.status === "winner"
              ? theme().success
              : event.status === "failure" ? RED : theme().textMuted
            return (
              <box flexDirection="row" gap={1} width="100%">
                <text fg={statusColor} flexShrink={0}>
                  {statusDot(event.status === "success" ? "ready" : event.status === "failure" ? "error" : "cooldown")}
                </text>
                <text fg={theme().text} flexShrink={0}>{event.phase}</text>
                <text fg={statusColor} flexShrink={0}>{event.status}</text>
                <text fg={theme().textMuted} flexShrink={0}>
                  {event.error_kind ?? fmtMs(event.latency_ms)}
                </text>
                <text fg={theme().textMuted}>{fmtTime(event.created_at)}</text>
              </box>
            )
          }}
        </For>
      </Show>
    </box>
  )
}
