/**
 * Shell LEFT pane — Sessions History (Phase 6D of TUIbomb).
 *
 * Renders into the `shell_left_active_pane` slot when `active_pane === "history"`.
 * The slot is single-winner, so we MUST return `null` for any other pane value
 * to keep concurrent panes (jnoccio, capability, …) from overlapping.
 *
 * Layout, top-to-bottom:
 *   ┌─────────────────────────────────────────────────────────┐
 *   │ Sessions · N total                                      │
 *   │ ─────────────────────────────────────────────────────── │
 *   │ ● Active session title                              now │
 *   │   Recent today title 1                            14:02 │
 *   │     └─ Forked from active                         13:55 │
 *   │   Recent today title 2                            09:31 │
 *   │ ─ Yesterday ─                                           │
 *   │   Older title                                       yest│
 *   │ ─ Older ─                                               │
 *   │   Ancient title                                    May 8│
 *   │ … show N more  ⏎                                        │
 *   └─────────────────────────────────────────────────────────┘
 *
 * TODO (v1 punt): Quick-jump number keys (1..9) were deferred. The keybind
 * system has no surface-scoping primitive yet, and the digits 1/2/3 collide
 * with the Phase 7 tab switcher. Once `useKeybind` grows focus scoping (or we
 * add an internal focus signal here), wire `1`..`9` to `route.navigate(...)`
 * for the nth session in the rendered list. For now, users open the full
 * picker via the trailing "… show N more  ⏎" affordance.
 */
import type { TuiPlugin, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { createMemo, For, Show } from "solid-js"
import { useTheme } from "@tui/context/theme"
import { useSync } from "@tui/context/sync"
import { useRoute } from "@tui/context/route"
import { useDialog } from "@tui/ui/dialog"
import { useKeybind } from "@tui/context/keybind"
import { DialogSessionList } from "@tui/component/dialog-session-list"

const id = "internal:shell-pane-history"

const MAX_ROWS = 8
const DEFAULT_PANE_WIDTH = 24
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]

type SessionEntry = NonNullable<ReturnType<typeof useSync>["data"]["session"]>[number]

type Group = "today" | "yesterday" | "older"

type Row = {
  session: SessionEntry
  group: Group
  /** True if this row is rendered indented under its parent fork. */
  isFork: boolean
}

function startOfDay(ms: number): number {
  return new Date(ms).setHours(0, 0, 0, 0)
}

function classifyGroup(now: number, updated: number): Group {
  const today = startOfDay(now)
  const day = startOfDay(updated)
  if (day === today) return "today"
  if (day === today - 86_400_000) return "yesterday"
  return "older"
}

function relativeTime(now: number, updated: number, group: Group): string {
  const delta = now - updated
  if (delta < 60_000) return "now"
  if (delta < 3_600_000) return `${Math.max(1, Math.floor(delta / 60_000))}m ago`
  if (group === "today") {
    const d = new Date(updated)
    const h = d.getHours().toString().padStart(2, "0")
    const m = d.getMinutes().toString().padStart(2, "0")
    return `${h}:${m}`
  }
  if (group === "yesterday") return "yest"
  const d = new Date(updated)
  return `${MONTHS[d.getMonth()]} ${d.getDate()}`
}

function truncate(text: string, max: number): string {
  if (text.length <= max) return text
  if (max <= 1) return text.slice(0, max)
  return text.slice(0, max - 1) + "…"
}

function PaneHistory(props: { contentWidth: number }) {
  const { theme } = useTheme()
  const sync = useSync()
  const route = useRoute()
  const dialog = useDialog()
  // useKeybind is grabbed here so Phase 7+ focus-scoped quick-jump can hook in
  // without re-wiring imports. Currently unused — see TODO at the top.
  void useKeybind

  const sessions = createMemo<SessionEntry[]>(() => sync.data.session ?? [])
  const totalCount = createMemo(() => sessions().length)

  const sortedByUpdated = createMemo<SessionEntry[]>(() =>
    sessions().toSorted((a, b) => b.time.updated - a.time.updated),
  )

  /** Active session id: current route's session, or fallback to most-recently-updated. */
  const activeId = createMemo<string | undefined>(() => {
    if (route.data.type === "session") return route.data.sessionID
    return sortedByUpdated()[0]?.id
  })

  const activeSession = createMemo<SessionEntry | undefined>(() => {
    const id = activeId()
    if (!id) return undefined
    return sessions().find((s) => s.id === id)
  })

  /**
   * Build rows grouped by Today/Yesterday/Older, sorted desc by updated time,
   * with fork children indented directly under their parent. Excludes the
   * active session (it has its own row up top).
   */
  const rows = createMemo<Row[]>(() => {
    const now = Date.now()
    const active = activeId()
    const all = sortedByUpdated()
    const byParent = new Map<string, SessionEntry[]>()
    const roots: SessionEntry[] = []
    for (const s of all) {
      if (s.id === active) continue
      if (s.parentID) {
        const bucket = byParent.get(s.parentID) ?? []
        bucket.push(s)
        byParent.set(s.parentID, bucket)
      } else {
        roots.push(s)
      }
    }
    const out: Row[] = []
    for (const root of roots) {
      out.push({
        session: root,
        group: classifyGroup(now, root.time.updated),
        isFork: false,
      })
      const forks = byParent.get(root.id)
      if (!forks) continue
      for (const fork of forks) {
        out.push({
          session: fork,
          group: classifyGroup(now, fork.time.updated),
          isFork: true,
        })
      }
    }
    // Forks whose parents are the active session (or orphaned) also belong here.
    for (const [parentID, forks] of byParent.entries()) {
      if (parentID === active) {
        for (const fork of forks) {
          out.push({
            session: fork,
            group: classifyGroup(now, fork.time.updated),
            isFork: true,
          })
        }
      } else if (!sessions().some((s) => s.id === parentID)) {
        // Orphaned fork — render as a root-like row.
        for (const fork of forks) {
          out.push({
            session: fork,
            group: classifyGroup(now, fork.time.updated),
            isFork: false,
          })
        }
      }
    }
    return out
  })

  const visibleRows = createMemo<Row[]>(() => rows().slice(0, MAX_ROWS))
  const overflowCount = createMemo(() => Math.max(0, rows().length - MAX_ROWS))
  const paneWidth = createMemo(() => Math.max(16, props.contentWidth || DEFAULT_PANE_WIDTH))

  const todayRows = createMemo(() => visibleRows().filter((r) => r.group === "today"))
  const yesterdayRows = createMemo(() => visibleRows().filter((r) => r.group === "yesterday"))
  const olderRows = createMemo(() => visibleRows().filter((r) => r.group === "older"))

  function openSessionList() {
    dialog.replace(() => <DialogSessionList />)
  }

  function GroupRow(props: { row: Row }) {
    const now = Date.now()
    const time = relativeTime(now, props.row.session.time.updated, props.row.group)
    const titleWidth = Math.max(4, paneWidth() - time.length - (props.row.isFork ? 9 : 5))
    return (
      <box flexDirection="row" justifyContent="space-between" flexShrink={0} paddingLeft={props.row.isFork ? 4 : 2}>
        <box flexDirection="row" gap={1} flexShrink={1} minWidth={0}>
          <Show when={props.row.isFork}>
            <text fg={theme.borderSubtle} wrapMode="none">
              └─
            </text>
          </Show>
          <text fg={theme.text} wrapMode="none">
            {truncate(props.row.session.title, titleWidth)}
          </text>
        </box>
        <text fg={theme.textMuted} wrapMode="none" flexShrink={0}>
          {time}
        </text>
      </box>
    )
  }

  return (
    <box flexDirection="column" gap={0} flexGrow={1} minHeight={0}>
      {/* 1. Title */}
      <text fg={theme.text} wrapMode="none">
        <b>Sessions</b>
        <span style={{ fg: theme.textMuted }}> · {totalCount()} total</span>
      </text>
      {/* 2. Divider — `width="100%"` lets the underlying text fill the parent. */}
      <text fg={theme.borderSubtle} wrapMode="none">
        {"─".repeat(paneWidth())}
      </text>

      {/* 3. Active session row */}
      <Show when={activeSession()}>
        {(s) => (
          <box flexDirection="row" justifyContent="space-between" flexShrink={0} paddingTop={1}>
            <box flexDirection="row" gap={1} flexShrink={1} minWidth={0}>
              <text fg={theme.accent} wrapMode="none">
                ●
              </text>
              <text fg={theme.text} wrapMode="none">
                <b>{truncate(s().title, Math.max(4, paneWidth() - 7))}</b>
              </text>
            </box>
            <text fg={theme.textMuted} wrapMode="none" flexShrink={0}>
              now
            </text>
          </box>
        )}
      </Show>

      {/* 4. Today group */}
      <Show when={todayRows().length > 0}>
        <box flexDirection="column" paddingTop={1} flexShrink={0}>
          <For each={todayRows()}>{(row) => <GroupRow row={row} />}</For>
        </box>
      </Show>

      {/* 5. Yesterday group */}
      <Show when={yesterdayRows().length > 0}>
        <box flexDirection="column" paddingTop={1} flexShrink={0}>
          <text fg={theme.textMuted} wrapMode="none">
            ─ Yesterday ─
          </text>
          <For each={yesterdayRows()}>{(row) => <GroupRow row={row} />}</For>
        </box>
      </Show>

      {/* 6. Older group */}
      <Show when={olderRows().length > 0}>
        <box flexDirection="column" paddingTop={1} flexShrink={0}>
          <text fg={theme.textMuted} wrapMode="none">
            ─ Older ─
          </text>
          <For each={olderRows()}>{(row) => <GroupRow row={row} />}</For>
        </box>
      </Show>

      {/* 7. Overflow affordance — opens the full session picker dialog. */}
      <Show when={overflowCount() > 0}>
        <box flexShrink={0} paddingTop={1} onMouseDown={openSessionList}>
          <text fg={theme.textMuted} wrapMode="none">
            {truncate(`… show ${overflowCount()} more  ⏎`, paneWidth())}
          </text>
        </box>
      </Show>
    </box>
  )
}

const tui: TuiPlugin = async (api) => {
  api.slots.register({
    order: 93,
    slots: {
      shell_left_active_pane(_ctx, props) {
        if (props.active_pane !== "history") return null
        return <PaneHistory contentWidth={props.left_content_width ?? DEFAULT_PANE_WIDTH} />
      },
    },
  })
}

const plugin: TuiPluginModule & { id: string } = {
  id,
  tui,
}

export default plugin
