import type { ScrollBoxRenderable } from "@opentui/core"
import { createEffect, createMemo, createSignal, on, onCleanup } from "solid-js"
import { useCommandDialog } from "@tui/component/dialog-command"
import { useDialog } from "../../ui/dialog"
import { useExit } from "../../context/exit"
import { useKeybind } from "@tui/context/keybind"
import { useKeyboard, useRenderer, useTerminalDimensions } from "@opentui/solid"
import type { PromptRef } from "@tui/component/prompt"
import { useLocal } from "@tui/context/local"
import { useRoute, useRouteData } from "@tui/context/route"
import { useProject } from "@tui/context/project"
import { useSync } from "@tui/context/sync"
import { useEvent } from "@tui/context/event"
import { useTheme } from "@tui/context/theme"
import { setZyalFlashSource, textHasZyalSentinel } from "@tui/context/zyal-flash"
import {
  bindSessionCommands,
  bindSessionExitMessage,
  bindSessionLoadEffect,
  bindSessionKeyboardExit,
  bindSessionPartSwitchHandler,
  bindSessionScrollKeybinds,
  bindSessionUpsellHandler,
} from "./session-body-core-support"
import { context } from "./context"
import { useTuiConfig } from "../../context/tui-config"
import { usePromptRef } from "../../context/prompt"
import { useKV } from "../../context/kv.tsx"
import * as Model from "../../util/model"
import { getScrollAcceleration } from "../../util/scroll"
import { useToast } from "../../ui/toast"
import { useSDK } from "@tui/context/sdk"
import { useEditorContext } from "@tui/context/editor"
import { useSessionDaemonPolling } from "./daemon-poll"
import { scrollToMessage, toBottom } from "./session-helpers"

export type SessionBodyStateOptions = {
  /**
   * Optional override for the active session ID. When supplied, the body
   * tracks this ID instead of `useRouteData("session").sessionID`. Used by
   * the Phase 6 shell activity-feed plugin to mount the session pipeline
   * inside the shell route (which has no session in its route data).
   */
  sessionID?: () => string | undefined
}

export function createSessionBodyState(options: SessionBodyStateOptions = {}) {
  const routeData = useRouteData("session")
  const { navigate } = useRoute()
  // Build a synthetic route view so callers that pass an explicit sessionID
  // (e.g. the shell route) get a stable {sessionID} surface and existing
  // session-route callers see the live route store untouched.
  const route = options.sessionID
    ? ({
        get sessionID() {
          return options.sessionID!() ?? ""
        },
      } as { sessionID: string })
    : routeData
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
  const contentWidth = createMemo(() => dimensions().width - 4)
  const providers = createMemo(() => Model.index(sync.data.provider))

  const scrollAcceleration = createMemo(() => getScrollAcceleration(tuiConfig))
  const toast = useToast()
  const sdk = useSDK()
  const editor = useEditorContext()
  const [daemonRun, setDaemonRun] = createSignal<any>()
  const scrollRef = { current: undefined as ScrollBoxRenderable | undefined }

  const setScroll = (next: ScrollBoxRenderable | undefined) => {
    scrollRef.current = next
  }
  const scrollProxy = {
    get height() {
      return scrollRef.current?.height ?? 0
    },
    get scrollHeight() {
      return scrollRef.current?.scrollHeight ?? 0
    },
    get y() {
      return scrollRef.current?.y ?? 0
    },
    scrollBy(amount: number) {
      scrollRef.current?.scrollBy(amount)
    },
    scrollTo(offset: number) {
      scrollRef.current?.scrollTo(offset)
    },
    getChildren() {
      return scrollRef.current?.getChildren() ?? []
    },
  } as unknown as ScrollBoxRenderable

  bindSessionLoadEffect({
    routeSessionID: () => route.sessionID,
    project,
    sdk,
    toast,
    navigate,
    sync,
    editor,
    scrollRef,
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
  bindSessionPartSwitchHandler({
    event,
    routeSessionID: () => route.sessionID,
    local,
  })

  const exit = useExit()
  bindSessionExitMessage({ exit, session })

  const keybind = useKeybind()
  const renderer = useRenderer()
  const dialog = useDialog()
  const command = useCommandDialog()
  let prompt: PromptRef | undefined
  const bind = (r: PromptRef | undefined) => {
    prompt = r
    promptRef.set(r)
  }

  bindSessionKeyboardExit(useKeyboard, keybind, exit, () => !!session()?.parentID)
  bindSessionScrollKeybinds(keybind, scrollRef)
  bindSessionCommands(command, {
    route,
    sdk,
    sync,
    session,
    messages,
    prompt,
    scroll: scrollProxy,
    toast,
    sidebarVisible,
    setSidebar,
    setSidebarOpen,
    conceal,
    setConceal,
    showTimestamps,
    setTimestamps,
    showThinking,
    setShowThinking,
    showDetails,
    setShowDetails,
    setShowScrollbar,
    showGenericToolOutput,
    setShowGenericToolOutput,
    navigate,
    showAssistantMetadata,
    renderer,
    scrollToMessage,
  })
  bindSessionUpsellHandler({
    event,
    routeSessionID: () => route.sessionID,
    kv,
    dialog,
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
