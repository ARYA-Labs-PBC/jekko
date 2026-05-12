import type { ScrollBoxRenderable } from "@opentui/core"
import { createEffect, createMemo, createSignal, on, onCleanup } from "solid-js"
import { useCommandDialog } from "@tui/component/dialog-command"
import { useDialog } from "../../ui/dialog"
import { useExit } from "../../context/exit"
import { useKeybind } from "@tui/context/keybind"
import { useKeyboard, useRenderer, useTerminalDimensions } from "@opentui/solid"
import type { PromptRef } from "@tui/component/prompt"
import { useLocal } from "@tui/context/local"
import { Locale } from "@/util/locale"
import { SessionRetry } from "@/session/retry"
import { DialogGoUpsell } from "../../component/dialog-go-upsell"
import { useRoute, useRouteData } from "@tui/context/route"
import { useProject } from "@tui/context/project"
import { useSync } from "@tui/context/sync"
import { useEvent } from "@tui/context/event"
import { useTheme } from "@tui/context/theme"
import { setZyalFlashSource, textHasZyalSentinel } from "@tui/context/zyal-flash"
import { GO_UPSELL_DONT_SHOW, GO_UPSELL_LAST_SEEN_AT, GO_UPSELL_WINDOW, toBottom, emptyPromptParts, scrollToMessage } from "./session-helpers"
import { UI } from "@/cli/ui"
import { registerSessionCommands } from "./session-commands"
import { context } from "./context"
import { useTuiConfig } from "../../context/tui-config"
import { usePromptRef } from "../../context/prompt"
import { useKV } from "../../context/kv.tsx"
import * as Model from "../../util/model"
import { getScrollAcceleration } from "../../util/scroll"
import { useToast } from "../../ui/toast"
import { errorMessage } from "@/util/error"
import { useSDK } from "@tui/context/sdk"
import { useEditorContext } from "@tui/context/editor"
import { useSessionDaemonPolling } from "./daemon-poll"

export function createSessionBodyState() {
  const route = useRouteData("session")
  const { navigate } = useRoute()
  const sync = useSync()
  const event = useEvent()
  const project = useProject()
  const tuiConfig = useTuiConfig()
  const kv = useKV()
  const { theme, setOverlay } = useTheme()
  const promptRef = usePromptRef()
  const session = createMemo(() => sync.session.get(route.sessionID))
  const children = createMemo(() => {
    const parentID = session()?.parentID ?? session()?.id
    return sync.data.session
      .filter((x) => x.parentID === parentID || x.id === parentID)
      .toSorted((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
  })
  const messages = createMemo(() => sync.data.message[route.sessionID] ?? [])
  const permissions = createMemo(() => {
    if (session()?.parentID) return []
    return children().flatMap((x) => sync.data.permission[x.id] ?? [])
  })
  const questions = createMemo(() => {
    if (session()?.parentID) return []
    return children().flatMap((x) => sync.data.question[x.id] ?? [])
  })
  const visible = createMemo(() => !session()?.parentID && permissions().length === 0 && questions().length === 0)
  const disabled = createMemo(() => permissions().length > 0 || questions().length > 0)

  const pending = createMemo(() => {
    return messages().findLast((x) => x.role === "assistant" && !x.time.completed)?.id
  })

  const lastAssistant = createMemo(() => {
    return messages().findLast((x) => x.role === "assistant")
  })

  // ZYAL gold flash: detect ZYAL sentinels in the latest assistant message
  // and any active daemon run, then toggle the gold theme overlay.
  const zyalInAssistant = createMemo(() => {
    const last = lastAssistant()
    if (!last) return false
    const parts = sync.data.part[last.id] ?? []
    for (const part of parts) {
      if (part.type === "text" && textHasZyalSentinel(part.text)) return true
    }
    return false
  })
  createEffect(() => {
    setZyalFlashSource("session:assistant", zyalInAssistant())
  })
  // session:daemon flash source is set directly inside the polling loop below
  // so the metrics panel activates in the same tick as the gold overlay.
  onCleanup(() => {
    setZyalFlashSource("session:assistant", false)
    setZyalFlashSource("session:daemon", false)
    setZyalFlashSource("prompt:submitted", false)
  })

  const dimensions = useTerminalDimensions()
  const [sidebar, setSidebar] = kv.signal<"auto" | "hide">("sidebar", "auto")
  const [sidebarOpen, setSidebarOpen] = createSignal(false)
  const [conceal, setConceal] = createSignal(true)
  const [showThinking, setShowThinking] = kv.signal("thinking_visibility", true)
  const [timestamps, setTimestamps] = kv.signal<"hide" | "show">("timestamps", "hide")
  const [showDetails, setShowDetails] = kv.signal("tool_details_visibility", true)
  const [showAssistantMetadata, _setShowAssistantMetadata] = kv.signal("assistant_metadata_visibility", true)
  const [showScrollbar, setShowScrollbar] = kv.signal("scrollbar_visible", false)
  const [diffWrapMode] = kv.signal<"word" | "none">("diff_wrap_mode", "word")
  const [_animationsEnabled, _setAnimationsEnabled] = kv.signal("animations_enabled", true)
  const [showGenericToolOutput, setShowGenericToolOutput] = kv.signal("generic_tool_output_visibility", false)

  const wide = createMemo(() => dimensions().width > 120)
  const sidebarVisible = createMemo(() => {
    if (session()?.parentID) return false
    if (sidebarOpen()) return true
    if (sidebar() === "auto" && wide()) return true
    return false
  })
  const showTimestamps = createMemo(() => timestamps() === "show")
  const contentWidth = createMemo(() => dimensions().width - (sidebarVisible() ? 42 : 0) - 4)
  const providers = createMemo(() => Model.index(sync.data.provider))

  const scrollAcceleration = createMemo(() => getScrollAcceleration(tuiConfig))
  const toast = useToast()
  const sdk = useSDK()
  const editor = useEditorContext()
  const [daemonRun, setDaemonRun] = createSignal<any>()
  const scrollRef = { current: undefined as ScrollBoxRenderable | undefined }
  let scroll: ScrollBoxRenderable

  const setScroll = (next: ScrollBoxRenderable | undefined) => {
    scroll = next
    scrollRef.current = next
  }

  createEffect(() => {
    const sessionID = route.sessionID
    void (async () => {
      const previousWorkspace = project.workspace.current()
      const result = await sdk.client.session.get({ sessionID }, { throwOnError: true })
      if (!result.data) {
        toast.show({
          message: `Session not found: ${sessionID}`,
          variant: "error",
          duration: 5000,
        })
        navigate({ type: "home" })
        return
      }

      if (result.data.workspaceID !== previousWorkspace) {
        project.workspace.set(result.data.workspaceID)

        // Sync all the data for this workspace. Note that this
        // workspace may not exist anymore which is why this is not
        // fatal. If it doesn't we still want to show the session
        // (which will be non-interactive)
        try {
          await sync.bootstrap({ fatal: false })
        } catch {}
      }
      editor.reconnect(result.data.directory)
      await sync.session.sync(sessionID)
      if (route.sessionID === sessionID && scrollRef.current) scrollRef.current.scrollBy(100_000)
    })().catch((error) => {
      if (route.sessionID !== sessionID) return
      toast.show({
        message: errorMessage(error),
        variant: "error",
        duration: 5000,
      })
      navigate({ type: "home" })
    })
  })

  useSessionDaemonPolling({
    sessionID: () => route.sessionID,
    sdk,
    toast,
    setOverlay,
    setDaemonRun,
    daemonRun,
  })

  let lastSwitch: string | undefined = undefined
  const local = useLocal()
  event.on("message.part.updated", (evt: any) => {
    const part = evt.properties.part
    if (part.type !== "tool") return
    if (part.sessionID !== route.sessionID) return
    if (part.state.status !== "completed") return
    if (part.id === lastSwitch) return

    if (part.tool === "plan_exit") {
      local.agent.set("build")
      lastSwitch = part.id
    } else if (part.tool === "plan_enter") {
      local.agent.set("plan")
      lastSwitch = part.id
    }
  })

  const exit = useExit()

  createEffect(() => {
    const title = Locale.truncate(session()?.title ?? "", 50)
    const pad = (text: string) => text.padEnd(10, " ")
    const weak = (text: string) => UI.Style.TEXT_DIM + pad(text) + UI.Style.TEXT_NORMAL
    const logo = UI.logo("  ").split(/\\r?\\n/)
    return exit.message.set([
      `${logo[0] ?? ""}`,
      `${logo[1] ?? ""}`,
      `${logo[2] ?? ""}`,
      `${logo[3] ?? ""}`,
      ``,
      `  ${weak("Session")}${UI.Style.TEXT_NORMAL_BOLD}${title}${UI.Style.TEXT_NORMAL}`,
      `  ${weak("Continue")}${UI.Style.TEXT_NORMAL_BOLD}jekko -s ${session()?.id}${UI.Style.TEXT_NORMAL}`,
      ``,
    ].join("\\n"))
  })

  const keybind = useKeybind()
  const renderer = useRenderer()
  const dialog = useDialog()
  const command = useCommandDialog()
  let prompt: PromptRef | undefined
  const bind = (r: PromptRef | undefined) => {
    prompt = r
    promptRef.set(r)
  }

  useKeyboard((evt: any) => {
    if (!session()?.parentID) return
    if (keybind.match("app_exit", evt)) {
      void exit()
    }
  })

  registerSessionCommands(command, { route, sdk, sync, session, messages, prompt, scroll, toast, sidebarVisible, setSidebar, setSidebarOpen, conceal, setConceal, showTimestamps, setTimestamps, showThinking, setShowThinking, showDetails, setShowDetails, setShowScrollbar, showGenericToolOutput, setShowGenericToolOutput, toBottom, emptyPromptParts, navigate, showAssistantMetadata, renderer, scrollToMessage })

  event.on("session.status", (evt: any) => {
    if (evt.properties.sessionID !== route.sessionID) return
    if (evt.properties.status.type !== "retry") return
    if (evt.properties.status.message !== SessionRetry.GO_UPSELL_MESSAGE) return
    if (dialog.stack.length > 0) return
    const seen = kv.get(GO_UPSELL_LAST_SEEN_AT)
    if (typeof seen === "number" && Date.now() - seen < GO_UPSELL_WINDOW) return
    if (kv.get(GO_UPSELL_DONT_SHOW)) return
    void DialogGoUpsell.show(dialog).then((dontShowAgain: any) => { if (dontShowAgain) kv.set(GO_UPSELL_DONT_SHOW, true); kv.set(GO_UPSELL_LAST_SEEN_AT, Date.now()) })
  })

  return {
    context,
    route,
    session,
    messages,
    permissions,
    questions,
    visible,
    disabled,
    pending,
    lastAssistant,
    contentWidth,
    showScrollbar,
    theme,
    scrollAcceleration,
    daemonRun,
    sidebarVisible,
    wide,
    keybind,
    prompt,
    setScroll,
    toBottom,
    renderer,
    conceal,
    showThinking,
    showTimestamps,
    showDetails,
    showGenericToolOutput,
    diffWrapMode,
    providers,
    sync,
    tuiConfig,
    bind,
  } as const
}
