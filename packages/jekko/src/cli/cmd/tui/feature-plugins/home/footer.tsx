import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { createMemo, Match, Show, Switch } from "solid-js"
import { Global } from "@jekko-ai/core/global"
import { useZyalMetrics } from "@tui/context/zyal-flash"
import { useJnoccioBootStatus, type JnoccioBootStatus } from "@tui/context/jnoccio-boot"
import { RGBA } from "@opentui/core"

const id = "internal:home-footer"

const HOT_PINK = RGBA.fromHex("#FF00B8")

function Directory(props: { api: TuiPluginApi }) {
  const theme = () => props.api.theme.current
  const dir = createMemo(() => {
    const dir = props.api.state.path.directory || process.cwd()
    const out = dir.replace(Global.Path.home, "~")
    const branch = props.api.state.vcs?.branch
    if (branch) return out + ":" + branch
    return out
  })

  return <text fg={theme().textMuted}>{dir()}</text>
}

function Mcp(props: { api: TuiPluginApi }) {
  const theme = () => props.api.theme.current
  const list = createMemo(() => props.api.state.mcp())
  const has = createMemo(() => list().length > 0)
  const err = createMemo(() => list().some((item) => item.status === "failed"))
  const count = createMemo(() => list().filter((item) => item.status === "connected").length)

  return (
    <Show when={has()}>
      <box gap={1} flexDirection="row" flexShrink={0}>
        <text fg={theme().text}>
          <Switch>
            <Match when={err()}>
              <span style={{ fg: theme().error }}>⊙ </span>
            </Match>
            <Match when={true}>
              <span style={{ fg: count() > 0 ? theme().success : theme().textMuted }}>⊙ </span>
            </Match>
          </Switch>
          {count()} MCP
        </text>
        <text fg={theme().textMuted}>/status</text>
      </box>
    </Show>
  )
}

function Jnoccio(props: { api: TuiPluginApi }) {
  const theme = () => props.api.theme.current
  const metrics = useZyalMetrics()
  const bootStatus = useJnoccioBootStatus()

  const connected = () => metrics().jnoccioConnected
  const status = (): JnoccioBootStatus => {
    // If the WS metrics say connected, that's authoritative
    if (connected()) return "ready"
    return bootStatus()
  }

  const dotColor = () => {
    const s = status()
    if (s === "ready") return theme().success
    if (s === "checking" || s === "starting") return HOT_PINK
    return theme().error
  }

  const label = () => {
    const s = status()
    if (s === "checking") return "checking…"
    if (s === "starting") return "starting…"
    if (s === "ready") return "Jnoccio"
    if (s === "failed") return "Jnoccio ✗"
    if (s === "unavailable") return "Jnoccio"
    return "Jnoccio"
  }

  const isBooting = () => {
    const s = status()
    return s === "checking" || s === "starting"
  }

  return (
    <box flexDirection="row" gap={1} flexShrink={0}>
      <Switch>
        <Match when={isBooting()}>
          <spinner
            frames={["◐", "◓", "◑", "◒"]}
            interval={200}
            color={HOT_PINK}
          />
        </Match>
        <Match when={true}>
          <spinner
            frames={["●", "○"]}
            interval={800}
            color={dotColor()}
          />
        </Match>
      </Switch>
      <text fg={isBooting() ? HOT_PINK : theme().text}>{label()}</text>
    </box>
  )
}

function Version(props: { api: TuiPluginApi }) {
  const theme = () => props.api.theme.current

  return (
    <box flexShrink={0}>
      <text fg={theme().textMuted}>{props.api.app.version}</text>
    </box>
  )
}

function View(props: { api: TuiPluginApi }) {
  return (
    <box
      width="100%"
      paddingTop={1}
      paddingBottom={1}
      paddingLeft={2}
      paddingRight={2}
      flexDirection="row"
      flexShrink={0}
      gap={2}
    >
      <Directory api={props.api} />
      <Jnoccio api={props.api} />
      <Mcp api={props.api} />
      <box flexGrow={1} />
      <Version api={props.api} />
    </box>
  )
}

const tui: TuiPlugin = async (api) => {
  api.slots.register({
    order: 100,
    slots: {
      home_footer() {
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
