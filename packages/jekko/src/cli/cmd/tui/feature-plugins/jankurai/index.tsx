/**
 * Jankurai audit-live dashboard plugin.
 *
 * Registers:
 *  - A plugin route "jankurai-audit-live" rendering the live panel
 *  - A Ctrl+K global keybind to open the panel
 *  - A home_footer slot showing the ^K shortcut hint (only when jankurai
 *    is installed locally — matches the jnoccio pattern of hiding shortcuts
 *    for tools the operator doesn't have).
 */
import { createMemo, Show } from "solid-js"
import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { RGBA } from "@opentui/core"
import { useJankuraiInstalled } from "../../context/jankurai-score"
import { JankuraiAuditLivePanel } from "./panel-audit-live"

const id = "internal:jankurai-audit-live"
const GOLD = RGBA.fromHex("#F5A623")

function JankuraiFooterHint(props: { api: TuiPluginApi }) {
  const theme = () => props.api.theme.current
  const installed = useJankuraiInstalled()
  const ready = createMemo(() => installed() === true)
  return (
    <Show when={ready()}>
      <box flexDirection="row" gap={1} flexShrink={0}>
        <text fg={theme().textMuted}>
          <span style={{ fg: GOLD }}>
            <b>^K</b>
          </span>{" "}
          Jankurai
        </text>
      </box>
    </Show>
  )
}

const tui: TuiPlugin = async (api) => {
  api.route.register([
    {
      name: "jankurai-audit-live",
      render: () => <JankuraiAuditLivePanel api={api} />,
    },
  ])

  api.command.register(() => {
    const installed = useJankuraiInstalled()
    if (installed() !== true) return []
    return [
      {
        title: "Jankurai Audit Live",
        value: "jankurai.audit-live.open",
        description: "Open the realtime audit dashboard (score, caps, findings, deltas, workers)",
        category: "Jankurai",
        keybind: "ctrl+k",
        onSelect: () => {
          const current = api.route.current
          if (current.name === "jankurai-audit-live") {
            api.route.navigateBack()
          } else {
            api.route.navigate("jankurai-audit-live")
          }
        },
      },
    ]
  })

  api.slots.register({
    order: 91, // Just after the Jnoccio Ctrl+J hint
    slots: {
      home_footer() {
        return <JankuraiFooterHint api={api} />
      },
    },
  })
}

const plugin: TuiPluginModule & { id: string } = {
  id,
  tui,
}

export default plugin
