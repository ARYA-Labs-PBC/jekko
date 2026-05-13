/** @jsxImportSource @opentui/solid */
import type { JSX } from "@opentui/solid"
import type { TuiPluginApi, TuiSlotPlugin } from "@jekko-ai/plugin/tui"
import { check, entry, host, picker, tone, warn } from "./tui-smoke-render"
import { ink, look, ui, type Cfg, type Keys } from "./tui-smoke-shared"

const home = (api: TuiPluginApi, input: Cfg) => ({
  slots: {
    home_logo(ctx) {
      const map = ctx.theme.current
      const skin = look(map)
      const art = [
        "                                  $$\\",
        "                                  $$ |",
        " $$$$$$$\\ $$$$$$\\$$$$\\   $$$$$$\\  $$ |  $$\\  $$$$$$\\",
        "$$  _____|$$  _$$  _$$\\ $$  __$$\\ $$ | $$  |$$  __$$\\",
        "\\$$$$$$\\  $$ / $$ / $$ |$$ /  $$ |$$$$$$  / $$$$$$$$ |",
        " \\____$$\\ $$ | $$ | $$ |$$ |  $$ |$$  _$$<  $$   ____|",
        "$$$$$$$  |$$ | $$ | $$ |\\$$$$$$  |$$ | \\$$\\ \\$$$$$$$\\",
        "\\_______/ \\__| \\__| \\__| \\______/ \\__|  \\__| \\_______|",
      ]
      const fill = [
        skin.accent,
        skin.muted,
        ink(map, "info", ui.accent),
        skin.text,
        ink(map, "success", ui.accent),
        ink(map, "warning", ui.accent),
        ink(map, "secondary", ui.accent),
        ink(map, "error", ui.accent),
      ]

      return (
        <box flexDirection="column">
          {art.map((line, i) => (
            <text fg={fill[i]}>{line}</text>
          ))}
        </box>
      )
    },
    home_prompt(ctx, value) {
      const skin = look(ctx.theme.current)
      const ui = api.ui
      const Prompt = ui.Prompt
      const Slot = ui.Slot
      const hint = (
        <box flexShrink={0} flexDirection="row" gap={1}>
          <text fg={skin.muted}>
            <span style={{ fg: skin.accent }}>•</span> smoke home prompt
          </text>
        </box>
      )

      return (
        <Prompt
          workspaceID={value.workspace_id}
          hint={hint}
          right={
            <box flexDirection="row" gap={1}>
              <Slot name="home_prompt_right" workspace_id={value.workspace_id} />
              <Slot name="smoke_prompt_right" workspace_id={value.workspace_id} label={input.label} />
            </box>
          }
        />
      )
    },
    home_prompt_right(ctx, value): JSX.Element {
      const skin = look(ctx.theme.current)
      const id = typeof value.workspace_id === "string" && value.workspace_id.match(/^[a-z0-9]+$/)
        ? value.workspace_id.slice(0, 8)
        : ""
      return (
        <text fg={skin.muted}>
          <span style={{ fg: skin.accent }}>{input.label}</span> home{id ? `:${id}` : ""}
        </text>
      )
    },
    session_prompt_right(ctx, value): JSX.Element {
      const skin = look(ctx.theme.current)
      const sessionId = typeof value.session_id === "string" && value.session_id.match(/^[a-z0-9]+$/)
        ? value.session_id.slice(0, 8)
        : ""
      return (
        <text fg={skin.muted}>
          <span style={{ fg: skin.accent }}>{input.label}</span> session:{sessionId}
        </text>
      )
    },
    smoke_prompt_right(ctx, value): JSX.Element {
      const skin = look(ctx.theme.current)
      const id = typeof value.workspace_id === "string" && value.workspace_id.match(/^[a-z0-9]+$/)
        ? value.workspace_id.slice(0, 8)
        : ""
      const label = typeof value.label === "string" ? value.label : input.label
      return (
        <text fg={skin.muted}>
          <span style={{ fg: skin.accent }}>{label}</span> custom{ id ? `:${id}` : "" }
        </text>
      )
    },
    home_bottom(ctx): JSX.Element {
      const skin = look(ctx.theme.current)
      const text = "extra content in the unified home bottom slot"

      return (
        <box width="100%" maxWidth={75} alignItems="center" paddingTop={1} flexShrink={0} gap={1}>
          <box
            border
            borderColor={skin.border}
            backgroundColor={skin.panel}
            paddingTop={1}
            paddingBottom={1}
            paddingLeft={2}
            paddingRight={2}
            width="100%"
          >
            <text fg={skin.muted}>
              <span style={{ fg: skin.accent }}>{input.label}</span> {text}{
                typeof value.session_id === "string" && value.session_id.match(/^[a-z0-9]+$/)
                  ? ` session:${value.session_id.slice(0, 8)}`
                  : ""
              }
            </text>
          </box>
        </box>
      )
    },
  },
})

const block = (input: Cfg, order: number, title: string, text: string): TuiSlotPlugin => ({
  order,
  slots: {
    sidebar_content(ctx, value): JSX.Element {
      const skin = look(ctx.theme.current)

      return (
        <box
          border
          borderColor={skin.border}
          backgroundColor={skin.panel}
          paddingTop={1}
          paddingBottom={1}
          paddingLeft={2}
          paddingRight={2}
          flexDirection="column"
          gap={1}
        >
          <text fg={skin.accent}>
            <b>{title}</b>
          </text>
          <text fg={skin.text}>{text}</text>
          <text fg={skin.muted}>
            {input.label} order {order} · session {
              typeof value.session_id === "string" && value.session_id.match(/^[a-z0-9]+$/)
                ? value.session_id.slice(0, 8)
                : ""
            }
          </text>
        </box>
      )
    },
  },
})

const slot = (api: TuiPluginApi, input: Cfg): TuiSlotPlugin[] => [
  home(api, input),
  block(input, 50, "Smoke above", "renders above internal sidebar blocks"),
  block(input, 250, "Smoke between", "renders between internal sidebar blocks"),
  block(input, 650, "Smoke below", "renders below internal sidebar blocks"),
]

const reg = (api: TuiPluginApi, input: Cfg, keys: Keys): void => {
  const route = { modal: `${input.route}.modal`, screen: `${input.route}.screen` }
  api.command.register(() => [
    {
      title: "Smoke modal",
      value: "plugin.smoke.modal",
      keybind: keys.get("modal"),
      category: "Plugin",
      slash: { name: "smoke" },
      onSelect: () => {
        api.route.navigate(route.modal, { source: "command" })
      },
    },
    {
      title: "Smoke screen",
      value: "plugin.smoke.screen",
      keybind: keys.get("screen"),
      category: "Plugin",
      slash: { name: "smoke-screen" },
      onSelect: () => {
        api.route.navigate(route.screen, { source: "command", tab: 0, count: 0 })
      },
    },
    {
      title: "Smoke alert dialog",
      value: "plugin.smoke.alert",
      category: "Plugin",
      slash: { name: "smoke-alert" },
      onSelect: () => {
        warn(api, route, { tab: 0, count: 0, source: "alert", note: "", selected: "", local: 0 })
      },
    },
    {
      title: "Smoke confirm dialog",
      value: "plugin.smoke.confirm",
      category: "Plugin",
      slash: { name: "smoke-confirm" },
      onSelect: () => {
        check(api, route, { tab: 0, count: 0, source: "confirm", note: "", selected: "", local: 0 })
      },
    },
    {
      title: "Smoke prompt dialog",
      value: "plugin.smoke.prompt",
      category: "Plugin",
      slash: { name: "smoke-prompt" },
      onSelect: () => {
        entry(api, route, { tab: 0, count: 0, source: "prompt", note: "", selected: "", local: 0 })
      },
    },
    {
      title: "Smoke select dialog",
      value: "plugin.smoke.select",
      category: "Plugin",
      slash: { name: "smoke-select" },
      onSelect: () => {
        picker(api, route, { tab: 0, count: 0, source: "select", note: "", selected: "", local: 0 })
      },
    },
    {
      title: "Smoke host overlay",
      value: "plugin.smoke.host",
      category: "Plugin",
      slash: { name: "smoke-host" },
      onSelect: () => {
        host(api, input, tone(api))
      },
    },
    {
      title: "Smoke go home",
      value: "plugin.smoke.home",
      category: "Plugin",
      enabled: api.route.current.name !== "home",
      onSelect: () => {
        api.route.navigate("home")
      },
    },
    {
      title: "Smoke toast",
      value: "plugin.smoke.toast",
      category: "Plugin",
      onSelect: () => {
        api.ui.toast({
          variant: "info",
          title: "Smoke",
          message: "Plugin toast works",
          duration: 2000,
        })
      },
    },
  ])
}

export { home, block, slot, reg }
