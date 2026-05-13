/** @jsxImportSource @opentui/solid */
import { RGBA } from "@opentui/core"
import type { JSX } from "@opentui/solid"
import type { TuiPluginApi, TuiPluginMeta } from "@jekko-ai/plugin/tui"
import { tabs, type Cfg, type Keys, type Route, type State, tone } from "./tui-smoke-shared"

const Btn = (props: { txt: string; run: () => void; skin: ReturnType<typeof tone>; on?: boolean }): JSX.Element => {
  return (
    <box
      onMouseUp={() => {
        props.run()
      }}
      backgroundColor={props.on ? props.skin.accent : props.skin.border}
      paddingLeft={1}
      paddingRight={1}
    >
      <text fg={props.on ? props.skin.selected : props.skin.text}>{props.txt}</text>
    </box>
  )
}

export const ScreenView = (props: {
  api: TuiPluginApi
  input: Cfg
  route: Route
  keys: Keys
  meta: TuiPluginMeta
  value: State
  skin: ReturnType<typeof tone>
  dim: () => { width: number; height: number }
  open: () => void
  push: () => void
  pop: () => void
  show: () => void
  host: () => void
  warn: () => void
  check: () => void
  entry: () => void
  picker: () => void
}): JSX.Element => {
  const dim = props.dim()
  const value = props.value
  const skin = props.skin

  return (
    <box width={dim.width} height={dim.height} backgroundColor={skin.panel} position="relative">
      <box
        flexDirection="column"
        width="100%"
        height="100%"
        paddingTop={1}
        paddingBottom={1}
        paddingLeft={2}
        paddingRight={2}
      >
        <box flexDirection="row" justifyContent="space-between" paddingBottom={1}>
          <text fg={skin.text}>
            <b>{props.input.label} screen</b>
            <span style={{ fg: skin.muted }}> plugin route</span>
          </text>
          <text fg={skin.muted}>{props.keys.print("home")} home</text>
        </box>

        <box flexDirection="row" gap={1} paddingBottom={1}>
          {tabs.map((item, i) => {
            const on = value.tab === i
            return (
              <Btn
                txt={item}
                run={() => props.api.route.navigate(props.route.screen, { ...value, tab: i })}
                skin={skin}
                on={on}
              />
            )
          })}
        </box>

        <box
          border
          borderColor={skin.border}
          paddingTop={1}
          paddingBottom={1}
          paddingLeft={2}
          paddingRight={2}
          flexGrow={1}
        >
          {value.tab === 0 ? (
            <box flexDirection="column" gap={1}>
              <text fg={skin.text}>Route: {props.route.screen}</text>
              <text fg={skin.muted}>plugin state: {props.meta.state}</text>
              <text fg={skin.muted}>
                first: {props.meta.state === "first" ? "yes" : "no"} · updated:{" "}
                {props.meta.state === "updated" ? "yes" : "no"} · loads: {props.meta.load_count}
              </text>
              <text fg={skin.muted}>plugin source: {props.meta.source}</text>
              {value.source ? <text fg={skin.muted}>source: {value.source}</text> : null}
              {value.note ? <text fg={skin.muted}>note: {value.note}</text> : null}
              {value.selected ? <text fg={skin.muted}>selected: {value.selected}</text> : null}
              <text fg={skin.muted}>local stack depth: {value.local}</text>
              <text fg={skin.muted}>host stack open: {props.api.ui.dialog.open ? "yes" : "no"}</text>
            </box>
          ) : null}

          {value.tab === 1 ? (
            <box flexDirection="column" gap={1}>
              <text fg={skin.text}>Counter: {value.count}</text>
              <text fg={skin.muted}>
                {props.keys.print("up")} / {props.keys.print("down")} change value
              </text>
            </box>
          ) : null}

          {value.tab === 2 ? (
            <box flexDirection="column" gap={1}>
              <text fg={skin.muted}>
                {props.keys.print("modal")} modal | {props.keys.print("alert")} alert | {props.keys.print("confirm")}{" "}
                confirm | {props.keys.print("prompt")} prompt | {props.keys.print("select")} select
              </text>
              <text fg={skin.muted}>
                {props.keys.print("local")} local stack | {props.keys.print("host")} host stack
              </text>
              <text fg={skin.muted}>
                local open: {props.keys.print("local_push")} push nested · esc or {props.keys.print("local_close")}{" "}
                close
              </text>
              <text fg={skin.muted}>{props.keys.print("home")} returns home</text>
            </box>
          ) : null}
        </box>

        <box flexDirection="row" gap={1} paddingTop={1}>
          <Btn txt="go home" run={() => props.api.route.navigate("home")} skin={skin} />
          <Btn txt="modal" run={() => props.api.route.navigate(props.route.modal, value)} skin={skin} on />
          <Btn txt="local overlay" run={props.show} skin={skin} />
          <Btn txt="host overlay" run={props.host} skin={skin} />
          <Btn txt="alert" run={props.warn} skin={skin} />
          <Btn txt="confirm" run={props.check} skin={skin} />
          <Btn txt="prompt" run={props.entry} skin={skin} />
          <Btn txt="select" run={props.picker} skin={skin} />
        </box>
      </box>

      <box
        visible={value.local > 0}
        width={dim.width}
        height={dim.height}
        alignItems="center"
        position="absolute"
        zIndex={3000}
        paddingTop={dim.height / 4}
        left={0}
        top={0}
        backgroundColor={RGBA.fromInts(0, 0, 0, 160)}
        onMouseUp={() => {
          props.pop()
        }}
      >
        <box
          onMouseUp={(evt) => {
            evt.stopPropagation()
          }}
          width={60}
          maxWidth={dim.width - 2}
          backgroundColor={skin.panel}
          border
          borderColor={skin.border}
          paddingTop={1}
          paddingBottom={1}
          paddingLeft={2}
          paddingRight={2}
          gap={1}
          flexDirection="column"
        >
          <text fg={skin.text}>
            <b>{props.input.label} local overlay</b>
          </text>
          <text fg={skin.muted}>Plugin-owned stack depth: {value.local}</text>
          <text fg={skin.muted}>
            {props.keys.print("local_push")} push nested · {props.keys.print("local_close")} pop/close
          </text>
          <box flexDirection="row" gap={1}>
            <Btn txt="push" run={props.push} skin={skin} on />
            <Btn txt="pop" run={props.pop} skin={skin} />
          </box>
        </box>
      </box>
    </box>
  )
}

export const ModalView = (props: {
  api: TuiPluginApi
  input: Cfg
  route: Route
  keys: Keys
  value: State
  skin: ReturnType<typeof tone>
  onClose: () => void
  onOpenScreen: () => void
}): JSX.Element => {
  const Dialog = props.api.ui.Dialog
  const skin = props.skin

  return (
    <box width="100%" height="100%" backgroundColor={skin.panel}>
      <Dialog onClose={props.onClose}>
        <box paddingBottom={1} paddingLeft={2} paddingRight={2} gap={1} flexDirection="column">
          <text fg={skin.text}>
            <b>{props.input.label} modal</b>
          </text>
          <text fg={skin.muted}>{props.keys.print("modal")} modal command</text>
          <text fg={skin.muted}>{props.keys.print("screen")} screen command</text>
          <text fg={skin.muted}>
            {props.keys.print("modal_accept")} opens screen · {props.keys.print("modal_close")} closes
          </text>
          <box flexDirection="row" gap={1}>
            <Btn txt="open screen" run={props.onOpenScreen} skin={skin} on />
            <Btn txt="cancel" run={props.onClose} skin={skin} />
          </box>
        </box>
      </Dialog>
    </box>
  )
}
