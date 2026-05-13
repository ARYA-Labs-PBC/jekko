/**
 * Shell LEFT panel — tab bar (Phase 6A of TUIbomb).
 *
 * Registers into the `shell_left_tabs` slot (single_winner). Renders a single
 * row of tab labels with an active-tab underline beneath. Tabs map to
 * `useLocal().shellPane` values:
 *
 *   1 / Jnoccio    →  "jnoccio"
 *   2 / Repo-Intel →  "capability"
 *   3 / History    →  "history"
 *
 * Active selection comes from `props.active_pane` (the host passes
 * `local.shellPane.get()`) so the tab bar stays in sync with kv-backed
 * state across restarts. The plugin also subscribes to Phase 7 keybind
 * events via `useKeybind().on(...)` so number keys, Tab, and Shift+Tab
 * dispatched from `app-bindings.tsx` switch panes without touching the
 * host.
 *
 * Narrow widths (< 30 cols) collapse the labels to single letters
 * (J / R / H) so the tab bar still fits inside the 28-col left panel.
 */
import { createMemo, For, Show } from "solid-js"
import { useTerminalDimensions } from "@opentui/solid"
import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { useKeybind, type KeybindEvent } from "@tui/context/keybind"
import { useLocal, type ShellPane } from "@tui/context/local"

const id = "internal:shell-tabs"

type TabDef = {
  key: ShellPane
  label: string
  short: string
}

// Order matches `shell.tab.set` 1/2/3 binding. Don't reorder without also
// updating the keybind dispatcher logic below.
const TABS: readonly TabDef[] = [
  { key: "jnoccio", label: "Jnoccio", short: "J" },
  { key: "capability", label: "Repo-Intel", short: "R" },
  { key: "history", label: "History", short: "H" },
] as const

function TabsView(props: { api: TuiPluginApi; activePane: string }) {
  const theme = () => props.api.theme.current
  const local = useLocal()
  const keybind = useKeybind()
  const dimensions = useTerminalDimensions()

  const narrow = createMemo(() => dimensions().width < 30)

  const activeKey = createMemo<ShellPane>(() => {
    const pane = props.activePane as ShellPane
    return TABS.some((t) => t.key === pane) ? pane : TABS[0].key
  })

  function setPane(next: ShellPane) {
    local.shellPane.set(next)
  }

  function cycle(direction: 1 | -1) {
    const current = activeKey()
    const index = TABS.findIndex((t) => t.key === current)
    if (index < 0) return setPane(TABS[0].key)
    let nextIndex = index + direction
    if (nextIndex < 0) nextIndex = TABS.length - 1
    if (nextIndex >= TABS.length) nextIndex = 0
    setPane(TABS[nextIndex].key)
  }

  // Phase 7 keybind subscribers. `app-bindings.tsx` dispatches these via
  // `keybind.emit(name, evt)`. The dispatcher inspects whether a subscriber
  // consumed the event and short-circuits the default behavior accordingly.
  keybind.on("shell.tab.set", (payload: KeybindEvent) => {
    const name = payload.event.name
    if (name === "1") setPane("jnoccio")
    else if (name === "2") setPane("capability")
    else if (name === "3") setPane("history")
  })
  keybind.on("shell.tab.cycle", () => cycle(1))
  keybind.on("shell.tab.cycleBack", () => cycle(-1))

  return (
    <box flexDirection="column" flexShrink={0}>
      {/* Tab labels row */}
      <box flexDirection="row" flexShrink={0}>
        <For each={TABS}>
          {(tab, index) => {
            const isActive = () => tab.key === activeKey()
            const text = () => (narrow() ? tab.short : tab.label)
            return (
              <>
                <Show when={index() > 0}>
                  <text fg={theme().textMuted}>{"  "}</text>
                </Show>
                <text
                  fg={isActive() ? theme().text : theme().textMuted}
                  onMouseUp={() => setPane(tab.key)}
                >
                  {isActive() ? <b>{text()}</b> : text()}
                </text>
              </>
            )
          }}
        </For>
      </box>
      {/* Active-tab underline row */}
      <Show when={!narrow()}>
        <box flexDirection="row" flexShrink={0}>
          <For each={TABS}>
            {(tab, index) => {
              const width = tab.label.length
              const isActive = () => tab.key === activeKey()
              return (
                <>
                  <Show when={index() > 0}>
                    <text fg={theme().textMuted}>{"  "}</text>
                  </Show>
                  <text fg={isActive() ? theme().accent : theme().background}>
                    {isActive() ? "▔".repeat(width) : " ".repeat(width)}
                  </text>
                </>
              )
            }}
          </For>
        </box>
      </Show>
    </box>
  )
}

const tui: TuiPlugin = async (api) => {
  api.slots.register({
    order: 50,
    slots: {
      shell_left_tabs(_ctx, props) {
        return <TabsView api={api} activePane={props.active_pane ?? "jnoccio"} />
      },
    },
  })
}

const plugin: TuiPluginModule & { id: string } = {
  id,
  tui,
}

export default plugin
