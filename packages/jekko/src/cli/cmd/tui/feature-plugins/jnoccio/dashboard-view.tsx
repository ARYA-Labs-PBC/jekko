import { Match, Show, Switch } from "solid-js"
import type { TuiPluginApi } from "@jekko-ai/plugin/tui"
import type { DashboardModel, DashboardSnapshot } from "../../context/jnoccio-types"
import type { DashboardState } from "./state"
import { TABS } from "./state"
import {
  AgentsPanel,
  FeedPanel,
  LatencyPanel,
  LeaderboardPanel,
  LimitsPanel,
  TokenVaultPanel,
} from "./panels"
import { DetailDrawer } from "./detail-drawer"
import { HelpOverlay } from "./help-overlay"
import { fmtM, fmtMs, fmtN, fmtPct, connLabel } from "./utils"
import { BLUE, GOLD, PURPLE, RED, TEAL } from "./panel-common"
import { RGBA } from "@opentui/core"

const HOT_PINK = RGBA.fromHex("#FF00B8")

export type SelectedDrawer = {
  kind: "selected"
  modelId: string
  model: DashboardModel
}

export function JnoccioDashboardView(props: {
  api: TuiPluginApi
  dims: { width: number; height: number }
  snapshot: DashboardSnapshot
  state: DashboardState
  connection: string
  lastHeartbeat: number | null
  selectedDrawer: SelectedDrawer | undefined
  wideEnough: boolean
}) {
  const theme = () => props.api.theme.current

  return (
    <box
      flexDirection="column"
      width={props.dims.width}
      height={props.dims.height}
      backgroundColor={theme().background}
    >
      <box
        flexDirection="row"
        width="100%"
        paddingLeft={2}
        paddingRight={2}
        paddingTop={1}
        justifyContent="space-between"
        flexShrink={0}
      >
        <box flexDirection="row" gap={2}>
          <text fg={GOLD}>
            <b>Jnoccio Fusion</b>
          </text>
          <text fg={theme().textMuted}>
            {props.snapshot.totals.enabled_models}/{props.snapshot.totals.total_models} models
          </text>
          <text fg={theme().textMuted}>
            {props.snapshot.agent_count}/{props.snapshot.max_agents} agents
          </text>
          <text fg={theme().textMuted}>x{props.snapshot.instance_count} gateways</text>
        </box>
        <box flexDirection="row" gap={2}>
          <Show when={props.state.paused()}>
            <text fg={HOT_PINK}>
              <b>⏸ PAUSED</b> ({props.state.getBufferCount()} buffered)
            </text>
          </Show>
          <Show when={props.state.searchActive()}>
            <text fg={GOLD}>/{props.state.searchQuery()}█</text>
          </Show>
          <text fg={props.connection === "live" ? theme().success : props.connection === "error" ? RED : HOT_PINK}>
            {connLabel(props.connection, props.lastHeartbeat)}
          </text>
        </box>
      </box>

      <box flexDirection="row" width="100%" paddingLeft={2} paddingRight={2} gap={3} flexShrink={0}>
        <text fg={theme().text}>
          {fmtN(props.snapshot.totals.calls)} <span style={{ fg: theme().textMuted }}>calls</span>
        </text>
        <text fg={GOLD}>
          {fmtN(props.snapshot.totals.wins)} <span style={{ fg: theme().textMuted }}>wins</span>
        </text>
        <text fg={RED}>
          {fmtN(props.snapshot.totals.failures)} <span style={{ fg: theme().textMuted }}>fails</span>
        </text>
        <text fg={PURPLE}>
          {fmtN(props.snapshot.totals.total_tokens)} <span style={{ fg: theme().textMuted }}>tokens</span>
        </text>
        <text fg={TEAL}>
          {fmtM(props.snapshot.token_rate.median_m_tokens_per_24h)}M
          <span style={{ fg: theme().textMuted }}>/24h</span>
        </text>
        <text fg={BLUE}>
          {fmtMs(props.snapshot.totals.average_latency_ms)} <span style={{ fg: theme().textMuted }}>avg</span>
        </text>
        <text fg={theme().text}>
          {fmtPct(props.snapshot.capacity.percent_used / 100)} <span style={{ fg: theme().textMuted }}>cap</span>
        </text>
      </box>

      <box
        flexDirection="row"
        width="100%"
        paddingLeft={2}
        paddingRight={2}
        paddingTop={1}
        gap={2}
        flexShrink={0}
      >
        {TABS.map((t) => (
          <text fg={props.state.tab() === t.id ? GOLD : theme().textMuted}>
            {props.state.tab() === t.id ? "▸ " : "  "}
            <b>{t.num}</b> {t.icon} {t.label}
          </text>
        ))}
      </box>

      <box flexDirection="row" flexGrow={1} width="100%" overflow="hidden">
        <box
          flexDirection="column"
          flexGrow={1}
          paddingLeft={2}
          paddingRight={props.selectedDrawer && props.wideEnough ? 0 : 2}
          paddingTop={1}
          overflow="hidden"
        >
          <Show when={props.state.helpOpen()}>
            <HelpOverlay api={props.api} onClose={() => props.state.closeHelp()} />
          </Show>
          <Show when={!props.state.helpOpen()}>
            <Switch>
              <Match when={props.state.tab() === "board"}>
                <LeaderboardPanel api={props.api} snapshot={props.snapshot} state={props.state} />
              </Match>
              <Match when={props.state.tab() === "speed"}>
                <LatencyPanel api={props.api} snapshot={props.snapshot} state={props.state} />
              </Match>
              <Match when={props.state.tab() === "vault"}>
                <TokenVaultPanel api={props.api} snapshot={props.snapshot} state={props.state} />
              </Match>
              <Match when={props.state.tab() === "limits"}>
                <LimitsPanel api={props.api} snapshot={props.snapshot} state={props.state} />
              </Match>
              <Match when={props.state.tab() === "feed"}>
                <FeedPanel api={props.api} snapshot={props.snapshot} state={props.state} />
              </Match>
              <Match when={props.state.tab() === "agents"}>
                <AgentsPanel api={props.api} snapshot={props.snapshot} state={props.state} />
              </Match>
            </Switch>
          </Show>
        </box>

        <Show when={props.selectedDrawer && props.wideEnough ? props.selectedDrawer : undefined}>
          {(drawer) => (
            <box
              flexDirection="column"
              width={Math.min(50, Math.floor(props.dims.width * 0.35))}
              flexShrink={0}
              borderLeft
              borderColor={theme().border}
              overflow="hidden"
            >
              <DetailDrawer
                api={props.api}
                model={drawer().model}
                snapshot={props.snapshot}
                onClose={() => props.state.closeDrawer()}
              />
            </box>
          )}
        </Show>
      </box>

      <Show when={props.selectedDrawer && !props.wideEnough ? props.selectedDrawer : undefined}>
        {(drawer) => (
          <box
            position="absolute"
            width={props.dims.width}
            height={props.dims.height}
            backgroundColor={theme().background}
          >
            <DetailDrawer
              api={props.api}
              model={drawer().model}
              snapshot={props.snapshot}
              onClose={() => props.state.closeDrawer()}
            />
          </box>
        )}
      </Show>

      <box
        flexDirection="row"
        width="100%"
        paddingLeft={2}
        paddingRight={2}
        paddingBottom={1}
        gap={2}
        flexShrink={0}
      >
        {["1-6 tabs", "j/k nav", "⏎ detail", "/ search", "s sort", "p pause", "? help", "q exit"].map((item) => {
          const [key, label] = item.split(" ")
          return (
            <text fg={theme().textMuted}>
              <b>{key}</b> {label}
            </text>
          )
        })}
      </box>
    </box>
  )
}
