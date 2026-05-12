/**
 * Jnoccio Fusion dashboard plugin.
 *
 * Registers:
 *  - A plugin route "jnoccio" rendering the full dashboard
 *  - A Ctrl+J global keybind to toggle the dashboard
 *  - A home_footer slot showing the ^J shortcut hint
 *
 * IMPORTANT: The shortcut, route, and footer hint are ONLY active
 * when the Jnoccio server is reachable (bootStatus === "ready").
 * When the server is off, nothing is shown.
 */
import { createMemo, Show } from "solid-js"
import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { useJnoccioBootStatus, type JnoccioBootStatus } from "../../context/jnoccio-boot"
import { useZyalMetrics } from "../../context/zyal-flash"
import { RGBA } from "@opentui/core"
import { JnoccioDashboard } from "./dashboard"

const id = "internal:jnoccio-dashboard"
const GOLD = RGBA.fromHex("#F5A623")

// ── Footer Slot ────────────────────────────────────────────────────────

function JnoccioFooterHint(props: { api: TuiPluginApi }) {
  const theme = () => props.api.theme.current
  const bootStatus = useJnoccioBootStatus()
  const metrics = useZyalMetrics()

  // Authoritative: WS says connected, or boot says ready
  const isReady = createMemo(() => metrics().jnoccioConnected || bootStatus() === "ready")

  return (
    <Show when={isReady()}>
      <box flexDirection="row" gap={1} flexShrink={0}>
        <text fg={theme().textMuted}>
          <span style={{ fg: GOLD }}><b>^J</b></span> Jnoccio
        </text>
      </box>
    </Show>
  )
}

// ── Plugin ─────────────────────────────────────────────────────────────

const tui: TuiPlugin = async (api) => {
  // Register route
  api.route.register([
    {
      name: "jnoccio",
      render: () => (
        <JnoccioDashboard
          api={api}
          onExit={() => api.route.navigateBack()}
        />
      ),
    },
  ])

  // Register Ctrl+J command (only activates when server is ready)
  api.command.register(() => {
    // Check if Jnoccio is available
    const boot = useJnoccioBootStatus()
    const metrics = useZyalMetrics()
    const isReady = metrics().jnoccioConnected || boot() === "ready"
    if (!isReady) return []

    return [
      {
        title: "Jnoccio Dashboard",
        value: "jnoccio.open",
        description: "Open the Jnoccio Fusion metrics dashboard",
        category: "Jnoccio",
        keybind: "ctrl+j",
        onSelect: () => {
          const current = api.route.current
          if (current.name === "jnoccio") {
            api.route.navigateBack()
          } else {
            api.route.navigate("jnoccio")
          }
        },
      },
    ]
  })

  // Register home_footer slot with the ^J hint
  api.slots.register({
    order: 90, // Just before the existing home footer
    slots: {
      home_footer() {
        return <JnoccioFooterHint api={api} />
      },
    },
  })
}

const plugin: TuiPluginModule & { id: string } = {
  id,
  tui,
}

export default plugin
