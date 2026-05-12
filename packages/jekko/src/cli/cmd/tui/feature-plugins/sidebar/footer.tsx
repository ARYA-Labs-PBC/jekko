import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { createMemo, Show } from "solid-js"
import { Global } from "@jekko-ai/core/global"
import { hasConnectedProvider } from "../../component/use-connected"
import { useJnoccioBootStatus, useJnoccioModelCount, type JnoccioBootStatus } from "../../context/jnoccio-boot"
import { RGBA } from "@opentui/core"

const id = "internal:sidebar-footer"

const HOT_PINK = RGBA.fromHex("#FF00B8")

function JnoccioStatus(props: { api: TuiPluginApi }) {
  const theme = () => props.api.theme.current
  const bootStatus = useJnoccioBootStatus()
  const modelCount = useJnoccioModelCount()

  const status = (): JnoccioBootStatus => bootStatus()

  // Only show when Jnoccio has been discovered (not idle or unavailable)
  const visible = createMemo(() => {
    const s = status()
    return s !== "idle" && s !== "unavailable"
  })

  const dotColor = () => {
    const s = status()
    if (s === "ready") return theme().success
    if (s === "checking" || s === "starting") return HOT_PINK
    return theme().error
  }

  const label = createMemo(() => {
    const s = status()
    const count = modelCount()
    if (s === "checking") return "checking…"
    if (s === "starting") return "starting…"
    if (s === "ready" && count !== null) return `Jnoccio · ${count} model${count !== 1 ? "s" : ""}`
    if (s === "ready") return "Jnoccio"
    if (s === "failed") return "Jnoccio ✗"
    return "Jnoccio"
  })

   const isBooting = () => {
     const s = status()
     return s === "checking" || s === "starting"
   }

   return (
      <Show when={visible()}>
        <box flexDirection="row" gap={1} flexShrink={0}>
          {isBooting() ? (
            <spinner frames={["◐", "◓", "◑", "◒"]} interval={200} color={HOT_PINK} />
          ) : (
            <spinner frames={["●", "○"]} interval={800} color={dotColor()} />
          )}
        <text fg={isBooting() ? HOT_PINK : theme().text}>{label()}</text>
      </box>
    </Show>
  )
}

function View(props: { api: TuiPluginApi }) {
  const theme = () => props.api.theme.current
  const has = createMemo(() => props.api.state.provider.some(hasConnectedProvider))
  const done = createMemo(() => props.api.kv.get("dismissed_getting_started", false))
  const show = createMemo(() => !has() && !done())
  const path = createMemo(() => {
    const dir = props.api.state.path.directory || process.cwd()
    const out = dir.replace(Global.Path.home, "~")
    const text = props.api.state.vcs?.branch ? out + ":" + props.api.state.vcs.branch : out
    const list = text.split("/")
    return {
      parent: list.slice(0, -1).join("/"),
      name: list.at(-1) ?? "",
    }
  })

  return (
    <box gap={1}>
      <Show when={show()}>
        <box
          backgroundColor={theme().backgroundElement}
          paddingTop={1}
          paddingBottom={1}
          paddingLeft={2}
          paddingRight={2}
          flexDirection="row"
          gap={1}
        >
          <text flexShrink={0} fg={theme().text}>
            ⬖
          </text>
          <box flexGrow={1} gap={1}>
            <box flexDirection="row" justifyContent="space-between">
              <text fg={theme().text}>
                <b>Getting started</b>
              </text>
              <text fg={theme().textMuted} onMouseDown={() => props.api.kv.set("dismissed_getting_started", true)}>
                ✕
              </text>
            </box>
            <text fg={theme().textMuted}>Jekko includes free models so you can start immediately.</text>
            <text fg={theme().textMuted}>
              Connect from 75+ providers to use other models, including Claude, GPT, Gemini etc
            </text>
            <box flexDirection="row" gap={1} justifyContent="space-between">
              <text fg={theme().text}>Connect provider</text>
              <text fg={theme().textMuted}>/connect</text>
            </box>
          </box>
        </box>
      </Show>
      <text>
        <span style={{ fg: theme().textMuted }}>{path().parent}/</span>
        <span style={{ fg: theme().text }}>{path().name}</span>
      </text>
      <text fg={theme().textMuted}>
        <span style={{ fg: theme().success }}>•</span> <b>Open</b>
        <span style={{ fg: theme().text }}>
          <b>Code</b>
        </span>{" "}
        <span>{props.api.app.version}</span>
      </text>
      <JnoccioStatus api={props.api} />
    </box>
  )
}

const tui: TuiPlugin = async (api) => {
  api.slots.register({
    order: 100,
    slots: {
      sidebar_footer() {
        return <View api={api} />
      },
    },
  })
}

const plugin: TuiPluginModule & { id: string } = {
  id,
  tui,
}

export default plugin
