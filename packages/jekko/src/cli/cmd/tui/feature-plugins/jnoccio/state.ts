/**
 * Jnoccio TUI dashboard local UI state.
 *
 * Manages tab selection, sort mode, search, pause/buffer,
 * flash highlights, and detail drawer state.
 */
import { createSignal, createMemo } from "solid-js"
import type { DashboardModel, MetricEvent } from "../../context/jnoccio-types"
import { nextSortMode, type SortMode } from "./panel-model"

// ── Tab State ─────────────────────────────────────────────────────────

export type TabId = "board" | "speed" | "vault" | "limits" | "feed" | "agents"

export const TABS: { id: TabId; num: string; icon: string; label: string }[] = [
  { id: "board", num: "1", icon: "🏆", label: "Board" },
  { id: "speed", num: "2", icon: "⚡", label: "Speed" },
  { id: "vault", num: "3", icon: "💰", label: "Vault" },
  { id: "limits", num: "4", icon: "🔒", label: "Limits" },
  { id: "feed", num: "5", icon: "📡", label: "Feed" },
  { id: "agents", num: "6", icon: "⚙", label: "Agents" },
]

export type DrawerState =
  | { kind: "closed" }
  | { kind: "selected"; modelId: string }
  | { kind: "missing"; modelId: string }

// ── Pause Buffer ──────────────────────────────────────────────────────

const MAX_BUFFER = 500

// ── State Factory ─────────────────────────────────────────────────────

export function createDashboardState() {
  const [tab, setTab] = createSignal<TabId>("board")
  const [sortMode, setSortMode] = createSignal<SortMode>("latest")
  const [selectedIndex, setSelectedIndex] = createSignal(0)
  const [searchQuery, setSearchQuery] = createSignal("")
  const [searchActive, setSearchActive] = createSignal(false)
  const [phaseFilter, setPhaseFilter] = createSignal("all")
  const [paused, setPaused] = createSignal(false)
  const [helpOpen, setHelpOpen] = createSignal(false)
  const [drawer, setDrawer] = createSignal<DrawerState>({ kind: "closed" })
  const drawerModelId = createMemo(() => {
    const current = drawer()
    return current.kind === "closed" ? undefined : current.modelId
  })
  const [flashMap, setFlashMap] = createSignal<Map<string, number>>(new Map())

  // Pause buffer: events queued while paused
  let buffer: MetricEvent[] = []

  const actions = {
    // ── Tab Navigation ──────────────────────────────────────────────
    switchTab(id: TabId) {
      setTab(id)
      setSelectedIndex(0)
      setSearchActive(false)
      setSearchQuery("")
    },

    nextTab() {
      const idx = TABS.findIndex((t) => t.id === tab())
      const next = TABS[(idx + 1) % TABS.length]!
      actions.switchTab(next.id)
    },

    prevTab() {
      const idx = TABS.findIndex((t) => t.id === tab())
      const prev = TABS[(idx - 1 + TABS.length) % TABS.length]!
      actions.switchTab(prev.id)
    },

    tabByNumber(num: string) {
      const match = TABS.find((t) => t.num === num)
      if (match) actions.switchTab(match.id)
    },

    // ── Selection / Navigation ──────────────────────────────────────
    moveDown(max: number) {
      setSelectedIndex((i) => Math.min(i + 1, Math.max(0, max - 1)))
    },

    moveUp() {
      setSelectedIndex((i) => Math.max(0, i - 1))
    },

    moveToTop() {
      setSelectedIndex(0)
    },

    moveToBottom(max: number) {
      setSelectedIndex(Math.max(0, max - 1))
    },

    // ── Detail Drawer ───────────────────────────────────────────────
    openDrawer(modelId: string) {
      setDrawer({ kind: "selected", modelId })
    },

    closeDrawer() {
      setDrawer({ kind: "closed" })
    },

    markDrawerMissing(modelId: string) {
      setDrawer({ kind: "missing", modelId })
    },

    toggleDrawer(models: DashboardModel[]) {
      if (drawer().kind !== "closed") {
        setDrawer({ kind: "closed" })
      } else {
        const idx = selectedIndex()
        const model = models[idx]
        if (model) setDrawer({ kind: "selected", modelId: model.id })
      }
    },

    // ── Search ──────────────────────────────────────────────────────
    startSearch() {
      setSearchActive(true)
      setSearchQuery("")
    },

    cancelSearch() {
      setSearchActive(false)
      setSearchQuery("")
    },

    setSearch(q: string) {
      setSearchQuery(q)
      setSelectedIndex(0)
    },

    // ── Sort ────────────────────────────────────────────────────────
    cycleSortMode() {
      setSortMode((mode) => nextSortMode(mode))
    },

    // ── Phase Filter ────────────────────────────────────────────────
    cyclePhaseFilter(phases: string[]) {
      const all = ["all", ...phases]
      const idx = all.indexOf(phaseFilter())
      setPhaseFilter(all[(idx + 1) % all.length]!)
    },

    // ── Pause ───────────────────────────────────────────────────────
    togglePause() {
      if (paused()) {
        // Resume: drain buffer is handled by the caller
        setPaused(false)
        const drained = [...buffer]
        buffer = []
        return drained
      } else {
        setPaused(true)
        buffer = []
        return []
      }
    },

    bufferEvent(event: MetricEvent) {
      if (!paused()) return false
      buffer.push(event)
      if (buffer.length > MAX_BUFFER) buffer.shift()
      return true
    },

    getBufferCount() {
      return buffer.length
    },

    // ── Flash ───────────────────────────────────────────────────────
    flashModel(modelId: string, agentId?: string | null) {
      setFlashMap((prev) => {
        const next = new Map(prev)
        const key = agentId ? `${modelId}::${agentId}` : modelId
        next.set(key, Date.now())
        return next
      })
    },

    pruneFlashes() {
      const now = Date.now()
      setFlashMap((prev) => {
        const next = new Map<string, number>()
        for (const [k, v] of prev) {
          if (now - v < 3000) next.set(k, v)
        }
        return next.size !== prev.size ? next : prev
      })
    },

    // ── Help ────────────────────────────────────────────────────────
    toggleHelp() {
      setHelpOpen((v) => !v)
    },

    closeHelp() {
      setHelpOpen(false)
    },
  }

  return {
    // Reactive signals
    tab,
    sortMode,
    selectedIndex,
    searchQuery,
    searchActive,
    phaseFilter,
    paused,
    helpOpen,
    drawer,
    drawerModelId,
    flashMap,
    // Actions
    ...actions,
  }
}

export type DashboardState = ReturnType<typeof createDashboardState>
