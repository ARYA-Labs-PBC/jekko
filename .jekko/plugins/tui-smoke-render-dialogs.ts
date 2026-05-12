import type { TuiPluginApi } from "@jekko-ai/plugin/tui"
import { opts, type Cfg, type Route, type State, tone } from "./tui-smoke-shared"

export const host = (api: TuiPluginApi, input: Cfg, skin: ReturnType<typeof tone>): void => {
  api.ui.dialog.setSize("medium")
  api.ui.dialog.replace(() => (
    <box paddingBottom={1} paddingLeft={2} paddingRight={2} gap={1} flexDirection="column">
      <text fg={skin.text}>
        <b>{input.label} host overlay</b>
      </text>
      <text fg={skin.muted}>Using api.ui.dialog stack with built-in backdrop</text>
      <text fg={skin.muted}>esc closes · depth {api.ui.dialog.depth}</text>
      <box flexDirection="row" gap={1}>
        <box
          onMouseUp={() => {
            api.ui.dialog.clear()
          }}
          backgroundColor={skin.accent}
          paddingLeft={1}
          paddingRight={1}
        >
          <text fg={skin.selected}>close</text>
        </box>
      </box>
    </box>
  ))
}

export const warn = (api: TuiPluginApi, route: Route, value: State): void => {
  const DialogAlert = api.ui.DialogAlert
  api.ui.dialog.setSize("medium")
  api.ui.dialog.replace(() => (
    <DialogAlert
      title="Smoke alert"
      message="Testing built-in alert dialog"
      onConfirm={() => api.route.navigate(route.screen, { ...value, source: "alert" })}
    />
  ))
}

export const check = (api: TuiPluginApi, route: Route, value: State): void => {
  const DialogConfirm = api.ui.DialogConfirm
  api.ui.dialog.setSize("medium")
  api.ui.dialog.replace(() => (
    <DialogConfirm
      title="Smoke confirm"
      message="Apply +1 to counter?"
      onConfirm={() => api.route.navigate(route.screen, { ...value, count: value.count + 1, source: "confirm" })}
      onCancel={() => api.route.navigate(route.screen, { ...value, source: "confirm-cancel" })}
    />
  ))
}

export const entry = (api: TuiPluginApi, route: Route, value: State): void => {
  const DialogPrompt = api.ui.DialogPrompt
  api.ui.dialog.setSize("medium")
  api.ui.dialog.replace(() => (
    <DialogPrompt
      title="Smoke prompt"
      value={value.note}
      onConfirm={(note) => {
        api.ui.dialog.clear()
        api.route.navigate(route.screen, { ...value, note, source: "prompt" })
      }}
      onCancel={() => {
        api.ui.dialog.clear()
        api.route.navigate(route.screen, value)
      }}
    />
  ))
}

export const picker = (api: TuiPluginApi, route: Route, value: State): void => {
  const DialogSelect = api.ui.DialogSelect
  api.ui.dialog.setSize("medium")
  api.ui.dialog.replace(() => (
    <DialogSelect
      title="Smoke select"
      options={opts}
      current={value.tab}
      onSelect={(item) => {
        api.ui.dialog.clear()
        api.route.navigate(route.screen, {
          ...value,
          tab: typeof item.value === "number" ? item.value : value.tab,
          selected: item.title,
          source: "select",
        })
      }}
    />
  ))
}
