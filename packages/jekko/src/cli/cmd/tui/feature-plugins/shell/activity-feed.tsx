/**
 * Shell CENTER region — activity feed (Phase 6A of TUIbomb).
 *
 * Registers into the `shell_center_feed` slot (single_winner) and mounts
 * the existing session-body pipeline inside the shell route's CENTER
 * region. No new message-render code lives here — the diff renderer,
 * tool calls, reasoning blocks, permission/question prompts, daemon
 * banner, and Prompt input all come for free from `SessionBody`.
 *
 * Active session: most-recently-updated root session from sync data.
 * When no session exists yet, render a centered hint instead of the
 * full session pipeline.
 */
import { createMemo, Show } from "solid-js"
import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { useSync } from "@tui/context/sync"
import { SessionBody } from "@tui/routes/session"

const id = "internal:shell-activity-feed"

function ActivityFeedView(props: { api: TuiPluginApi }) {
  const theme = () => props.api.theme.current
  const sync = useSync()

  // Most-recently-updated root (parentless) session. Mirrors the existing
  // resume logic in `app-view.tsx` / `app-bindings.tsx` so the shell route
  // shows the same session the user would resume with ctrl+R / --continue.
  const activeSessionID = createMemo<string | undefined>(() => {
    const sessions = sync.data.session ?? []
    return sessions
      .toSorted((a, b) => b.time.updated - a.time.updated)
      .find((s) => s.parentID === undefined)?.id
  })

  return (
    <Show
      when={activeSessionID()}
      fallback={
        <box flexGrow={1} alignItems="center" justifyContent="center">
          <text fg={theme().textMuted}>
            Press Enter on home to start. No active session.
          </text>
        </box>
      }
    >
      {(sessionID) => <SessionBody sessionID={sessionID()} />}
    </Show>
  )
}

const tui: TuiPlugin = async (api) => {
  api.slots.register({
    order: 50,
    slots: {
      shell_center_feed() {
        return <ActivityFeedView api={api} />
      },
    },
  })
}

const plugin: TuiPluginModule & { id: string } = {
  id,
  tui,
}

export default plugin
