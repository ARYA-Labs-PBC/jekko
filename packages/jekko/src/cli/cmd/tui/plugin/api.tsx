import type { useCommandDialog } from "@tui/component/dialog-command"
import type { useEvent } from "@tui/context/event"
import type { useKeybind } from "@tui/context/keybind"
import type { useRoute } from "@tui/context/route"
import type { useSDK } from "@tui/context/sdk"
import type { useSync } from "@tui/context/sync"
import type { useTheme } from "@tui/context/theme"
import type { useDialog } from "@tui/ui/dialog"
import type { TuiConfig } from "@/cli/cmd/tui/config/tui"
import type { useKV } from "../context/kv"
import type { useToast } from "../ui/toast"
import { appApi, createPluginKeybind, DialogAlert, DialogConfirm, DialogPrompt, DialogSelect, DialogUI, HostSlot, hasPromptSuggestions, mapOption, mapOptionCb, routeCurrent, routeNavigate, routeRegister, stateApi, type RouteMap } from "./api-helpers"

type Input = {
  command: ReturnType<typeof useCommandDialog>
  tuiConfig: TuiConfig.Info
  dialog: ReturnType<typeof useDialog>
  keybind: ReturnType<typeof useKeybind>
  kv: ReturnType<typeof useKV>
  route: ReturnType<typeof useRoute>
  routes: RouteMap
  bump: () => void
  event: ReturnType<typeof useEvent>
  sdk: ReturnType<typeof useSDK>
  sync: ReturnType<typeof useSync>
  theme: ReturnType<typeof useTheme>
  toast: ReturnType<typeof useToast>
  renderer: TuiPluginApi["renderer"]
}

export function createTuiApi(input: Input): TuiPluginApi {
  const lifecycle: TuiPluginApi["lifecycle"] = {
    signal: new AbortController().signal,
    onDispose() {
      return () => {}
    },
  }

  return {
    app: appApi(),
    command: {
      register(cb) {
        return input.command.register(() => cb())
      },
      trigger(value) {
        input.command.trigger(value)
      },
      show() {
        input.command.show()
      },
    },
    route: {
      register(list) {
        return routeRegister(input.routes, list, input.bump)
      },
      navigate(name, params) {
        routeNavigate(input.route, name, params)
      },
      navigateBack() {
        input.route.navigateBack()
      },
      get current() {
        return routeCurrent(input.route)
      },
    },
    ui: {
      Dialog(props) {
        return (
          <DialogUI size={props.size} onClose={props.onClose}>
            {props.children}
          </DialogUI>
        )
      },
      DialogAlert(props) {
        return <DialogAlert {...props} />
      },
      DialogConfirm(props) {
        return <DialogConfirm {...props} />
      },
      DialogPrompt(props) {
        return <DialogPrompt {...props} description={props.description} />
      },
      DialogSelect(props) {
        return (
          <DialogSelect
            title={props.title}
            default_value={props.default_value}
            options={props.options.map(mapOption)}
            flat={props.flat}
            onMove={mapOptionCb(props.onMove)}
            onFilter={props.onFilter}
            onSelect={mapOptionCb(props.onSelect)}
            skipFilter={props.skipFilter}
            current={props.current}
          />
        )
      },
      Slot<Name extends string>(props: TuiSlotProps<Name>) {
        return <HostSlot {...props} />
      },
      Prompt(props) {
        const promptSuggestions = hasPromptSuggestions(props) ? props.promptSuggestions : undefined
        return (
          <Prompt
            sessionID={props.sessionID}
            workspaceID={props.workspaceID}
            visible={props.visible}
            disabled={props.disabled}
            onSubmit={props.onSubmit}
            ref={props.ref}
            hint={props.hint}
            right={props.right}
            showSuggestion={props.showSuggestion}
            promptSuggestions={promptSuggestions}
          />
        )
      },
      toast(inputToast) {
        input.toast.show({
          title: inputToast.title,
          message: inputToast.message,
          variant: inputToast.variant ?? "info",
          duration: inputToast.duration,
        })
      },
      dialog: {
        replace(render, onClose) {
          input.dialog.replace(render, onClose)
        },
        clear() {
          input.dialog.clear()
        },
        setSize(size) {
          input.dialog.setSize(size)
        },
        get size() {
          return input.dialog.size
        },
        get depth() {
          return input.dialog.stack.length
        },
        get open() {
          return input.dialog.stack.length > 0
        },
      },
    },
    keybind: {
      match(key, evt: ParsedKey) {
        return input.keybind.match(key, evt)
      },
      print(key) {
        return input.keybind.print(key)
      },
      create(defaults, overrides) {
        return createPluginKeybind(input.keybind, defaults, overrides)
      },
    },
    get tuiConfig() {
      return input.tuiConfig
    },
    kv: {
      get(key, alternative_path) {
        return input.kv.get(key, alternative_path)
      },
      set(key, value) {
        input.kv.set(key, value)
      },
      get ready() {
        return input.kv.ready
      },
    },
    state: stateApi(input.sync),
    get client() {
      return input.sdk.client
    },
    event: input.event,
    renderer: input.renderer,
    slots: {
      register() {
        throw new Error("slots.register is only available in plugin context")
      },
    },
    plugins: {
      list() {
        return []
      },
      async activate() {
        return false
      },
      async deactivate() {
        return false
      },
      async add() {
        return false
      },
      async install() {
        return {
          ok: false,
          message: "plugins.install is only available in plugin context",
        }
      },
    },
    lifecycle,
    theme: {
      get current() {
        return input.theme.theme
      },
      get selected() {
        return input.theme.selected
      },
      has(name) {
        return input.theme.has(name)
      },
      set(name) {
        return input.theme.set(name)
      },
      async install(_jsonPath) {
        throw new Error("theme.install is only available in plugin context")
      },
      mode() {
        return input.theme.mode()
      },
      get ready() {
        return input.theme.ready
      },
    },
  }
}
