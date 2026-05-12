/**
 * Jnoccio TUI main dashboard layout.
 *
 * Composes the KPI strip, tab bar, active panel, and optional
 * detail drawer / help overlay into a full-screen dashboard.
 * All keyboard input is handled here and dispatched to state actions.
 */
import { createEffect, createMemo } from "solid-js"
import { useKeyboard, useTerminalDimensions } from "@opentui/solid"
import type { TuiPluginApi } from "@jekko-ai/plugin/tui"
import { useJnoccioSnapshot, useJnoccioLastHeartbeat } from "../../context/jnoccio-ws"
import { useJnoccioBootStatus } from "../../context/jnoccio-boot"
import type { DashboardModel } from "../../context/jnoccio-types"
import { createDashboardState } from "./state"
import type { TabId } from "./state"
import { JnoccioDashboardView } from "./dashboard-view"

type DrawerView =
  | { kind: "closed" }
  | { kind: "selected"; modelId: string; model: DashboardModel }
  | { kind: "missing"; modelId: string }

export function JnoccioDashboard(props: {
  api: TuiPluginApi
  onExit: () => void
}) {
  const dims = useTerminalDimensions()
  const snapshot = useJnoccioSnapshot()
  const lastHeartbeat = useJnoccioLastHeartbeat()
  const bootStatus = useJnoccioBootStatus()
  const state = createDashboardState()

  // Connection status derived from boot status + WS
  const connection = createMemo(() => {
    const bs = bootStatus()
    if (bs === "ready") return "live"
    if (bs === "checking" || bs === "starting") return "connecting"
    if (bs === "failed") return "error"
    return "loading"
  })

  const drawerView = createMemo<DrawerView>(() => {
    const current = state.drawer()
    if (current.kind === "closed") return current
    const model = snapshot.models.find((m) => m.id === current.modelId)
    return model ? { kind: "selected", modelId: current.modelId, model } : { kind: "missing", modelId: current.modelId }
  })

  createEffect(() => {
    const current = drawerView()
    if (current.kind === "missing") state.closeDrawer()
  })

  const selectedDrawer = createMemo(() => {
    const current = drawerView()
    return current.kind === "selected" ? current : undefined
  })

  // Item count for current tab (for j/k bounds)
  const itemCount = createMemo(() => {
    const tab = state.tab()
    if (tab === "board") return snapshot.models.filter((m) => m.call_count > 0).length
    if (tab === "speed") return snapshot.models.filter((m) => m.avg_latency_ms !== null).length
    if (tab === "vault") return snapshot.models.filter((m) => m.total_tokens > 0).length
    if (tab === "feed") return Math.min(snapshot.recent_events.length, 100)
    if (tab === "agents") return (snapshot.active_agents ?? []).length
    return 0
  })

  // Phase list for cycling
  const phases = createMemo(() =>
    Array.from(new Set(snapshot.recent_events.map((e) => e.phase))).sort(),
  )

  // Keyboard handler
  useKeyboard((evt) => {
    // Help overlay intercepts everything
    if (state.helpOpen()) {
      if (evt.name === "?" || evt.name === "escape") {
        state.closeHelp()
        evt.preventDefault()
        evt.stopPropagation()
      }
      return
    }

    // Detail drawer open
    if (selectedDrawer()) {
      if (evt.name === "escape" || evt.name === "q") {
        state.closeDrawer()
        evt.preventDefault()
        evt.stopPropagation()
      }
      return
    }

    // Search mode
    if (state.searchActive()) {
      if (evt.name === "escape") {
        state.cancelSearch()
        evt.preventDefault()
        evt.stopPropagation()
        return
      }
      if (evt.name === "return") {
        state.cancelSearch()
        evt.preventDefault()
        evt.stopPropagation()
        return
      }
      if (evt.name === "backspace") {
        state.setSearch(state.searchQuery().slice(0, -1))
        evt.preventDefault()
        evt.stopPropagation()
        return
      }
      // Printable character
      if (evt.sequence && evt.sequence.length === 1 && !evt.ctrl && !evt.meta) {
        state.setSearch(state.searchQuery() + evt.sequence)
        evt.preventDefault()
        evt.stopPropagation()
        return
      }
      return
    }

    // Global shortcuts
    switch (evt.name) {
      case "escape":
      case "q":
        props.onExit()
        evt.preventDefault()
        evt.stopPropagation()
        return
      case "?":
        state.toggleHelp()
        evt.preventDefault()
        evt.stopPropagation()
        return
      case "/":
        state.startSearch()
        evt.preventDefault()
        evt.stopPropagation()
        return
      case "p":
        state.togglePause()
        evt.preventDefault()
        evt.stopPropagation()
        return
      case "s":
        state.cycleSortMode()
        evt.preventDefault()
        evt.stopPropagation()
        return
      case "f":
        state.cyclePhaseFilter(phases())
        evt.preventDefault()
        evt.stopPropagation()
        return
      case "return":
        // Open detail drawer for selected model
        if (state.tab() !== "feed" && state.tab() !== "agents") {
          const models = snapshot.models.filter((m) => m.call_count > 0)
          const model = models[state.selectedIndex()]
          if (model) {
            state.openDrawer(model.id)
            evt.preventDefault()
            evt.stopPropagation()
          }
        }
        return
    }

    // Tab direct access 1-6
    if (evt.sequence && /^[1-6]$/.test(evt.sequence) && !evt.ctrl && !evt.meta) {
      state.tabByNumber(evt.sequence)
      evt.preventDefault()
      evt.stopPropagation()
      return
    }

    // Tab cycling
    if (evt.name === "tab" || evt.name === "right") {
      state.nextTab()
      evt.preventDefault()
      evt.stopPropagation()
      return
    }
    if (evt.shift && evt.name === "tab") {
      state.prevTab()
      evt.preventDefault()
      evt.stopPropagation()
      return
    }
    if (evt.name === "left") {
      state.prevTab()
      evt.preventDefault()
      evt.stopPropagation()
      return
    }

    // Navigation j/k/g/G
    if (evt.name === "j" || evt.name === "down") {
      state.moveDown(itemCount())
      evt.preventDefault()
      evt.stopPropagation()
      return
    }
    if (evt.name === "k" || evt.name === "up") {
      state.moveUp()
      evt.preventDefault()
      evt.stopPropagation()
      return
    }
    if (evt.name === "g" && !evt.shift) {
      state.moveToTop()
      evt.preventDefault()
      evt.stopPropagation()
      return
    }
    if (evt.name === "g" && evt.shift) {
      state.moveToBottom(itemCount())
      evt.preventDefault()
      evt.stopPropagation()
      return
    }
  })

  const wideEnough = createMemo(() => dims().width >= 120)

  return (
    <JnoccioDashboardView
      api={props.api}
      dims={dims()}
      snapshot={snapshot}
      state={state}
      connection={connection()}
      lastHeartbeat={lastHeartbeat}
      selectedDrawer={selectedDrawer()}
      wideEnough={wideEnough()}
    />
  )
}
