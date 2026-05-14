import { useKeyboard, useRenderer } from "@opentui/solid"
import * as Clipboard from "@tui/util/clipboard"
import * as Selection from "@tui/util/selection"
import { createEffect } from "solid-js"
import { Flag } from "@jekko-ai/core/flag/flag"
import { useConnected } from "@tui/component/use-connected"
import { useDialog } from "@tui/ui/dialog"
import { useKeybind } from "@tui/context/keybind"
import { useRoute } from "@tui/context/route"
import { useTheme } from "@tui/context/theme"
import { DialogHelp } from "./ui/dialog-help"
import { registerTuiCommands } from "./app-commands"
import { registerTuiEvents } from "./app-events"

// Phase 7 brand-defining bindings. Order is informational only — the
// dispatcher iterates this array per keystroke and dispatches the FIRST
// match. Global handlers below short-circuit; everything else fans out
// via `keybind.emit()` so component-local subscribers (registered through
// `useKeybind().on(name, handler)`) can react.
const PHASE7_BINDINGS = [
  "engage",
  "shell.tab.cycle",
  "shell.tab.cycleBack",
  "shell.tab.set",
  "shell.left.toggle",
  "theme.mode.toggle",
  "feed.scroll.pageUp",
  "feed.scroll.pageDown",
  "feed.scroll.top",
  "feed.scroll.bottom",
  "session.new",
  "session.resume",
  "help.show",
] as const

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
  const keybind = useKeybind()
  const dialog = useDialog()

  // ───────────────────────────────────────────────────────────────────
  // Phase 7 keybind dispatcher
  //
  // Runs after the prompt/dialog stack listeners — those call
  // `evt.preventDefault()` whenever they consume a key — so we short-
  // circuit on `evt.defaultPrevented`. We also skip when any dialog is
  // open: dialogs own escape/enter/etc., and intercepting them globally
  // would race with the dialog's own keyboard handler.
  //
  // For globally-handled bindings we perform the action inline and call
  // `preventDefault()`. For component-scoped bindings we emit via
  // `keybind.emit(name, evt)` and only `preventDefault()` if a
  // subscriber was registered. This means binding a key has no cost
  // until a Phase 4/5/6 component opts in via `useKeybind().on(name)`.
  // ───────────────────────────────────────────────────────────────────
  // Detect if the currently focused renderable is a text input. We
  // identify input-like renderables by name — InputRenderable and
  // TextareaRenderable both extend EditBufferRenderable in OpenTUI and
  // are the only renderables that consume printable characters into a
  // buffer. We use `constructor.name` to avoid pulling in the concrete
  // classes (cheap and reload-safe).
  const isTextInputFocused = () => {
    const f: any = input.renderer.currentFocusedRenderable
    if (!f) return false
    const ctorName = f.constructor?.name ?? ""
    return (
      ctorName === "InputRenderable" ||
      ctorName === "TextareaRenderable" ||
      ctorName === "EditBufferRenderable"
    )
  }

  // Single-character bindings (e.g. `?`, `y`, `r`, `g`, `1`/`2`/`3`)
  // would otherwise steal printable keys from a focused text input.
  // These names are gated when an input is focused.
  const TEXT_INPUT_SENSITIVE = new Set<string>([
    "help.show",
    "feed.scroll.top",
    "feed.scroll.bottom",
    "shell.tab.set",
    "engage",
  ])

  useKeyboard((evt) => {
    if (evt.defaultPrevented) return
    if (input.dialog.stack.length > 0) return

    // Skip the dispatcher while the leader key is held — the keybind
    // context owns leader-mode focus juggling and we don't want our
    // single-key bindings (e.g. `g`, `y`, `r`) to steal the chord.
    if (keybind.leader) return

    const textFocused = isTextInputFocused()

    // First match wins. The order in PHASE7_BINDINGS is informational;
    // chord uniqueness in the schema is what actually decides routing.
    for (const name of PHASE7_BINDINGS) {
      if (!keybind.match(name, evt)) continue
      if (textFocused && TEXT_INPUT_SENSITIVE.has(name)) continue

      // Global handlers — performed inline. The remainder fan out to
      // component-local subscribers via `keybind.emit`.
      if (name === "help.show") {
        evt.preventDefault()
        dialog.replace(() => <DialogHelp />)
        return
      }

      if (name === "theme.mode.toggle") {
        evt.preventDefault()
        const next = themeState.mode() === "dark" ? "light" : "dark"
        input.setMode(next)
        return
      }

      if (name === "session.new") {
        evt.preventDefault()
        // Mirror the existing `session.new` command from
        // app-commands-session.tsx: drop to the shell chat and clear any
        // open dialog. The prompt component remounts there and starts fresh.
        input.route.navigate({ type: "shell" })
        input.dialog.clear()
        return
      }

      if (name === "session.resume") {
        evt.preventDefault()
        // Most recently updated root (parentless) session, mirroring
        // `--continue` behavior in app-view.tsx.
        const match = input.sync.data.session
          .toSorted((a: any, b: any) => b.time.updated - a.time.updated)
          .find((s: any) => s.parentID === undefined)?.id
        if (match) {
          input.route.navigate({ type: "session", sessionID: match })
        } else {
          input.toast.show({
            variant: "info",
            message: "No previous session to resume",
            duration: 3000,
          })
        }
        return
      }

      // Component-scoped binding → emit. We only preventDefault when a
      // subscriber actually consumed the event; otherwise let the key
      // fall through to whatever default behavior owns it (e.g. number
      // keys typing into a focused input — though the prompt handler
      // would have already returned at the defaultPrevented check).
      const consumed = keybind.emit(name, evt)
      if (consumed) {
        evt.preventDefault()
        return
      }
    }
  })

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

    if (input.route.data.type === "home" || input.route.data.type === "shell") {
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
      input.renderer.setTerminalTitle(`Jekko | ${title}`)
      return
    }

    if (input.route.data.type === "plugin") {
      input.renderer.setTerminalTitle(`Jekko | ${input.route.data.id}`)
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
