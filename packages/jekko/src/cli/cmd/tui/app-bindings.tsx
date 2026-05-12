import { useKeyboard, useRenderer } from "@opentui/solid"
import * as Clipboard from "@tui/util/clipboard"
import * as Selection from "@tui/util/selection"
import { createEffect } from "solid-js"
import { Flag } from "@jekko-ai/core/flag/flag"
import { useConnected } from "@tui/component/use-connected"
import { useRoute } from "@tui/context/route"
import { useTheme } from "@tui/context/theme"
import { registerTuiCommands } from "./app-commands"
import { registerTuiEvents } from "./app-events"

type Input = {
  tuiConfig: any
  route: ReturnType<typeof useRoute>
  dialog: any
  local: any
  kv: any
  command: any
  event: any
  sdk: any
  toast: any
  renderer: ReturnType<typeof useRenderer>
  sync: any
  exit: any
  mode: () => string
  setMode: (mode: string) => void
  locked: () => boolean
  lock: () => void
  unlock: () => void
  terminalTitleEnabled: () => boolean
  setTerminalTitleEnabled: (next: boolean | ((prev: boolean) => boolean)) => void
  pasteSummaryEnabled: () => boolean
  setPasteSummaryEnabled: (next: boolean | ((prev: boolean) => boolean)) => void
  onSnapshot?: () => Promise<string[]>
}

export function setupAppBindings(input: Input) {
  const connected = useConnected()
  const themeState = useTheme()

  useKeyboard((evt) => {
    if (!Flag.JEKKO_EXPERIMENTAL_DISABLE_COPY_ON_SELECT) return
    const sel = input.renderer.getSelection()
    if (!sel) return

    if (evt.ctrl && evt.name === "c") {
      if (!Selection.copy(input.renderer, input.toast)) {
        input.renderer.clearSelection()
        return
      }

      evt.preventDefault()
      evt.stopPropagation()
      return
    }

    if (evt.name === "escape") {
      input.renderer.clearSelection()
      evt.preventDefault()
      evt.stopPropagation()
      return
    }

    const focus = input.renderer.currentFocusedRenderable
    if (focus?.hasSelection() && sel.selectedRenderables.includes(focus)) {
      return
    }

    input.renderer.clearSelection()
  })

  input.renderer.console.onCopySelection = async (text: string) => {
    if (!text || text.length === 0) return

    await Clipboard.copy(text)
      .then(() => input.toast.show({ message: "Copied to clipboard", variant: "info" }))
      .catch(input.toast.error)

    input.renderer.clearSelection()
  }

  createEffect(() => {
    if (!input.terminalTitleEnabled() || Flag.JEKKO_DISABLE_TERMINAL_TITLE) return

    if (input.route.data.type === "home") {
      input.renderer.setTerminalTitle("Jekko")
      return
    }

    if (input.route.data.type === "session") {
      const session = input.sync.session.get(input.route.data.sessionID)
      if (!session || session.title === "New Session") {
        input.renderer.setTerminalTitle("Jekko")
        return
      }

      const title = session.title.length > 40 ? session.title.slice(0, 37) + "..." : session.title
      input.renderer.setTerminalTitle(`OC | ${title}`)
      return
    }

    if (input.route.data.type === "plugin") {
      input.renderer.setTerminalTitle(`OC | ${input.route.data.id}`)
    }
  })

  registerTuiCommands({
    command: input.command,
    route: input.route,
    local: input.local,
    dialog: input.dialog,
    kv: input.kv,
    sync: input.sync,
    sdk: input.sdk,
    renderer: input.renderer,
    toast: input.toast,
    theme: themeState,
    exit: input.exit,
    connected,
    tuiConfig: input.tuiConfig,
    terminalTitleEnabled: input.terminalTitleEnabled,
    setTerminalTitleEnabled: input.setTerminalTitleEnabled,
    pasteSummaryEnabled: input.pasteSummaryEnabled,
    setPasteSummaryEnabled: input.setPasteSummaryEnabled,
    onSnapshot: input.onSnapshot,
  })

  registerTuiEvents({
    command: input.command,
    event: input.event,
    route: input.route,
    toast: input.toast,
    dialog: input.dialog,
    kv: input.kv,
    sdk: input.sdk,
    exit: input.exit,
  })
}
