/** @jsxImportSource @opentui/solid */
import { useKeyboard, useTerminalDimensions, type JSX } from "@opentui/solid"
import type { TuiPluginApi, TuiPluginMeta } from "@jekko-ai/plugin/tui"
import { current, parse, tabs, tone, type Cfg, type Keys, type Route, type State } from "./tui-smoke-shared"
import { check, entry, host, picker, warn } from "./tui-smoke-render-dialogs"
import { ModalView, ScreenView } from "./tui-smoke-render-view"

export const Screen = (props: {
  api: TuiPluginApi
  input: Cfg
  route: Route
  keys: Keys
  meta: TuiPluginMeta
  params?: Record<string, unknown>
}): JSX.Element => {
  const dim = useTerminalDimensions()
  const value = parse(props.params)
  const skin = tone(props.api)
  const set = (local: number, base?: State): void => {
    const next = base ?? current(props.api, props.route)
    props.api.route.navigate(props.route.screen, { ...next, local: Math.max(0, local), source: "local" })
  }
  const push = (base?: State): void => {
    const next = base ?? current(props.api, props.route)
    set(next.local + 1, next)
  }
  const open = (): void => {
    const next = current(props.api, props.route)
    if (next.local > 0) return
    set(1, next)
  }
  const pop = (base?: State): void => {
    const next = base ?? current(props.api, props.route)
    const local = Math.max(0, next.local - 1)
    set(local, next)
  }
  const show = (): void => {
    setTimeout(() => {
      open()
    }, 0)
  }

  useKeyboard((evt) => {
    if (props.api.route.current.name !== props.route.screen) return
    const next = current(props.api, props.route)
    if (props.api.ui.dialog.open) {
      if (props.keys.match("dialog_close", evt)) {
        evt.preventDefault()
        evt.stopPropagation()
        props.api.ui.dialog.clear()
        return
      }
      return
    }

    if (next.local > 0) {
      if (evt.name === "escape" || props.keys.match("local_close", evt)) {
        evt.preventDefault()
        evt.stopPropagation()
        pop(next)
        return
      }

      if (props.keys.match("local_push", evt)) {
        evt.preventDefault()
        evt.stopPropagation()
        push(next)
        return
      }
      return
    }

    if (props.keys.match("home", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      props.api.route.navigate("home")
      return
    }

    if (props.keys.match("left", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      props.api.route.navigate(props.route.screen, { ...next, tab: (next.tab - 1 + tabs.length) % tabs.length })
      return
    }

    if (props.keys.match("right", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      props.api.route.navigate(props.route.screen, { ...next, tab: (next.tab + 1) % tabs.length })
      return
    }

    if (props.keys.match("up", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      props.api.route.navigate(props.route.screen, { ...next, count: next.count + 1 })
      return
    }

    if (props.keys.match("down", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      props.api.route.navigate(props.route.screen, { ...next, count: next.count - 1 })
      return
    }

    if (props.keys.match("modal", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      props.api.route.navigate(props.route.modal, next)
      return
    }

    if (props.keys.match("local", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      open()
      return
    }

    if (props.keys.match("host", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      host(props.api, props.input, skin)
      return
    }

    if (props.keys.match("alert", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      warn(props.api, props.route, next)
      return
    }

    if (props.keys.match("confirm", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      check(props.api, props.route, next)
      return
    }

    if (props.keys.match("prompt", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      entry(props.api, props.route, next)
      return
    }

    if (props.keys.match("select", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      picker(props.api, props.route, next)
    }
  })

  return (
    <ScreenView
      api={props.api}
      input={props.input}
      route={props.route}
      keys={props.keys}
      meta={props.meta}
      value={value}
      skin={skin}
      dim={dim}
      open={open}
      push={push}
      pop={pop}
      show={show}
      host={() => host(props.api, props.input, skin)}
      warn={() => warn(props.api, props.route, value)}
      check={() => check(props.api, props.route, value)}
      entry={() => entry(props.api, props.route, value)}
      picker={() => picker(props.api, props.route, value)}
    />
  )
}

export const Modal = (props: {
  api: TuiPluginApi
  input: Cfg
  route: Route
  keys: Keys
  params?: Record<string, unknown>
}) => {
  const value = parse(props.params)
  const skin = tone(props.api)

  useKeyboard((evt) => {
    if (props.api.route.current.name !== props.route.modal) return

    if (props.keys.match("modal_accept", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      props.api.route.navigate(props.route.screen, { ...value, source: "modal" })
      return
    }

    if (props.keys.match("modal_close", evt)) {
      evt.preventDefault()
      evt.stopPropagation()
      props.api.route.navigate("home")
    }
  })

  return (
    <ModalView
      api={props.api}
      input={props.input}
      route={props.route}
      keys={props.keys}
      value={value}
      skin={skin}
      onClose={() => props.api.route.navigate("home")}
      onOpenScreen={() => props.api.route.navigate(props.route.screen, { ...value, source: "modal" })}
    />
  )
}
