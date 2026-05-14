/**
 * Shell LEFT panel — Jnoccio agents pane (Phase 6B of TUIbomb).
 *
 * Registers into the `shell_left_active_pane` slot and wins only when
 * `active_pane === "jnoccio"`. Reuses the existing AgentsPanel + FeedPanel
 * components from `feature-plugins/jnoccio/` — the pane is essentially a
 * thin wrapper that gates on boot status, then composes the two panels
 * inside a scrollbox so the wider dashboard widgets degrade cleanly inside
 * the 28–44 col LEFT panel.
 */
import { createMemo, Show, Switch, Match } from "solid-js"
import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { useJnoccioBootStatus } from "../../context/jnoccio-boot"
import { useJnoccioSnapshot } from "../../context/jnoccio-ws"
import { AgentsPanel } from "../jnoccio/panel-agents"
import { FeedPanel } from "../jnoccio/panel-feed"
import { createDashboardState } from "../jnoccio/state"

const id = "internal:shell-pane-jnoccio"
const DEFAULT_PANE_WIDTH = 24

function PaneJnoccio(props: { api: TuiPluginApi; contentWidth: number }) {
  const theme = () => props.api.theme.current
  const bootStatus = useJnoccioBootStatus()
  const snapshot = useJnoccioSnapshot()
  // Local UI state for the embedded panels. The agents/feed panels read
  // `selectedIndex`, `phaseFilter`, `searchQuery`, `paused` — keep them at
  // their defaults; the shell pane is read-only (no j/k navigation here).
  const state = createDashboardState()

  const isReady = createMemo(() => bootStatus() === "ready")
  const isBooting = createMemo(() => bootStatus() === "checking" || bootStatus() === "starting")
  const activeCount = createMemo(() => snapshot.active_agents.length)
  // Cap feed events so the embedded view stays compact in the narrow panel.
  const feedSnapshot = createMemo(() => ({
    ...snapshot,
    recent_events: snapshot.recent_events.slice(0, 10),
  }))
  const paneWidth = createMemo(() => Math.max(16, props.contentWidth || DEFAULT_PANE_WIDTH))
  const divider = createMemo(() => "─".repeat(paneWidth()))

  return (
    <Switch>
      <Match when={isBooting()}>
        <box flexDirection="row" gap={1}>
          <spinner frames={["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]} interval={80} color={theme().textMuted} />
          <text fg={theme().textMuted}>Jnoccio booting…</text>
        </box>
      </Match>
      <Match when={!isReady()}>
        <text fg={theme().textMuted}>Jnoccio not installed · run `jnoccio init`</text>
      </Match>
      <Match when={true}>
        <box flexDirection="column" width="100%" height="100%">
          <text fg={theme().text}>
            <b>Jnoccio agents · {activeCount()} active</b>
          </text>
          <box flexShrink={0} paddingTop={1} paddingBottom={1}>
            <text fg={theme().borderSubtle}>{divider()}</text>
          </box>
          <scrollbox
            viewportOptions={{ paddingRight: 0 }}
            verticalScrollbarOptions={{ visible: false }}
            flexGrow={1}
          >
            <AgentsPanel api={props.api} snapshot={snapshot} state={state} />
            <box flexShrink={0} paddingTop={1} paddingBottom={1}>
              <text fg={theme().borderSubtle}>{divider()}</text>
            </box>
            <FeedPanel api={props.api} snapshot={feedSnapshot()} state={state} />
          </scrollbox>
        </box>
      </Match>
    </Switch>
  )
}

const tui: TuiPlugin = async (api) => {
  api.slots.register({
    order: 91,
    slots: {
      shell_left_active_pane(_ctx, props) {
        if (props.active_pane !== "jnoccio") return null
        return <PaneJnoccio api={api} contentWidth={props.left_content_width ?? DEFAULT_PANE_WIDTH} />
      },
    },
  })
}

const plugin: TuiPluginModule & { id: string } = {
  id,
  tui,
}

export default plugin
